use super::host::{none, tool_result, Args, ToolHost};
use super::parse_run::{compile_log_match, is_mcp_log_cursor, is_mcp_run_action, is_mcp_run_id, MCP_LOG_LINES_DEFAULT, MCP_LOG_LINES_MAX, MCP_LOG_MATCH_MAX};
use super::t_common::{as_int, ask, mask_session, page_error, refusal_text, session_line, str_of, truthy};
use super::text::{mask_run, quote};
use serde_json::{json, Value};

pub const RUN_APP_RATE_MAX: usize = 6;
pub const RUN_APP_RATE_MS: u64 = 60_000;
pub const RUN_STATUS_RATE_MAX: usize = 30;
pub const RUN_STATUS_RATE_MS: u64 = 60_000;
pub const RUN_LOG_RATE_MAX: usize = 30;
pub const RUN_LOG_RATE_MS: u64 = 60_000;
pub const SHOT_RATE_MAX: usize = 10;
pub const SHOT_RATE_MS: u64 = 60_000;

fn arg_error(error: &str, text: String) -> Value {
    tool_result(&text, json!({ "ok": false, "error": error }), true)
}

fn page_fail(res: &Value) -> Value {
    tool_result(page_error(str_of(res, "error")), res.clone(), true)
}

fn page_wrong() -> Value {
    tool_result(page_error("failed"), json!({ "ok": false, "error": "failed" }), true)
}

pub fn run_app(args: &Args, host: &dyn ToolHost) -> Value {
    let fail = |error: &str| {
        let text = match error {
            "rate" => format!("At most {RUN_APP_RATE_MAX} run_app calls a minute per session; wait a little."),
            "action" => "action must be one of start, reload, restart, stop.".to_string(),
            "config" => "config must be a Run configuration id from run_status.".to_string(),
            "device" => "device must be a device serial / id from run_status.".to_string(),
            _ => "full must be true or false, and only goes with action reload.".to_string(),
        };
        arg_error(error, text)
    };
    if !host.take("run", RUN_APP_RATE_MAX, RUN_APP_RATE_MS) {
        return fail("rate");
    }
    let action = args.get("action");
    if !is_mcp_run_action(action) {
        return fail("action");
    }
    let action = action.and_then(Value::as_str).unwrap_or("");
    let config = args.get("config");
    let device = args.get("device");
    let full = args.get("full");
    if !none(config) && !is_mcp_run_id(config) {
        return fail("config");
    }
    if !none(device) && !is_mcp_run_id(device) {
        return fail("device");
    }
    if !none(full) {
        match full {
            Some(Value::Bool(f)) => {
                if *f && action != "reload" {
                    return fail("full");
                }
            }
            _ => return fail("full"),
        }
    }
    let mut call = json!({ "tool": "run_app", "action": action });
    if let Some(c) = config.and_then(Value::as_str) {
        call["config"] = json!(c);
    }
    if let Some(d) = device.and_then(Value::as_str) {
        call["device"] = json!(d);
    }
    let full_on = full == Some(&json!(true));
    if full_on {
        call["full"] = json!(true);
    }
    let res = ask(host, call);
    if res.get("ok") != Some(&json!(true)) {
        return page_fail(&res);
    }
    if str_of(&res, "tool") != "run_app" {
        return page_wrong();
    }
    let session = mask_session(res.get("session").unwrap_or(&Value::Null));
    let refused = res.get("refused").filter(|r| r.is_object());
    if let Some(r) = refused {
        if str_of(r, "code") == "waiting" {
            return tool_result(&refusal_text(r), json!({ "ok": true, "action": action, "pending": true, "session": session }), false);
        }
        let text = refusal_text(r);
        let text = if session.is_object() { format!("{text}\nCurrent run: {}", session_line(&session)) } else { text };
        return tool_result(&text, json!({ "ok": false, "action": action, "refused": { "code": r["code"] }, "session": session }), true);
    }
    let what = if action == "reload" && full_on {
        "Hot restarted"
    } else {
        match action {
            "start" => "Started",
            "restart" => "Restarted",
            "reload" => "Reloaded",
            _ => "Stopped",
        }
    };
    let text = if session.is_object() {
        format!("{what}: {}.{}", session_line(&session), if action == "stop" { "" } else { " Call run_status / run_log to follow it." })
    } else {
        format!("{what}.")
    };
    tool_result(&text, json!({ "ok": true, "action": action, "session": session }), false)
}

