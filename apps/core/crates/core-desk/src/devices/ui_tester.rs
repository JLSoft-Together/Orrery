use super::adb::Adb;
use super::history::num;
use super::parse::err;
use super::toggles::*;
use crate::js;
use crate::run::android::jsre::Js;
use crate::run::collect::Collected;
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::Mutex;

pub const UI_ORIGINALS_FILE: &str = "adb-ui-originals.json";
const DEVICES_MAX: usize = 100;

#[derive(Clone, Default)]
pub struct Originals {
    night: Option<String>,
    font_scale: Option<Option<String>>,
    density: Option<Option<i64>>,
    animations: Option<Vec<Option<String>>>,
    locales: Option<Vec<(String, String)>>,
}

impl Originals {
    fn keys(&self) -> Vec<&'static str> {
        let mut k = Vec::new();
        if self.night.is_some() {
            k.push("night");
        }
        if self.font_scale.is_some() {
            k.push("fontScale");
        }
        if self.density.is_some() {
            k.push("density");
        }
        if self.animations.is_some() {
            k.push("animations");
        }
        if self.locales.as_ref().is_some_and(|l| !l.is_empty()) {
            k.push("locale");
        }
        k
    }

    fn to_json(&self) -> Value {
        let mut o = Map::new();
        if let Some(n) = &self.night {
            o.insert("night".into(), json!(n));
        }
        if let Some(f) = &self.font_scale {
            o.insert("fontScale".into(), json!(f));
        }
        if let Some(d) = &self.density {
            o.insert("density".into(), json!(d));
        }
        if let Some(a) = &self.animations {
            o.insert("animations".into(), json!(a));
        }
        if let Some(l) = &self.locales {
            o.insert("locales".into(), Value::Object(l.iter().map(|(k, v)| (k.clone(), json!(v))).collect()));
        }
        Value::Object(o)
    }
}

fn numeric(s: &str) -> bool {
    static RE: Js = Js::new(r"^\d+(\.\d+)?$", "");
    RE.test(s)
}

fn clean_originals(v: &Value) -> Option<Originals> {
    let o = v.as_object()?;
    let mut out = Originals::default();
    if let Some(n @ ("yes" | "no" | "auto" | "custom")) = o.get("night").and_then(Value::as_str) {
        out.night = Some(n.to_string());
    }
    match o.get("fontScale") {
        Some(Value::Null) => out.font_scale = Some(None),
        Some(Value::String(s)) if numeric(s) => out.font_scale = Some(Some(s.clone())),
        _ => {}
    }
    match o.get("density") {
        Some(Value::Null) => out.density = Some(None),
        Some(n) if n.as_f64().is_some_and(|f| f.fract() == 0.0) => out.density = Some(n.as_f64().map(|f| f as i64)),
        _ => {}
    }
    if let Some(a) = o.get("animations").and_then(Value::as_array).filter(|a| a.len() == ANIMATION_KEYS.len()) {
        let items: Option<Vec<Option<String>>> = a.iter().map(|x| match x {
            Value::Null => Some(None),
            Value::String(s) if numeric(s) => Some(Some(s.clone())),
            _ => None,
        }).collect();
        out.animations = items;
    }
    if let Some(l) = o.get("locales").and_then(Value::as_object) {
        let list: Vec<(String, String)> = l.iter().filter_map(|(k, v)| v.as_str().filter(|t| is_package(k) && is_locale_list(t)).map(|t| (k.clone(), t.to_string()))).collect();
        if !list.is_empty() {
            out.locales = Some(list);
        }
    }
    (!out.keys().is_empty()).then_some(out)
}

fn failed(r: &Option<Collected>) -> bool {
    static BAD: Js = Js::new(r"^(Error|Exception|Unknown|Bad|java\.)", "m");
    match r {
        None => true,
        Some(r) => r.timed_out || r.code != Some(0) || BAD.test(&r.out),
    }
}

