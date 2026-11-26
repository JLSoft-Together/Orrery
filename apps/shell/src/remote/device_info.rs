use serde_json::Value;

use super::devices::{Device, Devices};
use super::pair::is_hidden;

pub const MAX_VERSION: usize = 16;
pub const MAX_MODEL: usize = 40;
pub const MAX_CAPS: usize = 32;
pub const MAX_CAP_LEN: usize = 32;

pub fn clean_text(raw: &str, max: usize) -> Option<String> {
    let text = raw.trim();
    let ok = !text.is_empty() && text.chars().count() <= max && text.chars().all(|c| !c.is_control() && !is_hidden(c));
    ok.then(|| text.to_owned())
}

fn text_field(hello: &Value, key: &str, max: usize) -> Option<String> {
    match hello.get(key)? {
        Value::String(s) => clean_text(s, max),
        Value::Number(n) => clean_text(&n.to_string(), max),
        _ => None,
    }
}

pub fn parse_caps(hello: &Value) -> Vec<String> {
    let Some(list) = hello.get("caps").and_then(Value::as_array) else { return Vec::new() };
    let mut caps: Vec<String> = Vec::new();
    for cap in list.iter().filter_map(Value::as_str).filter(|c| !c.is_empty() && c.len() <= MAX_CAP_LEN && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')) {
        if caps.len() < MAX_CAPS && !caps.iter().any(|c| c == cap) {
            caps.push(cap.to_owned());
        }
    }
    caps
}

impl Device {
    pub fn apply_hello(&mut self, hello: &Value) -> bool {
        let before = self.clone();
        for (slot, key, max) in [
            (&mut self.app_version, "appVersion", MAX_VERSION),
            (&mut self.app_build, "appBuild", MAX_VERSION),
            (&mut self.model, "model", MAX_MODEL),
            (&mut self.os, "os", MAX_MODEL),
        ] {
            if let Some(value) = text_field(hello, key, max) {
                *slot = Some(value);
            }
        }
        *self != before
    }
}

impl Devices {
    pub fn set_info(&mut self, id: &str, hello: &Value) {
        if self.update(id, |d| d.apply_hello(hello)) {
            self.save();
        }
    }
}
