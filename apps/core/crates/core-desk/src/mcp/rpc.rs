use super::defs::{has_tool, instructions, protocol_version, tool_defs};
use super::host::{Args, ToolHost};
use super::tools::call_tool;
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub const BATCH_MAX: usize = 16;
pub const BODY_MAX: usize = 64 * 1024;

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn invalid() -> Value {
    rpc_error(Value::Null, -32600, "Invalid Request")
}

fn date_like(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap()).is_match(s)
}

fn answer(msg: &Value, host: &dyn ToolHost, version: &str, deepseek: bool) -> Option<Value> {
    let Some(m) = msg.as_object() else { return Some(invalid()) };
    if m.get("jsonrpc") != Some(&json!("2.0")) {
        return Some(invalid());
    }
    let id = m.get("id");
    if let Some(i) = id {
        if !(i.is_null() || i.is_string() || i.is_number()) {
            return Some(invalid());
        }
    }
    let Some(method) = m.get("method").and_then(Value::as_str) else {
        let reply = id.is_some() && (m.contains_key("result") || m.contains_key("error"));
        return if reply { None } else { Some(invalid()) };
    };
    let id = id?.clone();
    let ok = |result: Value| Some(json!({ "jsonrpc": "2.0", "id": id.clone(), "result": result }));
    let empty = Args::new();
    let params = m.get("params").and_then(Value::as_object).unwrap_or(&empty);
    let defs = tool_defs(deepseek);
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).filter(|v| date_like(v));
            ok(json!({
                "protocolVersion": asked.unwrap_or(protocol_version()),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "orrery", "version": version },
                "instructions": instructions(has_tool(&defs, "hand_back")),
            }))
        }
        "ping" => ok(json!({})),
        "tools/list" => ok(json!({ "tools": defs })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).filter(|n| has_tool(&defs, n));
            let Some(name) = name else { return Some(rpc_error(id, -32602, "Unknown tool")) };
            let args = match params.get("arguments") {
                None | Some(Value::Null) => Args::new(),
                Some(Value::Object(a)) => a.clone(),
                Some(_) => return Some(rpc_error(id, -32602, "Invalid arguments")),
            };
            ok(call_tool(name, &args, host))
        }
        _ => Some(rpc_error(id, -32601, "Method not found")),
    }
}

pub fn handle_mcp(body: &Value, host: &dyn ToolHost, version: &str, deepseek: bool) -> (u16, Option<Value>) {
    let Some(list) = body.as_array() else {
        return match answer(body, host, version, deepseek) {
            Some(r) => (200, Some(r)),
            None => (202, None),
        };
    };
    if list.is_empty() {
        return (200, Some(invalid()));
    }
    if list.len() > BATCH_MAX {
        return (200, Some(rpc_error(Value::Null, -32600, &format!("Invalid Request: at most {BATCH_MAX} messages per batch"))));
    }
    let out: Vec<Value> = list.iter().filter_map(|m| answer(m, host, version, deepseek)).collect();
    if out.is_empty() {
        (202, None)
    } else {
        (200, Some(Value::Array(out)))
    }
}

pub enum Gate {
    Deny(u16, Option<&'static str>),
    Token(String),
}

pub fn mcp_gate(method: &str, url: &str, host: Option<&str>, origin: Option<&str>, authorization: Option<&str>, port: u16) -> Gate {
    if url.split('?').next() != Some("/mcp") {
        return Gate::Deny(404, None);
    }
    if host != Some(format!("127.0.0.1:{port}").as_str()) {
        return Gate::Deny(403, None);
    }
    if origin.is_some_and(|o| !o.is_empty()) {
        return Gate::Deny(403, None);
    }
    if method != "POST" {
        return Gate::Deny(405, Some("POST"));
    }
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"^(\S+) ([0-9a-f]{64})$").unwrap());
    match re.captures(authorization.unwrap_or("")) {
        Some(c) if c[1].eq_ignore_ascii_case("bearer") => Gate::Token(c[2].to_string()),
        _ => Gate::Deny(401, None),
    }
}
