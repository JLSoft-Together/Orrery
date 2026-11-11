use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::{broadcast, Notify};

use super::patch;
use crate::state::{lock, take_slot};

pub const WS_EVENTS: usize = 16;
pub const WS_PER_MINUTE: usize = 30;
pub const MAX_ATTACH: usize = 4;
pub const WS_COMMANDS: [&str; 8] = ["ws.open", "ws.close", "ws.rename", "ws.move", "ws.reopen", "ws.compact", "ws.send", "ws.keys"];
pub const SUMMARY_WAIT: Duration = Duration::from_secs(2);
const STALE_FOR: Duration = Duration::from_secs(10);
const MINUTE: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct WsEvent {
    pub base: Option<u64>,
    pub rev: u64,
    pub ops: Option<Arc<Value>>,
}

pub struct Board {
    state: Mutex<Option<Value>>,
    tx: broadcast::Sender<WsEvent>,
    rates: Mutex<HashMap<String, Vec<Instant>>>,
    attached: Mutex<HashMap<String, usize>>,
    stale_until: Mutex<Option<Instant>>,
    fresh: Notify,
}

impl Default for Board {
    fn default() -> Board {
        Board { state: Mutex::new(None), tx: broadcast::channel(WS_EVENTS).0, rates: Mutex::new(HashMap::new()), attached: Mutex::new(HashMap::new()), stale_until: Mutex::new(None), fresh: Notify::new() }
    }
}

pub fn rev_of(summary: &Value) -> u64 {
    summary.get("rev").and_then(Value::as_u64).unwrap_or(0)
}

pub fn snapshot_event(summary: &Value) -> Option<Value> {
    let mut obj = summary.as_object()?.clone();
    obj.insert("t".into(), Value::from("ws.snapshot"));
    Some(Value::Object(obj))
}

pub fn patch_event(event: &WsEvent) -> Option<Value> {
    let ops = event.ops.as_ref()?;
    Some(json!({ "t": "ws.patch", "base": event.base, "rev": event.rev, "ops": ops.as_ref() }))
}

fn has_pane(summary: &Value, id: &str) -> bool {
    summary["tabs"].as_array().is_some_and(|tabs| tabs.iter().any(|t| t["panes"].as_array().is_some_and(|ps| ps.iter().any(|p| p["sessionId"].as_str() == Some(id)))))
}

impl Board {
    pub fn mark_connect(&self) {
        *lock(&self.stale_until) = Some(Instant::now() + STALE_FOR);
    }

    fn is_stale(&self) -> bool {
        lock(&self.stale_until).is_some_and(|t| Instant::now() < t)
    }

    pub async fn has_session_fresh(&self, id: &str, wait: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let published = self.fresh.notified();
            tokio::pin!(published);
            published.as_mut().enable();
            if self.has_session(id) {
                return true;
            }
            if !self.is_stale() || tokio::time::timeout_at(deadline, published).await.is_err() {
                return self.has_session(id);
            }
        }
    }

    pub fn publish(&self, summary: Value) {
        let event = {
            let mut cur = lock(&self.state);
            let rev = rev_of(&summary);
            let event = match cur.as_ref() {
                Some(old) if *old == summary => None,
                Some(old) => {
                    let ops = Value::Array(patch::diff(old, &summary));
                    let small = ops.to_string().len() < summary.to_string().len();
                    Some(WsEvent { base: Some(rev_of(old)), rev, ops: small.then(|| Arc::new(ops)) })
                }
                None => Some(WsEvent { base: None, rev, ops: None }),
            };
            *cur = Some(summary);
            event
        };
        if let Some(e) = event {
            let _ = self.tx.send(e);
        }
        *lock(&self.stale_until) = None;
        self.fresh.notify_waiters();
    }

    pub fn summary(&self) -> Option<Value> {
        lock(&self.state).clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn has_session(&self, id: &str) -> bool {
        lock(&self.state).as_ref().is_some_and(|s| has_pane(s, id))
    }

    pub fn take_rate(&self, device: &str, now: Instant) -> bool {
        let mut map = lock(&self.rates);
        map.retain(|_, times| times.iter().any(|t| now.duration_since(*t) < MINUTE));
        take_slot(map.entry(device.to_owned()).or_default(), now, WS_PER_MINUTE, MINUTE)
    }

    pub fn take_attach(&self, device: &str) -> bool {
        let mut map = lock(&self.attached);
        let n = map.entry(device.to_owned()).or_default();
        if *n >= MAX_ATTACH {
            return false;
        }
        *n += 1;
        true
    }

    pub fn release_attach(&self, device: &str) {
        let mut map = lock(&self.attached);
        if let Some(n) = map.get_mut(device) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                map.remove(device);
            }
        }
    }

    #[cfg(test)]
    pub fn attached(&self, device: &str) -> usize {
        lock(&self.attached).get(device).copied().unwrap_or(0)
    }
}
