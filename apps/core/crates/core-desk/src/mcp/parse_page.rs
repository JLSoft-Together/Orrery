use super::parse_run::parse_mcp_run_result;
use super::text::{clean_note_text, is_agent_session_id, is_preset, u16_len, u16_slice};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

const ERRORS: [&str; 6] = ["unavailable", "caller", "failed", "limit", "target", "busy"];
const SESSIONS_MAX: usize = 16;
const EDITORS_MAX: usize = 16;
const RUN_TOOLS: [&str; 4] = ["run_app", "run_status", "run_log", "device_screenshot"];

fn rx(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).unwrap())
}

fn status_ok(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    rx(&R, r"^[a-z]{1,20}$").is_match(s)
}

fn ref_ok(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    !s.starts_with('-') && !s.contains("..") && rx(&R, r"^[A-Za-z0-9._/-]{1,255}$").is_match(s)
}

fn window_ok(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    rx(&R, r"^[a-z0-9_]{1,40}$").is_match(s)
}

fn str_of(v: Option<&Value>, max: usize) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| u16_len(s) <= max)
}

fn name_of(v: Option<&Value>) -> Option<String> {
    let t = clean_note_text(str_of(v, 400)?)?;
    let t = u16_slice(&t, 80);
    (!t.is_empty()).then_some(t)
}

fn iso(v: Option<&Value>) -> Option<&str> {
    str_of(v, 40).filter(|t| !t.is_empty() && core_transcript::time::parse_iso(t).is_some())
}

fn editor(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let name = name_of(o.get("name"))?;
    let root = str_of(o.get("root"), 4096).filter(|s| !s.is_empty())?;
    let file = str_of(o.get("file"), 4096).filter(|s| !s.is_empty())?;
    let agent = o.get("agent").and_then(Value::as_str).filter(|a| is_preset(a))?;
    if format!("{root}{file}").contains(['\0', '\r', '\n']) {
        return None;
    }
    let mut out = Map::new();
    out.insert("name".into(), json!(name));
    out.insert("agent".into(), json!(agent));
    if let Some(s) = str_of(o.get("status"), 20).filter(|s| !s.is_empty() && status_ok(s)) {
        out.insert("status".into(), json!(s));
    }
    let branch = str_of(o.get("branch"), 255).filter(|b| !b.is_empty() && ref_ok(b));
    out.insert("branch".into(), branch.map_or(Value::Null, |b| json!(b)));
    out.insert("root".into(), json!(root));
    out.insert("sameCheckout".into(), json!(o.get("sameCheckout") == Some(&json!(true))));
    out.insert("file".into(), json!(file));
    if let Some(at) = str_of(o.get("lastActivityAt"), 40).filter(|s| !s.is_empty()) {
        out.insert("lastActivityAt".into(), json!(at));
    }
    Some(Value::Object(out))
}

fn session_ref(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let session_id = str_of(o.get("sessionId"), 200).filter(|s| !s.is_empty())?;
    let name = name_of(o.get("name"))?;
    let status = str_of(o.get("status"), 20).filter(|s| !s.is_empty() && status_ok(s))?;
    let agent = o.get("agent").and_then(Value::as_str).filter(|a| is_preset(a))?;
    let mut out = Map::new();
    out.insert("sessionId".into(), json!(session_id));
    if let Some(a) = o.get("agentSessionId").and_then(Value::as_str).filter(|a| is_agent_session_id(a)) {
        out.insert("agentSessionId".into(), json!(a));
    }
    out.insert("name".into(), json!(name));
    out.insert("agent".into(), json!(agent));
    out.insert("status".into(), json!(status));
    out.insert("self".into(), json!(o.get("self") == Some(&json!(true))));
    out.insert("openedByYou".into(), json!(o.get("openedByYou") == Some(&json!(true))));
    if let Some(l) = o.get("limit").and_then(Value::as_object) {
        let mut lim = Map::new();
        if let Some(r) = iso(l.get("resetsAt")) {
            lim.insert("resetsAt".into(), json!(r));
        }
        if let Some(w) = str_of(l.get("window"), 40).filter(|w| !w.is_empty() && window_ok(w)) {
            lim.insert("window".into(), json!(w));
        }
        out.insert("limit".into(), Value::Object(lim));
    }
    if let Some(i) = iso(o.get("idleSince")) {
        out.insert("idleSince".into(), json!(i));
    }
    Some(Value::Object(out))
}

