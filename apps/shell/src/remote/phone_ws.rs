use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};

use super::confirm::{ask_event, command_of, Confirms};
use super::ctx::Asked;
use super::dashboard::{error, Phone};
use super::pty_bridge::{Attach, FromHub};
use super::workspace::{patch_event, rev_of, snapshot_event, WsEvent, WS_COMMANDS};

pub const WS_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_IN_FLIGHT: usize = 4;
pub const SUMMARY_GET: &str = "remote.summary.get";
const CODES: [&str; 10] = ["denied", "busy", "limit", "notFound", "invalid", "timeout", "offline", "rate", "proto", "internal"];

pub struct Done {
    id: String,
    action: String,
    cmd: Value,
    confirmed: bool,
    result: Result<Value, &'static str>,
}

pub struct Extra {
    pub(super) done_tx: mpsc::Sender<Done>,
    pub(super) done_rx: mpsc::Receiver<Done>,
    pub(super) in_flight: usize,
    pub(super) confirms: Confirms,
    pub(super) ws_rx: broadcast::Receiver<WsEvent>,
    pub(super) ws_rev: Option<u64>,
    pub(super) attaches: HashMap<u32, Attach>,
    pub(super) next_gen: u64,
    pub(super) hub_tx: mpsc::Sender<FromHub>,
    pub(super) hub_rx: mpsc::Receiver<FromHub>,
    pub(super) claims: HashSet<u32>,
    pub(super) backlog: Arc<AtomicUsize>,
    pub(super) nonce: String,
}

impl Extra {
    pub fn new(ws_rx: broadcast::Receiver<WsEvent>) -> Extra {
        let (done_tx, done_rx) = mpsc::channel(16);
        let (hub_tx, hub_rx) = mpsc::channel(64);
        Extra {
            done_tx,
            done_rx,
            in_flight: 0,
            confirms: Confirms::default(),
            ws_rx,
            ws_rev: None,
            attaches: HashMap::new(),
            next_gen: 0,
            hub_tx,
            hub_rx,
            claims: HashSet::new(),
            backlog: Arc::new(AtomicUsize::new(0)),
            nonce: crate::config::try_random_hex(8).unwrap_or_default(),
        }
    }
}

pub fn error_code(raw: Option<&str>) -> &'static str {
    match raw {
        Some(c) if CODES.contains(&c) => CODES.iter().find(|x| **x == c).copied().unwrap_or("internal"),
        Some("args") => "invalid",
        Some("unavailable") => "offline",
        Some("source" | "unknown") => "denied",
        _ => "internal",
    }
}

pub fn reply_of(id: &str, v: &Value) -> Value {
    match v.as_object() {
        Some(obj) if obj.get("ok") == Some(&Value::Bool(true)) => {
            let mut out = obj.clone();
            out.insert("re".into(), Value::from(id));
            Value::Object(out)
        }
        Some(obj) => error(Value::from(id), error_code(obj.get("error").and_then(Value::as_str))),
        None => error(Value::from(id), "internal"),
    }
}

pub fn rid_of(nonce: &str, id: &str, confirmed: bool) -> String {
    let hex: String = id.bytes().map(|b| format!("{b:02x}")).collect();
    format!("{nonce}.{}{hex}", if confirmed { "c." } else { "" })
}

pub async fn next_ws(rx: &mut broadcast::Receiver<WsEvent>, live: bool) -> Option<WsEvent> {
    if !live {
        return std::future::pending().await;
    }
    match rx.recv().await {
        Ok(e) => Some(e),
        Err(RecvError::Lagged(_)) => None,
        Err(RecvError::Closed) => std::future::pending().await,
    }
}

impl Phone {
    pub(super) async fn send_ws_snapshot(&mut self) -> bool {
        let Some(summary) = self.ctx.summary() else { return true };
        let rev = rev_of(&summary);
        if self.x.ws_rev == Some(rev) {
            return true;
        }
        let Some(event) = snapshot_event(&summary) else { return true };
        self.x.ws_rev = Some(rev);
        self.sec.json(&event).await
    }

    pub(super) async fn on_ws(&mut self, event: Option<WsEvent>) -> bool {
        let Some(e) = event else { return self.send_ws_snapshot().await };
        if self.x.ws_rev == Some(e.rev) {
            return true;
        }
        match patch_event(&e) {
            Some(p) if self.x.ws_rev.is_some() && e.base == self.x.ws_rev => {
                self.x.ws_rev = Some(e.rev);
                self.sec.json(&p).await
            }
            _ => self.send_ws_snapshot().await,
        }
    }