fn put(ns: &str, key: &str, value: Option<&str>) -> Vec<String> {
    match value {
        None => ["settings", "delete", ns, key].iter().map(|s| s.to_string()).collect(),
        Some(v) => ["settings", "put", ns, key, v].iter().map(|s| s.to_string()).collect(),
    }
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn change_commands(change: &Value, o: &Originals) -> Vec<Vec<String>> {
    match change["key"].as_str().unwrap_or("") {
        "night" => vec![strs(&["cmd", "uimode", "night", change["value"].as_str().unwrap_or("")])],
        "fontScale" => vec![put("system", "font_scale", Some(&js::number(change["value"].as_f64().unwrap_or(1.0))))],
        "density" => vec![match change["value"].as_i64() {
            None => strs(&["wm", "density", "reset"]),
            Some(d) => strs(&["wm", "density", &d.to_string()]),
        }],
        "animations" => {
            let off = change["off"] == json!(true);
            ANIMATION_KEYS.iter().enumerate().map(|(i, k)| put("global", k, Some(if off { "0" } else { o.animations.as_ref().and_then(|a| a.get(i)).and_then(|x| x.as_deref()).unwrap_or("1") }))).collect()
        }
        _ => {
            let v = change["value"].as_str().unwrap_or("");
            vec![strs(&["cmd", "locale", "set-app-locales", change["pkg"].as_str().unwrap_or(""), "--locales", if v.is_empty() { "''" } else { v }])]
        }
    }
}

fn restore_commands(o: &Originals) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    if let Some(n) = &o.night {
        out.push(strs(&["cmd", "uimode", "night", if n == "custom" { "auto" } else { n }]));
    }
    if let Some(f) = &o.font_scale {
        out.push(put("system", "font_scale", f.as_deref()));
    }
    if let Some(d) = &o.density {
        out.push(match d {
            None => strs(&["wm", "density", "reset"]),
            Some(d) => strs(&["wm", "density", &d.to_string()]),
        });
    }
    if let Some(a) = &o.animations {
        for (i, k) in ANIMATION_KEYS.iter().enumerate() {
            out.push(put("global", k, a.get(i).and_then(|x| x.as_deref())));
        }
    }
    for (pkg, tags) in o.locales.iter().flatten() {
        if is_package(pkg) && is_locale_list(tags) {
            out.push(strs(&["cmd", "locale", "set-app-locales", pkg, "--locales", if tags.is_empty() { "''" } else { tags }]));
        }
    }
    out
}

pub struct UiTester {
    adb: Adb,
    file: PathBuf,
    map: Mutex<Option<Vec<(String, Originals)>>>,
}

impl UiTester {
    pub fn new(adb: Adb, file: PathBuf) -> Self {
        UiTester { adb, file, map: Mutex::new(None) }
    }

    fn with_map<T>(&self, f: impl FnOnce(&mut Vec<(String, Originals)>) -> T) -> T {
        let mut g = self.map.lock().unwrap();
        if g.is_none() {
            let mut list: Vec<(String, Originals)> = Vec::new();
            if let Some(raw) = std::fs::read_to_string(&self.file).ok().and_then(|t| js::parse(&t)) {
                if let Some(devs) = raw.get("devices").and_then(Value::as_object) {
                    for (id, v) in devs {
                        if let Some(o) = clean_originals(v) {
                            if list.len() < DEVICES_MAX {
                                list.push((id.clone(), o));
                            }
                        }
                    }
                }
            }
            *g = Some(list);
        }
        f(g.as_mut().unwrap())
    }

    fn persist(&self) {
        let devices: Map<String, Value> = self.with_map(|m| m.iter().map(|(k, o)| (k.clone(), o.to_json())).collect());
        let body = js::stringify_indent(&json!({"version": 1, "devices": Value::Object(devices)}), "  ");
        let mut tmp = self.file.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        if std::fs::write(&tmp, body).is_ok() {
            let _ = std::fs::rename(&tmp, &self.file);
        }
    }

    fn get(&self, id: &str, args: &[&str]) -> Option<String> {
        let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.adb.shell(id, &a, None).filter(|r| !r.timed_out && r.code == Some(0)).map(|r| r.out)
    }

    fn sdk(&self, id: &str) -> Option<i64> {
        parse_sdk(&self.get(id, &["getprop", "ro.build.version.sdk"]).unwrap_or_default())
    }

    fn raw_animations(&self, id: &str) -> Vec<Option<String>> {
        ANIMATION_KEYS.iter().map(|k| parse_setting(&self.get(id, &["settings", "get", "global", k]).unwrap_or_default())).collect()
    }

