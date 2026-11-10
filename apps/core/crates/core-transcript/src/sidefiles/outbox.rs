use super::common::iso;
use crate::jsstr::units;
use serde_json::{json, Map, Value};

const EVENTS: [&str; 7] = [
    "session_start",
    "session_end",
    "turn_start",
    "turn_end",
    "turn_stalled",
    "subagent_spawn",
    "subagent_complete",
];

fn ident(s: &str, max: usize, extra: &[u8]) -> bool {
    (1..=max).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
}

fn kind(s: &str) -> bool {
    let mut p = s.split('.');
    let (a, b) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
    let part = |x: &str| {
        (1..=24).contains(&x.len()) && x.bytes().all(|c| c.is_ascii_lowercase() || c == b'_')
    };
    p.next().is_none() && part(a) && part(b)
}

fn absolute(s: &str) -> bool {
    let b = s.as_bytes();
    matches!(b.first(), Some(b'/' | b'\\'))
        || (b.len() > 2
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && matches!(b[2], b'/' | b'\\'))
}

fn workspace(v: Option<&Value>) -> Option<&str> {
    let s = v?.as_str()?;
    (!s.is_empty()
        && units(s) <= 1024
        && !s.chars().any(|c| c < ' ' || c == '\u{7f}')
        && absolute(s))
    .then_some(s)
}

fn outcome(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "completed" | "complete" | "success" | "succeeded" | "ok" | "done" => Some("completed"),
        "failed" | "error" | "errored" => Some("error"),
        "cancelled" | "canceled" | "interrupted" | "aborted" => Some("cancelled"),
        _ => None,
    }
}

fn pick(o: &Map<String, Value>, keys: &[&str], valid: impl Fn(&str) -> bool) -> Option<String> {
    keys.iter().find_map(|k| {
        let s = o.get(*k)?.as_str()?;
        valid(s).then(|| s.to_owned())
    })
}

fn subagent(payload: Option<&Map<String, Value>>) -> Value {
    let status = payload
        .and_then(|p| p.get("status"))
        .and_then(Value::as_str)
        .and_then(outcome);
    let sub_id = |s: &str| ident(s, 64, b"._:-");
    let mut sub = Map::new();
    match payload.and_then(|p| p.get("subagent")) {
        Some(Value::String(s)) => {
            if sub_id(s) {
                sub.insert("id".into(), json!(s));
            }
            if let Some(v) = status {
                sub.insert("outcome".into(), json!(v));
            }
        }
        Some(Value::Object(x)) => {
            if let Some(v) = pick(x, &["id", "agent_id", "subagent_id", "thread_id"], sub_id) {
                sub.insert("id".into(), json!(v));
            }
            if let Some(v) = pick(x, &["name", "agent_type", "type", "role"], |s| {
                ident(s, 40, b"_-")
            }) {
                sub.insert("name".into(), json!(v));
            }
            let own = x.get("status").and_then(Value::as_str).and_then(outcome);
            if let Some(v) = own.or(status) {
                sub.insert("outcome".into(), json!(v));
            }
        }
        _ => {
            if let Some(v) = status {
                sub.insert("outcome".into(), json!(v));
            }
        }
    }
    Value::Object(sub)
}

pub fn outbox_line(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    if o.get("schema_version")
        .is_some_and(|v| v.as_f64() != Some(1.0))
    {
        return None;
    }
    let event = o.get("event")?.as_str()?;
    if !EVENTS.contains(&event) {
        return None;
    }
    let id = o.get("thread_id")?.as_str()?;
    if !ident(id, 128, b"._:-") {
        return None;
    }
    let payload = o.get("payload").and_then(Value::as_object);
    let mut r = json!({"event": event, "threadId": id});
    if let Some(k) = o.get("kind").and_then(Value::as_str).filter(|s| kind(s)) {
        r["kind"] = json!(k);
    }
    if let Some(at) = iso(o.get("timestamp")).or_else(|| iso(o.get("created_at"))) {
        r["at"] = json!(at);
    }
    if let Some(w) = workspace(payload.and_then(|p| p.get("workspace"))) {
        r["workspace"] = json!(w);
    }
    if event == "subagent_spawn" || event == "subagent_complete" {
        r["subagent"] = subagent(payload);
    }
    Some(r)
}
