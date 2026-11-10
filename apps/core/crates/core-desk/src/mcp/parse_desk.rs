use super::text::{js_trim, u16_len, u16_slice};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub const DIAG_MAX: usize = 12;
const SERVERS_MAX: usize = 100;
pub const DESK_ERRORS: [&str; 29] = [
    "unavailable", "failed", "starting", "not-layout", "missing", "too-large", "jar-missing", "bundle-missing", "jdk-missing", "jdk-old", "crash-loop", "render-failed", "timeout", "no-report",
    "not-found", "not-mine", "outside", "emulator", "self", "busy", "gone", "cancelled", "confirm-open", "telegram-off", "telegram-config", "no-topic", "not-paired", "rate", "buttons-file",
];

fn failed() -> Value {
    json!({ "ok": false, "error": "failed" })
}

fn int(v: Option<&Value>, lo: i64, hi: i64) -> Option<i64> {
    let f = v?.as_f64()?;
    if !f.is_finite() || f.fract() != 0.0 {
        return None;
    }
    let i = f as i64;
    (i >= lo && i <= hi).then_some(i)
}

fn strv(v: Option<&Value>, max: usize) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| u16_len(s) <= max).map(str::to_string)
}

fn line(v: Option<&Value>, max: usize) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    let s = v?.as_str()?;
    let re = R.get_or_init(|| Regex::new(r"[\x00-\x1f\x7f-\x{9f}\x{2028}\x{2029}\x{202a}-\x{202e}\x{2066}-\x{2069}]+").unwrap());
    let flat = re.replace_all(s, " ");
    let t = js_trim(&flat);
    (!t.is_empty()).then(|| u16_slice(t, max)).filter(|t| !t.is_empty())
}

fn arr(v: Option<&Value>, max: usize) -> Vec<&Value> {
    v.and_then(Value::as_array).map(|a| a.iter().take(max).collect()).unwrap_or_default()
}

fn parse_server(v: &Value) -> Option<Value> {
    static K: OnceLock<Regex> = OnceLock::new();
    let o = v.as_object()?;
    let pid = int(o.get("pid"), 1, 0x7fff_ffff).filter(|p| *p != 0)?;
    let name = line(o.get("name"), 120)?;
    let kind = strv(o.get("kind"), 20)?;
    let ports = o.get("ports")?.as_array()?;
    if !K.get_or_init(|| Regex::new(r"^[a-z-]+$").unwrap()).is_match(&kind) {
        return None;
    }
    let ports: Vec<Value> = ports.iter().take(32).filter_map(|p| int(Some(p), 1, 65535).map(Value::from)).collect();
    let mine = o.get("mine") == Some(&json!(true));
    let cmd = if mine { line(o.get("cmd"), 200) } else { None };
    let idle = if o.get("idleSec") == Some(&Value::Null) { None } else { int(o.get("idleSec"), 0, 10 * 365 * 86400) };
    let mut out = json!({ "pid": pid, "name": name, "kind": kind, "ports": ports, "mine": mine });
    if let Some(c) = cmd {
        out["cmd"] = json!(c);
    }
    out["idleSec"] = idle.map(Value::from).unwrap_or(Value::Null);
    out["orphan"] = json!(o.get("orphan") == Some(&json!(true)));
    Some(out)
}

fn parse_layout(o: &serde_json::Map<String, Value>, tool: &str) -> Value {
    let Some(png) = strv(o.get("png"), 4096).filter(|p| !p.is_empty()) else { return failed() };
    let width = int(o.get("width"), 1, 16384);
    let height = int(o.get("height"), 1, 16384);
    let (Some(width), Some(height)) = (width, height) else { return failed() };
    if png.contains(['\0', '\r', '\n']) || !png.to_ascii_lowercase().ends_with(".png") {
        return failed();
    }
    let diagnostics: Vec<Value> = arr(o.get("diagnostics"), DIAG_MAX)
        .into_iter()
        .filter_map(|d| {
            let d = d.as_object()?;
            let message = line(d.get("message"), 300)?;
            let severity = match d.get("severity").and_then(Value::as_str) {
                Some("error") => "error",
                Some("warning") => "warning",
                _ => "info",
            };
            let mut out = json!({ "severity": severity, "message": message });
            if let Some(at) = int(d.get("line"), 1, 10_000_000) {
                out["line"] = json!(at);
            }
            Some(out)
        })
        .collect();
    json!({
        "ok": true, "tool": tool, "png": png, "width": width, "height": height,
        "renderMs": int(o.get("renderMs"), 0, 600_000).unwrap_or(0),
        "frameworkOnly": o.get("frameworkOnly") == Some(&json!(true)),
        "diagnostics": diagnostics,
    })
}

