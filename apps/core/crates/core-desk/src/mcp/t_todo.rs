use super::host::{tool_result, Args, ToolHost};
use super::text::AGENT_NOTE_MAX;
use crate::preview::checklist;
use serde_json::{json, Value};
use std::path::PathBuf;

pub const TODO_RATE_MAX: usize = 20;
pub const TODO_RATE_MS: u64 = 60_000;

fn error_text(code: &str) -> &'static str {
    match code {
        "text" => "text must be one non-empty line of at most 200 characters.",
        "id" => "id must be an item id from todo_list, for example t3.",
        "unknown" => "No item with that id in this worktree's checklist. Call todo_list for the current ones.",
        "full" => "The checklist already has the maximum of 50 items; finish or drop some first.",
        "rate" => "More than 20 checklist changes a minute from this session; try again shortly.",
        "unavailable" => "Orrery has no checklist store ready right now.",
        _ => "Orrery could not update the checklist.",
    }
}

fn fail(code: &str) -> Value {
    tool_result(error_text(code), json!({ "ok": false, "error": code }), true)
}

fn target(host: &dyn ToolHost) -> Option<(PathBuf, String)> {
    let data = host.checklist_dir()?;
    Some((data, checklist::toplevel(&host.root())))
}

fn line(item: &Value) -> String {
    let mark = if item["done"] == json!(true) { "x" } else { " " };
    format!("[{mark}] {} ({}) {}", item["id"].as_str().unwrap_or(""), item["by"].as_str().unwrap_or("owner"), item["text"].as_str().unwrap_or(""))
}

fn open_count(doc: &Value) -> usize {
    doc["items"].as_array().map(|a| a.iter().filter(|i| i["done"] != json!(true)).count()).unwrap_or(0)
}

pub fn todo_add(args: &Args, host: &dyn ToolHost) -> Value {
    let Some(raw) = args.get("text").and_then(Value::as_str).filter(|t| t.chars().count() <= AGENT_NOTE_MAX) else { return fail("text") };
    let Some((data, root)) = target(host) else { return fail("unavailable") };
    if !host.take("todo", TODO_RATE_MAX, TODO_RATE_MS) {
        return fail("rate");
    }
    match checklist::add(&data, &root, "agent", raw, host.now_ms()) {
        Ok(item) => {
            let doc = checklist::list(&data, &root);
            let open = open_count(&doc);
            tool_result(&format!("Added {} to this worktree's checklist ({open} open).", item["id"].as_str().unwrap_or("")), json!({ "ok": true, "item": item, "open": open }), false)
        }
        Err(e) => fail(e),
    }
}

pub fn todo_done(args: &Args, host: &dyn ToolHost) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str).filter(|i| !i.is_empty() && i.chars().count() <= 32) else { return fail("id") };
    let Some((data, root)) = target(host) else { return fail("unavailable") };
    if !host.take("todo", TODO_RATE_MAX, TODO_RATE_MS) {
        return fail("rate");
    }
    match checklist::set_done(&data, &root, id, Some(true)) {
        Ok(item) => {
            let open = open_count(&checklist::list(&data, &root));
            tool_result(&format!("Marked {id} done ({open} open)."), json!({ "ok": true, "item": item, "open": open }), false)
        }
        Err(e) => fail(e),
    }
}

pub fn todo_list(_args: &Args, host: &dyn ToolHost) -> Value {
    let Some((data, root)) = target(host) else { return fail("unavailable") };
    let doc = checklist::list(&data, &root);
    let items = doc["items"].as_array().cloned().unwrap_or_default();
    let check = &doc["check"];
    let script = check["script"].as_str().unwrap_or("");
    let tail = if script.is_empty() { "No check script chosen.".to_string() } else { format!("Check script \"{script}\": {}.", check["state"].as_str().unwrap_or("idle")) };
    let text = if items.is_empty() { format!("The checklist is empty. {tail}") } else { format!("{}\n{tail}", items.iter().map(line).collect::<Vec<_>>().join("\n")) };
    tool_result(&text, json!({ "ok": true, "items": items, "check": check, "open": open_count(&doc) }), false)
}
