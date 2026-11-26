use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use core_remote_proto::frame::{Frame, Kind, Reassembler};
use core_remote_proto::MIN_PROTO;
use core_serve::{TapError, TapGuard};
use serde_json::{json, Value};
use tokio::sync::{mpsc, watch};

use super::config::now_secs;
use super::ctx::{Control, Ctx};
use super::device_info::parse_caps;
use super::fit::{fit_snapshot, fits_message, limit_event, Fit, SNAPSHOT_BUDGET};
use super::limits::{Rate, MAX_COMMANDS_PER_MINUTE};
use super::liveness::{due, Liveness};
use super::link::{Secure, CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_OFF, CLOSE_PROTO};
use super::phone_ws::{next_ws, Extra};
use super::viewer::flush_due;
use crate::state::lock;

pub use super::dashboard_msg::{client_message, command, error, events_event, phone_events, snapshot_event, turn_changed, BATCH, DASHBOARD, EVENT_QUEUE};
use super::dashboard_msg::{expects_reply, parse};

struct Feed {
    rx: mpsc::Receiver<Arc<str>>,
    overflow: Arc<AtomicBool>,
    _guard: TapGuard,
}

pub struct Phone {
    pub(super) ctx: Arc<Ctx>,
    pub(super) sec: Secure,
    pub(super) device: String,
    reasm: Reassembler,
    pub(super) alive: Option<Liveness>,
    pub(super) via: &'static str,
    pub(super) pings: std::collections::VecDeque<std::time::Instant>,
    hello: bool,
    feed: Option<Feed>,
    last_seq: u64,
    rate: Rate,
    direct_tx: mpsc::Sender<(String, Arc<str>)>,
    direct_rx: mpsc::Receiver<(String, Arc<str>)>,
    push_rx: watch::Receiver<Value>,
    pub(super) caps: Vec<String>,
    pub(super) x: Extra,
}

async fn next_event(feed: &mut Option<Feed>) -> Option<Arc<str>> {
    match feed {
        Some(f) => f.rx.recv().await,
        None => std::future::pending().await,
    }
}

impl Phone {
    pub fn new(ctx: Arc<Ctx>, sec: Secure, device: String) -> Phone {
        let (direct_tx, direct_rx) = mpsc::channel(32);
        let x = Extra::new(ctx.board.subscribe(), &device);
        let push_rx = ctx.push_cfg.subscribe();
        Phone { ctx, sec, device, reasm: Reassembler::new(), alive: None, via: "lan", pings: std::collections::VecDeque::new(), hello: false, feed: None, last_seq: 0, rate: Rate::default(), direct_tx, direct_rx, push_rx, x, caps: Vec::new() }
    }

    pub fn first_hello(&mut self, msg: &[u8]) -> Option<(String, Value)> {
        let plain = self.sec.transport.open(msg).ok()?;
        let frame = Frame::decode(&plain).ok()?;
        if frame.kind != Kind::Json || frame.more {
            return None;
        }
        let (id, t, v) = command(&frame.payload).ok()?;
        (t == "hello").then_some((id, v))
    }

    pub fn watch(&mut self, every: Duration, dead: Duration) {
        self.alive = Some(Liveness::new(every, dead));
    }

