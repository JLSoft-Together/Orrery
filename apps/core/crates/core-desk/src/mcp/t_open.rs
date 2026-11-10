use super::host::{tool_result, Args, ToolHost};
use serde_json::{json, Value};

fn label(preset: &str) -> &'static str {
    match preset {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "deepseek" => "DeepSeek (Codewhale)",
        _ => "a shell",
    }
}

pub const PANE_ERRORS: [&str; 9] = ["limit", "cwd", "rate", "unavailable", "preset", "failed", "resume", "live", "setup"];

fn error_text(code: &str) -> &'static str {
    match code {
        "limit" => "Orrery already runs 16 terminals; the user has to close one first.",
        "cwd" => "That folder does not exist or is not allowed; cwd must be an absolute path to an existing local folder.",
        "rate" => "Too many sessions were opened from this pane in the last minute; wait before opening another one.",
        "unavailable" => "The Orrery window is not ready to open a pane right now.",
        "preset" => "Unknown agent; use claude, codex or shell (deepseek only while the user has it turned on).",
        "resume" => "resume must be the UUID of a Claude Code, Codex or DeepSeek session.",
        "live" => "That session is still running in another terminal; it has to be closed there first, two instances would write into the same transcript.",
        "setup" => "DeepSeek (Codewhale) is not installed or not logged in on this machine; Orrery showed the user a notice with a button to the setup page. Do the task yourself or ask the user to finish the setup, do not open an external terminal.",
        _ => "Orrery could not open the pane.",
    }
}

pub fn open_text(o: &Value) -> String {
    if o["ok"] != json!(true) {
        return format!("{} Do not open an external terminal window instead unless the user asks for it.", error_text(o["error"].as_str().unwrap_or("failed")));
    }
    let s = |k: &str| o[k].as_str().filter(|v| !v.is_empty());
    let id = s("sessionId").map(|v| format!(" Session id: {v}.")).unwrap_or_default();
    let pane = s("paneId").map(|v| format!(" Pane id: {v} (pass it to send_to_session, close_session and wait_session).")).unwrap_or_default();
    let prompt = if o["submitted"] == json!(true) {
        " The prompt is sent as soon as the agent is ready; it works on it without the user."
    } else if o["prompt"] == json!(true) {
        " The prompt is waiting in its input box; the user presses Enter to send it."
    } else {
        ""
    };
    let preset = label(o["preset"].as_str().unwrap_or(""));
    let cwd = o["cwd"].as_str().unwrap_or("");
    if o["resumed"] == json!(true) {
        let sid = o["sessionId"].as_str().unwrap_or("undefined");
        return format!("Reopened {preset} session {sid} in {cwd} as an Orrery pane.{pane}{prompt}");
    }
    format!("Opened {preset} in {cwd} as a new Orrery pane next to this session.{id}{pane}{prompt}")
}

pub fn to_open_result(v: &Value) -> Option<Value> {
    let r = v.as_object()?;
    if r.get("ok") == Some(&json!(true)) {
        return Some(json!({ "ok": true, "paneKey": r.get("paneKey").and_then(Value::as_str).unwrap_or("") }));
    }
    let e = r.get("error").and_then(Value::as_str)?;
    if r.get("ok") != Some(&json!(false)) {
        return None;
    }
    Some(json!({ "ok": false, "error": if PANE_ERRORS.contains(&e) { e } else { "failed" } }))
}

pub fn open_session(args: &Args, host: &dyn ToolHost) -> Value {
    let outcome = host.open_session(args);
    let outcome = if outcome.is_object() { outcome } else { json!({ "ok": false, "error": "failed" }) };
    let err = outcome["ok"] != json!(true);
    tool_result(&open_text(&outcome), outcome, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_match_the_ts_wording() {
        let o = json!({ "ok": true, "preset": "codex", "cwd": "/p", "paneId": "x1", "prompt": true, "submitted": true });
        assert_eq!(open_text(&o), "Opened Codex in /p as a new Orrery pane next to this session. Pane id: x1 (pass it to send_to_session, close_session and wait_session). The prompt is sent as soon as the agent is ready; it works on it without the user.");
        let r = json!({ "ok": true, "preset": "claude", "cwd": "/p", "sessionId": "u", "prompt": false, "resumed": true });
        assert_eq!(open_text(&r), "Reopened Claude Code session u in /p as an Orrery pane.");
        assert!(open_text(&json!({ "ok": false, "error": "weird" })).starts_with("Orrery could not open the pane. Do not"));
        assert_eq!(to_open_result(&json!({ "ok": false, "error": "zzz" })), Some(json!({ "ok": false, "error": "failed" })));
        assert_eq!(to_open_result(&json!({ "ok": true })), Some(json!({ "ok": true, "paneKey": "" })));
        assert_eq!(to_open_result(&json!(3)), None);
    }
}
