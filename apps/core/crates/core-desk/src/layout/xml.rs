use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

pub const DRAWABLE_SNIFF_BYTES: usize = 4096;
pub const MAX_XML: usize = 1024 * 1024;
const MAX_NODES: usize = 5000;
const MAX_DEPTH: usize = 64;

const DRAWABLE_ROOTS: [&str; 18] = [
    "shape", "selector", "layer-list", "vector", "animated-vector", "animated-selector", "ripple", "inset", "clip", "scale", "rotate", "level-list", "transition", "bitmap", "nine-patch",
    "adaptive-icon", "animation-list", "color",
];

const NOT_LAYOUT: [&str; 39] = [
    "manifest", "resources", "menu", "navigation", "preferencescreen", "preference-headers", "searchable", "appwidget-provider", "font-family", "set", "objectanimator", "animator", "alpha",
    "translate", "propertyvaluesholder", "keyframe", "interpolator", "transitionmanager", "transitionset", "motionscene", "constraintset", "keyframeset", "paths", "network-security-config",
    "data-extraction-rules", "full-backup-content", "accessibility-service", "shortcuts", "locale-config", "automotiveapp", "device-admin", "input-method", "wallpaper", "dream", "plurals",
    "string-array", "integer-array", "array", "string",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrawableKind {
    Drawable,
    Color,
}

impl DrawableKind {
    pub fn name(self) -> &'static str {
        match self {
            DrawableKind::Drawable => "drawable",
            DrawableKind::Color => "color",
        }
    }
}

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

fn segments(p: &str) -> Vec<&str> {
    p.split(['/', '\\']).filter(|s| !s.is_empty()).collect()
}

fn is_xml(name: &str) -> bool {
    name.len() >= 4 && name[name.len() - 4..].eq_ignore_ascii_case(".xml")
}

pub fn drawable_folder_kind(path: &str) -> Option<DrawableKind> {
    static COLOR: OnceLock<Regex> = OnceLock::new();
    static DRAW: OnceLock<Regex> = OnceLock::new();
    let parts = segments(path);
    let n = parts.len();
    if n < 3 || !is_xml(parts[n - 1]) || parts[n - 3].to_lowercase() != "res" {
        return None;
    }
    let dir = parts[n - 2].to_lowercase();
    if re(&COLOR, r"^color(-[a-z0-9_+-]+)?$").is_match(&dir) {
        return Some(DrawableKind::Color);
    }
    re(&DRAW, r"^(drawable|mipmap)(-[a-z0-9_+-]+)?$").is_match(&dir).then_some(DrawableKind::Drawable)
}

pub fn xml_root_name(head: &str) -> Option<String> {
    static NAME: OnceLock<Regex> = OnceLock::new();
    let mut s = head.strip_prefix('\u{feff}').unwrap_or(head);
    loop {
        s = s.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        if let Some(r) = s.strip_prefix("<?") {
            s = &r[r.find("?>")? + 2..];
        } else if let Some(r) = s.strip_prefix("<!--") {
            s = &r[r.find("-->")? + 3..];
        } else if let Some(r) = s.strip_prefix("<!") {
            s = &r[r.find('>')? + 1..];
        } else {
            break;
        }
    }
    re(&NAME, r"^<([A-Za-z_][A-Za-z0-9_.:-]*)").captures(s).map(|m| m[1].to_string())
}

fn sniff(text: &str) -> &str {
    let mut end = 0;
    let mut units = 0;
    for (i, c) in text.char_indices() {
        units += c.len_utf16();
        if units > DRAWABLE_SNIFF_BYTES {
            break;
        }
        end = i + c.len_utf8();
    }
    &text[..end]
}

fn looks_like_color_list(head: &str) -> bool {
    static COLOR: OnceLock<Regex> = OnceLock::new();
    static DRAWABLE: OnceLock<Regex> = OnceLock::new();
    static NESTED: OnceLock<Regex> = OnceLock::new();
    re(&COLOR, r"<item\b[^>]*\bandroid:color\s*=").is_match(head) && !re(&DRAWABLE, r"\bandroid:drawable\s*=").is_match(head) && !re(&NESTED, r"(?i)<item\b[^>]*[^/]>\s*<[a-z]").is_match(head)
}