    pub async fn start(mut self, id: &str, hello: &Value, rx: mpsc::Receiver<Vec<u8>>, ctl: mpsc::Receiver<Control>) {
        if self.hello(id, hello).await {
            self.run(rx, ctl).await;
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<Vec<u8>>, mut ctl: mpsc::Receiver<Control>) {
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let Some(msg) = msg else { break };
                    if let Some(a) = self.alive.as_mut() {
                        a.heard();
                    }
                    if !self.inbound(&msg).await {
                        break;
                    }
                }
                _ = due(&mut self.alive) => {
                    if !self.keepalive().await {
                        break;
                    }
                }
                c = ctl.recv() => {
                    if c == Some(Control::Probe) {
                        if !self.probe().await {
                            break;
                        }
                        continue;
                    }
                    self.control(c).await;
                    break;
                }
                text = next_event(&mut self.feed) => {
                    let Some(text) = text else { break };
                    if !self.forward(text).await {
                        break;
                    }
                }
                Some(done) = self.x.done_rx.recv() => {
                    if !self.on_done(done).await {
                        break;
                    }
                }
                Some(msg) = self.x.hub_rx.recv() => {
                    if !self.on_hub(msg).await {
                        break;
                    }
                }
                Some(ev) = self.x.v.rx.recv() => self.on_vevent(ev),
                Some(msg) = self.x.chat.rx.recv() => {
                    if !self.on_chat(msg).await {
                        break;
                    }
                }
                _ = flush_due(self.x.chat.due) => self.chat_tick(),
                _ = flush_due(self.x.v.flush_at) => {
                    if !self.flush_vevents().await {
                        break;
                    }
                }
                event = next_ws(&mut self.x.ws_rx, self.hello) => {
                    if !self.on_ws(event).await {
                        break;
                    }
                }
                Ok(()) = self.push_rx.changed(), if self.hello => {
                    let event = self.push_rx.borrow_and_update().clone();
                    if !self.sec.json(&event).await {
                        break;
                    }
                }
                Some((id, text)) = self.direct_rx.recv() => {
                    let events = parse(&text).map(|(_, e)| phone_events(e)).unwrap_or_default();
                    if !self.sec.json(&json!({ "re": id, "ok": true, "events": events })).await {
                        break;
                    }
                }
            }
        }
    }

    async fn control(&mut self, c: Option<Control>) {
        let (event, code, reason) = match c {
            Some(Control::Revoked) => (Some("device.revoked"), CLOSE_DEVICE, "revoked"),
            Some(Control::Restarting) => (Some("machine.restarting"), CLOSE_OFF, "restarting"),
            Some(Control::Held) => (None, CLOSE_OFF, "held"),
            Some(Control::Off | Control::Probe) | None => (Some("remote.off"), CLOSE_OFF, "off"),
        };
        if let Some(event) = event {
            self.sec.json(&json!({ "t": event })).await;
        }
        self.sec.close(code, reason).await;
    }

    async fn inbound(&mut self, msg: &[u8]) -> bool {
        let frame = match self.sec.transport.open(msg).map(|plain| Frame::decode(&plain)) {
            Ok(Ok(f)) => f,
            _ => {
                self.sec.close(CLOSE_HANDSHAKE, "frame").await;
                return false;
            }
        };
        let message = match self.reasm.push(frame) {
            Ok(Some(m)) => m,
            Ok(None) => return true,
            Err(_) => {
                self.sec.close(CLOSE_HANDSHAKE, "frame").await;
                return false;
            }
        };
        match message.kind {
            Kind::Json => self.command(&message.payload).await,
            Kind::Ping => self.pong().await,
            Kind::PtyIn => self.pty_input(message.stream, message.payload).await,
            Kind::PtyOut => true,
        }
    }

    async fn command(&mut self, raw: &[u8]) -> bool {
        let (id, t, v) = match command(raw) {
            Ok(c) => c,
            Err(reply) => return self.sec.json(&reply).await,
        };
        if !self.rate.take(std::time::Instant::now(), MAX_COMMANDS_PER_MINUTE) {
            return self.sec.json(&error(Value::from(id), "rate")).await;
        }
        match t.as_str() {
            "hello" => self.hello(&id, &v).await,
            _ if !self.hello => self.sec.json(&error(Value::from(id), "invalid")).await,
            "resync" => self.resync(&id).await,
            k if DASHBOARD.contains(&k) => self.tap(&id, k, &v).await,
            k => self.extra(&id, k, &v).await,
        }
    }

    async fn hello(&mut self, id: &str, v: &Value) -> bool {
        let proto = v.get("proto").and_then(Value::as_u64).unwrap_or(0);
        if proto < u64::from(MIN_PROTO) {
            let mut reply = error(Value::from(id), "proto");
            reply["min"] = Value::from(MIN_PROTO);
            self.sec.json(&reply).await;
            self.sec.close(CLOSE_PROTO, "old").await;
            return false;
        }
        if let Some(p) = v.get("platform").and_then(Value::as_str) {
            lock(&self.ctx.devices).set_platform(&self.device, p);
        }
        lock(&self.ctx.devices).set_info(&self.device, v);
        self.caps = parse_caps(v);
        self.hello = true;
        self.set_chat_cap();
        let welcome = self.ctx.welcome(id);
        if !self.sec.json(&welcome).await {
            return false;
        }
        if let Some(event) = self.ctx.renew_token(&self.device, now_secs()) {
            if !self.sec.json(&event).await {
                return false;
            }
        }
        let push = self.push_rx.borrow_and_update().clone();
        if !self.sec.json(&push).await {
            return false;
        }
        if v.get("platform").and_then(Value::as_str) == Some("web") && !self.sec.json(&self.ctx.webpush_config_event()).await {
            return false;
        }
        if self.feed.is_some() {
            return self.resync_event().await;
        }
        let Some(tap) = self.ctx.host.tap() else { return self.workspace().await };
        let (tx, rx) = mpsc::channel(EVENT_QUEUE);
        let overflow = Arc::new(AtomicBool::new(false));
        let flag = overflow.clone();
        let (snapshot, guard) = tap.attach(move |text| {
            if tx.try_send(text).is_err() {
                flag.store(true, Ordering::SeqCst);
            }
        });
        self.feed = Some(Feed { rx, overflow, _guard: guard });
        self.snapshot(&snapshot).await && self.workspace().await
    }

    async fn workspace(&mut self) -> bool {
        self.send_ws_snapshot().await
    }

    async fn snapshot(&mut self, text: &str) -> bool {
        let Some((seq, mut event)) = snapshot_event(text) else { return true };
        self.last_seq = seq;
        if let Fit::Failed(bytes) = fit_snapshot(&mut event, SNAPSHOT_BUDGET) {
            return self.sec.json(&limit_event("snapshot", bytes)).await;
        }
        self.sec.json(&event).await
    }

    async fn resync_event(&mut self) -> bool {
        let Some(tap) = self.ctx.host.tap() else { return true };
        let text = tap.snapshot();
        self.snapshot(&text).await
    }

    async fn resync(&mut self, id: &str) -> bool {
        self.x.chat.full_last();
        self.sec.json(&json!({ "re": id, "ok": true })).await && self.resync_event().await
    }

    async fn tap(&mut self, id: &str, t: &str, v: &Value) -> bool {
        let Some(tap) = self.ctx.host.tap() else { return self.sec.json(&error(Value::from(id), "offline")).await };
        let reply_tx = self.direct_tx.clone();
        let rid = id.to_owned();
        let sent = tap.send(client_message(t, v), move |text| {
            let _ = reply_tx.try_send((rid.clone(), text));
        });
        match sent {
            Ok(None) if expects_reply(t) => true,
            Ok(_) => {
                self.logged(t, v);
                self.sec.json(&json!({ "re": id, "ok": true })).await
            }
            Err(TapError::Denied) => self.sec.json(&error(Value::from(id), "denied")).await,
            Err(TapError::Invalid) => self.sec.json(&error(Value::from(id), "invalid")).await,
        }
    }

    fn logged(&self, t: &str, v: &Value) {
        if t != "terminal.alias" {
            return;
        }
        let args = json!({ "device": self.device, "kind": "alias", "terminalId": v.get("terminalId").cloned().unwrap_or(Value::Null), "alias": v.get("alias").cloned().unwrap_or(Value::Null) });
        let host = self.ctx.host.clone();
        tokio::task::spawn_blocking(move || {
            host.ask("remote.logged", args);
        });
    }

    async fn forward(&mut self, first: Arc<str>) -> bool {
        tokio::time::sleep(BATCH).await;
        let mut batch = vec![first];
        let mut overflow = false;
        if let Some(feed) = self.feed.as_mut() {
            while let Ok(text) = feed.rx.try_recv() {
                batch.push(text);
            }
            overflow = feed.overflow.swap(false, Ordering::SeqCst);
        }
        self.x.chat.nudge(batch.iter().any(|t| turn_changed(t)));
        if overflow {
            return self.resync_event().await;
        }
        for text in batch {
            let Some((seq, event)) = events_event(&text) else { continue };
            if seq != 0 && seq <= self.last_seq {
                continue;
            }
            if !fits_message(&event) {
                eprintln!("[remote] events batch seq {seq} over {} B, sending a snapshot instead", core_remote_proto::frame::MAX_MESSAGE);
                return self.resync_event().await;
            }
            self.last_seq = seq;
            if !self.sec.json(&event).await {
                return false;
            }
        }
        true
    }
}
