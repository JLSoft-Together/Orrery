use super::types::RunSettings;
use crate::js;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const SETTINGS_FILE: &str = "run-settings.json";
pub const TEMPLATES_FILE: &str = "run-templates.json";
pub const MAX_PROJECT_SDKS: usize = 200;

fn is_abs(v: &str) -> bool {
    crate::preview::paths::is_absolute(v)
}

fn abs(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| is_abs(s)).map(String::from)
}

pub fn read_json(file: &Path) -> Option<Value> {
    std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok())
}

pub fn write_json(file: &Path, data: &Value) -> std::io::Result<()> {
    crate::util::write_atomic(file, &js::stringify_indent(data, "  "))
}

type Stamp = Option<(std::time::SystemTime, u64)>;

pub struct SettingsStore {
    dir: PathBuf,
    win: bool,
    inner: Mutex<(RunSettings, Stamp)>,
}

fn parse(file: &Path) -> RunSettings {
    let mut s = RunSettings::new();
    if let Some(Value::Object(raw)) = read_json(file) {
        s.flutter_sdk = abs(raw.get("flutterSdk"));
        s.android_sdk = abs(raw.get("androidSdk"));
        s.jdk = abs(raw.get("jdk"));
        if let Some(Value::Object(p)) = raw.get("flutterSdkProjects") {
            for (k, v) in p.iter().take(MAX_PROJECT_SDKS) {
                if is_abs(k) {
                    if let Some(v) = abs(Some(v)) {
                        s.flutter_sdk_projects.insert(k.clone(), v);
                    }
                }
            }
        }
        s.layout_auto_compile = raw.get("layoutAutoCompile") != Some(&Value::Bool(false));
    }
    s
}

fn stamp(file: &Path) -> Stamp {
    std::fs::metadata(file).ok().and_then(|m| Some((m.modified().ok()?, m.len())))
}

impl SettingsStore {
    pub fn load(dir: &Path, win: bool) -> Arc<SettingsStore> {
        let file = dir.join(SETTINGS_FILE);
        let st = stamp(&file);
        Arc::new(SettingsStore { dir: dir.to_path_buf(), win, inner: Mutex::new((parse(&file), st)) })
    }

    fn file(&self) -> PathBuf {
        self.dir.join(SETTINGS_FILE)
    }

    fn fresh(&self, g: &mut (RunSettings, Stamp)) {
        let file = self.file();
        let now = stamp(&file);
        if now != g.1 {
            *g = (parse(&file), now);
        }
    }

    pub fn get(&self) -> RunSettings {
        let mut g = self.inner.lock().unwrap();
        self.fresh(&mut g);
        g.0.clone()
    }

    fn save(&self, g: &mut (RunSettings, Stamp)) {
        let file = self.file();
        let _ = write_json(&file, &g.0.to_value());
        g.1 = stamp(&file);
    }

    pub fn set(&self, next: &Value) -> RunSettings {
        let mut g = self.inner.lock().unwrap();
        self.fresh(&mut g);
        let prev = &g.0;
        let pick = |key: &str, prev: &Option<String>| -> Option<String> {
            match next.get(key) {
                None => prev.clone(),
                Some(v) => abs(Some(v)),
            }
        };
        let updated = RunSettings {
            flutter_sdk: pick("flutterSdk", &prev.flutter_sdk),
            android_sdk: pick("androidSdk", &prev.android_sdk),
            jdk: pick("jdk", &prev.jdk),
            flutter_sdk_projects: prev.flutter_sdk_projects.clone(),
            layout_auto_compile: next.get("layoutAutoCompile").and_then(Value::as_bool).unwrap_or(prev.layout_auto_compile),
        };
        g.0 = updated.clone();
        self.save(&mut g);
        updated
    }

    pub fn set_project_flutter_sdk(&self, dir: &str, sdk: Option<&str>) -> RunSettings {
        let mut g = self.inner.lock().unwrap();
        self.fresh(&mut g);
        let key = super::flutter::env::project_sdk_key(dir, self.win);
        let mut projects = g.0.flutter_sdk_projects.clone();
        match sdk {
            Some(s) if is_abs(s) => {
                projects.insert(key, s.to_string());
            }
            _ => {
                projects.remove(&key);
            }
        }
        if projects.len() > MAX_PROJECT_SDKS {
            return g.0.clone();
        }
        g.0.flutter_sdk_projects = projects;
        self.save(&mut g);
        g.0.clone()
    }
}

pub fn settings_json(s: &RunSettings) -> Value {
    json!(s.to_value())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_picks_up_a_file_written_by_another_process() {
        let root = std::env::temp_dir().join(format!("ccm-d4b-set-{}", crate::util::uuid()));
        std::fs::create_dir_all(&root).unwrap();
        let dir = root.as_path();
        let store = SettingsStore::load(dir, false);
        assert_eq!(store.get().flutter_sdk, None);
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"flutterSdk":"/sdk/a","layoutAutoCompile":false}"#).unwrap();
        let s = store.get();
        assert_eq!(s.flutter_sdk.as_deref(), Some("/sdk/a"));
        assert!(!s.layout_auto_compile);
        let set = store.set(&json!({"jdk": "/jdk"}));
        assert_eq!(set.flutter_sdk.as_deref(), Some("/sdk/a"));
        assert_eq!(store.get().jdk.as_deref(), Some("/jdk"));
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"androidSdk":"/android/sdk/longer"}"#).unwrap();
        let s = store.get();
        assert_eq!(s.android_sdk.as_deref(), Some("/android/sdk/longer"));
        assert_eq!(s.jdk, None);
        assert!(s.layout_auto_compile);
        let _ = std::fs::remove_dir_all(&root);
    }
}
