use super::host::{tool_result, Args, ToolHost};
use super::text::{clean_note_text, AGENT_NOTE_MAX};
use serde_json::{json, Value};

pub const NOTE_RATE_MAX: usize = 6;
pub const NOTE_RATE_MS: f64 = 60_000.0;

pub fn is_level(v: &str) -> bool {
    matches!(v, "info" | "attention" | "done")
}

pub fn parse_note_args(args: &Args) -> Result<(String, String), &'static str> {
    let level = match args.get("level") {
        None | Some(Value::Null) => "info",
        Some(Value::String(s)) if is_level(s) => s.as_str(),
        _ => return Err("level"),
    };
    let Some(raw) = args.get("text").and_then(Value::as_str) else { return Err("text") };
    if raw.chars().count() > AGENT_NOTE_MAX {
        return Err("text");
    }
    clean_note_text(raw).map(|t| (t, level.to_string())).ok_or("text")
}

fn error_text(code: &str) -> &'static str {
    match code {
        "text" => "text must be one non-empty line of at most 200 characters.",
        "level" => "level must be info, attention or done.",
        "unavailable" => "Orrery is not ready to show notes right now.",
        _ => "Orrery could not deliver the note.",
    }
}

pub fn notify_owner(args: &Args, host: &dyn ToolHost) -> Value {
    let (text, level) = match parse_note_args(args) {
        Ok(v) => v,
        Err(e) => return tool_result(error_text(e), json!({ "ok": false, "error": e }), true),
    };
    let out = host.notify(&text, &level);
    let out = if out.is_object() { out } else { json!({ "ok": false, "error": "failed" }) };
    if out["ok"] != json!(true) {
        let e = out["error"].as_str().unwrap_or("failed").to_string();
        return tool_result(error_text(&e), out, true);
    }
    if out["sent"] != json!(true) {
        let in_ms = out["inMs"].as_f64().unwrap_or(0.0);
        let secs = (in_ms / 1000.0).ceil();
        return tool_result(&format!("More than {NOTE_RATE_MAX} notes a minute from this session: this one is held and shown together with the others in about {secs} s."), out, false);
    }
    let name = out["name"].as_str().unwrap_or("").to_string();
    let text = if out["inbox"] == json!(true) { format!("Note shown to the user in Orrery's Inbox as {name}.") } else { format!("Note delivered to the user as {name}.") };
    tool_result(&text, out, false)
}
