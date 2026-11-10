use super::host::{Args, ToolHost};
use super::rpc::handle_mcp;
use crate::{Ctx, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};

struct HostTools<'a> {
    ctx: &'a Ctx<'a>,
    root: String,
    preset: Option<String>,
    desk: bool,
}

fn failed() -> Value {
    json!({ "ok": false, "error": "failed" })
}

impl HostTools<'_> {
    fn call(&self, method: &str, params: Value) -> Option<Value> {
        self.ctx.host(method, params).ok()
    }
}

impl ToolHost for HostTools<'_> {
    fn root(&self) -> String {
        self.root.clone()
    }

    fn preset(&self) -> Option<String> {
        self.preset.clone()
    }

    fn take(&self, key: &str, max: usize, window_ms: u64) -> bool {
        self.call("mcp.take", json!({ "key": key, "max": max, "windowMs": window_ms })) == Some(json!(true))
    }

    fn open_session(&self, args: &Args) -> Value {
        self.call("mcp.open", json!({ "args": args })).filter(Value::is_object).unwrap_or_else(failed)
    }

    fn notify(&self, text: &str, level: &str) -> Value {
        self.call("mcp.notify", json!({ "text": text, "level": level })).filter(Value::is_object).unwrap_or_else(failed)
    }

    fn checkout_roots(&self) -> Vec<String> {
        self.call("mcp.roots", Value::Null).and_then(|v| v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())).unwrap_or_default()
    }

    fn page(&self, call: Value) -> Value {
        self.call("mcp.page", json!({ "call": call })).filter(Value::is_object).unwrap_or_else(failed)
    }

    fn desk(&self, call: Value) -> Option<Value> {
        if !self.desk {
            return None;
        }
        Some(self.call("mcp.desk", json!({ "call": call })).filter(Value::is_object).unwrap_or_else(failed))
    }
}

pub fn handle(ctx: &Ctx, req: &Value) -> Value {
    let host = HostTools {
        ctx,
        root: req["root"].as_str().unwrap_or("").to_string(),
        preset: req["preset"].as_str().map(str::to_string),
        desk: req["desk"] != json!(false),
    };
    let (status, body) = handle_mcp(&req["body"], &host, req["version"].as_str().unwrap_or("0.0.0"), req["deepseek"] == json!(true));
    let mut out = json!({ "status": status });
    if let Some(b) = body {
        out["json"] = b;
    }
    out
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    desk.on("mcp", "desk:mcp-handle", |ctx, args| {
        let req = args.first().filter(|v| v.is_object()).ok_or_else(|| DeskError::Failed("bad request".into()))?;
        Ok(handle(ctx, req))
    });
}
