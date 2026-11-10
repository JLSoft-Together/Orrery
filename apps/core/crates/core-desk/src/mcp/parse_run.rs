use super::t_common::jnum;
use super::text::{clean_run_text, js_trim, u16_len};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

pub const MCP_RUN_ACTIONS: [&str; 4] = ["start", "reload", "restart", "stop"];
pub const MCP_LOG_LINES_MAX: i64 = 200;
pub const MCP_LOG_LINES_DEFAULT: i64 = 50;
pub const MCP_LOG_MATCH_MAX: usize = 200;
const LINE_CHARS: usize = 1000;
const NAME_CHARS: usize = 80;
const DETAIL_CHARS: usize = 300;
const CONFIGS_MAX: usize = 50;
const DEVICES_MAX: usize = 30;
const SESSIONS_MAX: usize = 10;
const ERRORS_MAX: usize = 20;
const KINDS: [&str; 3] = ["flutter", "android", "custom"];
const STATES: [&str; 11] = ["before-launch", "building", "installing", "launching", "starting", "running", "reloading", "stopping", "stopped", "failed", "cancelled"];
const DEVICE_STATES: [&str; 3] = ["ready", "unauthorized", "offline"];
const STREAMS: [&str; 3] = ["build", "app", "logcat"];
const LEVELS: [&str; 6] = ["V", "D", "I", "W", "E", "F"];

fn cached(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).unwrap())
}

pub fn is_mcp_run_action(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| MCP_RUN_ACTIONS.contains(&s))
}

pub fn is_mcp_run_id(v: Option<&Value>) -> bool {
    let Some(s) = v.and_then(Value::as_str) else { return false };
    let n = s.chars().count();
    (1..=128).contains(&n) && !s.starts_with('-') && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '@' | '%' | '-'))
}

pub fn is_mcp_log_cursor(v: Option<&Value>) -> bool {
    static A: OnceLock<Regex> = OnceLock::new();
    v.and_then(Value::as_str).is_some_and(|s| cached(&A, r"^[A-Za-z0-9-]{1,64}\.[0-9]{1,6}\.[0-9]{1,12}$").is_match(s))
}

pub fn compile_log_match(m: &str, ignore_case: bool) -> bool {
    if m.is_empty() || u16_len(m) > MCP_LOG_MATCH_MAX {
        return false;
    }
    regress::Regex::with_flags(m, if ignore_case { "i" } else { "" }).is_ok()
}

pub fn run_phase(state: &str) -> &'static str {
    match state {
        "running" | "reloading" => "running",
        "stopping" => "stopping",
        "failed" => "crashed",
        "stopped" | "cancelled" => "stopped",
        _ => "starting",
    }
}

fn time(v: Option<&Value>) -> Option<Value> {
    let f = v?.as_f64()?;
    (f.is_finite() && f >= 0.0).then(|| jnum(f.floor()))
}

fn time_f(v: Option<&Value>) -> Option<f64> {
    let f = v?.as_f64()?;
    (f.is_finite() && f >= 0.0).then(|| f.floor())
}

fn list(v: Option<&Value>, max: usize) -> Vec<&Value> {
    v.and_then(Value::as_array).map(|a| a.iter().take(max).collect()).unwrap_or_default()
}

fn one_of(list: &[&str], v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| list.contains(s)).map(str::to_string)
}

fn name(v: Option<&Value>) -> Option<String> {
    let t = clean_run_text(v?.as_str()?, NAME_CHARS)?;
    let t = js_trim(&t);
    (!t.is_empty()).then(|| t.to_string())
}

fn cleaned(v: Option<&Value>, max: usize) -> Option<String> {
    clean_run_text(v?.as_str()?, max)
}

fn matches_re(cell: &'static OnceLock<Regex>, src: &str, v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| cached(cell, src).is_match(s))
}

fn refusal(v: Option<&Value>) -> Option<Value> {
    static A: OnceLock<Regex> = OnceLock::new();
    let o = v?.as_object()?;
    let code = o.get("code")?.as_str()?;
    if !cached(&A, r"^[a-z0-9][a-z0-9-]{0,39}$").is_match(code) {
        return None;
    }
    match cleaned(o.get("detail"), DETAIL_CHARS) {
        Some(d) => Some(json!({ "code": code, "detail": d })),
        None => Some(json!({ "code": code })),
    }
}

