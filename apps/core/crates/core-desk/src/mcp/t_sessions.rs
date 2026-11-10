use super::host::{tool_result, Args, ToolHost};
use super::t_common::{ask, as_int, js_round, jnum, page_error, str_of, truthy};
use super::text::{clean_pane_prompt, js_trim, quote, u16_len, PANE_OPEN_PROMPT_MAX};
use serde_json::{json, Value};

pub const SEND_RATE_MAX: usize = 20;
pub const SEND_RATE_MS: u64 = 60_000;
pub const WAIT_RATE_MAX: usize = 30;
pub const WAIT_DEFAULT_MS: i64 = 120_000;
pub const WAIT_MIN_MS: i64 = 1000;
pub const WAIT_MAX_MS: i64 = 600_000;
pub const WAIT_POLL_MS: f64 = 2000.0;
const SESSION_ID_MAX: usize = 200;

fn agent(preset: &str) -> &'static str {
    match preset {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "deepseek" => "DeepSeek",
        _ => "shell",
    }
}

fn error_text(code: &str) -> String {
    match code {
        "rate" => "Too many calls of this tool from this session in the last minute; wait a little.".to_string(),
        "session" => "session must be a session id from list_sessions.".to_string(),
        "text" => format!("text must be 1 to {PANE_OPEN_PROMPT_MAX} characters of plain text."),
        "until" => "until must be 'idle' or 'exited'.".to_string(),
        "timeoutMs" => format!("timeoutMs must be a whole number from {WAIT_MIN_MS} to {WAIT_MAX_MS}."),
        other => page_error(other).to_string(),
    }
}

fn fail(error: &str) -> Value {
    tool_result(&error_text(error), json!({ "ok": false, "error": error }), true)
}

fn id_of(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    let n = u16_len(s);
    (n > 0 && n <= SESSION_ID_MAX && !s.contains('\0')).then(|| s.to_string())
}

fn limit_text(l: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(w) = l.get("window").and_then(Value::as_str).filter(|w| !w.is_empty()) {
        parts.push(format!("window {w}"));
    }
    if let Some(r) = l.get("resetsAt").and_then(Value::as_str).filter(|r| !r.is_empty()) {
        parts.push(format!("resets {r}"));
    }
    let joined = parts.join(", ");
    format!("at usage limit{}", if joined.is_empty() { String::new() } else { format!(" ({joined})") })
}

fn line(s: &Value) -> String {
    let mut flags: Vec<String> = Vec::new();
    if s.get("self") == Some(&json!(true)) {
        flags.push("this is you".into());
    }
    if s.get("openedByYou") == Some(&json!(true)) {
        flags.push("opened by you".into());
    }
    if let Some(i) = s.get("idleSince").and_then(Value::as_str).filter(|i| !i.is_empty()) {
        flags.push(format!("idle since {i}"));
    }
    if truthy(s.get("limit")) {
        flags.push(limit_text(&s["limit"]));
    }
    let flags = if flags.is_empty() { String::new() } else { format!(", {}", flags.join(", ")) };
    let agent_session = match s.get("agentSessionId").and_then(Value::as_str).filter(|a| !a.is_empty()) {
        Some(a) => format!(", agent session {a}"),
        None => String::new(),
    };
    format!("- {} ({}, {}{flags}) id {}{agent_session}", quote(str_of(s, "name")), agent(str_of(s, "agent")), str_of(s, "status"), str_of(s, "sessionId"))
}

fn page_failed(res: &Value) -> String {
    str_of(res, "error").to_string()
}

pub fn list_sessions(_args: &Args, host: &dyn ToolHost) -> Value {
    let res = ask(host, json!({ "tool": "list_sessions" }));
    if res.get("ok") != Some(&json!(true)) {
        return fail(&page_failed(&res));
    }
    if str_of(&res, "tool") != "list_sessions" {
        return fail("failed");
    }
    let sessions = res.get("sessions").and_then(Value::as_array).cloned().unwrap_or_default();
    let text = if sessions.is_empty() {
        "No session of this project is open in the workspace.".to_string()
    } else {
        format!("Sessions in this project:\n{}", sessions.iter().map(line).collect::<Vec<_>>().join("\n"))
    };
    tool_result(&text, json!({ "ok": true, "sessions": sessions }), false)
}

