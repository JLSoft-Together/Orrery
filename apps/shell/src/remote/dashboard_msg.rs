use std::time::Duration;

use serde_json::{json, Value};

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

pub fn expects_reply(t: &str) -> bool {
    matches!(t, "history.get" | "tool-spans.get" | "toolSpans.get")
}

pub fn parse(text: &str) -> Option<(u64, Vec<Value>)> {
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

pub fn turn_changed(text: &str) -> bool {
    text.contains("\"terminal.updated\"") && text.contains("\"status\"")
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
