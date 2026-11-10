use super::host::{tool_result, Args, ToolHost};
use super::paths::{resolve_inside, Inside};
use super::t_common::{str_of, truthy};
use super::t_desk::desk_call;
use super::text::js_trim;
use crate::telegram::parse::{is_secret_file_name, parse_buttons};
use serde_json::{json, Value};

pub const TG_NOTIFY_RATE_MAX: usize = 10;
pub const TG_NOTIFY_RATE_MS: u64 = 60_000;
pub const TG_SEND_RATE_MAX: usize = 6;
pub const TG_SEND_RATE_MS: u64 = 60_000;
pub const TG_RECEIVE_RATE_MAX: usize = 10;
pub const TG_RECEIVE_RATE_MS: u64 = 60_000;
pub const TELEGRAM_REPORT_MAX: usize = 3000;

fn state_arg(args: &Args) -> Option<&'static str> {
    let raw = match args.get("state") {
        None | Some(Value::Null) => return Some("status"),
        Some(Value::String(s)) if s.is_empty() => return Some("status"),
        Some(v) => v,
    };
    let s = raw.as_str()?;
    match js_trim(s).to_lowercase().as_str() {
        "on" => Some("on"),
        "off" => Some("off"),
        "status" => Some("status"),
        _ => None,
    }
}

fn notify_text(code: &str) -> &'static str {
    match code {
        "state" => "state must be on, off or status.",
        "rate" => "Too many calls from this session in the last minute; wait a little.",
        "telegram-config" => "No Telegram bot is set up yet: ask the user to add the bot token in Orrery Settings › Notifications › Telegram.",
        "unavailable" => "Orrery cannot change Telegram reports right now.",
        _ => "Orrery could not change Telegram reports.",
    }
}

fn receive_text(code: &str) -> &'static str {
    match code {
        "state" => "state must be on, off or status.",
        "rate" => "Too many calls from this session in the last minute; wait a little.",
        "telegram-config" => "No Telegram bot is set up yet: ask the user to add the bot token in Orrery Settings › Notifications › Telegram.",
        "not-paired" => "No Telegram account is paired yet: in Orrery Settings › Notifications › Telegram the user clicks Pair and sends the code to the bot; only that account can send commands.",
        "cancelled" => "The user did not allow turning on Telegram commands in the confirm dialog; they stay off.",
        "confirm-open" => "Another Orrery confirm dialog is open; try again after the user answers it.",
        "unavailable" => "Orrery cannot change Telegram commands right now.",
        _ => "Orrery could not change Telegram commands.",
    }
}

fn plain_fail(text: &str, error: &str) -> Value {
    tool_result(text, json!({ "ok": false, "error": error }), true)
}

pub fn notify_summary(res: &Value) -> String {
    let enabled = truthy(res.get("enabled"));
    let projects: Vec<&str> = res.get("projects").and_then(Value::as_array).map(|p| p.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let mut lines = vec![format!("Telegram reports are {} for the whole app.", if enabled { "ON" } else { "OFF" })];
    lines.push(if projects.is_empty() {
        "No project has a Telegram topic yet; the user assigns them in Orrery Settings › Notifications › Telegram.".to_string()
    } else {
        format!("Projects with a topic: {}.", projects.join(", "))
    });
    if enabled {
        lines.push(if truthy(res.get("routed")) {
            "This session's project has a topic: when a task finishes, send a short result summary (and any build file) with telegram_send.".to_string()
        } else {
            "This session's project has no topic, so it is not reported.".to_string()
        });
    }
    lines.join("\n")
}

pub fn telegram_notify(args: &Args, host: &dyn ToolHost) -> Value {
    let fail = |e: &str| plain_fail(notify_text(e), e);
    let Some(state) = state_arg(args) else { return fail("state") };
    if !host.take("telegram-notify", TG_NOTIFY_RATE_MAX, TG_NOTIFY_RATE_MS) {
        return fail("rate");
    }
    let Some(res) = desk_call(host, json!({ "tool": "telegram_notify", "state": state })) else { return fail("unavailable") };
    if res.get("ok") != Some(&json!(true)) {
        return fail(str_of(&res, "error"));
    }
    if str_of(&res, "tool") != "telegram_notify" {
        return fail("failed");
    }
    tool_result(&notify_summary(&res), res.clone(), false)
}

pub fn telegram_receive(args: &Args, host: &dyn ToolHost) -> Value {
    let fail = |e: &str| plain_fail(receive_text(e), e);
    let Some(state) = state_arg(args) else { return fail("state") };
    if !host.take("telegram-receive", TG_RECEIVE_RATE_MAX, TG_RECEIVE_RATE_MS) {
        return fail("rate");
    }
    let Some(res) = desk_call(host, json!({ "tool": "telegram_receive", "state": state })) else { return fail("unavailable") };
    if res.get("ok") != Some(&json!(true)) {
        return fail(str_of(&res, "error"));
    }
    if str_of(&res, "tool") != "telegram_receive" {
        return fail("failed");
    }
    let who = match res.get("owner").and_then(Value::as_str).filter(|o| !o.is_empty()) {
        Some(o) => format!(" Only {o} can send."),
        None => String::new(),
    };
    let tail = if truthy(res.get("paired")) { who } else { " No account is paired yet.".to_string() };
    tool_result(&format!("Telegram commands are {}.{tail}", if truthy(res.get("receiving")) { "ON" } else { "OFF" }), res.clone(), false)
}

fn send_text(code: &str) -> &'static str {
    match code {
        "text" => "text must be non-empty and at most 3000 characters.",
        "path" => "file must be a file path (no UNC or device paths).",
        "outside" => "That file is outside this session's project folder; only project files can be sent.",
        "missing" => "That file does not exist.",
        "folder" => "That is a folder; give a file.",
        "secret" => "That file looks like a secret (.env, keystore, credentials) and is never sent.",
        "buttons" => "buttons must be 1–6 items of {label: 1–32 characters, value: 1–200 characters}.",
        "buttons-file" => "buttons cannot go with file; send the file in its own telegram_send.",
        "not-paired" => "Buttons need a paired Telegram account; the user pairs one in Orrery Settings › Notifications › Telegram.",
        "rate" => "Too many Telegram reports from this session in the last minute; wait a little.",
        "telegram-off" => "Telegram reports are off. The user turns them on with /bot-tele-send-notify on; do not turn them on yourself.",
        "telegram-config" => "No Telegram bot is set up yet in Orrery Settings › Notifications › Telegram.",
        "no-topic" => "This session's project has no Telegram topic; the user assigns one in Orrery Settings › Notifications › Telegram.",
        "token" => "The Telegram bot token was refused; ask the user to check it in Orrery Settings.",
        "chat" => "Telegram refused to post in that topic (bot removed, not admin, or topic deleted).",
        "too-large" => "Telegram refused the file as too large.",
        "network" => "Telegram could not be reached.",
        "timeout" => "Telegram did not answer in time.",
        "status" => "Telegram returned an error.",
        "unavailable" => "Orrery cannot send to Telegram right now.",
        _ => "Orrery could not send to Telegram.",
    }
}

