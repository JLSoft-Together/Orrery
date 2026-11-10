use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ZOOM_MIN: f64 = -3.0;
pub const ZOOM_MAX: f64 = 3.0;
pub const ZOOM_STEP: f64 = 0.5;
const LOOK_KEYS: [&str; 6] = ["canvas", "surface", "ink", "muted", "faint", "accent"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Appearance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<Value>,
    #[serde(default)]
    pub zoom: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>,
    #[serde(default)]
    pub hidden_notice: bool,
}

pub fn is_hex(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn parse_look(raw: &Value) -> Option<Value> {
    let o = raw.as_object()?;
    let mut out = serde_json::Map::new();
    for k in LOOK_KEYS {
        let v = o.get(k)?.as_str().filter(|v| is_hex(v))?;
        out.insert(k.to_owned(), Value::String(v.to_ascii_lowercase()));
    }
    match o.get("bar") {
        None => {}
        Some(Value::String(b)) if is_hex(b) => {
            out.insert("bar".into(), Value::String(b.to_ascii_lowercase()));
        }
        Some(_) => return None,
    }
    out.insert("dark".into(), Value::Bool(o.get("dark")?.as_bool()?));
    Some(Value::Object(out))
}

pub fn clamp_zoom(level: f64) -> f64 {
    if !level.is_finite() {
        return 0.0;
    }
    ((level / ZOOM_STEP).round() * ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX)
}

pub fn next_zoom(level: f64, action: &str) -> Option<f64> {
    match action {
        "reset" => Some(0.0),
        "in" => Some(clamp_zoom(level + ZOOM_STEP)),
        "out" => Some(clamp_zoom(level - ZOOM_STEP)),
        _ => None,
    }
}

pub fn zoom_factor(level: f64) -> f64 {
    1.2f64.powf(level)
}

pub fn zoom_percent(level: f64) -> i64 {
    (100.0 * zoom_factor(level)).round() as i64
}

pub fn rgba(hex: &str) -> Option<(u8, u8, u8)> {
    if !is_hex(hex) {
        return None;
    }
    let n = u32::from_str_radix(&hex[1..], 16).ok()?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

pub struct Store {
    file: PathBuf,
    pub state: Appearance,
}

impl Store {
    pub fn open(file: PathBuf) -> Store {
        let state = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Store { file, state }
    }

    pub fn save(&self) {
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(&self.state) {
            let _ = std::fs::write(&self.file, text);
        }
    }

    pub fn locale(&self) -> &str {
        match self.state.locale.as_deref() {
            Some("en") => "en",
            _ => "vi",
        }
    }

    pub fn background(&self) -> Option<String> {
        self.state.look.as_ref().and_then(|l| l.get("canvas")).and_then(Value::as_str).map(str::to_owned).or_else(|| self.state.background.clone())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn zoom_follows_the_electron_steps() {
        assert_eq!(next_zoom(0.0, "in"), Some(0.5));
        assert_eq!(next_zoom(3.0, "in"), Some(3.0));
        assert_eq!(next_zoom(-3.0, "out"), Some(-3.0));
        assert_eq!(next_zoom(2.0, "reset"), Some(0.0));
        assert_eq!(next_zoom(0.0, "get"), None);
        assert_eq!(clamp_zoom(0.74), 0.5);
        assert_eq!(clamp_zoom(f64::NAN), 0.0);
        assert_eq!(zoom_percent(0.0), 100);
        assert_eq!(zoom_percent(1.0), 120);
        assert_eq!(zoom_percent(-0.5), 91);
    }

    #[test]
    fn looks_must_be_all_hex_colours() {
        let ok = json!({ "canvas": "#AABBCC", "surface": "#000000", "ink": "#ffffff", "muted": "#111111", "faint": "#222222", "accent": "#0a84ff", "dark": true });
        assert_eq!(parse_look(&ok).unwrap()["canvas"], "#aabbcc");
        let mut bad = ok.clone();
        bad["ink"] = json!("red");
        assert!(parse_look(&bad).is_none());
        let mut no_dark = ok.clone();
        no_dark.as_object_mut().unwrap().remove("dark");
        assert!(parse_look(&no_dark).is_none());
        let mut bar = ok.clone();
        bar["bar"] = json!(1);
        assert!(parse_look(&bar).is_none());
        assert_eq!(rgba("#0a84ff"), Some((10, 132, 255)));
        assert_eq!(rgba("0a84ff"), None);
    }

    #[test]
    fn the_store_round_trips_and_falls_back_to_defaults() {
        let file = std::env::temp_dir().join(format!("orrery-shell-appearance-{}.json", std::process::id()));
        let _ = std::fs::write(&file, "not json");
        let mut s = Store::open(file.clone());
        assert_eq!(s.state, Appearance::default());
        assert_eq!(s.locale(), "vi");
        s.state.zoom = 1.5;
        s.state.locale = Some("en".into());
        s.state.background = Some("#101010".into());
        s.save();
        let again = Store::open(file.clone());
        assert_eq!(again.state.zoom, 1.5);
        assert_eq!(again.locale(), "en");
        assert_eq!(again.background().as_deref(), Some("#101010"));
        let _ = std::fs::remove_file(file);
    }
}
