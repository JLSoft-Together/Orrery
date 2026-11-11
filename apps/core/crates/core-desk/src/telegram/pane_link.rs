use super::base::Tg;
use crate::phone::link::{from_host, label};
use serde_json::{json, Value};

pub fn add_row(keys: Option<Value>, url: &str, vi: bool) -> Value {
    let row = json!([{"text": label(!vi), "url": url}]);
    let mut keys = keys.unwrap_or_else(|| json!({"inline_keyboard": []}));
    if let Some(rows) = keys["inline_keyboard"].as_array_mut() {
        rows.push(row);
    }
    keys
}

impl Tg {
    pub fn with_pane_link(&self, keys: Option<Value>, e: &Value, vi: bool) -> Option<Value> {
        let terminal = e["terminalId"].as_str().filter(|t| !t.is_empty());
        let url = terminal.and_then(|t| from_host(self.host("remote.link", json!({"terminalId": t}))));
        match url {
            Some(url) => Some(add_row(keys, &url, vi)),
            None => keys,
        }
    }
}
