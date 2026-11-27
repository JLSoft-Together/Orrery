use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::ctx::Ctx;
use super::revoke::MAX_ID;
use super::status::relay_kind;
use super::Remote;
use crate::state::lock;

pub const PROBE_WAIT: Duration = Duration::from_secs(3);
pub const PUSH_GAP: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(25);
const MAX_ENTRIES: usize = 256;

#[derive(Clone, Copy)]
struct Rtt {
    ms: u64,
    at: Instant,
}

#[derive(Default)]
pub struct Probes {
    rtt: Mutex<HashMap<(String, &'static str), Rtt>>,
    push_at: Mutex<HashMap<String, Instant>>,
}

impl Probes {
    pub fn record(&self, device: &str, via: &'static str, took: Duration) {
        let ms = u64::try_from(took.as_millis()).unwrap_or(u64::MAX);
        let key = (device.to_owned(), via);
        let mut map = lock(&self.rtt);
        if map.len() >= MAX_ENTRIES && !map.contains_key(&key) {
            map.clear();
        }
        map.insert(key, Rtt { ms, at: Instant::now() });
    }

    pub fn since(&self, device: &str, via: &'static str, start: Instant) -> Option<u64> {
        lock(&self.rtt).get(&(device.to_owned(), via)).filter(|r| r.at >= start).map(|r| r.ms)
    }

    pub fn take_push(&self, device: &str, now: Instant) -> bool {
        let mut map = lock(&self.push_at);
        map.retain(|_, t| now.duration_since(*t) < PUSH_GAP);
        if map.contains_key(device) || map.len() >= MAX_ENTRIES {
            return false;
        }
        map.insert(device.to_owned(), now);
        true
    }
}

pub fn mark_held(status: &mut Value, held: &[String]) {
    if held.is_empty() {
        return;
    }
    let Some(list) = status.get_mut("devices").and_then(Value::as_array_mut) else { return };
    for d in list.iter_mut() {
        if d.get("deviceId").and_then(Value::as_str).is_some_and(|id| held.iter().any(|h| h == id)) {
            d["revoking"] = Value::Bool(true);
        }
    }
}

fn device_id(arg: &Value) -> Option<&str> {
    let id = match arg {
        Value::String(s) => Some(s.as_str()),
        _ => arg.get("deviceId").and_then(Value::as_str),
    };
    id.filter(|s| !s.is_empty() && s.len() <= MAX_ID)
}

fn with_ms(mut v: Value, ms: Option<u64>) -> Value {
    if let Some(ms) = ms {
        v["ms"] = Value::from(ms);
    }
    v
}

pub fn probe_links(ctx: &Ctx, id: &str, wait: Duration) -> (Value, Value) {
    let start = Instant::now();
    let vias = ctx.probe(id);
    let deadline = start + wait;
    while vias.iter().any(|v| ctx.probes.since(id, v, start).is_none()) && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    let on = |via: &'static str| vias.contains(&via).then(|| ctx.probes.since(id, via, start)).flatten();
    let lan = with_ms(json!({ "ok": vias.contains(&"lan") }), on("lan"));
    let relay = with_ms(json!({ "state": relay_kind(ctx.relay_state()) }), on("relay"));
    (lan, relay)
}

fn push_error(code: &str) -> &str {
    match code {
        "not_configured" | "no_device" => "none",
        other => other,
    }
}

impl Remote {
    fn check_push(&self, id: &str) -> Value {
        let Ok(ctx) = self.ctx() else { return json!({ "ok": false, "error": "keyring" }) };
        if !ctx.probes.take_push(id, Instant::now()) {
            return json!({ "ok": false, "error": "rate" });
        }
        let sent = self.push_test(&json!({ "deviceId": id }));
        if sent.get("ok") == Some(&Value::Bool(true)) {
            return json!({ "ok": true });
        }
        let code = sent.get("error").and_then(Value::as_str).unwrap_or("failed");
        json!({ "ok": false, "error": push_error(code) })
    }

    pub fn remote_check(&self, arg: &Value) -> Value {
        let Some(id) = device_id(arg) else { return Value::Null };
        let (lan, relay) = match self.running_ctx() {
            Ok(ctx) => probe_links(&ctx, id, PROBE_WAIT),
            Err(_) => (json!({ "ok": false }), json!({ "state": "off" })),
        };
        json!({ "lan": lan, "relay": relay, "push": self.check_push(id) })
    }
}