fn session(v: Option<&Value>) -> Option<Value> {
    static ID: OnceLock<Regex> = OnceLock::new();
    static APP: OnceLock<Regex> = OnceLock::new();
    let o = v?.as_object()?;
    if !is_mcp_run_id(o.get("configId")) || !matches_re(&ID, r"^[A-Za-z0-9-]{1,64}$", o.get("id")) {
        return None;
    }
    let kind = one_of(&KINDS, o.get("type"))?;
    let raw_state = one_of(&STATES, o.get("rawState"))?;
    let started = time(o.get("startedAt"))?;
    let config_id = o["configId"].as_str()?.to_string();
    let device_id = is_mcp_run_id(o.get("deviceId")).then(|| o["deviceId"].clone());
    let app_id = if matches_re(&APP, r"^[A-Za-z0-9._-]{1,200}$", o.get("appId")) { o["appId"].clone() } else { Value::Null };
    let device_name = if device_id.is_some() { name(o.get("deviceName")).map(Value::from).unwrap_or(Value::Null) } else { Value::Null };
    Some(json!({
        "id": o["id"],
        "configId": config_id,
        "configName": name(o.get("configName")).unwrap_or_else(|| config_id.clone()),
        "type": kind,
        "deviceId": device_id.unwrap_or(Value::Null),
        "deviceName": device_name,
        "state": run_phase(&raw_state),
        "rawState": raw_state,
        "startedAt": started,
        "endedAt": time(o.get("endedAt")).unwrap_or(Value::Null),
        "appId": app_id,
        "lastError": cleaned(o.get("lastError"), DETAIL_CHARS).map(Value::from).unwrap_or(Value::Null),
    }))
}

fn config(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    if !is_mcp_run_id(o.get("id")) {
        return None;
    }
    let kind = one_of(&KINDS, o.get("type"))?;
    let id = o["id"].as_str()?;
    let mut out = json!({ "id": id, "name": name(o.get("name")).unwrap_or_else(|| id.to_string()), "type": kind, "selected": o.get("selected") == Some(&json!(true)) });
    if is_mcp_run_id(o.get("device")) {
        out["device"] = o["device"].clone();
    }
    Some(out)
}

fn device(v: &Value) -> Option<Value> {
    static P: OnceLock<Regex> = OnceLock::new();
    let o = v.as_object()?;
    if !is_mcp_run_id(o.get("serial")) {
        return None;
    }
    let state = one_of(&DEVICE_STATES, o.get("state"))?;
    let serial = o["serial"].as_str()?;
    let platform = if matches_re(&P, r"^[a-z0-9_-]{1,40}$", o.get("platform")) { o["platform"].clone() } else { json!("unknown") };
    Some(json!({ "serial": serial, "name": name(o.get("name")).unwrap_or_else(|| serial.to_string()), "platform": platform, "state": state, "selected": o.get("selected") == Some(&json!(true)) }))
}

fn log_line(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let stream = one_of(&STREAMS, o.get("stream"))?;
    let text = cleaned(o.get("text"), LINE_CHARS)?;
    let mut out = json!({ "stream": stream });
    if let Some(l) = one_of(&LEVELS, o.get("level")) {
        out["level"] = json!(l);
    }
    out["text"] = json!(text);
    Some(out)
}

fn build_error(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let file = cleaned(o.get("file"), 1024).filter(|f| !f.is_empty())?;
    let message = cleaned(o.get("message"), DETAIL_CHARS).filter(|m| !m.is_empty())?;
    let line = time(o.get("line"))?;
    let col = time(o.get("col"))?;
    let severity = if o.get("severity") == Some(&json!("warning")) { "warning" } else { "error" };
    Some(json!({ "file": file, "line": line, "col": col, "message": message, "severity": severity }))
}

fn shot(v: Option<&Value>) -> Option<Value> {
    let o = v?.as_object()?;
    let path = o.get("path")?.as_str()?;
    if u16_len(path) > 4096 || path.contains(['\0', '\r', '\n']) || !path.to_ascii_lowercase().ends_with(".png") {
        return None;
    }
    let width = time_f(o.get("width")).filter(|w| *w != 0.0)?;
    let height = time_f(o.get("height")).filter(|h| *h != 0.0)?;
    if !is_mcp_run_id(o.get("serial")) {
        return None;
    }
    let serial = o["serial"].as_str()?;
    let saved = if o.get("saved") == Some(&json!("capture")) { "capture" } else { "temp" };
    Some(json!({ "path": path, "width": jnum(width), "height": jnum(height), "serial": serial, "name": name(o.get("name")).unwrap_or_else(|| serial.to_string()), "saved": saved }))
}

