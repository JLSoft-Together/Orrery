use std::path::{Path, PathBuf};
use std::sync::Mutex;

use core_desk::preview::paths::{is_absolute, root_key};
use serde_json::{json, Value};

use crate::state::lock;

pub const GRANTS_FILE: &str = "preview-grants.json";
pub const FULL_ACCESS_FILE: &str = "preview-full-access.json";
pub const MAX_GRANTS: usize = 100;

pub fn is_fs_root(p: &str) -> bool {
    Path::new(p).parent().is_none()
}

pub fn usable(dir: &str) -> bool {
    is_absolute(dir) && !is_fs_root(dir) && !dir.starts_with("//") && !dir.starts_with("\\\\") && std::fs::metadata(dir).is_ok_and(|m| m.is_dir())
}

fn write_atomic(file: &Path, text: &str) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = file.with_extension("json.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, file);
    }
}

pub struct GrantStore {
    file: Option<PathBuf>,
    dirs: Mutex<Vec<String>>,
}

impl GrantStore {
    pub fn load(file: Option<PathBuf>) -> GrantStore {
        let store = GrantStore { file, dirs: Mutex::new(Vec::new()) };
        let Some(raw) = store.file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()) else { return store };
        let list: Vec<Value> = serde_json::from_str::<Value>(&raw).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let mut kept: Vec<String> = Vec::new();
        for dir in list.iter().filter_map(Value::as_str).filter(|d| usable(d)) {
            if kept.iter().any(|k| root_key(k) == root_key(dir)) {
                continue;
            }
            kept.push(dir.to_owned());
            if kept.len() >= MAX_GRANTS {
                break;
            }
        }
        let changed = kept.len() != list.len();
        *lock(&store.dirs) = kept;
        if changed {
            store.save();
        }
        store
    }

    pub fn list(&self) -> Vec<String> {
        lock(&self.dirs).clone()
    }

    pub fn add(&self, dir: &str) {
        if !usable(dir) {
            return;
        }
        {
            let key = root_key(dir);
            let mut dirs = lock(&self.dirs);
            dirs.retain(|d| root_key(d) != key);
            dirs.insert(0, dir.to_owned());
            dirs.truncate(MAX_GRANTS);
        }
        self.save();
    }

    pub fn clear(&self) {
        lock(&self.dirs).clear();
        self.save();
    }

    fn save(&self) {
        if let Some(file) = &self.file {
            let text = serde_json::to_string_pretty(&*lock(&self.dirs)).unwrap_or_else(|_| "[]".into());
            write_atomic(file, &text);
        }
    }
}

pub struct FullAccess {
    file: Option<PathBuf>,
    state: Mutex<(bool, bool)>,
}

impl FullAccess {
    pub fn load(file: Option<PathBuf>) -> FullAccess {
        let full = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|raw| serde_json::from_str::<Value>(&raw).ok()).and_then(|v| v.get("full").and_then(Value::as_bool));
        let on = full.unwrap_or(file.is_some());
        FullAccess { file, state: Mutex::new((on, full.is_some() || on)) }
    }

    pub fn get(&self) -> bool {
        lock(&self.state).0
    }

    pub fn asked(&self) -> bool {
        lock(&self.state).1
    }

    pub fn set(&self, on: bool) {
        *lock(&self.state) = (on, true);
        if let Some(file) = &self.file {
            write_atomic(file, &json!({"full": on}).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("orrery-grants-{tag}-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn grants_are_cleaned_on_load_and_persisted_newest_first() {
        let base = temp("store");
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let (sa, sb) = (a.to_string_lossy().into_owned(), b.to_string_lossy().into_owned());
        let file = base.join(GRANTS_FILE);
        std::fs::write(&file, json!([sa, sa, "/", "relative", base.join("gone").to_string_lossy()]).to_string()).unwrap();
        let store = GrantStore::load(Some(file.clone()));
        assert_eq!(store.list(), vec![sa.clone()]);
        assert_eq!(serde_json::from_str::<Value>(&std::fs::read_to_string(&file).unwrap()).unwrap(), json!([sa]));
        store.add(&sb);
        store.add("/");
        assert_eq!(GrantStore::load(Some(file.clone())).list(), vec![sb.clone(), sa.clone()]);
        store.clear();
        assert!(GrantStore::load(Some(file)).list().is_empty());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn full_access_is_on_and_decided_without_a_file() {
        let base = temp("full");
        let file = base.join(FULL_ACCESS_FILE);
        let full = FullAccess::load(Some(file.clone()));
        assert!(full.get() && full.asked());
        full.set(false);
        let again = FullAccess::load(Some(file));
        assert!(!again.get() && again.asked());
        let memory = FullAccess::load(None);
        assert!(!memory.get() && !memory.asked());
        let _ = std::fs::remove_dir_all(base);
    }
}
