use core_store::persist::paths::is_absolute;
use serde_json::{json, Value};

use crate::repo::style;

pub const REQUEST_TYPE: &str = "host.req";
pub const RESPONSE_TYPE: &str = "host.res";
pub const MIN_PID: u32 = 5;
pub const MAX_DIR_UTF16: usize = 1024;

pub const FOCUS_TIMEOUT_MS: u64 = 25_000;
pub const OPEN_TIMEOUT_MS: u64 = 20_000;
pub const LAUNCH_TIMEOUT_MS: u64 = 70_000;
pub const PICK_TIMEOUT_MS: u64 = 10 * 60_000 + 15_000;

pub fn valid_pid(pid: u32) -> bool {
    pid >= MIN_PID
}

pub fn valid_dir(dir: &str) -> bool {
    !dir.is_empty() && dir.encode_utf16().count() <= MAX_DIR_UTF16 && !dir.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f) && is_absolute(dir, style())
}

pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "type": REQUEST_TYPE, "id": id, "method": method, "params": params })
}

pub fn response_id(v: &Value) -> Option<u64> {
    let o = v.as_object()?;
    if o.get("type")?.as_str()? != RESPONSE_TYPE {
        return None;
    }
    o.get("id")?.as_u64()
}

pub fn response_body(v: &Value) -> Result<Value, String> {
    if let Some(e) = v.get("error") {
        return Err(e.as_str().unwrap_or("error").to_owned());
    }
    v.get("result").cloned().ok_or_else(|| "no result".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_and_dir_are_validated() {
        assert!(!valid_pid(0));
        assert!(!valid_pid(4));
        assert!(valid_pid(5));
        assert!(!valid_dir(""));
        assert!(!valid_dir("relative/dir"));
        assert!(!valid_dir(&format!("{}\u{7}x", if cfg!(windows) { "C:\\" } else { "/" })));
        assert!(!valid_dir(&format!("{}{}", if cfg!(windows) { "C:\\" } else { "/" }, "a".repeat(1100))));
        assert!(valid_dir(if cfg!(windows) { "C:\\work\\app" } else { "/work/app" }));
    }

    #[test]
    fn responses_are_matched_by_type_and_id() {
        assert_eq!(response_id(&json!({"type": "host.res", "id": 7, "result": {}})), Some(7));
        assert_eq!(response_id(&json!({"type": "host.req", "id": 7})), None);
        assert_eq!(response_id(&json!({"type": "host.res", "id": -1})), None);
        assert_eq!(response_id(&json!({"type": "host.res"})), None);
        assert_eq!(response_body(&json!({"result": {"a": 1}})), Ok(json!({"a": 1})));
        assert_eq!(response_body(&json!({"error": "bad"})), Err("bad".to_owned()));
        assert!(response_body(&json!({})).is_err());
    }
}
