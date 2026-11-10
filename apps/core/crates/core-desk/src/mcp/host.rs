use serde_json::{json, Map, Value};

pub type Args = Map<String, Value>;

pub trait ToolHost: Send + Sync {
    fn root(&self) -> String;
    fn preset(&self) -> Option<String>;
    fn take(&self, key: &str, max: usize, window_ms: u64) -> bool;
    fn open_session(&self, args: &Args) -> Value;
    fn notify(&self, text: &str, level: &str) -> Value;
    fn checkout_roots(&self) -> Vec<String>;
    fn page(&self, call: Value) -> Value;
    fn desk(&self, call: Value) -> Option<Value>;
    fn now_ms(&self) -> f64 {
        crate::util::now_ms()
    }
    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms))
    }
}

pub fn tool_result(text: &str, structured: Value, error: bool) -> Value {
    let mut out = json!({ "content": [{ "type": "text", "text": text }], "structuredContent": structured });
    if error {
        out["isError"] = json!(true);
    }
    out
}

pub fn fail_with(text: &str, error: &str) -> Value {
    tool_result(text, json!({ "ok": false, "error": error }), true)
}

pub fn none(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null))
}