pub fn drawable_kind(path: &str, head: Option<&str>) -> Option<DrawableKind> {
    let by_folder = drawable_folder_kind(path);
    if by_folder == Some(DrawableKind::Color) {
        return by_folder;
    }
    if !is_xml(path) {
        return None;
    }
    let parts = segments(path);
    if parts.len() >= 3 && parts[parts.len() - 3].to_lowercase() == "res" && by_folder.is_none() {
        return None;
    }
    let Some(head) = head else { return by_folder };
    let head = sniff(head);
    let Some(root) = xml_root_name(head) else { return by_folder };
    if root == "selector" && looks_like_color_list(head) {
        return Some(DrawableKind::Color);
    }
    if DRAWABLE_ROOTS.contains(&root.as_str()) {
        return Some(DrawableKind::Drawable);
    }
    by_folder
}

fn is_layout_root_name(name: &str) -> bool {
    static CLASS: OnceLock<Regex> = OnceLock::new();
    static DOTTED: OnceLock<Regex> = OnceLock::new();
    let lower = name.to_lowercase();
    let last = lower.rsplit('.').next().unwrap_or("");
    if DRAWABLE_ROOTS.contains(&name) || NOT_LAYOUT.iter().any(|n| *n == lower || *n == last) {
        return false;
    }
    if matches!(lower.as_str(), "merge" | "view" | "layout" | "fragment") {
        return true;
    }
    if name.contains(':') {
        return false;
    }
    re(&CLASS, r"^[A-Z][A-Za-z0-9_]*$").is_match(name) || re(&DOTTED, r"^[a-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_$]*)+$").is_match(name)
}

pub fn is_loose_layout_xml(path: &str, head: Option<&str>) -> bool {
    static NS: OnceLock<Regex> = OnceLock::new();
    let Some(head) = head else { return false };
    if !is_xml(path) {
        return false;
    }
    let text = sniff(head);
    xml_root_name(text).is_some_and(|r| is_layout_root_name(&r)) && re(&NS, r#"xmlns:(android|app|tools)\s*=\s*["']http://schemas\.android\.com/(apk/res/android|apk/res-auto|tools)["']"#).is_match(text)
}

pub fn is_layout_file(file: &str) -> bool {
    static DIR: OnceLock<Regex> = OnceLock::new();
    let fold = cfg!(any(windows, target_os = "macos"));
    let f = |s: &str| if fold { s.to_lowercase() } else { s.to_string() };
    let parts = segments(file);
    let n = parts.len();
    if n < 3 || f(parts[n - 1]).rsplit_once('.').map(|(_, e)| e) != Some("xml") {
        return false;
    }
    let dir = f(parts[n - 2]);
    f(parts[n - 3]) == "res" && re(&DIR, r"(?i)^layout(-[a-z0-9_+-]+)?$").is_match(&dir) && dir.starts_with("layout")
}

pub fn is_layout_target(file: &str, text: Option<&str>, head: impl Fn() -> Option<String>) -> bool {
    if is_layout_file(file) {
        return true;
    }
    if !is_xml(file) || drawable_folder_kind(file).is_some() {
        return false;
    }
    let parts = segments(file);
    if parts.len() >= 3 && parts[parts.len() - 3].to_lowercase() == "res" {
        return false;
    }
    match text {
        Some(t) => is_loose_layout_xml(file, Some(t)),
        None => is_loose_layout_xml(file, head().as_deref()),
    }
}

#[derive(Debug, PartialEq)]
pub struct TooLarge;

pub fn clean_text(v: &Value) -> Result<Option<String>, TooLarge> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) if s.len() <= MAX_XML => Ok(Some(s.clone())),
        _ => Err(TooLarge),
    }
}

