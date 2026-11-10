use crate::{arg, js, Desk, DeskEnv};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const ZOOM_MIN: f64 = -3.0;
pub const ZOOM_MAX: f64 = 3.0;
pub const ZOOM_STEP: f64 = 0.5;
const LOOK_KEYS: [&str; 6] = ["canvas", "surface", "ink", "muted", "faint", "accent"];

#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub canvas: String,
    pub surface: String,
    pub ink: String,
    pub muted: String,
    pub faint: String,
    pub accent: String,
    pub bar: Option<String>,
    pub dark: bool,
}

impl Look {
    pub fn new(canvas: &str, surface: &str, ink: &str, muted: &str, faint: &str, accent: &str, dark: bool) -> Self {
        Look { canvas: canvas.into(), surface: surface.into(), ink: ink.into(), muted: muted.into(), faint: faint.into(), accent: accent.into(), bar: None, dark }
    }

    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        for (k, v) in LOOK_KEYS.iter().zip([&self.canvas, &self.surface, &self.ink, &self.muted, &self.faint, &self.accent]) {
            m.insert((*k).into(), json!(v));
        }
        if let Some(b) = &self.bar {
            m.insert("bar".into(), json!(b));
        }
        m.insert("dark".into(), json!(self.dark));
        Value::Object(m)
    }
}

pub fn is_hex(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn parse_look(raw: &Value) -> Option<Look> {
    let r = raw.as_object()?;
    let mut vals = Vec::new();
    for k in LOOK_KEYS {
        vals.push(r.get(k).and_then(Value::as_str).filter(|s| is_hex(s))?.to_lowercase());
    }
    let dark = r.get("dark")?.as_bool()?;
    let bar = match r.get("bar") {
        None => None,
        Some(Value::String(b)) if is_hex(b) => Some(b.to_lowercase()),
        Some(_) => return None,
    };
    Some(Look { canvas: vals[0].clone(), surface: vals[1].clone(), ink: vals[2].clone(), muted: vals[3].clone(), faint: vals[4].clone(), accent: vals[5].clone(), bar, dark })
}

fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

pub fn clamp_zoom(level: &Value) -> f64 {
    match level.as_f64().filter(|l| l.is_finite()) {
        Some(l) => ZOOM_MAX.min(ZOOM_MIN.max(js_round(l / ZOOM_STEP) * ZOOM_STEP)),
        None => 0.0,
    }
}

pub fn next_zoom(level: f64, action: &str) -> f64 {
    match action {
        "reset" => 0.0,
        "in" => clamp_zoom(&json!(level + ZOOM_STEP)),
        _ => clamp_zoom(&json!(level - ZOOM_STEP)),
    }
}

pub fn zoom_percent(level: f64) -> f64 {
    js_round(100.0 * 1.2f64.powf(level))
}

fn num(v: f64) -> Value {
    core_transcript::jsstr::num(v)
}

pub fn read_appearance(file: &Path) -> Value {
    let Some(Value::Object(raw)) = std::fs::read(file).ok().and_then(|b| serde_json::from_str::<Value>(&String::from_utf8_lossy(&b)).ok()) else {
        return json!({});
    };
    let mut out = Map::new();
    if let Some(bg) = raw.get("background").and_then(Value::as_str).filter(|s| is_hex(s)) {
        out.insert("background".into(), json!(bg));
    }
    if let Some(look) = parse_look(raw.get("look").unwrap_or(&Value::Null)) {
        out.insert("look".into(), look.to_value());
    }
    if let Some(z) = raw.get("zoom") {
        out.insert("zoom".into(), num(clamp_zoom(z)));
    }
    if let Some(l) = raw.get("locale").and_then(Value::as_str).filter(|l| *l == "en" || *l == "vi") {
        out.insert("locale".into(), json!(l));
    }
    Value::Object(out)
}

pub fn write_appearance(file: &Path, appearance: &Value) {
    let _ = std::fs::write(file, js::stringify(appearance));
}

pub struct AppearanceStore {
    file: PathBuf,
    cached: Mutex<Option<Value>>,
}

impl AppearanceStore {
    pub fn new(file: PathBuf) -> Self {
        AppearanceStore { file, cached: Mutex::new(None) }
    }

    pub fn saved(&self) -> Value {
        let mut g = self.cached.lock().unwrap();
        g.get_or_insert_with(|| read_appearance(&self.file)).clone()
    }

    pub fn save(&self, patch: &Value) -> Value {
        let mut next = self.saved().as_object().cloned().unwrap_or_default();
        if let Some(p) = patch.as_object() {
            for (k, v) in p {
                next.insert(k.clone(), v.clone());
            }
        }
        let next = Value::Object(next);
        write_appearance(&self.file, &next);
        *self.cached.lock().unwrap() = Some(next.clone());
        next
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let dir = env.user_data.clone().unwrap_or_else(|| env.data_dir.clone());
    let store = Arc::new(AppearanceStore::new(dir.join("appearance.json")));
    let s = store.clone();
    desk.on("appearance", "desk:appearance-read", move |_, _| Ok(s.saved()));
    let s = store;
    desk.on("appearance", "desk:appearance-save", move |_, args| Ok(s.save(arg(args, 0))));
    desk.on("appearance", "desk:appearance-look", |_, args| Ok(parse_look(arg(args, 0)).map(|l| l.to_value()).unwrap_or(Value::Null)));
    desk.on("appearance", "desk:appearance-zoom", |_, args| {
        let level = clamp_zoom(arg(args, 0));
        let action = arg(args, 1).as_str().filter(|a| matches!(*a, "in" | "out" | "reset"));
        let next = action.map(|a| next_zoom(level, a)).unwrap_or(level);
        Ok(json!({"level": num(next), "percent": num(zoom_percent(next))}))
    });
}