pub fn parse_mcp_run_result(raw: &Map<String, Value>, tool: &str) -> Option<Value> {
    if tool == "run_app" {
        if !is_mcp_run_action(raw.get("action")) {
            return None;
        }
        let mut out = json!({ "ok": true, "tool": tool, "action": raw["action"] });
        if let Some(r) = refusal(raw.get("refused")) {
            out["refused"] = r;
        }
        out["session"] = session(raw.get("session")).unwrap_or(Value::Null);
        return Some(out);
    }
    if tool == "run_status" {
        let focused = raw.get("focused").and_then(Value::as_str).filter(|f| f.encode_utf16().count() <= 64);
        return Some(json!({
            "ok": true,
            "tool": tool,
            "configs": list(raw.get("configs"), CONFIGS_MAX).into_iter().filter_map(config).collect::<Vec<_>>(),
            "devices": list(raw.get("devices"), DEVICES_MAX).into_iter().filter_map(device).collect::<Vec<_>>(),
            "sessions": list(raw.get("sessions"), SESSIONS_MAX).into_iter().filter_map(|s| session(Some(s))).collect::<Vec<_>>(),
            "focused": focused,
        }));
    }
    if tool == "run_log" {
        let mut out = json!({
            "ok": true,
            "tool": tool,
            "session": session(raw.get("session")).unwrap_or(Value::Null),
            "lines": list(raw.get("lines"), MCP_LOG_LINES_MAX as usize).into_iter().filter_map(log_line).collect::<Vec<_>>(),
            "errors": list(raw.get("errors"), ERRORS_MAX).into_iter().filter_map(build_error).collect::<Vec<_>>(),
            "partial": raw.get("partial") == Some(&json!(true)),
        });
        if is_mcp_log_cursor(raw.get("cursor")) {
            out["cursor"] = raw["cursor"].clone();
        }
        if raw.get("reset") == Some(&json!(true)) {
            out["reset"] = json!(true);
        }
        if raw.get("truncated") == Some(&json!(true)) {
            out["truncated"] = json!(true);
        }
        return Some(out);
    }
    if let Some(refused) = refusal(raw.get("refused")) {
        let devices: Vec<Value> = list(raw.get("devices"), DEVICES_MAX)
            .into_iter()
            .filter_map(|d| {
                let o = d.as_object()?;
                if !is_mcp_run_id(o.get("serial")) {
                    return None;
                }
                let serial = o["serial"].as_str()?;
                Some(json!({ "serial": serial, "name": name(o.get("name")).unwrap_or_else(|| serial.to_string()) }))
            })
            .collect();
        let mut out = json!({ "ok": true, "tool": tool, "refused": refused });
        if !devices.is_empty() {
            out["devices"] = json!(devices);
        }
        return Some(out);
    }
    shot(raw.get("shot")).map(|s| json!({ "ok": true, "tool": tool, "shot": s }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn base() -> Value {
        json!({
            "id": "0b6f8f4e-1111-4222-8333-944445555666", "configId": "auto-android-app", "configName": "app", "type": "android",
            "deviceId": "emulator-5554", "deviceName": "Pixel 7", "rawState": "building", "startedAt": 1_700_000_000_000.0,
            "endedAt": null, "appId": "com.example", "lastError": null
        })
    }

    #[test]
    fn ids_and_cursors() {
        assert!(is_mcp_run_id(Some(&json!("emulator-5554"))));
        assert!(is_mcp_run_id(Some(&json!("192.168.1.2:5555"))));
        assert!(!is_mcp_run_id(Some(&json!("-rf"))));
        assert!(!is_mcp_run_id(Some(&json!("a b"))));
        assert!(!is_mcp_run_id(Some(&json!(""))));
        assert!(!is_mcp_run_id(Some(&json!(5))));
        assert!(is_mcp_log_cursor(Some(&json!("abc-1.0.12"))));
        assert!(!is_mcp_log_cursor(Some(&json!("abc-1.0.12\n"))));
        assert!(!is_mcp_log_cursor(Some(&json!("abc.x.1"))));
        assert!(is_mcp_run_action(Some(&json!("reload"))));
        assert!(!is_mcp_run_action(Some(&json!("deploy"))));
    }

    #[test]
    fn log_match_is_a_javascript_regex() {
        assert!(compile_log_match("err(or)?", false));
        assert!(compile_log_match("(?<n>a)\\k<n>", true));
        assert!(!compile_log_match("(", false));
        assert!(!compile_log_match("", false));
        assert!(!compile_log_match(&"a".repeat(201), false));
    }

    #[test]
    fn run_app_keeps_session_and_phase() {
        let r = parse_mcp_run_result(&raw(json!({ "action": "start", "session": base() })), "run_app").unwrap();
        assert_eq!(r["session"]["state"], "starting");
        assert_eq!(r["session"]["rawState"], "building");
        assert_eq!(r["session"]["startedAt"], json!(1_700_000_000_000i64));
        assert_eq!(r["session"]["lastError"], Value::Null);
        assert!(parse_mcp_run_result(&raw(json!({ "action": "x" })), "run_app").is_none());
        let none = parse_mcp_run_result(&raw(json!({ "action": "stop", "session": { "id": "x" }, "refused": { "code": "Bad Code" } })), "run_app").unwrap();
        assert_eq!(none, json!({ "ok": true, "tool": "run_app", "action": "stop", "session": null }));
    }

    #[test]
    fn status_filters_and_caps() {
        let r = parse_mcp_run_result(
            &raw(json!({
                "configs": [{ "id": "a", "name": "\u{1b}[31mApp\u{1b}[0m", "type": "flutter", "selected": true, "device": "x1" }, { "id": "-b", "type": "flutter" }, { "id": "c", "type": "nope" }],
                "devices": [{ "serial": "s1", "state": "ready", "platform": "Bad Platform" }, { "serial": "s2", "state": "weird" }],
                "sessions": [base()],
                "focused": "x".repeat(65)
            })),
            "run_status",
        )
        .unwrap();
        assert_eq!(r["configs"], json!([{ "id": "a", "name": "App", "type": "flutter", "selected": true, "device": "x1" }]));
        assert_eq!(r["devices"], json!([{ "serial": "s1", "name": "s1", "platform": "unknown", "state": "ready", "selected": false }]));
        assert_eq!(r["focused"], Value::Null);
        assert_eq!(r["sessions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn log_lines_errors_and_flags() {
        let r = parse_mcp_run_result(
            &raw(json!({
                "session": base(),
                "lines": [{ "stream": "app", "level": "E", "text": "boom\t" }, { "stream": "bad", "text": "x" }, { "stream": "build", "level": "Z", "text": "   " }, { "stream": "build", "level": "Z", "text": "ok" }],
                "errors": [{ "file": "a.kt", "line": 3.7, "col": 0, "message": "m", "severity": "warning" }, { "file": "", "line": 1, "col": 1, "message": "m" }],
                "cursor": "r-1.0.5", "reset": true, "truncated": false
            })),
            "run_log",
        )
        .unwrap();
        assert_eq!(r["lines"], json!([{ "stream": "app", "level": "E", "text": "boom" }, { "stream": "build", "text": "ok" }]));
        assert_eq!(r["errors"], json!([{ "file": "a.kt", "line": 3, "col": 0, "message": "m", "severity": "warning" }]));
        assert_eq!(r["partial"], false);
        assert_eq!(r["cursor"], "r-1.0.5");
        assert_eq!(r["reset"], true);
        assert!(r.get("truncated").is_none());
    }

    #[test]
    fn screenshot_branches() {
        let refused = parse_mcp_run_result(&raw(json!({ "refused": { "code": "several", "detail": "d" }, "devices": [{ "serial": "a", "name": "A" }, { "serial": "-x" }] })), "device_screenshot").unwrap();
        assert_eq!(refused["refused"], json!({ "code": "several", "detail": "d" }));
        assert_eq!(refused["devices"], json!([{ "serial": "a", "name": "A" }]));
        let ok = parse_mcp_run_result(&raw(json!({ "shot": { "path": "/t/a.PNG", "width": 10, "height": 20, "serial": "s", "saved": "capture" } })), "device_screenshot").unwrap();
        assert_eq!(ok["shot"], json!({ "path": "/t/a.PNG", "width": 10, "height": 20, "serial": "s", "name": "s", "saved": "capture" }));
        assert!(parse_mcp_run_result(&raw(json!({ "shot": { "path": "/t/a.jpg", "width": 10, "height": 20, "serial": "s" } })), "device_screenshot").is_none());
        assert!(parse_mcp_run_result(&raw(json!({ "shot": { "path": "/t/a.png", "width": 0, "height": 20, "serial": "s" } })), "device_screenshot").is_none());
    }
}