pub fn telegram_send(args: &Args, host: &dyn ToolHost) -> Value {
    let fail = |e: &str| plain_fail(send_text(e), e);
    let Some(text) = args.get("text").and_then(Value::as_str).filter(|t| !js_trim(t).is_empty() && t.chars().count() <= TELEGRAM_REPORT_MAX) else { return fail("text") };
    let mut file: Option<String> = None;
    let file_arg = args.get("file").filter(|f| !matches!(f, Value::Null) && *f != &json!(""));
    if let Some(f) = file_arg {
        match resolve_inside(&host.root(), Some(f), false) {
            Inside::Ok { path, .. } => {
                if is_secret_file_name(path.rsplit(['\\', '/']).next().unwrap_or("")) {
                    return fail("secret");
                }
                file = Some(path);
            }
            Inside::Err(e) => return fail(e),
        }
    }
    let buttons = match parse_buttons(args.get("buttons").unwrap_or(&Value::Null)) {
        None => return fail("buttons"),
        Some(b) => b,
    };
    if buttons.is_some() && file.is_some() {
        return fail("buttons-file");
    }
    if !host.take("telegram-send", TG_SEND_RATE_MAX, TG_SEND_RATE_MS) {
        return fail("rate");
    }
    let mut call = json!({ "tool": "telegram_send", "text": text });
    if let Some(f) = &file {
        call["file"] = json!(f);
    }
    if let Some(b) = &buttons {
        call["buttons"] = Value::Array(b.iter().map(|(l, v)| json!({ "label": l, "value": v })).collect());
    }
    let Some(res) = desk_call(host, call) else { return fail("unavailable") };
    if res.get("ok") != Some(&json!(true)) {
        let key = res.get("detail").and_then(Value::as_str).filter(|d| !d.is_empty()).unwrap_or_else(|| str_of(&res, "error"));
        return fail(key);
    }
    if str_of(&res, "tool") != "telegram_send" {
        return fail("failed");
    }
    let topic = str_of(&res, "topic");
    let file_name = str_of(&res, "file");
    let n_buttons = res.get("buttons").and_then(Value::as_i64).unwrap_or(0);
    if !file_name.is_empty() && truthy(res.get("uploading")) {
        return tool_result(&format!("Sent the report to Telegram topic {topic}; {file_name} is uploading in the background and posts there when done."), res.clone(), false);
    }
    let what = if !file_name.is_empty() {
        if truthy(res.get("linkOnly")) {
            format!("report (file {file_name} too large, sent as path)")
        } else {
            format!("report with {file_name}")
        }
    } else if n_buttons > 0 {
        format!("report with {n_buttons} button{}", if n_buttons > 1 { "s" } else { "" })
    } else {
        "report".to_string()
    };
    let tap = if n_buttons > 0 { " The user's tap arrives here as your next prompt." } else { "" };
    tool_result(&format!("Sent the {what} to Telegram topic {topic}.{tap}"), res.clone(), false)
}