pub fn int_in(v: &Value, lo: i64, hi: i64) -> Option<i64> {
    let f = v.as_f64()?;
    (f.fract() == 0.0 && f >= lo as f64 && f <= hi as f64).then_some(f as i64)
}

pub fn clean_device(v: &Value) -> Option<Value> {
    let (w, h, d) = (int_in(&v["width"], 64, 8192)?, int_in(&v["height"], 64, 8192)?, int_in(&v["density"], 72, 960)?);
    Some(json!({"width": w, "height": h, "density": d}))
}

pub fn clean_theme(v: &Value) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    v.as_str().filter(|s| re(&R, r"^[@A-Za-z0-9_.:/]{1,128}$").is_match(s)).map(String::from)
}

pub fn clean_locale(v: &Value) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    v.as_str().filter(|s| re(&R, r"^[A-Za-z0-9+_-]{1,35}$").is_match(s)).map(String::from)
}

pub fn clean_scale(v: &Value) -> f64 {
    match v.as_f64() {
        Some(f) if f.is_finite() && f < 0.95 => ((f * 8.0).ceil() / 8.0).clamp(0.125, 1.0),
        _ => 1.0,
    }
}

fn positive_floor(v: &Value) -> i64 {
    v.as_f64().filter(|f| *f > 0.0).map(|f| f.floor() as i64).unwrap_or(0)
}

pub fn clean_tree(raw: &Value) -> Value {
    fn walk(n: &Value, depth: usize, count: &mut usize) -> Option<Value> {
        let o = n.as_object()?;
        *count += 1;
        if depth > MAX_DEPTH || *count > MAX_NODES {
            return None;
        }
        let b = o.get("bounds").and_then(Value::as_array).filter(|b| b.len() == 4 && b.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)))?;
        let kids: Vec<Value> = match o.get("children").and_then(Value::as_array) {
            Some(a) => a.iter().filter_map(|c| walk(c, depth + 1, count)).collect(),
            None => Vec::new(),
        };
        let cut = |k: &str| o.get(k).and_then(Value::as_str).map(|s| crate::js::slice(s, 200));
        Some(json!({
            "class": cut("class").unwrap_or_else(|| "?".into()),
            "xmlId": cut("xmlId"),
            "line": positive_floor(o.get("line").unwrap_or(&Value::Null)),
            "column": positive_floor(o.get("column").unwrap_or(&Value::Null)),
            "bounds": b,
            "children": kids,
        }))
    }
    let mut count = 0;
    walk(raw, 0, &mut count).unwrap_or(Value::Null)
}

pub fn clean_diagnostics(raw: &Value) -> Value {
    let Some(a) = raw.as_array() else { return json!([]) };
    let out: Vec<Value> = a
        .iter()
        .take(200)
        .filter_map(|d| {
            let o = d.as_object()?;
            let severity = o.get("severity").and_then(Value::as_str).filter(|s| matches!(*s, "error" | "warning")).unwrap_or("info");
            let message = match o.get("message") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Null) | None => String::new(),
                Some(other) => crate::js::stringify(other),
            };
            let mut m = Map::new();
            m.insert("severity".into(), json!(severity));
            m.insert("message".into(), json!(crate::js::slice(&message, 1000)));
            for k in ["line", "column"] {
                if o.get(k).and_then(Value::as_f64).is_some_and(|f| f > 0.0) {
                    m.insert(k.into(), json!(positive_floor(o.get(k).unwrap_or(&Value::Null))));
                }
            }
            Some(Value::Object(m))
        })
        .collect();
    Value::Array(out)
}

pub fn drawable_device(w: &Value, h: &Value, d: &Value) -> Option<Value> {
    let (w, h, dpi) = (int_in(w, 1, 1024)?, int_in(h, 1, 1024)?, int_in(d, 72, 960)?);
    let px = |dp: i64| (((dp * dpi) as f64 / 160.0).round() as i64).clamp(16, 4096);
    Some(json!({"width": px(w), "height": px(h), "density": dpi}))
}

