use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use core_serve::alerts::Emit;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};

use crate::state::lock;

pub const TICK: Duration = Duration::from_secs(60);
const SINCE_MAX: f64 = 8_640_000_000_000_000.0;
const NAME_MAX: usize = 80;
const EPISODE_MAX: usize = 300;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;
const PHONE_REASONS: [&str; 8] = ["waiting", "question", "permission", "done", "job", "limit", "disk", "note"];
pub const PHONE_CHANNEL: &str = "desk:phone-episode";

type Queue = (Mutex<Sender<Value>>, Mutex<Option<Receiver<Value>>>);

fn queue() -> &'static Queue {
    static Q: OnceLock<Queue> = OnceLock::new();
    Q.get_or_init(|| {
        let (tx, rx) = channel();
        (Mutex::new(tx), Mutex::new(Some(rx)))
    })
}

pub fn is_episode(v: &Value) -> bool {
    matches!(v.get("type").and_then(Value::as_str), Some("phone" | "telegram"))
}

pub fn sink() -> Emit {
    Arc::new(|v: &Value| {
        if is_episode(v) {
            let _ = lock(&queue().0).send(v.clone());
            return;
        }
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{v}");
        let _ = out.flush();
    })
}

pub fn js_num(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

fn finite(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite())
}

pub fn tool_ok(v: &Value) -> Option<&str> {
    v.as_str().filter(|t| (1..=80).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-')))
}

pub fn format_gb(bytes: f64) -> String {
    let gb = bytes / GB;
    if gb >= 100.0 {
        format!("{} GB", gb.round())
    } else {
        format!("{gb:.1} GB")
    }
}

pub fn parse_phone(raw: &Value) -> Option<Value> {
    if raw.get("type").and_then(Value::as_str) != Some("phone") {
        return None;
    }
    let episode = raw.get("episode").and_then(Value::as_str).filter(|e| !e.is_empty() && core_desk::js::len(e) <= EPISODE_MAX)?;
    let reason = raw.get("reason").and_then(Value::as_str).filter(|r| PHONE_REASONS.contains(r))?;
    let name = raw.get("name").and_then(Value::as_str).filter(|n| !n.is_empty())?;
    let text = if reason == "note" { Some(core_desk::mcp::text::clean_note_text(raw.get("text").and_then(Value::as_str)?)?) } else { None };
    let mut m = Map::new();
    m.insert("type".into(), json!("phone"));
    m.insert("episode".into(), json!(episode));
    m.insert("reason".into(), json!(reason));
    m.insert("name".into(), json!(core_desk::js::slice(name, NAME_MAX)));
    if let Some(t) = raw.get("tool").and_then(tool_ok) {
        m.insert("tool".into(), json!(t));
    }
    if let Some(s) = raw.get("since").and_then(finite).filter(|s| *s > 0.0 && *s < SINCE_MAX) {
        m.insert("since".into(), js_num(s));
    }
    if let Some(f) = raw.get("freeBytes").and_then(finite).filter(|f| *f >= 0.0) {
        m.insert("freeBytes".into(), js_num(f));
    }
    if let Some(t) = text {
        m.insert("text".into(), json!(t));
    }
    if reason == "limit" {
        if let Some(w) = raw.get("window").and_then(Value::as_str).filter(|w| matches!(*w, "five_hour" | "seven_day")) {
            m.insert("window".into(), json!(w));
        }
        if let Some(p) = raw.get("usedPct").and_then(finite) {
            m.insert("usedPct".into(), js_num(p.clamp(0.0, 100.0).round()));
        }
        if let Some(r) = raw.get("resetsAt").and_then(finite).filter(|r| *r > 0.0 && *r < SINCE_MAX) {
            m.insert("resetsAt".into(), js_num(r));
        }
        if raw.get("hit") == Some(&Value::Bool(true)) {
            m.insert("hit".into(), json!(true));
        }
    }
    Some(Value::Object(m))
}

pub fn alert_of(phone: &Value) -> Option<Value> {
    match phone["reason"].as_str()? {
        "limit" => {
            let mut detail = Map::new();
            for k in ["window", "usedPct", "resetsAt", "hit"] {
                if let Some(v) = phone.get(k) {
                    detail.insert(k.into(), v.clone());
                }
            }
            Some(json!({ "kind": "limit", "detail": detail }))
        }
        "disk" => {
            let free = phone.get("freeBytes").and_then(Value::as_f64).map_or_else(|| "?".to_owned(), format_gb);
            Some(json!({ "kind": "disk", "detail": { "label": phone["name"], "free": free } }))
        }
        _ => None,
    }
}

pub fn calls_for(msg: &Value) -> Vec<(&'static str, Value)> {
    match msg.get("type").and_then(Value::as_str) {
        Some("phone") => {
            let Some(p) = parse_phone(msg) else { return Vec::new() };
            let mut out = Vec::new();
            if let Some(a) = alert_of(&p) {
                out.push(("desk:tg-alert", json!([a])));
            }
            out.insert(0, (PHONE_CHANNEL, json!([p])));
            out
        }
        Some("telegram") => vec![("desk:tg-handle", json!([{ "episode": msg }]))],
        _ => Vec::new(),
    }
}

fn call(app: &AppHandle, channel: &str, args: Value) {
    if let Err(e) = crate::desk::call_inside(app, channel, args) {
        eprintln!("[shell] {channel}: {e}");
    }
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = std::thread::Builder::new().name(name.into()).spawn(f) {
        eprintln!("[shell] {name}: {e}");
    }
}

pub fn start(app: &AppHandle) {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(rx) = lock(&queue().1).take() else { return };
    let (phone_tx, phone_rx) = channel::<Value>();
    let phone_app = app.clone();
    spawn("phone-episodes", move || {
        for args in phone_rx {
            call(&phone_app, PHONE_CHANNEL, args);
        }
    });
    let episodes_app = app.clone();
    spawn("episodes", move || {
        for msg in rx {
            if msg.get("type").and_then(Value::as_str) == Some("telegram") {
                if let Some(r) = episodes_app.try_state::<crate::remote::Remote>() {
                    r.note_episode(&msg);
                }
            }
            for (channel, args) in calls_for(&msg) {
                if channel == PHONE_CHANNEL {
                    let _ = phone_tx.send(args);
                } else {
                    call(&episodes_app, channel, args);
                }
            }
        }
    });
    let tick_app = app.clone();
    spawn("telegram-tick", move || loop {
        std::thread::sleep(TICK);
        call(&tick_app, "desk:tg-tick", json!([null]));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_phone_and_telegram_messages_are_episodes() {
        assert!(is_episode(&json!({"type": "phone"})));
        assert!(is_episode(&json!({"type": "telegram"})));
        assert!(!is_episode(&json!({"type": "toast.show"})));
        assert!(!is_episode(&json!({"type": "listening"})));
        assert!(!is_episode(&json!({})));
    }

    #[test]
    fn a_done_phone_episode_goes_only_to_the_phone() {
        let msg = json!({"type": "phone", "episode": "d:a|", "reason": "done", "name": "app", "since": 5, "tool": "Bash"});
        let calls = calls_for(&msg);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, PHONE_CHANNEL);
        assert_eq!(calls[0].1, json!([{"type": "phone", "episode": "d:a|", "reason": "done", "name": "app", "since": 5, "tool": "Bash"}]));
    }

    #[test]
    fn a_limit_episode_also_alerts_telegram_with_the_clean_fields() {
        let msg = json!({"type": "phone", "episode": "l:5h", "reason": "limit", "name": "Claude", "window": "five_hour", "usedPct": 92.6, "resetsAt": 1000, "hit": false});
        let calls = calls_for(&msg);
        assert_eq!(calls.iter().map(|c| c.0).collect::<Vec<_>>(), [PHONE_CHANNEL, "desk:tg-alert"]);
        assert_eq!(calls[1].1, json!([{"kind": "limit", "detail": {"window": "five_hour", "usedPct": 93, "resetsAt": 1000}}]));
        let hit = calls_for(&json!({"type": "phone", "episode": "l", "reason": "limit", "name": "C", "window": "weekly", "usedPct": 150, "hit": true}));
        assert_eq!(hit[1].1, json!([{"kind": "limit", "detail": {"usedPct": 100, "hit": true}}]));
    }

    #[test]
    fn a_disk_episode_alerts_with_the_label_and_free_space() {
        let calls = calls_for(&json!({"type": "phone", "episode": "disk:C", "reason": "disk", "name": "C:", "freeBytes": 5.0 * GB}));
        assert_eq!(calls[1].1, json!([{"kind": "disk", "detail": {"label": "C:", "free": "5.0 GB"}}]));
        let unknown = calls_for(&json!({"type": "phone", "episode": "disk:C", "reason": "disk", "name": "C:"}));
        assert_eq!(unknown[1].1, json!([{"kind": "disk", "detail": {"label": "C:", "free": "?"}}]));
    }

    #[test]
    fn gigabytes_read_like_the_shared_formatter() {
        assert_eq!(format_gb(0.0), "0.0 GB");
        assert_eq!(format_gb(12.34 * GB), "12.3 GB");
        assert_eq!(format_gb(250.6 * GB), "251 GB");
    }

    #[test]
    fn bad_phone_episodes_are_dropped() {
        assert!(calls_for(&json!({"type": "phone", "episode": "", "reason": "done", "name": "a"})).is_empty());
        assert!(calls_for(&json!({"type": "phone", "episode": "x", "reason": "boom", "name": "a"})).is_empty());
        assert!(calls_for(&json!({"type": "phone", "episode": "x", "reason": "done", "name": ""})).is_empty());
        assert!(calls_for(&json!({"type": "phone", "episode": "x".repeat(301), "reason": "done", "name": "a"})).is_empty());
        assert!(calls_for(&json!({"type": "phone", "episode": "x", "reason": "note", "name": "a", "text": " \u{200b} "})).is_empty());
    }

    #[test]
    fn phone_fields_are_cleaned_like_the_electron_parser() {
        let p = parse_phone(&json!({"type": "phone", "episode": "n:1", "reason": "note", "name": "n".repeat(100), "text": "hi\n\nthere", "tool": "bad tool", "since": -1, "freeBytes": 1, "usedPct": 50})).unwrap();
        assert_eq!(p["name"].as_str().unwrap().len(), NAME_MAX);
        assert_eq!(p["text"], "hi there");
        assert!(p.get("tool").is_none());
        assert!(p.get("since").is_none());
        assert_eq!(p["freeBytes"], json!(1));
        assert_eq!(js_num(1.5), json!(1.5));
        assert!(p.get("usedPct").is_none());
    }

    #[test]
    fn telegram_episodes_are_handed_over_whole() {
        let msg = json!({"type": "telegram", "episode": "td:t1|0", "reason": "done", "terminalId": "t1", "name": "app", "cwd": "/p", "worked": 5000, "reply": "ok"});
        assert_eq!(calls_for(&msg), vec![("desk:tg-handle", json!([{ "episode": msg }]))]);
    }

    #[test]
    fn other_messages_make_no_calls() {
        assert!(calls_for(&json!({"type": "toast.show", "tag": "t"})).is_empty());
        assert!(calls_for(&Value::Null).is_empty());
    }
}
