use super::host::{tool_result, Args, ToolHost};
use super::paths::{mtime_of, real_of, resolve_inside, Inside};
use super::text::{clean_note_text, num, quote, AGENT_NOTE_MAX};
use crate::util::{is_inside_or_same, iso_string};
use serde_json::{json, Value};
use std::path::Path;

pub const VIEW_RATE_MAX: usize = 20;
pub const WHO_RATE_MAX: usize = 20;
pub const HAND_BACK_RATE_MS: u64 = 120_000;
pub const REPORT_MAX_BYTES: u64 = 1024 * 1024;
const LINE_MAX: f64 = 10_000_000.0;

fn page_error(code: &str) -> Option<&'static str> {
    Some(match code {
        "unavailable" => "The Orrery window is not ready right now.",
        "caller" => "This session is not an Orrery workspace pane the window knows.",
        "target" | "busy" => "Orrery could not do that.",
        _ => return None,
    })
}

fn view_error(code: &str) -> &'static str {
    match code {
        "path" => "path must be a file path (no UNC or device paths).",
        "outside" => "That file is outside this session's project folder; only files of the project can be shown.",
        "missing" => "That file does not exist.",
        "folder" => "That is a folder; give a file.",
        "line" => "line must be a whole number from 1.",
        "diff" => "diff must be true or false.",
        "rate" => "Too many files were shown from this session in the last minute; wait a little.",
        "limit" => "This session already has 4 file viewers with unsaved edits in its tab; ask the user to save or close one.",
        c => page_error(c).unwrap_or("Orrery could not show the file."),
    }
}

fn who_error(code: &str) -> &'static str {
    match code {
        "path" => "path must be a file path (no UNC or device paths).",
        "outside" => "That file is outside this session's project folder.",
        "missing" => "Neither that file nor its folder exists.",
        "folder" => "That is a folder; give a file.",
        "rate" => "Too many who_is_editing calls from this session in the last minute; wait a little.",
        c => page_error(c).unwrap_or("Orrery could not check the other sessions."),
    }
}

fn back_error(code: &str) -> &'static str {
    match code {
        "preset" => "hand_back is only for DeepSeek sessions opened by another agent",
        "path" => "report must be a file path (no UNC or device paths).",
        "outside" => "The report is outside this session's project folder; write it inside the project.",
        "missing" => "The report file does not exist; write it first.",
        "folder" => "report is a folder; give the .md file.",
        "markdown" => "The report must be a Markdown file ending in .md.",
        "size" => "The report is larger than 1 MB; shorten it.",
        "summary" => "summary must be one non-empty line of at most 200 characters.",
        "rate" => "This session already handed back in the last 2 minutes; wait before handing back again.",
        "unavailable" => "The Orrery window is not ready right now; try again in a moment.",
        "caller" => "This session was not opened by another Orrery pane (or that pane is closed), so there is nobody to hand back to; tell the user instead.",
        "limit" => "Orrery could not queue the hand-back.",
        "target" | "busy" => "Orrery could not do that.",
        _ => "Orrery could not hand the work back.",
    }
}

fn fail(text: fn(&str) -> &'static str, code: &str) -> Value {
    tool_result(text(code), json!({ "ok": false, "error": code }), true)
}

fn page_fail(res: &Value) -> Option<String> {
    if res["ok"] == json!(true) {
        None
    } else {
        Some(res["error"].as_str().unwrap_or("failed").to_string())
    }
}

pub fn open_in_workspace(args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("view", VIEW_RATE_MAX, 60_000) {
        return fail(view_error, "rate");
    }
    if args.get("diff").is_some_and(|v| !v.is_boolean()) {
        return fail(view_error, "diff");
    }
    let diff = args.get("diff") == Some(&json!(true));
    let line = match args.get("line") {
        None | Some(Value::Null) => None,
        Some(v) => match v.as_f64() {
            Some(n) if n.fract() == 0.0 && (1.0..=LINE_MAX).contains(&n) => Some(n),
            _ => return fail(view_error, "line"),
        },
    };
    let path = match resolve_inside(&host.root(), args.get("path"), diff) {
        Inside::Ok { path, .. } => path,
        Inside::Err(e) => return fail(view_error, e),
    };
    let mut call = json!({ "tool": "open_in_workspace", "path": path });
    if let (Some(n), false) = (line, diff) {
        call["line"] = num(n);
    }
    call["diff"] = json!(diff);
    let res = host.page(call);
    if let Some(e) = page_fail(&res) {
        return fail(view_error, &e);
    }
    if res["tool"] != "open_in_workspace" {
        return fail(view_error, "failed");
    }
    let what = if diff { format!("the changes of {path}") } else { format!("{path}{}", line.map(|n| format!(" at line {n}")).unwrap_or_default()) };
    let reused = res["reused"] == json!(true);
    tool_result(&format!("Showing {what} in the Orrery workspace{}.", if reused { " (its open viewer)" } else { "" }), json!({ "ok": true, "path": path, "diff": diff, "reused": res["reused"].clone() }), false)
}