pub fn clean_swatches(raw: &Value) -> Value {
    static HEX: OnceLock<Regex> = OnceLock::new();
    let Some(a) = raw.as_array() else { return Value::Null };
    let out: Vec<Value> = a
        .iter()
        .take(20)
        .filter_map(|s| {
            let state = s.get("state")?.as_str()?;
            let color = s.get("color")?.as_str().filter(|c| re(&HEX, r"(?i)^#[0-9A-F]{8}$").is_match(c))?;
            Some(json!({"state": crate::js::slice(state, 40), "color": color.to_uppercase()}))
        })
        .collect();
    Value::Array(out)
}

pub fn js_number(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse().unwrap_or(f64::NAN)
            }
        }
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Null => 0.0,
        _ => f64::NAN,
    }
}

pub fn num_or(v: &Value, fallback: i64) -> Value {
    let f = js_number(v);
    if f == 0.0 || f.is_nan() {
        json!(fallback)
    } else if f.fract() == 0.0 && f.abs() < 9e15 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

pub fn num_or_zero(v: &Value) -> Value {
    num_or(v, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_files_and_kinds() {
        assert!(is_layout_file("/p/app/src/main/res/layout-land/a.xml"));
        assert!(!is_layout_file("/p/res/drawable/a.xml"));
        assert_eq!(drawable_folder_kind("/p/res/drawable-hdpi/a.xml"), Some(DrawableKind::Drawable));
        assert_eq!(drawable_folder_kind("/p/res/color/a.xml"), Some(DrawableKind::Color));
        assert_eq!(drawable_kind("/x/a.xml", Some("<?xml version=\"1.0\"?><shape/>")), Some(DrawableKind::Drawable));
        assert_eq!(drawable_kind("/x/a.xml", Some("<selector><item android:color=\"#fff\"/></selector>")), Some(DrawableKind::Color));
        assert_eq!(drawable_kind("/p/res/layout/a.xml", Some("<shape/>")), None);
        let ns = "<LinearLayout xmlns:android=\"http://schemas.android.com/apk/res/android\"/>";
        assert!(is_loose_layout_xml("/x/a.xml", Some(ns)));
        assert!(!is_loose_layout_xml("/x/a.xml", Some("<Project/>")));
        assert!(is_layout_target("/x/scratch.xml", Some(ns), || None));
    }

    #[test]
    fn cleaners() {
        assert_eq!(clean_device(&json!({"width": 1080, "height": 2400, "density": 420})), Some(json!({"width": 1080, "height": 2400, "density": 420})));
        assert_eq!(clean_device(&json!({"width": 10, "height": 2400, "density": 420})), None);
        assert_eq!(clean_scale(&json!(0.3)), 0.375);
        assert_eq!(clean_scale(&json!(0.99)), 1.0);
        assert_eq!(clean_scale(&json!(0.01)), 0.125);
        assert_eq!(drawable_device(&json!(24), &json!(24), &json!(320)), Some(json!({"width": 48, "height": 48, "density": 320})));
        assert_eq!(clean_swatches(&json!([{"state": "x", "color": "#aabbccdd"}, {"state": "y", "color": "red"}])), json!([{"state": "x", "color": "#AABBCCDD"}]));
        let tree = json!({"class": "A", "bounds": [0, 0, 1, 1], "children": [{"class": "B"}, {"bounds": [0, 0, 1, 1], "line": 3.7}]});
        assert_eq!(clean_tree(&tree), json!({"class": "A", "xmlId": null, "line": 0, "column": 0, "bounds": [0, 0, 1, 1], "children": [{"class": "?", "xmlId": null, "line": 3, "column": 0, "bounds": [0, 0, 1, 1], "children": []}]}));
        assert_eq!(clean_diagnostics(&json!([{"severity": "error", "message": "m", "line": 2}, 5])), json!([{"severity": "error", "message": "m", "line": 2}]));
        assert_eq!(clean_text(&json!(5)), Err(TooLarge));
    }
}