fn masked_names(list: Option<&Value>, key: &str) -> Vec<Value> {
    list.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|c| {
                    let mut c = c.clone();
                    c[key] = json!(mask_run(str_of(&c, key)));
                    c
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn run_status(_args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("runStatus", RUN_STATUS_RATE_MAX, RUN_STATUS_RATE_MS) {
        return arg_error("rate", format!("At most {RUN_STATUS_RATE_MAX} run_status calls a minute per session; wait a little."));
    }
    let res = ask(host, json!({ "tool": "run_status" }));
    if res.get("ok") != Some(&json!(true)) {
        return page_fail(&res);
    }
    if str_of(&res, "tool") != "run_status" {
        return page_wrong();
    }
    let configs = masked_names(res.get("configs"), "name");
    let devices = masked_names(res.get("devices"), "name");
    let sessions: Vec<Value> = res.get("sessions").and_then(Value::as_array).map(|a| a.iter().map(mask_session).collect()).unwrap_or_default();
    let focused = res.get("focused").cloned().unwrap_or(Value::Null);
    let mut blocks: Vec<String> = Vec::new();
    blocks.push(if configs.is_empty() {
        "This project has no Run configuration.".to_string()
    } else {
        let rows: Vec<String> = configs
            .iter()
            .map(|c| {
                let pinned = if truthy(c.get("device")) { format!(", pinned to {}", str_of(c, "device")) } else { String::new() };
                let sel = if truthy(c.get("selected")) { " [selected]" } else { "" };
                format!("- {}: {} ({}{pinned}){sel}", str_of(c, "id"), quote(str_of(c, "name")), str_of(c, "type"))
            })
            .collect();
        format!("Run configurations:\n{}", rows.join("\n"))
    });
    blocks.push(if devices.is_empty() {
        "No device is connected.".to_string()
    } else {
        let rows: Vec<String> = devices
            .iter()
            .map(|d| format!("- {}: {} ({}, {}){}", str_of(d, "serial"), quote(str_of(d, "name")), str_of(d, "platform"), str_of(d, "state"), if truthy(d.get("selected")) { " [selected]" } else { "" }))
            .collect();
        format!("Devices:\n{}", rows.join("\n"))
    });
    blocks.push(if sessions.is_empty() {
        "Nothing has run in this project yet.".to_string()
    } else {
        let rows: Vec<String> = sessions
            .iter()
            .map(|s| format!("- {}{}", if s.get("id") == Some(&focused) && !focused.is_null() { "[shown] " } else { "" }, session_line(s)))
            .collect();
        format!("Runs (newest first):\n{}", rows.join("\n"))
    });
    tool_result(&blocks.join("\n\n"), json!({ "ok": true, "configs": configs, "devices": devices, "sessions": sessions, "focused": focused }), false)
}

pub fn run_log(args: &Args, host: &dyn ToolHost) -> Value {
    let fail = |error: &str| {
        let text = match error {
            "rate" => format!("At most {RUN_LOG_RATE_MAX} run_log calls a minute per session; wait a little."),
            "lines" => format!("lines must be a whole number from 1 to {MCP_LOG_LINES_MAX}."),
            "errorsOnly" => "errorsOnly must be true or false.".to_string(),
            "match" => format!("match must be a valid JavaScript regular expression of 1 to {MCP_LOG_MATCH_MAX} characters."),
            "ignoreCase" => "ignoreCase must be true or false.".to_string(),
            _ => "since must be the cursor string of a previous run_log result.".to_string(),
        };
        arg_error(error, text)
    };
    if !host.take("runLog", RUN_LOG_RATE_MAX, RUN_LOG_RATE_MS) {
        return fail("rate");
    }
    let n = match args.get("lines") {
        None | Some(Value::Null) => MCP_LOG_LINES_DEFAULT,
        Some(v) => match as_int(Some(v)).filter(|n| v.is_number() && (1..=MCP_LOG_LINES_MAX).contains(n)) {
            Some(n) => n,
            None => return fail("lines"),
        },
    };
    let errors_only_arg = args.get("errorsOnly");
    if !none(errors_only_arg) && !matches!(errors_only_arg, Some(Value::Bool(_))) {
        return fail("errorsOnly");
    }
    let errors_only = errors_only_arg == Some(&json!(true));
    let ignore_arg = args.get("ignoreCase");
    if !none(ignore_arg) && !matches!(ignore_arg, Some(Value::Bool(_))) {
        return fail("ignoreCase");
    }
    let ignore_case = ignore_arg == Some(&json!(true));
    let match_arg = args.get("match");
    if !none(match_arg) {
        match match_arg.and_then(Value::as_str) {
            Some(m) if compile_log_match(m, ignore_case) => {}
            _ => return fail("match"),
        }
    }
    let since_arg = args.get("since");
    if !none(since_arg) && !is_mcp_log_cursor(since_arg) {
        return fail("since");
    }
    let m = if none(match_arg) { None } else { match_arg.and_then(Value::as_str) };
    let since = if none(since_arg) { None } else { since_arg.and_then(Value::as_str) };
    let mut call = json!({ "tool": "run_log", "lines": n, "errorsOnly": errors_only });
    if let Some(m) = m {
        call["match"] = json!(m);
        if ignore_case {
            call["ignoreCase"] = json!(true);
        }
    }
    if let Some(s) = since {
        call["since"] = json!(s);
    }
    let res = ask(host, call);
    if res.get("ok") != Some(&json!(true)) {
        return page_fail(&res);
    }
    if str_of(&res, "tool") != "run_log" {
        return page_wrong();
    }
    let session = mask_session(res.get("session").unwrap_or(&Value::Null));
    if !session.is_object() {
        return tool_result("Nothing has run in this project yet, so there is no Run log.", json!({ "ok": true, "session": null, "lines": [], "errors": [] }), false);
    }
    let all: Vec<Value> = res.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let n = n as usize;
    let kept: Vec<Value> = if since.is_some() { all.into_iter().take(n).collect() } else { all.iter().skip(all.len().saturating_sub(n)).cloned().collect() };
    let lines = kept
        .into_iter()
        .map(|mut l| {
            l["text"] = json!(mask_run(str_of(&l, "text")));
            l
        })
        .collect::<Vec<_>>();
    let errors: Vec<Value> = res
        .get("errors")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|e| {
                    let mut e = e.clone();
                    e["file"] = json!(mask_run(str_of(&e, "file")));
                    e["message"] = json!(mask_run(str_of(&e, "message")));
                    e
                })
                .collect()
        })
        .unwrap_or_default();
    let flag = |k: &str| res.get(k) == Some(&json!(true));
    let mut notes: Vec<&str> = Vec::new();
    if flag("partial") {
        notes.push("(Output from before the Orrery window reloaded is not available.)");
    }
    if flag("reset") {
        notes.push("(The cursor belonged to another run or a cleared log: this is the log from its start.)");
    }
    if flag("truncated") {
        notes.push("(Lines after the cursor were dropped from the log buffer before this call; some output is missing.)");
    }
    let head = format!("Run: {}{}", session_line(&session), notes.iter().map(|n| format!("\n{n}")).collect::<String>());
    let filtered = errors_only || m.is_some();
    let body = if !lines.is_empty() {
        lines
            .iter()
            .map(|l| {
                let level = match l.get("level").and_then(Value::as_str).filter(|v| !v.is_empty()) {
                    Some(v) => format!(" {v}"),
                    None => String::new(),
                };
                format!("[{}{level}] {}", str_of(l, "stream"), str_of(l, "text"))
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if since.is_some() {
        "(no new lines)".to_string()
    } else if filtered {
        "(no matching lines)".to_string()
    } else {
        "(no output yet)".to_string()
    };
    let errs = if errors.is_empty() {
        String::new()
    } else {
        let rows: Vec<String> = errors
            .iter()
            .map(|e| format!("- {}:{}:{} {}: {}", str_of(e, "file"), e["line"], e["col"], str_of(e, "severity"), str_of(e, "message")))
            .collect();
        format!("\n\nBuild errors:\n{}", rows.join("\n"))
    };
    let cursor = res.get("cursor").and_then(Value::as_str).filter(|c| !c.is_empty());
    let cur = cursor.map(|c| format!("\n\ncursor: {c}")).unwrap_or_default();
    let mut out = json!({ "ok": true, "session": session, "lines": lines, "errors": errors, "partial": res.get("partial").cloned().unwrap_or(Value::Null) });
    if let Some(c) = cursor {
        out["cursor"] = json!(c);
    }
    if flag("reset") {
        out["reset"] = json!(true);
    }
    if flag("truncated") {
        out["truncated"] = json!(true);
    }
    tool_result(&format!("{head}\n\n{body}{errs}{cur}"), out, false)
}

pub fn device_screenshot(args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("shot", SHOT_RATE_MAX, SHOT_RATE_MS) {
        return arg_error("rate", format!("At most {SHOT_RATE_MAX} screenshots a minute per session; wait a little."));
    }
    let serial = args.get("serial");
    if !none(serial) && !is_mcp_run_id(serial) {
        return arg_error("serial", "serial must be an adb serial from run_status.".to_string());
    }
    let mut call = json!({ "tool": "device_screenshot" });
    if let Some(s) = serial.and_then(Value::as_str) {
        call["serial"] = json!(s);
    }
    let res = ask(host, call);
    if res.get("ok") != Some(&json!(true)) {
        return page_fail(&res);
    }
    if str_of(&res, "tool") != "device_screenshot" {
        return page_wrong();
    }
    let shot = res.get("shot").filter(|s| s.is_object());
    let refused = res.get("refused").filter(|r| r.is_object());
    if refused.is_some() || shot.is_none() {
        let refused = refused.cloned().unwrap_or_else(|| json!({ "code": "capture-failed" }));
        let devices: Option<Vec<Value>> = res.get("devices").and_then(Value::as_array).map(|a| {
            a.iter()
                .map(|d| json!({ "serial": d["serial"], "name": mask_run(str_of(d, "name")) }))
                .collect()
        });
        let list = match &devices {
            Some(d) if !d.is_empty() => format!("\nConnected: {}", d.iter().map(|d| format!("{} ({})", str_of(d, "serial"), quote(str_of(d, "name")))).collect::<Vec<_>>().join(", ")),
            _ => String::new(),
        };
        let mut out = json!({ "ok": false, "refused": { "code": refused["code"] } });
        if let Some(d) = devices {
            out["devices"] = json!(d);
        }
        return tool_result(&format!("{}{list}", refusal_text(&refused)), out, true);
    }
    let s = shot.cloned().unwrap_or(Value::Null);
    let where_ = if str_of(&s, "saved") == "capture" { "the user's capture folder" } else { "a temp folder (no capture folder is set in Orrery)" };
    let name = mask_run(str_of(&s, "name"));
    let text = format!("Saved a {}×{} screenshot of {} ({}) to {where_}: {}", s["width"], s["height"], quote(&name), str_of(&s, "serial"), str_of(&s, "path"));
    let mut out = json!({ "ok": true });
    if let Some(o) = s.as_object() {
        for (k, v) in o {
            out[k.as_str()] = v.clone();
        }
    }
    out["name"] = json!(name);
    tool_result(&text, out, false)
}