fn base(p: &str) -> String {
    Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn unc(p: &str) -> bool {
    p.starts_with("//") || p.starts_with("\\\\") || p.starts_with("/\\") || p.starts_with("\\/")
}

fn with_mtime(e: &Value, name: &str, roots: &[String]) -> Value {
    let file = e["file"].as_str().unwrap_or("");
    let root = e["root"].as_str().unwrap_or("");
    if !Path::new(file).is_absolute() || !Path::new(root).is_absolute() || unc(file) || unc(root) || base(file) != name {
        return e.clone();
    }
    let Some(r) = roots.iter().find(|r| is_inside_or_same(r, root) && is_inside_or_same(root, r)) else { return e.clone() };
    let (Some(real), Some(real_root)) = (real_of(file), real_of(r)) else { return e.clone() };
    if base(&real) != name || !is_inside_or_same(&real_root, &real) {
        return e.clone();
    }
    match mtime_of(&real) {
        Some(at) if at != 0.0 => {
            let mut out = e.clone();
            out["modifiedAt"] = num(at);
            out
        }
        _ => e.clone(),
    }
}

fn agent(p: &str) -> &'static str {
    match p {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "deepseek" => "DeepSeek",
        _ => "shell",
    }
}

fn editor_line(e: &Value) -> String {
    let s = |k: &str| e[k].as_str().filter(|v| !v.is_empty());
    let where_ = if e["sameCheckout"] == json!(true) {
        "same folder as you".to_string()
    } else {
        format!("{}{}", s("branch").map(|b| format!("branch {}, ", quote(b))).unwrap_or_default(), quote(e["root"].as_str().unwrap_or("")))
    };
    let when = e["modifiedAt"].as_f64().filter(|v| *v != 0.0).map(|v| format!(", file written {}", iso_string(v))).unwrap_or_default();
    let active = s("lastActivityAt").map(|v| format!(", last active {v}")).unwrap_or_default();
    let status = s("status").map(|v| format!(", {v}")).unwrap_or_default();
    format!("- {} ({}{status}; {where_}{active}{when})", quote(e["name"].as_str().unwrap_or("")), agent(e["agent"].as_str().unwrap_or("")))
}

pub fn who_is_editing(args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("who", WHO_RATE_MAX, 60_000) {
        return fail(who_error, "rate");
    }
    let path = match resolve_inside(&host.root(), args.get("path"), true) {
        Inside::Ok { path, .. } => path,
        Inside::Err(e) => return fail(who_error, e),
    };
    let res = host.page(json!({ "tool": "who_is_editing", "path": path }));
    if let Some(e) = page_fail(&res) {
        return fail(who_error, &e);
    }
    if res["tool"] != "who_is_editing" {
        return fail(who_error, "failed");
    }
    let list = res["editors"].as_array().cloned().unwrap_or_default();
    let roots = if list.is_empty() { vec![] } else { host.checkout_roots() };
    let name = base(&path);
    let editors: Vec<Value> = list.iter().map(|e| with_mtime(e, &name, &roots)).collect();
    let repo = res["repo"] == json!(true);
    let out = json!({ "ok": true, "path": path, "repo": res["repo"].clone(), "editors": editors });
    if !repo {
        return tool_result(&format!("{path} is not in a git repository, so Orrery cannot tell which sessions changed it."), out, false);
    }
    if editors.is_empty() {
        return tool_result(&format!("No other Orrery agent session has an unmerged change to {path}."), out, false);
    }
    let shared = if editors.iter().any(|e| e["sameCheckout"] == json!(true)) {
        "\nSessions in the same folder share one working tree: the file has uncommitted changes there, but which of them made them cannot be told."
    } else {
        ""
    };
    let lines: Vec<String> = editors.iter().map(editor_line).collect();
    tool_result(&format!("Other Orrery sessions with unmerged changes to {path}:\n{}{shared}", lines.join("\n")), out, false)
}

pub fn hand_back(args: &Args, host: &dyn ToolHost) -> Value {
    if host.preset().as_deref() != Some("deepseek") {
        return fail(back_error, "preset");
    }
    let Some(raw) = args.get("summary").and_then(Value::as_str).filter(|s| s.chars().count() <= AGENT_NOTE_MAX) else { return fail(back_error, "summary") };
    let Some(summary) = clean_note_text(raw) else { return fail(back_error, "summary") };
    let path = match resolve_inside(&host.root(), args.get("report"), false) {
        Inside::Ok { path, .. } => path,
        Inside::Err(e) => return fail(back_error, e),
    };
    if !path.to_lowercase().ends_with(".md") {
        return fail(back_error, "markdown");
    }
    match std::fs::metadata(&path) {
        Ok(m) if m.len() > REPORT_MAX_BYTES => return fail(back_error, "size"),
        Ok(_) => {}
        Err(_) => return fail(back_error, "missing"),
    }
    if !host.take("hand_back", 1, HAND_BACK_RATE_MS) {
        return fail(back_error, "rate");
    }
    let res = host.page(json!({ "tool": "hand_back", "report": path, "summary": summary }));
    if let Some(e) = page_fail(&res) {
        return fail(back_error, &e);
    }
    if res["tool"] != "hand_back" {
        return fail(back_error, "failed");
    }
    let origin = res["origin"].as_str().unwrap_or("");
    tool_result(&format!("Handed back to {origin}. Orrery submits the prompt there as soon as it is idle; stop working on this task now."), json!({ "ok": true, "report": path, "origin": origin }), false)
}
