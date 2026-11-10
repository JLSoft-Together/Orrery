use crate::js;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub const DEVICES_FILE: &str = "adb-devices.json";
const MAX_ENTRIES: usize = 200;
const MAX_ALIAS: usize = 64;

#[derive(Clone)]
pub struct Entry {
    pub alias: Option<String>,
    pub model: Option<String>,
    pub wifi: bool,
    pub last_seen: f64,
}

pub fn num(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

fn clean(v: &Value) -> Option<Entry> {
    let o = v.as_object()?;
    let last_seen = o.get("lastSeen")?.as_f64().filter(|f| f.is_finite())?;
    Some(Entry {
        alias: o.get("alias").and_then(Value::as_str).filter(|s| !s.is_empty()).map(|s| js::slice(s, MAX_ALIAS)),
        model: o.get("model").and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from),
        wifi: o.get("wifi") == Some(&Value::Bool(true)),
        last_seen,
    })
}

pub struct History {
    file: PathBuf,
    devices: Vec<(String, Entry)>,
    auto: bool,
}

impl History {
    pub fn load(file: PathBuf) -> Self {
        let mut h = History { file, devices: Vec::new(), auto: true };
        let raw = std::fs::read_to_string(&h.file).ok().and_then(|t| js::parse(&t));
        if let Some(raw) = raw {
            if let Some(b) = raw.get("autoConnect").and_then(Value::as_bool) {
                h.auto = b;
            }
            if let Some(devs) = raw.get("devices").and_then(Value::as_object) {
                for (id, v) in devs {
                    if id == "__proto__" {
                        continue;
                    }
                    if let Some(e) = clean(v) {
                        h.devices.push((id.clone(), e));
                    }
                }
            }
        }
        h
    }

    pub fn all(&self) -> &[(String, Entry)] {
        &self.devices
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.devices.iter().find(|(k, _)| k == id).map(|(_, e)| e)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.devices.iter_mut().find(|(k, _)| k == id).map(|(_, e)| e)
    }

    pub fn auto_connect(&self) -> bool {
        self.auto
    }

    pub fn set_auto_connect(&mut self, on: bool) {
        if self.auto == on {
            return;
        }
        self.auto = on;
        self.write();
    }

    pub fn wifi_ids(&self) -> Vec<String> {
        self.devices.iter().filter(|(_, e)| e.wifi).map(|(k, _)| k.clone()).collect()
    }

    pub fn seen(&mut self, rows: &[super::parse::Row], now: f64) {
        let mut dirty = false;
        for r in rows {
            if r.state != "ready" || r.id.starts_with("emulator-") {
                continue;
            }
            match self.get_mut(&r.id) {
                None => {
                    self.devices.push((r.id.clone(), Entry { alias: None, model: r.model.clone(), wifi: super::parse::is_wifi_id(&r.id), last_seen: now }));
                    dirty = true;
                }
                Some(cur) => {
                    cur.last_seen = now;
                    if let Some(m) = r.model.as_ref().filter(|m| Some(*m) != cur.model.as_ref()) {
                        cur.model = Some(m.clone());
                        dirty = true;
                    }
                }
            }
        }
        if dirty {
            self.write();
        }
    }

    pub fn set_alias(&mut self, id: &str, alias: Option<&str>) {
        let Some(cur) = self.get_mut(id) else { return };
        let next = alias.map(|a| js::slice(a.trim(), MAX_ALIAS)).filter(|a| !a.is_empty());
        if cur.alias == next {
            return;
        }
        cur.alias = next;
        self.write();
    }

    pub fn remove(&mut self, ids: &[String]) {
        self.devices.retain(|(k, _)| !ids.contains(k));
        self.write();
    }

    fn write(&mut self) {
        if self.devices.len() > MAX_ENTRIES {
            let mut order: Vec<usize> = (0..self.devices.len()).collect();
            order.sort_by(|a, b| self.devices[*a].1.last_seen.partial_cmp(&self.devices[*b].1.last_seen).unwrap_or(std::cmp::Ordering::Equal));
            let drop: Vec<String> = order.iter().take(self.devices.len() - MAX_ENTRIES).map(|i| self.devices[*i].0.clone()).collect();
            self.devices.retain(|(k, _)| !drop.contains(k));
        }
        let mut devs = Map::new();
        for (id, e) in &self.devices {
            devs.insert(id.clone(), json!({"alias": e.alias, "model": e.model, "wifi": e.wifi, "lastSeen": num(e.last_seen)}));
        }
        let body = json!({"version": 1, "autoConnect": self.auto, "devices": Value::Object(devs)});
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut tmp = self.file.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        if std::fs::write(&tmp, js::stringify_indent(&body, "  ")).is_ok() {
            let _ = std::fs::rename(&tmp, &self.file);
        }
    }
}
