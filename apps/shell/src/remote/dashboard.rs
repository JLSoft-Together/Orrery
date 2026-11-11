use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use core_remote_proto::frame::{Frame, Kind, Reassembler};
use core_remote_proto::MIN_PROTO;
use core_serve::{TapError, TapGuard};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::config::now_secs;
use super::ctx::{Control, Ctx};
use super::limits::{Rate, MAX_COMMANDS_PER_MINUTE};
use super::link::{Secure, CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_OFF, CLOSE_PROTO};
use crate::state::lock;

pub const BATCH: Duration = Duration::from_millis(30);
pub const EVENT_QUEUE: usize = 512;
pub const MAX_ID_LEN: usize = 32;
pub const DASHBOARD: [&str; 7] = ["terminal.alias", "terminal.note", "terminal.snooze", "terminal.dismiss", "history.get", "tool-spans.get", "toolSpans.get"];
pub const DESKTOP_ONLY: [&str; 13] = [
    "office.snapshot",
    "folder.picked",
    "terminal.focusResult",
    "terminal.openResult",
    "promptSearch.progress",
    "promptSearch.done",
    "promptSearch.openResult",
    "notify.settings",
    "notify.testResult",
    "sensitive.settings",
    "diagnostics.data",
    "resources",
    "favorite.rejected",
];

fn expects_reply(t: &str) -> bool {
    matches!(t, "history.get" | "tool-spans.get" | "toolSpans.get")
}

fn parse(text: &str) -> Option<(u64, Vec<Value>)> {
    let v: Value = serde_json::from_str(text).ok()?;
    let seq = v.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let events = v.get("events")?.as_array()?.clone();
    Some((seq, events))
}

pub fn snapshot_event(text: &str) -> Option<(u64, Value)> {
    let (seq, events) = parse(text)?;
    let payload = events.into_iter().find(|e| e.get("type").and_then(Value::as_str) == Some("snapshot"))?.get("payload")?.clone();
    Some((seq, json!({ "t": "snapshot", "seq": seq, "data": payload })))
}

pub fn phone_events(events: Vec<Value>) -> Vec<Value> {
    events.into_iter().filter(|e| e.get("type").and_then(Value::as_str).is_none_or(|t| !DESKTOP_ONLY.contains(&t))).collect()
}

pub fn events_event(text: &str) -> Option<(u64, Value)> {
    let (seq, events) = parse(text)?;
    Some((seq, json!({ "t": "events", "seq": seq, "events": phone_events(events) })))
}

pub fn ws_snapshot(summary: &Value) -> Option<Value> {
    let mut obj = summary.as_object()?.clone();
    obj.insert("t".into(), Value::from("ws.snapshot"));
    Some(Value::Object(obj))
}

pub fn command(raw: &[u8]) -> Result<(String, String, Value), Value> {
    let v: Value = serde_json::from_slice(raw).map_err(|_| error(Value::Null, "invalid"))?;
    let id = v.get("id").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= MAX_ID_LEN).map(str::to_owned);
    let t = v.get("t").and_then(Value::as_str).map(str::to_owned);
    match (id, t) {
        (Some(id), Some(t)) => Ok((id, t, v)),
        (id, _) => Err(error(id.map_or(Value::Null, Value::from), "invalid")),
    }
}

pub fn client_message(t: &str, v: &Value) -> Value {
    let mut msg = v.as_object().cloned().unwrap_or_default();
    msg.remove("id");
    msg.remove("t");
    msg.insert("type".into(), Value::from(t));
    Value::Object(msg)
}

pub fn error(re: Value, code: &str) -> Value {
    json!({ "re": re, "ok": false, "error": code })
}

struct Feed {
    rx: mpsc::Receiver<Arc<str>>,
    overflow: Arc<AtomicBool>,
    _guard: TapGuard,
}

pub struct Phone {
    ctx: Arc<Ctx>,
    sec: Secure,
    device: String,
    reasm: Reassembler,
    hello: bool,
    feed: Option<Feed>,
    last_seq: u64,
    rate: Rate,
    direct_tx: mpsc::Sender<(String, Arc<str>)>,
    direct_rx: mpsc::Receiver<(String, Arc<str>)>,
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
        Phone { ctx, sec, device, reasm: Reassembler::new(), hello: false, feed: None, last_seq: 0, rate: Rate::default(), direct_tx, direct_rx }
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
                    if !self.inbound(&msg).await {
                        break;
                    }
                }
                c = ctl.recv() => {
                    self.control(c).await;
                    break;
                }
                text = next_event(&mut self.feed) => {
                    let Some(text) = text else { break };
                    if !self.forward(text).await {
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
            Some(Control::Revoked) => ("device.revoked", CLOSE_DEVICE, "revoked"),
            Some(Control::Restarting) => ("machine.restarting", CLOSE_OFF, "restarting"),
            Some(Control::Off) | None => ("remote.off", CLOSE_OFF, "off"),
        };
        self.sec.json(&json!({ "t": event })).await;
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
            Kind::Ping => self.sec.frame(Kind::Ping, 0, &[]).await,
            Kind::PtyIn | Kind::PtyOut => true,
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
            _ => self.sec.json(&error(Value::from(id), "denied")).await,
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
        self.hello = true;
        let welcome = self.ctx.welcome(id);
        if !self.sec.json(&welcome).await {
            return false;
        }
        if let Some(event) = self.ctx.renew_token(&self.device, now_secs()) {
            if !self.sec.json(&event).await {
                return false;
            }
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
        match self.ctx.summary().as_ref().and_then(ws_snapshot) {
            Some(event) => self.sec.json(&event).await,
            None => true,
        }
    }

    async fn snapshot(&mut self, text: &str) -> bool {
        match snapshot_event(text) {
            Some((seq, event)) => {
                self.last_seq = seq;
                self.sec.json(&event).await
            }
            None => true,
        }
    }

    async fn resync_event(&mut self) -> bool {
        let Some(tap) = self.ctx.host.tap() else { return true };
        let text = tap.snapshot();
        self.snapshot(&text).await
    }

    async fn resync(&mut self, id: &str) -> bool {
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
            Ok(_) => self.sec.json(&json!({ "re": id, "ok": true })).await,
            Err(TapError::Denied) => self.sec.json(&error(Value::from(id), "denied")).await,
            Err(TapError::Invalid) => self.sec.json(&error(Value::from(id), "invalid")).await,
        }
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
        if overflow {
            return self.resync_event().await;
        }
        for text in batch {
            let Some((seq, event)) = events_event(&text) else { continue };
            if seq != 0 && seq <= self.last_seq {
                continue;
            }
            self.last_seq = seq;
            if !self.sec.json(&event).await {
                return false;
            }
        }
        true
    }
}
