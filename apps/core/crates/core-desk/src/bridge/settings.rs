use crate::claude::{self, Obj};
use crate::js;
use serde_json::{json, Value};
use std::path::Path;

pub const BRIDGE_MARK: &str = "statusline-bridge";
pub const HOOK_MARK: &str = "hook-bridge";
pub const HOOK_EVENTS: [&str; 4] = ["PreCompact", "PostCompact", "PermissionDenied", "PermissionRequest"];

pub enum Read {
    Unreadable,
    Missing(Obj),
    Ok(Obj),
}

pub fn read_settings(file: &Path) -> Read {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Missing(Obj::new()),
        Err(_) => return Read::Unreadable,
    };
    let text = String::from_utf8_lossy(&bytes);
    if claude::js_trim(&text).is_empty() {
        return Read::Ok(Obj::new());
    }
    match claude::parse_text(&text) {
        Some(Value::Object(m)) => Read::Ok(m),
        _ => Read::Unreadable,
    }
}

pub fn status_state(read: &Read) -> (&'static str, Option<String>) {
    let s = match read {
        Read::Unreadable => return ("unreadable", None),
        Read::Missing(_) => return ("none", None),
        Read::Ok(s) => s,
    };
    let command = s.get("statusLine").filter(|l| l.is_object() || l.is_array()).and_then(|l| l.get("command")).and_then(Value::as_str);
    match command.filter(|c| !claude::js_trim(c).is_empty()) {
        None => ("none", None),
        Some(c) if c.contains(BRIDGE_MARK) => ("ours", Some(c.into())),
        Some(c) => ("other", Some(c.into())),
    }
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn bridge_command(bridge: &str, previous: Option<&str>) -> String {
    match previous.filter(|p| !p.is_empty()) {
        Some(p) => format!("{bridge} --chain-b64 {}", b64(p.as_bytes())),
        None => bridge.to_string(),
    }
}

pub fn planned_status_line(settings: &Obj, state: &str, command: Option<&str>, bridge: &str) -> Value {
    let mut keep = settings.get("statusLine").and_then(Value::as_object).cloned().unwrap_or_default();
    keep.insert("type".into(), json!("command"));
    keep.insert("command".into(), Value::String(bridge_command(bridge, if state == "other" { command } else { None })));
    Value::Object(keep)
}

pub fn write_json(file: &Path, next: &Value) -> bool {
    claude::write_with_backup(file, &format!("{}\n", js::stringify_indent(next, "  ")))
}

pub fn write_status_line(file: &Path, bridge: &str) -> &'static str {
    let read = read_settings(file);
    let (state, command) = status_state(&read);
    if state == "ours" {
        return "ours";
    }
    let settings = match read {
        Read::Unreadable => return "unreadable",
        Read::Missing(s) | Read::Ok(s) => s,
    };
    let mut next = settings.clone();
    next.insert("statusLine".into(), planned_status_line(&settings, state, command.as_deref(), bridge));
    if write_json(file, &Value::Object(next)) {
        "ok"
    } else {
        "failed"
    }
}

fn is_ours(h: &Value) -> bool {
    h.get("command").and_then(Value::as_str).is_some_and(|c| c.contains(HOOK_MARK)) && h.is_object()
}

fn groups_of(settings: &Obj, event: &str) -> Vec<Value> {
    let hooks = settings.get("hooks").and_then(Value::as_object);
    hooks.and_then(|h| h.get(event)).and_then(Value::as_array).map(|a| a.iter().filter(|g| g.is_object()).cloned().collect()).unwrap_or_default()
}

pub fn hooks_state(read: &Read) -> &'static str {
    let s = match read {
        Read::Unreadable => return "unreadable",
        Read::Missing(_) => return "none",
        Read::Ok(s) => s,
    };
    let have = HOOK_EVENTS
        .iter()
        .filter(|e| groups_of(s, e).iter().any(|g| g.get("hooks").and_then(Value::as_array).is_some_and(|h| h.iter().any(is_ours))))
        .count();
    if have == HOOK_EVENTS.len() {
        "ours"
    } else if have > 0 {
        "partial"
    } else {
        "none"
    }
}

pub fn planned_hooks(settings: &Obj, command: Option<&str>) -> Obj {
    let mut hooks: Obj = settings.get("hooks").and_then(Value::as_object).cloned().unwrap_or_default();
    for event in HOOK_EVENTS {
        let mut kept: Vec<Value> = groups_of(settings, event)
            .into_iter()
            .map(|g| match g.get("hooks").and_then(Value::as_array) {
                Some(list) => {
                    let mut ng = g.as_object().cloned().unwrap_or_default();
                    ng.insert("hooks".into(), Value::Array(list.iter().filter(|h| !is_ours(h)).cloned().collect()));
                    Value::Object(ng)
                }
                None => g,
            })
            .filter(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|l| !l.is_empty()))
            .collect();
        if let Some(c) = command {
            kept.push(json!({"hooks": [{"type": "command", "command": c, "async": true}]}));
        }
        if kept.is_empty() {
            hooks.shift_remove(event);
        } else {
            hooks.insert(event.into(), Value::Array(kept));
        }
    }
    hooks
}

pub fn write_hooks(file: &Path, command: Option<&str>) -> &'static str {
    let settings = match read_settings(file) {
        Read::Unreadable => return "unreadable",
        Read::Missing(s) | Read::Ok(s) => s,
    };
    let hooks = planned_hooks(&settings, command);
    let mut next = settings.clone();
    let empty = hooks.is_empty();
    next.insert("hooks".into(), Value::Object(hooks));
    if empty {
        next.shift_remove("hooks");
    }
    if write_json(file, &Value::Object(next)) {
        "ok"
    } else {
        "failed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chains_and_plans() {
        assert_eq!(bridge_command("b", Some("node \"x\"")), "b --chain-b64 bm9kZSAieCI=");
        let s: Obj = serde_json::from_str(r#"{"hooks":{"PreCompact":[{"hooks":[{"type":"command","command":"hook-bridge old"},{"command":"other"}]}],"Stop":[]}}"#).unwrap();
        let h = planned_hooks(&s, Some("new hook-bridge"));
        assert_eq!(js::stringify(&Value::Object(h.clone())), r#"{"PreCompact":[{"hooks":[{"command":"other"}]},{"hooks":[{"type":"command","command":"new hook-bridge","async":true}]}],"Stop":[],"PostCompact":[{"hooks":[{"type":"command","command":"new hook-bridge","async":true}]}],"PermissionDenied":[{"hooks":[{"type":"command","command":"new hook-bridge","async":true}]}],"PermissionRequest":[{"hooks":[{"type":"command","command":"new hook-bridge","async":true}]}]}"#);
        assert_eq!(hooks_state(&Read::Ok(serde_json::from_str(&format!("{{\"hooks\":{}}}", js::stringify(&Value::Object(h)))).unwrap())), "ours");
    }
}