pub fn parse_mcp_desk_result(raw: &Value, tool: &str) -> Value {
    let Some(o) = raw.as_object() else { return failed() };
    if o.get("ok") != Some(&json!(true)) {
        let error = o.get("error").and_then(Value::as_str).filter(|e| DESK_ERRORS.contains(e)).unwrap_or("failed");
        let mut out = json!({ "ok": false, "error": error });
        if let Some(r) = int(o.get("retryInSec"), 1, 600) {
            out["retryInSec"] = json!(r);
        }
        if let Some(d) = line(o.get("detail"), 300) {
            out["detail"] = json!(d);
        }
        return out;
    }
    match tool {
        "preview_layout" => parse_layout(o, tool),
        "list_servers" => {
            let servers: Vec<Value> = arr(o.get("servers"), SERVERS_MAX).into_iter().filter_map(parse_server).collect();
            let outside: Vec<Value> = arr(o.get("outside"), SERVERS_MAX)
                .into_iter()
                .filter_map(|v| {
                    let v = v.as_object()?;
                    let port = int(v.get("port"), 1, 65535)?;
                    let name = line(v.get("name"), 120)?;
                    Some(json!({ "port": port, "name": name }))
                })
                .collect();
            json!({ "ok": true, "tool": tool, "at": strv(o.get("at"), 40).unwrap_or_default(), "servers": servers, "outside": outside })
        }
        "telegram_notify" => {
            let projects: Vec<Value> = arr(o.get("projects"), 100).into_iter().filter_map(|p| line(Some(p), 120)).map(Value::from).collect();
            json!({
                "ok": true, "tool": tool, "enabled": o.get("enabled") == Some(&json!(true)), "configured": o.get("configured") == Some(&json!(true)),
                "projects": projects, "routed": o.get("routed") == Some(&json!(true)),
            })
        }
        "telegram_receive" => {
            let mut out = json!({ "ok": true, "tool": tool, "receiving": o.get("receiving") == Some(&json!(true)), "paired": o.get("paired") == Some(&json!(true)) });
            if let Some(owner) = line(o.get("owner"), 80) {
                out["owner"] = json!(owner);
            }
            out
        }
        "telegram_send" => {
            let Some(topic) = line(o.get("topic"), 120) else { return failed() };
            let mut out = json!({ "ok": true, "tool": tool, "topic": topic });
            if let Some(f) = line(o.get("file"), 200) {
                out["file"] = json!(f);
            }
            if o.get("linkOnly") == Some(&json!(true)) {
                out["linkOnly"] = json!(true);
            }
            if o.get("uploading") == Some(&json!(true)) {
                out["uploading"] = json!(true);
            }
            if let Some(b) = int(o.get("buttons"), 1, 6) {
                out["buttons"] = json!(b);
            }
            out
        }
        _ => {
            let (Some(port), Some(stopped)) = (int(o.get("port"), 1, 65535), o.get("stopped").and_then(Value::as_array)) else { return failed() };
            let stopped: Vec<Value> = stopped
                .iter()
                .take(50)
                .filter_map(|s| {
                    let s = s.as_object()?;
                    let pid = int(s.get("pid"), 1, 0x7fff_ffff)?;
                    let name = line(s.get("name"), 120)?;
                    Some(json!({ "pid": pid, "name": name }))
                })
                .collect();
            json!({ "ok": true, "tool": tool, "port": port, "stopped": stopped })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_whitelisted() {
        assert_eq!(parse_mcp_desk_result(&json!(5), "free_port"), failed());
        assert_eq!(parse_mcp_desk_result(&json!({ "ok": false, "error": "weird" }), "free_port"), failed());
        let r = parse_mcp_desk_result(&json!({ "ok": false, "error": "starting", "retryInSec": 12, "detail": " a\nb " }), "preview_layout");
        assert_eq!(r, json!({ "ok": false, "error": "starting", "retryInSec": 12, "detail": "a b" }));
        let r = parse_mcp_desk_result(&json!({ "ok": false, "error": "busy", "retryInSec": 601.5 }), "preview_layout");
        assert_eq!(r, json!({ "ok": false, "error": "busy" }));
    }

    #[test]
    fn layout_checks_png_and_caps_diagnostics() {
        let diags: Vec<Value> = (0..20).map(|i| json!({ "severity": "error", "message": format!("m{i}"), "line": i })).collect();
        let r = parse_mcp_desk_result(&json!({ "ok": true, "png": "/t/a.PNG", "width": 10, "height": 20, "renderMs": 5.5, "diagnostics": diags }), "preview_layout");
        assert_eq!(r["diagnostics"].as_array().unwrap().len(), 12);
        assert_eq!(r["diagnostics"][0], json!({ "severity": "error", "message": "m0" }));
        assert_eq!(r["diagnostics"][1]["line"], 1);
        assert_eq!(r["renderMs"], 0);
        assert_eq!(r["frameworkOnly"], false);
        assert_eq!(parse_mcp_desk_result(&json!({ "ok": true, "png": "/t/a.jpg", "width": 1, "height": 1 }), "preview_layout"), failed());
        assert_eq!(parse_mcp_desk_result(&json!({ "ok": true, "png": "/t/a.png", "width": 0, "height": 1 }), "preview_layout"), failed());
    }

    #[test]
    fn servers_and_outside() {
        let raw = json!({
            "ok": true, "at": "2026-01-01T00:00:00.000Z",
            "servers": [
                { "pid": 10, "name": "vite", "kind": "dev-server", "ports": [5173, 0, 70000, 3000], "mine": true, "cmd": "node vite", "idleSec": null, "orphan": false },
                { "pid": 11, "name": "x", "kind": "Bad", "ports": [] },
                { "pid": 12, "name": "y", "kind": "daemon", "ports": [], "cmd": "hidden", "idleSec": 7.5 }
            ],
            "outside": [{ "port": 80, "name": "nginx" }, { "port": 0, "name": "z" }]
        });
        let r = parse_mcp_desk_result(&raw, "list_servers");
        assert_eq!(r["servers"][0], json!({ "pid": 10, "name": "vite", "kind": "dev-server", "ports": [5173, 3000], "mine": true, "cmd": "node vite", "idleSec": null, "orphan": false }));
        assert_eq!(r["servers"][1], json!({ "pid": 12, "name": "y", "kind": "daemon", "ports": [], "mine": false, "idleSec": null, "orphan": false }));
        assert_eq!(r["servers"].as_array().unwrap().len(), 2);
        assert_eq!(r["outside"], json!([{ "port": 80, "name": "nginx" }]));
    }

    #[test]
    fn telegram_and_free_port() {
        let r = parse_mcp_desk_result(&json!({ "ok": true, "enabled": true, "projects": ["a", "", 5, "b"], "routed": true }), "telegram_notify");
        assert_eq!(r, json!({ "ok": true, "tool": "telegram_notify", "enabled": true, "configured": false, "projects": ["a", "b"], "routed": true }));
        let r = parse_mcp_desk_result(&json!({ "ok": true, "receiving": true, "paired": true, "owner": "@me" }), "telegram_receive");
        assert_eq!(r["owner"], "@me");
        assert_eq!(parse_mcp_desk_result(&json!({ "ok": true }), "telegram_send"), failed());
        let r = parse_mcp_desk_result(&json!({ "ok": true, "topic": "T", "file": "a.apk", "linkOnly": true, "buttons": 9 }), "telegram_send");
        assert_eq!(r, json!({ "ok": true, "tool": "telegram_send", "topic": "T", "file": "a.apk", "linkOnly": true }));
        let r = parse_mcp_desk_result(&json!({ "ok": true, "port": 3000, "stopped": [{ "pid": 4, "name": "node" }, { "pid": 0, "name": "x" }] }), "free_port");
        assert_eq!(r, json!({ "ok": true, "tool": "free_port", "port": 3000, "stopped": [{ "pid": 4, "name": "node" }] }));
        assert_eq!(parse_mcp_desk_result(&json!({ "ok": true, "port": 3000 }), "free_port"), failed());
    }
}