fn failed() -> Value {
    json!({ "ok": false, "error": "failed" })
}

fn list(raw: &Map<String, Value>, key: &str, max: usize, f: fn(&Value) -> Option<Value>) -> Vec<Value> {
    raw.get(key).and_then(Value::as_array).map(|a| a.iter().take(max).filter_map(f).collect()).unwrap_or_default()
}

pub fn parse_mcp_page_result(raw: &Value, tool: &str) -> Value {
    let Some(r) = raw.as_object() else { return failed() };
    if r.get("ok") != Some(&json!(true)) {
        let e = r.get("error").and_then(Value::as_str).filter(|e| ERRORS.contains(e)).unwrap_or("failed");
        return json!({ "ok": false, "error": e });
    }
    if RUN_TOOLS.contains(&tool) {
        return parse_mcp_run_result(r, tool).unwrap_or_else(failed);
    }
    match tool {
        "notify_owner" => {
            let Some(name) = str_of(r.get("name"), 120).filter(|s| !s.is_empty()) else { return failed() };
            let mut out = json!({ "ok": true, "tool": tool, "name": name });
            if let Some(t) = str_of(r.get("terminalId"), 200).filter(|s| !s.is_empty()) {
                out["terminalId"] = json!(t);
            }
            out["inbox"] = json!(r.get("inbox") == Some(&json!(true)));
            out
        }
        "hand_back" => match name_of(r.get("origin")) {
            Some(origin) => json!({ "ok": true, "tool": tool, "origin": origin }),
            None => failed(),
        },
        "list_sessions" => json!({ "ok": true, "tool": tool, "sessions": list(r, "sessions", SESSIONS_MAX, session_ref) }),
        "send_to_session" | "close_session" => {
            let Some(name) = name_of(r.get("name")) else { return failed() };
            if tool == "send_to_session" {
                json!({ "ok": true, "tool": tool, "name": name })
            } else {
                json!({ "ok": true, "tool": tool, "name": name, "confirm": r.get("confirm") == Some(&json!(true)) })
            }
        }
        "open_in_workspace" => match str_of(r.get("paneKey"), 100).filter(|s| !s.is_empty()) {
            Some(k) => json!({ "ok": true, "tool": tool, "paneKey": k, "reused": r.get("reused") == Some(&json!(true)) }),
            None => failed(),
        },
        _ => json!({ "ok": true, "tool": tool, "repo": r.get("repo") == Some(&json!(true)), "editors": list(r, "editors", EDITORS_MAX, editor) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_answers_are_checked() {
        assert_eq!(parse_mcp_page_result(&json!({ "ok": false, "error": "nope" }), "list_sessions"), failed());
        let s = parse_mcp_page_result(&json!({ "ok": true, "sessions": [{ "sessionId": "p1", "name": " A\u{202e}b ", "agent": "codex", "status": "idle", "idleSince": "2026-11-10T01:02:03.000Z", "limit": { "window": "BAD" } }, { "sessionId": "p2" }] }), "list_sessions");
        assert_eq!(s["sessions"], json!([{ "sessionId": "p1", "name": "Ab", "agent": "codex", "status": "idle", "self": false, "openedByYou": false, "limit": {}, "idleSince": "2026-11-10T01:02:03.000Z" }]));
        let e = parse_mcp_page_result(&json!({ "ok": true, "repo": true, "editors": [{ "name": "x", "agent": "claude", "root": "/r", "file": "/r/a", "branch": "-x" }] }), "who_is_editing");
        assert_eq!(e["editors"][0]["branch"], Value::Null);
        assert_eq!(parse_mcp_page_result(&json!({ "ok": true, "name": "" }), "notify_owner"), failed());
    }
}
