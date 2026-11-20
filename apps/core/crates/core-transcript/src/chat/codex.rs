use super::{line_count, ms_of, strip_tag_blocks, Builder};
use crate::toolsum::{host_home, host_platform, sanitize_tool_summary};
use serde_json::Value;

const SKIP_TYPES: [&str; 5] = ["token_usage_record", "world_state", "turn_context", "inter_agent_communication_metadata", "session_meta"];
const INJECTED: [&str; 7] = ["# AGENTS.md instructions", "<environment_context>", "<user_instructions>", "<INSTRUCTIONS>", "<turn_aborted>", "<subagent_notification>", "<user_shell_command>"];
const WRAPPERS: [&str; 1] = ["exec"];

pub fn wants(line: &[u8]) -> bool {
    let head = &line[..line.len().min(96)];
    let text = String::from_utf8_lossy(head);
    !SKIP_TYPES.iter().any(|t| text.contains(&format!("\"type\":\"{t}\"")))
}

fn s(v: &Value) -> Option<&str> {
    v.as_str()
}

fn text_of(content: &Value) -> String {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| match s(&b["type"]) {
            Some("input_text" | "output_text" | "text" | "Text") => s(&b["text"]).map(str::to_owned),
            Some("input_image" | "image") => Some("[ảnh]".to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn prompt_text(content: &Value) -> Option<String> {
    let raw = text_of(content);
    let t = raw.trim_start();
    if INJECTED.iter().any(|p| t.starts_with(p)) {
        return None;
    }
    let text = strip_tag_blocks(&raw, "system-reminder");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn command_target(cmd: &Value) -> Option<String> {
    let raw = match cmd {
        Value::String(c) => c.clone(),
        Value::Array(parts) => {
            let p: Vec<&str> = parts.iter().filter_map(Value::as_str).collect();
            match p.as_slice() {
                [_, flag, body] if matches!(*flag, "-c" | "-lc" | "-Command" | "/c") => (*body).to_owned(),
                _ => p.join(" "),
            }
        }
        _ => return None,
    };
    sanitize_tool_summary(&raw, host_home(), host_platform())
}

fn patch_counts(patch: &str) -> Vec<(String, u64, u64)> {
    let mut out: Vec<(String, u64, u64)> = Vec::new();
    for line in patch.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(rest) = line.strip_prefix("*** ") {
            let file = ["Add File: ", "Update File: ", "Delete File: "].iter().find_map(|p| rest.strip_prefix(p));
            if let Some(f) = file.map(str::trim).filter(|f| !f.is_empty()) {
                out.push((f.to_owned(), 0, 0));
            }
            continue;
        }
        let Some(cur) = out.last_mut() else { continue };
        if line.starts_with('+') {
            cur.1 += 1;
        } else if line.starts_with('-') {
            cur.2 += 1;
        }
    }
    out
}

fn diff_counts(diff: &str) -> (u64, u64) {
    let (mut add, mut del) = (0, 0);
    for line in diff.split('\n') {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            add += 1;
        } else if line.starts_with('-') {
            del += 1;
        }
    }
    (add, del)
}

fn patch_input(p: &Value) -> Option<String> {
    if let Some(i) = s(&p["input"]) {
        return Some(i.to_owned());
    }
    let args: Value = serde_json::from_str(s(&p["arguments"])?).ok()?;
    s(&args["input"]).map(str::to_owned)
}

fn call_target(p: &Value) -> Option<String> {
    let args: Value = serde_json::from_str(s(&p["arguments"])?).ok()?;
    let cmd = args.get("cmd").or_else(|| args.get("command"))?;
    command_target(cmd)
}

fn response_item(b: &mut Builder, p: &Value, off: u64, at: Option<i64>) {
    match s(&p["type"]) {
        Some("message") => match s(&p["role"]) {
            Some("user") => {
                if let Some(text) = prompt_text(&p["content"]) {
                    b.me(off, at, text);
                }
            }
            Some("assistant") => {
                let text = text_of(&p["content"]);
                b.ai(off, at, s(&p["id"]).unwrap_or(""), text.trim());
            }
            _ => {}
        },
        Some(kind @ ("function_call" | "custom_tool_call")) => {
            let Some(name) = s(&p["name"]) else { return };
            if kind == "custom_tool_call" && WRAPPERS.contains(&name) {
                return;
            }
            if name == "apply_patch" {
                let files = patch_input(p).map(|i| patch_counts(&i)).unwrap_or_default();
                let target = files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>().join(", ");
                b.tool(off, at, s(&p["call_id"]), name, (!target.is_empty()).then_some(target));
                for (path, add, del) in files {
                    b.file(at, &path, add, del);
                }
                return;
            }
            let target = if kind == "function_call" { call_target(p) } else { None };
            b.tool(off, at, s(&p["call_id"]), name, target);
        }
        Some("local_shell_call") => {
            let target = command_target(&p["action"]["command"]);
            b.tool(off, at, s(&p["call_id"]), "shell", target);
        }
        _ => {}
    }
}

fn completed(b: &mut Builder, item: &Value, off: u64, at: Option<i64>) {
    match s(&item["type"]) {
        Some("CommandExecution") => {
            let kind = item["parsed_cmd"].as_array().and_then(|a| a.first()).and_then(|c| s(&c["type"]));
            let name = match kind {
                Some("read") => "Read",
                Some("search" | "list_files") => "Grep",
                _ => "shell",
            };
            let id = s(&item["id"]);
            b.tool(off, at, id, name, command_target(&item["command"]));
            let failed = s(&item["status"]) == Some("failed") || item["exit_code"].as_i64().is_some_and(|c| c != 0);
            if let (true, Some(id)) = (failed, id) {
                b.tool_err(id);
            }
        }
        Some("FileChange") => {
            let Some(changes) = item["changes"].as_object() else { return };
            let target = changes.keys().map(String::as_str).collect::<Vec<_>>().join(", ");
            b.tool(off, at, s(&item["id"]), "apply_patch", sanitize_tool_summary(&target, host_home(), host_platform()));
            for (path, c) in changes {
                let (add, del) = match s(&c["type"]) {
                    Some("add") => (line_count(s(&c["content"]).unwrap_or("")), 0),
                    Some("delete") => (0, line_count(s(&c["content"]).unwrap_or(""))),
                    _ => diff_counts(s(&c["unified_diff"]).unwrap_or("")),
                };
                b.file(at, path, add, del);
            }
        }
        Some("Extension") if s(&item["kind"]).is_some_and(|k| k.starts_with("web.")) => {
            let query = s(&item["query"]).and_then(|q| sanitize_tool_summary(q, host_home(), host_platform()));
            b.tool(off, at, s(&item["id"]), "web_search", query);
        }
        Some("ImageView") => {
            let path = s(&item["path"]).map(|p| p.trim_start_matches("file:///").to_owned());
            b.tool(off, at, s(&item["id"]), "view_image", path.and_then(|p| sanitize_tool_summary(&p, host_home(), host_platform())));
        }
        _ => {}
    }
}

pub fn feed(b: &mut Builder, rec: &Value, off: u64) {
    let p = &rec["payload"];
    let at = ms_of(s(&rec["timestamp"]));
    match s(&rec["type"]) {
        Some("response_item") => response_item(b, p, off, at),
        Some("compacted") => b.sys(off, at, "compact", None),
        Some("event_msg") => match s(&p["type"]) {
            Some("item_completed") => completed(b, &p["item"], off, at),
            Some("turn_aborted") => b.sys(off, at, "stop", None),
            Some("context_compacted") => b.sys(off, at, "compact", None),
            Some("error") => b.sys(off, at, "error", s(&p["message"]).map(str::to_owned)),
            _ => {}
        },
        _ => {}
    }
}

pub fn last_reply(lines: &[(u64, &[u8])]) -> Option<(String, Option<i64>)> {
    for (_, line) in lines.iter().rev() {
        if !wants(line) {
            continue;
        }
        let Ok(rec) = serde_json::from_slice::<Value>(line) else { continue };
        let p = &rec["payload"];
        if s(&rec["type"]) == Some("event_msg") && s(&p["type"]) == Some("task_complete") {
            if let Some(t) = s(&p["last_agent_message"]).filter(|t| !t.trim().is_empty()) {
                return Some((t.to_owned(), ms_of(s(&rec["timestamp"]))));
            }
        }
    }
    None
}
