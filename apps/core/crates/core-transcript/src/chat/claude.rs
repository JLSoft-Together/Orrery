use super::{line_count, ms_of, strip_tag_blocks, Builder};
use crate::toolsum::{host_home, host_platform, tool_summary_of};
use serde_json::Value;

const SKIP_TYPES: [&str; 12] = ["api-request", "api-request-blob", "api-request-shape", "file-history-snapshot", "attachment", "mode", "session-owner", "last-prompt", "atis-latch", "ai-title", "tools-used", "summary"];
const HIDDEN_TAGS: [&str; 5] = ["system-reminder", "local-command-stdout", "local-command-stderr", "bash-stdout", "bash-stderr"];
const STOP_PREFIX: &str = "[Request interrupted by user";

pub fn wants(line: &[u8]) -> bool {
    let head = &line[..line.len().min(64)];
    let Some(rest) = head.strip_prefix(b"{\"type\":\"") else { return true };
    !SKIP_TYPES.iter().any(|t| rest.starts_with(t.as_bytes()) && rest.get(t.len()) == Some(&b'"'))
}

fn s(v: &Value) -> Option<&str> {
    v.as_str()
}

fn command_text(text: &str) -> Option<String> {
    let name = between(text, "<command-name>", "</command-name>")?;
    let args = between(text, "<command-args>", "</command-args>").unwrap_or("");
    let name = name.trim();
    let name = if name.starts_with('/') { name.to_owned() } else { format!("/{name}") };
    let args = args.trim();
    Some(if args.is_empty() { name } else { format!("{name} {args}") })
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = text.find(open)? + open.len();
    let b = text[a..].find(close)? + a;
    Some(&text[a..b])
}

pub fn prompt_text(content: &Value) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    match content {
        Value::String(t) => parts.push(t.clone()),
        Value::Array(blocks) => {
            for b in blocks {
                match s(&b["type"]) {
                    Some("text") => parts.extend(s(&b["text"]).map(str::to_owned)),
                    Some("image") => parts.push("[ảnh]".to_owned()),
                    Some("document") => parts.push("[file]".to_owned()),
                    Some("tool_result") => return None,
                    _ => {}
                }
            }
        }
        _ => return None,
    }
    let joined = parts.join("\n");
    if let Some(cmd) = command_text(&joined) {
        return Some(cmd);
    }
    if let Some(bash) = between(&joined, "<bash-input>", "</bash-input>") {
        return Some(format!("!{}", bash.trim()));
    }
    let mut text = joined;
    for tag in HIDDEN_TAGS {
        text = strip_tag_blocks(&text, tag);
    }
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn tool_results(b: &mut Builder, content: &Value) {
    for block in content.as_array().into_iter().flatten() {
        if s(&block["type"]) == Some("tool_result") && block["is_error"] == Value::Bool(true) {
            if let Some(id) = s(&block["tool_use_id"]) {
                b.tool_err(id);
            }
        }
    }
}

fn edits_of(name: &str, input: &Value) -> Option<(String, u64, u64)> {
    let path = s(&input["file_path"]).or_else(|| s(&input["notebook_path"]))?.to_owned();
    match name {
        "Edit" => Some((path, line_count(s(&input["new_string"]).unwrap_or("")), line_count(s(&input["old_string"]).unwrap_or("")))),
        "MultiEdit" => {
            let (mut add, mut del) = (0, 0);
            for e in input["edits"].as_array().into_iter().flatten() {
                add += line_count(s(&e["new_string"]).unwrap_or(""));
                del += line_count(s(&e["old_string"]).unwrap_or(""));
            }
            Some((path, add, del))
        }
        "Write" => Some((path, line_count(s(&input["content"]).unwrap_or("")), 0)),
        "NotebookEdit" => Some((path, 0, 0)),
        _ => None,
    }
}

fn texts(content: &Value) -> String {
    match content {
        Value::String(t) => t.clone(),
        Value::Array(a) => a.iter().filter(|b| s(&b["type"]) == Some("text")).filter_map(|b| s(&b["text"])).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

fn assistant(b: &mut Builder, rec: &Value, off: u64, at: Option<i64>) {
    let msg = &rec["message"];
    if rec["isApiErrorMessage"] == Value::Bool(true) {
        let kind = if s(&rec["apiError"]) == Some("usage_limit_reached") { "limit" } else { "error" };
        let text = texts(&msg["content"]);
        b.sys(off, at, kind, (!text.trim().is_empty()).then_some(text));
        return;
    }
    if s(&msg["model"]) == Some("<synthetic>") {
        return;
    }
    let key = s(&msg["id"]).unwrap_or("").to_owned();
    for block in msg["content"].as_array().into_iter().flatten() {
        match s(&block["type"]) {
            Some("text") => {
                if let Some(t) = s(&block["text"]) {
                    b.ai(off, at, &key, t.trim());
                }
            }
            Some("tool_use") => {
                let Some(name) = s(&block["name"]) else { continue };
                let input = &block["input"];
                let target = tool_summary_of(name, input, host_home(), host_platform()).map(|t| match super::rel_path(&t, &b.root) {
                    Some(r) if matches!(name, "Read" | "Edit" | "Write" | "NotebookEdit") => r,
                    _ => t,
                });
                b.tool(off, at, s(&block["id"]), name, target);
                if let Some((path, add, del)) = edits_of(name, input) {
                    b.file(at, &path, add, del);
                }
            }
            _ => {}
        }
    }
}

pub fn feed(b: &mut Builder, rec: &Value, off: u64) {
    if rec["isSidechain"] == Value::Bool(true) {
        return;
    }
    let at = ms_of(s(&rec["timestamp"]));
    match s(&rec["type"]) {
        Some("assistant") => assistant(b, rec, off, at),
        Some("user") => {
            let content = &rec["message"]["content"];
            tool_results(b, content);
            if rec["isMeta"] == Value::Bool(true) || rec["isCompactSummary"] == Value::Bool(true) || rec["isVisibleInTranscriptOnly"] == Value::Bool(true) {
                return;
            }
            let Some(text) = prompt_text(content) else { return };
            if text.starts_with(STOP_PREFIX) {
                b.sys(off, at, "stop", None);
                return;
            }
            b.me(off, at, text);
        }
        Some("system") => match s(&rec["subtype"]) {
            Some("compact_boundary") => b.sys(off, at, "compact", None),
            Some("api_error") => b.sys(off, at, "error", s(&rec["content"]).map(str::to_owned)),
            _ => {}
        },
        _ => {}
    }
}

pub fn last_reply(lines: &[(u64, &[u8])]) -> Option<(String, Option<i64>)> {
    let mut ended = false;
    for (_, line) in lines.iter().rev() {
        if !wants(line) {
            continue;
        }
        let Ok(rec) = serde_json::from_slice::<Value>(line) else { continue };
        if rec["isSidechain"] == Value::Bool(true) {
            continue;
        }
        match s(&rec["type"]) {
            Some("assistant") => {
                let msg = &rec["message"];
                if s(&msg["stop_reason"]) == Some("end_turn") {
                    ended = true;
                }
                if !ended || rec["isApiErrorMessage"] == Value::Bool(true) || s(&msg["model"]) == Some("<synthetic>") {
                    continue;
                }
                let text = texts(&msg["content"]);
                if !text.trim().is_empty() {
                    return Some((text, ms_of(s(&rec["timestamp"]))));
                }
            }
            Some("user") if ended => {
                if rec["isMeta"] != Value::Bool(true) && prompt_text(&rec["message"]["content"]).is_some() {
                    ended = false;
                }
            }
            _ => {}
        }
    }
    None
}