pub fn send_to_session(args: &Args, host: &dyn ToolHost) -> Value {
    let Some(session) = id_of(args.get("session")) else { return fail("session") };
    let Some(raw) = args.get("text").and_then(Value::as_str).filter(|t| u16_len(t) <= PANE_OPEN_PROMPT_MAX * 2) else { return fail("text") };
    let Some(text) = clean_pane_prompt(raw) else { return fail("text") };
    if u16_len(js_trim(raw)) > PANE_OPEN_PROMPT_MAX {
        return fail("text");
    }
    if !host.take("send_to_session", SEND_RATE_MAX, SEND_RATE_MS) {
        return fail("rate");
    }
    let res = ask(host, json!({ "tool": "send_to_session", "session": session, "text": text }));
    if res.get("ok") != Some(&json!(true)) {
        return fail(&page_failed(&res));
    }
    if str_of(&res, "tool") != "send_to_session" {
        return fail("failed");
    }
    let name = str_of(&res, "name");
    tool_result(
        &format!("Typed your message into {}. It is waiting in its input box; the user presses Enter to send it.", quote(name)),
        json!({ "ok": true, "session": session, "name": name }),
        false,
    )
}

pub fn close_session(args: &Args, host: &dyn ToolHost) -> Value {
    let Some(session) = id_of(args.get("session")) else { return fail("session") };
    let res = ask(host, json!({ "tool": "close_session", "session": session }));
    if res.get("ok") != Some(&json!(true)) {
        return fail(&page_failed(&res));
    }
    if str_of(&res, "tool") != "close_session" {
        return fail("failed");
    }
    let name = str_of(&res, "name");
    let confirm = res.get("confirm").cloned().unwrap_or(Value::Null);
    let text = if truthy(Some(&confirm)) {
        format!("{} still runs something, so Orrery asked the owner to confirm the close.", quote(name))
    } else {
        format!("Closed {}; the owner can undo it.", quote(name))
    };
    tool_result(&text, json!({ "ok": true, "session": session, "name": name, "confirm": confirm }), false)
}

fn timeout_of(v: Option<&Value>) -> Option<i64> {
    match v {
        None => Some(WAIT_DEFAULT_MS),
        Some(n) if n.is_number() => as_int(Some(n)).filter(|t| (WAIT_MIN_MS..=WAIT_MAX_MS).contains(t)),
        Some(_) => None,
    }
}

pub fn wait_session(args: &Args, host: &dyn ToolHost) -> Value {
    let Some(session) = id_of(args.get("session")) else { return fail("session") };
    let until = match args.get("until") {
        None => "idle",
        Some(Value::String(u)) if u == "idle" || u == "exited" => u.as_str(),
        Some(_) => return fail("until"),
    };
    let Some(timeout_ms) = timeout_of(args.get("timeoutMs")) else { return fail("timeoutMs") };
    if !host.take("wait_session", WAIT_RATE_MAX, SEND_RATE_MS) {
        return fail("rate");
    }
    let started = host.now_ms();
    let mut first = true;
    let mut status = "unknown".to_string();
    loop {
        let res = ask(host, json!({ "tool": "list_sessions" }));
        let ok = res.get("ok") == Some(&json!(true));
        if !ok || str_of(&res, "tool") != "list_sessions" {
            let error = if ok { "failed".to_string() } else { page_failed(&res) };
            if first || error == "caller" {
                return fail(&error);
            }
        } else {
            let rows = res.get("sessions").and_then(Value::as_array).cloned().unwrap_or_default();
            let row = rows.iter().find(|r| str_of(r, "sessionId") == session);
            if row.is_some_and(|r| r.get("self") == Some(&json!(true))) || (first && row.is_none()) {
                return fail("target");
            }
            status = row.map(|r| str_of(r, "status").to_string()).unwrap_or_else(|| "exited".to_string());
            if status == "exited" || (until == "idle" && status == "idle") {
                break;
            }
        }
        first = false;
        let left = timeout_ms as f64 - (host.now_ms() - started);
        if left <= 0.0 {
            break;
        }
        host.sleep_ms(left.min(WAIT_POLL_MS).ceil() as u64);
    }
    let reached = status == until;
    let waited_ms = host.now_ms() - started;
    let secs = js_round(waited_ms / 1000.0);
    let secs = if secs.fract() == 0.0 { format!("{}", secs as i64) } else { format!("{secs}") };
    let text = if reached {
        format!("The session is {until} (waited {secs} s).")
    } else {
        format!("The session is {status}, not {until}, after {secs} s.")
    };
    tool_result(&text, json!({ "ok": true, "tool": "wait_session", "session": session, "status": status, "reached": reached, "waitedMs": jnum(waited_ms) }), false)
}
