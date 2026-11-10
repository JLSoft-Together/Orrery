use serde_json::{json, Map, Value};
use tauri::{AppHandle, Emitter, Manager, Webview};

use crate::guest::{emit, emit_history, id_of, toggle_devtools, Guests};

const TEXT_MAX: usize = 4000;
const URL_MAX: usize = 2048;
const KEY_MAX: usize = 32;
const ICONS_MAX: usize = 8;

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

pub fn console_payload(data: &Value) -> Option<Value> {
    let level = match data.get("level")?.as_str()? {
        "error" => "error",
        "warning" => "warning",
        _ => return None,
    };
    let message = clip(data.get("message")?.as_str()?, TEXT_MAX);
    let mut out = json!({ "type": "console", "level": level, "message": message });
    if let Some(line) = data.get("line").and_then(Value::as_u64) {
        out["line"] = json!(line);
    }
    if let Some(source) = data.get("source").and_then(Value::as_str).filter(|s| s.starts_with("http://") || s.starts_with("https://")) {
        out["source"] = json!(clip(source, URL_MAX));
    }
    Some(out)
}

pub fn favicons(data: &Value) -> Value {
    let list: Vec<Value> = data
        .get("favicons")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).filter(|u| (u.starts_with("http://") || u.starts_with("https://")) && u.len() <= URL_MAX).take(ICONS_MAX).map(|u| json!(u)).collect())
        .unwrap_or_default();
    json!({ "type": "favicon", "favicons": list })
}

pub fn chord(data: &Value) -> Option<Value> {
    let text = |k: &str| data.get(k).and_then(Value::as_str).filter(|s| !s.is_empty() && s.chars().count() <= KEY_MAX && !s.chars().any(char::is_control)).map(str::to_owned);
    let flag = |k: &str| data.get(k).and_then(Value::as_bool).unwrap_or(false);
    let (key, code) = (text("key")?, text("code")?);
    let (ctrl, shift, alt, meta) = (flag("ctrlKey"), flag("shiftKey"), flag("altKey"), flag("metaKey"));
    if !(ctrl || alt || meta) {
        return None;
    }
    let mut out = Map::new();
    out.insert("key".into(), json!(key));
    out.insert("code".into(), json!(code));
    out.insert("ctrlKey".into(), json!(ctrl));
    out.insert("shiftKey".into(), json!(shift));
    out.insert("altKey".into(), json!(alt));
    out.insert("metaKey".into(), json!(meta));
    Some(Value::Object(out))
}

fn reply(app: &AppHandle, id: u64, data: &Value) {
    let Some(nonce) = data.get("nonce").and_then(Value::as_str) else { return };
    let result = if data.get("ok") == Some(&Value::Bool(true)) {
        Ok(data.get("value").cloned().unwrap_or(Value::Null))
    } else {
        Err(clip(data.get("error").and_then(Value::as_str).unwrap_or("error"), TEXT_MAX))
    };
    app.state::<Guests>().reply(id, nonce, result);
}

#[tauri::command]
pub async fn guest(app: AppHandle, webview: Webview, kind: String, data: Option<Value>) -> Result<(), String> {
    let id = id_of(webview.label()).ok_or("denied")?;
    let owner = app.state::<Guests>().owner(id).ok_or("denied")?;
    let data = data.unwrap_or(Value::Null);
    match kind.as_str() {
        "reply" => reply(&app, id, &data),
        "nav" => emit_history(&app, id, "nav"),
        "console" => {
            if let Some(p) = console_payload(&data) {
                emit(&app, id, p);
            }
        }
        "favicon" => emit(&app, id, favicons(&data)),
        "key" => {
            if let Some(c) = chord(&data) {
                let _ = app.emit_to(owner.as_str(), "ccm:forward-key", c);
            }
        }
        "devtools" => toggle_devtools(&app, id),
        _ => return Err("unknown".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_keeps_errors_and_warnings_only() {
        assert_eq!(console_payload(&json!({"level": "log", "message": "x"})), None);
        let p = console_payload(&json!({"level": "error", "message": "boom", "line": 3, "source": "http://127.0.0.1:1/a.js"})).unwrap();
        assert_eq!(p, json!({"type": "console", "level": "error", "message": "boom", "line": 3, "source": "http://127.0.0.1:1/a.js"}));
        let p = console_payload(&json!({"level": "warning", "message": "w", "source": "file:///x"})).unwrap();
        assert!(p.get("source").is_none());
        assert_eq!(console_payload(&json!({"level": "error", "message": "y".repeat(5000)})).unwrap()["message"].as_str().unwrap().len(), TEXT_MAX);
    }

    #[test]
    fn favicons_are_web_urls_only() {
        let p = favicons(&json!({"favicons": ["https://a/x.ico", "data:image/png;base64,AA", 3, "http://b/y.png"]}));
        assert_eq!(p["favicons"], json!(["https://a/x.ico", "http://b/y.png"]));
        assert_eq!(favicons(&Value::Null)["favicons"], json!([]));
    }

    #[test]
    fn forwarded_chords_need_a_modifier_and_plain_names() {
        let c = chord(&json!({"key": "t", "code": "KeyT", "ctrlKey": true, "shiftKey": true, "extra": "x"})).unwrap();
        assert_eq!(c, json!({"key": "t", "code": "KeyT", "ctrlKey": true, "shiftKey": true, "altKey": false, "metaKey": false}));
        assert_eq!(chord(&json!({"key": "t", "code": "KeyT"})), None);
        assert_eq!(chord(&json!({"key": "\u{7}", "code": "KeyT", "ctrlKey": true})), None);
        assert_eq!(chord(&json!({"key": "t".repeat(40), "code": "KeyT", "ctrlKey": true})), None);
    }
}