    pub fn read(&self, id: &str, pkg: Option<&str>) -> Value {
        let Some(sdk) = self.sdk(id) else { return err("not-connected") };
        let night = self.get(id, &["cmd", "uimode", "night"]);
        let font = self.get(id, &["settings", "get", "system", "font_scale"]);
        let density = self.get(id, &["wm", "density"]);
        let anims = self.raw_animations(id);
        let supported = sdk >= LOCALE_MIN_SDK;
        let locale = if supported { pkg.and_then(|p| parse_app_locales(&self.get(id, &["cmd", "locale", "get-app-locales", p]).unwrap_or_default())) } else { None };
        let (physical, over) = parse_density(&density.unwrap_or_default());
        let font_scale = parse_setting(&font.unwrap_or_default()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
        let changed = self.with_map(|m| m.iter().find(|(k, _)| k == id).map(|(_, o)| o.keys()).unwrap_or_default());
        json!({"ok": true, "state": {
            "sdk": sdk,
            "night": parse_night(&night.unwrap_or_default()),
            "fontScale": num(font_scale),
            "density": {"physical": physical, "override": over},
            "animationsOff": anims.iter().all(|a| a.as_deref().and_then(|s| s.parse::<f64>().ok()) == Some(0.0)),
            "locale": {"supported": supported, "pkg": pkg, "current": locale},
            "changed": changed,
        }})
    }

    fn remember(&self, id: &str, change: &Value, o: &mut Originals) -> Option<Value> {
        match change["key"].as_str().unwrap_or("") {
            "night" if o.night.is_none() => o.night = Some(parse_night(&self.get(id, &["cmd", "uimode", "night"]).unwrap_or_default()).unwrap_or_else(|| "auto".into())),
            "fontScale" if o.font_scale.is_none() => o.font_scale = Some(parse_setting(&self.get(id, &["settings", "get", "system", "font_scale"]).unwrap_or_default())),
            "density" if o.density.is_none() => o.density = Some(parse_density(&self.get(id, &["wm", "density"]).unwrap_or_default()).1),
            "animations" if o.animations.is_none() => o.animations = Some(self.raw_animations(id)),
            "locale" => {
                let pkg = change["pkg"].as_str().unwrap_or("");
                if !o.locales.iter().flatten().any(|(k, _)| k == pkg) {
                    let cur = parse_app_locales(&self.get(id, &["cmd", "locale", "get-app-locales", pkg]).unwrap_or_default());
                    let Some(cur) = cur.filter(|c| is_locale_list(c)) else { return Some(err("unsupported")) };
                    o.locales.get_or_insert_with(Vec::new).push((pkg.to_string(), cur));
                }
            }
            _ => {}
        }
        None
    }

    fn run_all(&self, id: &str, cmds: Vec<Vec<String>>) -> Option<Option<Collected>> {
        let mut bad: Option<Option<Collected>> = None;
        for c in cmds {
            let r = self.adb.shell(id, &c, None);
            if failed(&r) && bad.is_none() {
                bad = Some(r);
            }
        }
        bad
    }

    fn unsupported(r: Option<Collected>) -> Value {
        let detail = r.map(|r| js::slice(r.out.trim(), 300)).unwrap_or_default();
        super::parse::err_detail("unsupported", &detail)
    }

    pub fn apply(&self, id: &str, change: &Value, pkg: Option<&str>) -> Value {
        let Some(sdk) = self.sdk(id) else { return err("not-connected") };
        let is_locale = change["key"] == json!("locale");
        if is_locale && sdk < LOCALE_MIN_SDK {
            return err("unsupported");
        }
        let mut o = self.with_map(|m| m.iter().find(|(k, _)| k == id).map(|(_, o)| o.clone()).unwrap_or_default());
        if let Some(miss) = self.remember(id, change, &mut o) {
            return miss;
        }
        self.with_map(|m| {
            match m.iter_mut().find(|(k, _)| k == id) {
                Some(slot) => slot.1 = o.clone(),
                None => {
                    if m.len() >= DEVICES_MAX {
                        m.remove(0);
                    }
                    m.push((id.to_string(), o.clone()));
                }
            }
        });
        self.persist();
        let commands = change_commands(change, &o);
        for c in commands {
            let r = self.adb.shell(id, &c, None);
            if failed(&r) {
                return Self::unsupported(r);
            }
        }
        if change["key"] == json!("animations") && change["off"] == json!(false) {
            o.animations = None;
            self.with_map(|m| {
                if o.keys().is_empty() {
                    m.retain(|(k, _)| k != id);
                } else if let Some(slot) = m.iter_mut().find(|(k, _)| k == id) {
                    slot.1 = o.clone();
                }
            });
            self.persist();
        }
        self.read(id, pkg)
    }

    pub fn restore(&self, id: &str, pkg: Option<&str>) -> Value {
        let o = self.with_map(|m| m.iter().find(|(k, _)| k == id).map(|(_, o)| o.clone()));
        if let Some(o) = o {
            if let Some(bad) = self.run_all(id, restore_commands(&o)) {
                return Self::unsupported(bad);
            }
            self.with_map(|m| m.retain(|(k, _)| k != id));
            self.persist();
        }
        self.read(id, pkg)
    }
}
