use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

const MAX_BYTES: usize = 512 * 1024;
const MIN_WINDOW: f64 = 1_000.0;
const MAX_WINDOW: f64 = 100_000_000.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelCatalog {
    pub names: IndexMap<String, String>,
    pub windows: IndexMap<String, f64>,
}

pub fn read_settings_model(claude_root: &Path) -> Option<String> {
    let text = std::fs::read(claude_root.join("settings.json")).ok()?;
    let v: Value = serde_json::from_slice(&text).ok()?;
    let m = v.as_object()?.get("model")?.as_str()?;
    (m.encode_utf16().count() <= 100).then(|| m.to_owned())
}

fn is_model_id(id: &str) -> bool {
    let b = id.as_bytes();
    (3..=81).contains(&b.len()) && b[0].is_ascii_alphanumeric() && b[1..].iter().all(|c| c.is_ascii_alphanumeric() || b".-[]".contains(c))
}

fn list_of(models: Option<&Value>) -> Vec<&Value> {
    match models {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(Value::Object(o)) => o.values().collect(),
        _ => Vec::new(),
    }
}

fn path<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(v, |cur, k| cur.as_object()?.get(*k))
}

pub fn parse_model_catalog(text: &str) -> IndexMap<String, String> {
    let mut out = IndexMap::new();
    let Ok(raw) = serde_json::from_str::<Value>(text) else { return out };
    for m in list_of(path(&raw, &["catalog", "config", "models"])) {
        let Some(o) = m.as_object() else { continue };
        let (Some(id), Some(name)) = (o.get("id").and_then(Value::as_str), o.get("name").and_then(Value::as_str)) else { continue };
        let len = name.encode_utf16().count();
        if is_model_id(id) && len > 0 && len <= 40 {
            out.insert(id.to_owned(), name.to_owned());
        }
    }
    out
}

fn windows_of(models: Option<&Value>, out: &mut IndexMap<String, f64>) {
    for m in list_of(models) {
        let Some(o) = m.as_object() else { continue };
        let Some(id) = o.get("id").and_then(Value::as_str) else { continue };
        if !is_model_id(id) || out.contains_key(id) {
            continue;
        }
        let max = o.get("runtime").and_then(|r| r.as_object()).and_then(|r| r.get("max_input_tokens")).and_then(Value::as_f64);
        if let Some(max) = max.filter(|n| n.is_finite() && (MIN_WINDOW..=MAX_WINDOW).contains(n)) {
            out.insert(id.to_owned(), (max + 0.5).floor());
        }
    }
}

pub fn base64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

pub fn parse_model_windows(text: &str) -> IndexMap<String, f64> {
    let mut out = IndexMap::new();
    let Ok(wrapper) = serde_json::from_str::<Value>(text) else { return out };
    let doc = match path(&wrapper, &["documentBytes"]).and_then(Value::as_str) {
        Some(b) => serde_json::from_str::<Value>(&String::from_utf8_lossy(&base64_decode(b))).unwrap_or(Value::Null),
        None => wrapper.clone(),
    };
    if let Some(Value::Array(configs)) = path(&doc, &["surfaces", "cc", "model_selector_config"]) {
        for c in configs {
            windows_of(c.as_object().and_then(|o| o.get("models")), &mut out);
        }
    }
    windows_of(path(&wrapper, &["catalog", "config", "models"]), &mut out);
    out
}

pub fn read_model_catalog(claude_root: &Path) -> ModelCatalog {
    let mut out = ModelCatalog::default();
    let Ok(entries) = std::fs::read_dir(claude_root.join("cache").join("model-catalog")) else { return out };
    for e in entries.flatten() {
        let Ok(name) = e.file_name().into_string() else { continue };
        if !name.ends_with(".json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(e.path()) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        if text.encode_utf16().count() > MAX_BYTES {
            continue;
        }
        out.names.extend(parse_model_catalog(&text));
        for (id, w) in parse_model_windows(&text) {
            out.windows.entry(id).or_insert(w);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ids_to_display_names_only() {
        let names = parse_model_catalog(r#"{"catalog":{"config":{"models":[{"id":"claude-opus-5-5","name":"Opus 5.5","secret":1},{"id":"x","name":"short id"},{"id":"claude-a","name":""},{"id":"bad id!","name":"n"}]}}}"#);
        assert_eq!(names.into_iter().collect::<Vec<_>>(), [("claude-opus-5-5".to_owned(), "Opus 5.5".to_owned())]);
        assert!(parse_model_catalog("nope").is_empty());
        let by_object = parse_model_catalog(r#"{"catalog":{"config":{"models":{"a":{"id":"claude-x1","name":"X"}}}}}"#);
        assert_eq!(by_object.get("claude-x1").map(String::as_str), Some("X"));
    }

    #[test]
    fn reads_max_input_tokens_from_the_signed_published_catalog() {
        let doc = r#"{"surfaces":{"cc":{"model_selector_config":[{"models":[{"id":"claude-opus-5-5","runtime":{"max_input_tokens":1000000.4}},{"id":"claude-tiny","runtime":{"max_input_tokens":10}}]}]}}}"#;
        let b64 = encode(doc.as_bytes());
        let text = format!(r#"{{"documentBytes":"{b64}","catalog":{{"config":{{"models":[{{"id":"claude-opus-5-5","runtime":{{"max_input_tokens":5}}}},{{"id":"claude-sonnet-5","runtime":{{"max_input_tokens":200000}}}}]}}}}}}"#);
        let w = parse_model_windows(&text);
        assert_eq!(w.get("claude-opus-5-5"), Some(&1_000_000.0));
        assert_eq!(w.get("claude-sonnet-5"), Some(&200_000.0));
        assert_eq!(w.get("claude-tiny"), None);
    }

    fn encode(b: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in b.chunks(3) {
            let n = chunk.iter().enumerate().fold(0u32, |a, (i, c)| a | (u32::from(*c) << (16 - 8 * i)));
            for i in 0..4 {
                out.push(if i <= chunk.len() { T[((n >> (18 - 6 * i)) & 63) as usize] as char } else { '=' });
            }
        }
        out
    }

    #[test]
    fn decodes_base64_like_buffer() {
        assert_eq!(base64_decode("aGVsbG8="), b"hello");
        assert_eq!(base64_decode("aGVs\nbG8"), b"hello");
    }
}
