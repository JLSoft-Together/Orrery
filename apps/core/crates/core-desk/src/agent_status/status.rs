use crate::claude::{is_within, real};
use crate::util;
use serde_json::{json, Map, Value};
use std::io::Read;
use std::path::Path;

pub const DIR: &str = ".orrery";
pub const FILE: &str = "status.json";
pub const MAX_BYTES: u64 = 4096;
const PHASE_MAX: usize = 40;
const NOTE_MAX: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub enum StatusRead {
    Ok(Value),
    Missing,
    Invalid,
}

fn unsafe_char(c: char) -> bool {
    let n = c as u32;
    n <= 0x1f || (0x7f..=0x9f).contains(&n) || n == 0x061c || (0x200b..=0x200f).contains(&n) || (0x2028..=0x202e).contains(&n) || (0x2060..=0x206f).contains(&n) || n == 0xfeff
}

fn js_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn clean_text(raw: Option<&Value>, max: usize) -> String {
    let Some(s) = raw.and_then(Value::as_str) else { return String::new() };
    let mut out = String::new();
    let mut gap = false;
    for c in s.chars() {
        let c = if unsafe_char(c) { ' ' } else { c };
        if js_space(c) {
            gap = true;
            continue;
        }
        if gap && !out.is_empty() {
            out.push(' ');
        }
        gap = false;
        out.push(c);
    }
    let cut: String = out.chars().take(max).collect();
    cut.trim_end_matches(js_space).to_string()
}

pub fn parse_status(text: &str, updated_at: f64) -> Option<Value> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Value::Object(o) = serde_json::from_str::<Value>(text).ok()? else { return None };
    let phase = clean_text(o.get("phase"), PHASE_MAX);
    if phase.is_empty() {
        return None;
    }
    let note = clean_text(o.get("note"), NOTE_MAX);
    let mut m = Map::new();
    m.insert("phase".into(), json!(phase));
    if let Some(p) = o.get("progress").and_then(Value::as_f64).filter(|p| p.is_finite()) {
        m.insert("progress".into(), json!(p.clamp(0.0, 1.0)));
    }
    if !note.is_empty() {
        m.insert("note".into(), json!(note));
    }
    m.insert("updatedAt".into(), json!(updated_at));
    Some(Value::Object(m))
}

pub fn resolve_dir(dir: &str) -> Option<String> {
    let root = real(dir)?;
    let orrery = real(&util::path_str(&Path::new(&root).join(DIR)))?;
    (orrery != root && is_within(&root, &orrery, cfg!(windows))).then_some(orrery)
}

pub fn read_status(orrery: &str) -> StatusRead {
    let real_dir = real(orrery).unwrap_or_else(|| orrery.to_string());
    let Some(file) = real(&util::path_str(&Path::new(orrery).join(FILE))) else { return StatusRead::Missing };
    if !is_within(&real_dir, &file, cfg!(windows)) {
        return StatusRead::Missing;
    }
    let Ok(mut f) = std::fs::File::open(&file) else { return StatusRead::Missing };
    let Ok(meta) = f.metadata() else { return StatusRead::Missing };
    if !meta.is_file() {
        return StatusRead::Missing;
    }
    if meta.len() > MAX_BYTES {
        return StatusRead::Invalid;
    }
    let mut buf = Vec::new();
    if (&mut f).take(MAX_BYTES + 1).read_to_end(&mut buf).is_err() {
        return StatusRead::Missing;
    }
    if buf.len() as u64 > MAX_BYTES {
        return StatusRead::Invalid;
    }
    let at = (util::mtime_ms(&meta) + 0.5).floor();
    match parse_status(&String::from_utf8_lossy(&buf), at) {
        Some(s) => StatusRead::Ok(s),
        None => StatusRead::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_the_page() {
        assert_eq!(parse_status("\u{feff}{\"phase\":\"  build\\n\\u202e x \",\"progress\":2,\"note\":\"\"}", 5.0), Some(json!({"phase": "build x", "progress": 1.0, "updatedAt": 5.0})));
        assert_eq!(parse_status("{\"phase\":\" \"}", 1.0), None);
        assert_eq!(parse_status("[1]", 1.0), None);
        assert_eq!(parse_status("nope", 1.0), None);
        let long = "a".repeat(50);
        assert_eq!(parse_status(&format!("{{\"phase\":\"{long}\",\"note\":7}}", ), 1.0).unwrap()["phase"].as_str().unwrap().len(), 40);
    }
}