    pub(super) async fn extra(&mut self, id: &str, t: &str, v: &Value) -> bool {
        match t {
            "confirm.answer" => self.confirm_answer(id, v).await,
            "pty.attach" => self.pty_attach(id, v).await,
            "pty.detach" => self.pty_detach(id, v).await,
            "pty.resize" => self.pty_resize(id, v).await,
            "ws.get" | "ws.open" | "ws.close" | "ws.rename" | "ws.move" | "ws.reopen" | "ws.compact" | "ws.send" | "ws.keys" => {
                if !self.ctx.board.take_rate(&self.device, Instant::now()) {
                    return self.sec.json(&error(Value::from(id), "rate")).await;
                }
                if t == "ws.get" {
                    return self.ws_get(id).await;
                }
                self.call(id, t, command_of(v), false).await
            }
            _ => self.sec.json(&error(Value::from(id), "denied")).await,
        }
    }

    async fn ws_get(&mut self, id: &str) -> bool {
        if self.ctx.summary().is_some() {
            self.x.ws_rev = None;
            return self.sec.json(&json!({ "re": id, "ok": true })).await && self.send_ws_snapshot().await;
        }
        self.call(id, SUMMARY_GET, json!({}), false).await
    }

    async fn call(&mut self, id: &str, action: &str, cmd: Value, confirmed: bool) -> bool {
        if self.x.in_flight >= MAX_IN_FLIGHT {
            return self.sec.json(&error(Value::from(id), "busy")).await;
        }
        self.x.in_flight += 1;
        let args = if action == SUMMARY_GET {
            json!({})
        } else if confirmed {
            json!({ "device": self.device, "cmd": cmd, "confirmed": true, "rid": rid_of(&self.x.nonce, id, true) })
        } else {
            json!({ "device": self.device, "cmd": cmd, "rid": rid_of(&self.x.nonce, id, false) })
        };
        let host = self.ctx.host.clone();
        let tx = self.x.done_tx.clone();
        let (id, action) = (id.to_owned(), action.to_owned());
        let name = action.clone();
        tokio::spawn(async move {
            let asked = tokio::time::timeout(WS_TIMEOUT, tokio::task::spawn_blocking(move || host.ask(&name, args))).await;
            let result = match asked {
                Ok(Ok(Asked::Reply(v))) => Ok(v),
                Ok(Ok(Asked::Offline) | Err(_)) => Err("offline"),
                Err(_) => Err("timeout"),
            };
            let _ = tx.send(Done { id, action, cmd, confirmed, result }).await;
        });
        true
    }

    pub(super) async fn on_done(&mut self, done: Done) -> bool {
        self.x.in_flight = self.x.in_flight.saturating_sub(1);
        let id = done.id.as_str();
        let v = match done.result {
            Ok(v) => v,
            Err(code) => return self.sec.json(&error(Value::from(id), code)).await,
        };
        if done.action == SUMMARY_GET {
            if !v.is_object() || serde_json::to_vec(&v).map_or(true, |b| b.len() > super::MAX_SUMMARY) {
                return self.sec.json(&error(Value::from(id), "offline")).await;
            }
            self.ctx.publish(v);
            self.x.ws_rev = None;
            return self.sec.json(&json!({ "re": id, "ok": true })).await && self.send_ws_snapshot().await;
        }
        let Some(needs) = v.get("needsConfirm").filter(|n| n.is_object()) else { return self.sec.json(&reply_of(id, &v)).await };
        if done.confirmed {
            return self.sec.json(&error(Value::from(id), "busy")).await;
        }
        let Some(cid) = self.x.confirms.add(done.cmd, Instant::now()) else { return self.sec.json(&error(Value::from(id), "internal")).await };
        let event = ask_event(&cid, id, needs);
        self.sec.json(&json!({ "re": id, "ok": true, "confirm": cid })).await && self.sec.json(&event).await
    }

    async fn confirm_answer(&mut self, id: &str, v: &Value) -> bool {
        let Some(cid) = v.get("cid").and_then(Value::as_str) else { return self.sec.json(&error(Value::from(id), "invalid")).await };
        let Some(cmd) = self.x.confirms.take(cid, Instant::now()) else { return self.sec.json(&error(Value::from(id), "notFound")).await };
        if v.get("ok") != Some(&Value::Bool(true)) {
            return self.sec.json(&json!({ "re": id, "ok": true, "cancelled": true })).await;
        }
        let Some(t) = cmd.get("t").and_then(Value::as_str).filter(|t| WS_COMMANDS.contains(t)).map(str::to_owned) else {
            return self.sec.json(&error(Value::from(id), "invalid")).await;
        };
        self.call(id, &t, cmd, true).await
    }
}
