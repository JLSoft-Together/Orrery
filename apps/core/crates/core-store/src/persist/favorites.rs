use std::path::PathBuf;

use indexmap::IndexMap;
use serde_json::{json, Value};

use super::aliases::sanitize_alias;
use super::collate::locale_cmp;
use super::json_file::{file_label, read_json, write_json};
use super::paths::{basename, cwd_string, is_absolute, resolve, Style};
use super::{system_clock, Clock};
use crate::extra::Favorite;
use crate::persist::datadir::Platform;
use crate::time::iso_of_ms;

pub const MAX_FAVORITES: usize = 50;

pub fn favorite_key(dir: &str, platform: Platform) -> String {
    favorite_key_at(dir, platform, Style::HOST, &cwd_string())
}

fn favorite_key_at(dir: &str, platform: Platform, style: Style, cwd: &str) -> String {
    let resolved = resolve(dir, style, cwd);
    if platform == Platform::Win32 {
        resolved.to_lowercase()
    } else {
        resolved
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddResult {
    Ok,
    Exists,
    Limit,
}

pub struct FavoriteStore {
    map: IndexMap<String, Favorite>,
    file: Option<PathBuf>,
    platform: Platform,
    style: Style,
    cwd: String,
    clock: Clock,
}

impl FavoriteStore {
    pub fn new(file: Option<PathBuf>, platform: Platform) -> Self {
        Self::with_clock(file, platform, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, platform: Platform, clock: Clock) -> Self {
        Self { map: IndexMap::new(), file, platform, style: Style::HOST, cwd: cwd_string(), clock }
    }

    pub fn with_style(mut self, style: Style, cwd: &str) -> Self {
        self.style = style;
        self.cwd = cwd.to_string();
        self
    }

    fn key_of(&self, dir: &str) -> String {
        favorite_key_at(dir, self.platform, self.style, &self.cwd)
    }

    fn resolved(&self, dir: &str) -> String {
        resolve(dir, self.style, &self.cwd)
    }

    fn now_iso(&self) -> String {
        iso_of_ms((self.clock)())
    }

    pub fn load(&mut self) {
        let Some(file) = &self.file else { return };
        let Some(Value::Array(items)) = read_json(file) else { return };
        let mut loaded: Vec<(Favorite, Option<f64>)> = Vec::new();
        for item in &items {
            let Value::Object(o) = item else { continue };
            let Some(Value::String(dir)) = o.get("dir") else { continue };
            if !is_absolute(dir, self.style) {
                continue;
            }
            let label = o.get("label").and_then(|l| sanitize_alias(l, super::aliases::MAX_ALIAS_LENGTH)).unwrap_or_else(|| basename(dir, self.style));
            let added_at = match o.get("addedAt") {
                Some(Value::String(s)) => s.clone(),
                _ => iso_of_ms(0),
            };
            let order = o.get("order").and_then(Value::as_f64).filter(|n| n.is_finite());
            loaded.push((Favorite { dir: self.resolved(dir), label, added_at }, order));
        }
        loaded.sort_by(|a, b| match (a.1, b.1) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => locale_cmp(&a.0.label, &b.0.label),
        });
        for (favorite, _) in loaded {
            let key = self.key_of(&favorite.dir);
            self.map.insert(key, favorite);
        }
    }

    pub fn list(&self) -> Vec<Favorite> {
        self.map.values().cloned().collect()
    }

    pub fn get(&self, dir: &str) -> Option<&Favorite> {
        self.map.get(&self.key_of(dir))
    }

    pub fn toggle(&mut self, dir: &str, label: &str) -> bool {
        let key = self.key_of(dir);
        if self.map.shift_remove(&key).is_some() {
            self.save();
            return false;
        }
        if self.map.len() >= MAX_FAVORITES {
            return false;
        }
        let favorite = Favorite {
            dir: self.resolved(dir),
            label: sanitize_alias(&Value::String(label.into()), super::aliases::MAX_ALIAS_LENGTH).unwrap_or_else(|| basename(dir, self.style)),
            added_at: self.now_iso(),
        };
        self.map.insert(key, favorite);
        self.save();
        true
    }

    pub fn add(&mut self, dir: &str, label: &Value) -> AddResult {
        let key = self.key_of(dir);
        if self.map.contains_key(&key) {
            return AddResult::Exists;
        }
        if self.map.len() >= MAX_FAVORITES {
            return AddResult::Limit;
        }
        let resolved = self.resolved(dir);
        let favorite = Favorite {
            label: sanitize_alias(label, super::aliases::MAX_ALIAS_LENGTH).unwrap_or_else(|| basename(&resolved, self.style)),
            dir: resolved,
            added_at: self.now_iso(),
        };
        self.map.insert(key, favorite);
        self.save();
        AddResult::Ok
    }

    pub fn rename(&mut self, dir: &str, label: &Value) -> bool {
        let key = self.key_of(dir);
        let Some(current) = self.map.get(&key) else { return false };
        let next = sanitize_alias(label, super::aliases::MAX_ALIAS_LENGTH).unwrap_or_else(|| basename(&current.dir, self.style));
        if next == current.label {
            return false;
        }
        let mut updated = current.clone();
        updated.label = next;
        self.map.insert(key, updated);
        self.save();
        true
    }

    pub fn remove(&mut self, dir: &str) -> bool {
        let removed = self.map.shift_remove(&self.key_of(dir)).is_some();
        if removed {
            self.save();
        }
        removed
    }

    pub fn reorder(&mut self, dirs: &[String]) -> bool {
        let before: Vec<String> = self.map.keys().cloned().collect();
        let mut next: IndexMap<String, Favorite> = IndexMap::new();
        for dir in dirs {
            let key = self.key_of(dir);
            if let Some(favorite) = self.map.get(&key) {
                if !next.contains_key(&key) {
                    next.insert(key, favorite.clone());
                }
            }
        }
        for (key, favorite) in &self.map {
            if !next.contains_key(key) {
                next.insert(key.clone(), favorite.clone());
            }
        }
        if next.keys().zip(before.iter()).all(|(a, b)| a == b) {
            return false;
        }
        self.map = next;
        self.save();
        true
    }

    pub fn save(&self) {
        let Some(file) = &self.file else { return };
        let rows: Vec<Value> = self
            .map
            .values()
            .enumerate()
            .map(|(order, f)| json!({ "dir": f.dir, "label": f.label, "addedAt": f.added_at, "order": order }))
            .collect();
        if let Err(err) = write_json(file, &Value::Array(rows), true) {
            eprintln!("[collector] cannot save {}: {err}", file_label(file));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use std::fs;

    fn labels(s: &FavoriteStore) -> Vec<String> {
        s.list().into_iter().map(|f| f.label).collect()
    }

    #[test]
    fn sorts_legacy_files_by_label_then_keeps_and_persists_manual_order() {
        let root = temp_dir("fav");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("favorites.json");
        let r = root.to_string_lossy().into_owned();
        let (a, b, c) = (format!("{r}/a"), format!("{r}/b"), format!("{r}/c"));
        fs::write(
            &file,
            json!([
                { "dir": c, "label": "Charlie", "addedAt": "2026-01-01T00:00:00.000Z" },
                { "dir": a, "label": "Alpha", "addedAt": "2026-01-02T00:00:00.000Z" },
                { "dir": b, "label": "Bravo", "addedAt": "2026-01-03T00:00:00.000Z" },
            ])
            .to_string(),
        )
        .unwrap();
        let mut store = FavoriteStore::new(Some(file.clone()), Platform::Win32);
        store.load();
        assert_eq!(labels(&store), ["Alpha", "Bravo", "Charlie"]);

        assert!(store.reorder(&[c.to_uppercase(), a.clone(), "Z:/missing".into(), c.clone()]));
        assert_eq!(labels(&store), ["Charlie", "Alpha", "Bravo"]);
        assert!(!store.reorder(&[c.clone(), a.clone()]));
        assert!(store.rename(&a, &json!("Zulu")));
        assert_eq!(labels(&store), ["Charlie", "Zulu", "Bravo"]);
        assert_eq!(store.add(&format!("{r}/d"), &json!("Delta")), AddResult::Ok);

        let saved: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        let rows: Vec<(String, i64)> = saved.as_array().unwrap().iter().map(|f| (f["label"].as_str().unwrap().into(), f["order"].as_i64().unwrap())).collect();
        assert_eq!(rows, [("Charlie".into(), 0), ("Zulu".into(), 1), ("Bravo".into(), 2), ("Delta".into(), 3)]);

        let mut again = FavoriteStore::new(Some(file), Platform::Darwin);
        again.load();
        assert_eq!(labels(&again), ["Charlie", "Zulu", "Bravo", "Delta"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn key_folds_case_only_on_windows() {
        assert_eq!(favorite_key("/Work/App", Platform::Win32), favorite_key("/work/app", Platform::Win32));
        assert_ne!(favorite_key("/Work/App", Platform::Darwin), favorite_key("/work/app", Platform::Darwin));
    }

    #[test]
    fn toggle_add_remove_and_limit() {
        let mut s = FavoriteStore::new(None, Platform::Darwin).with_style(Style::Posix, "/cwd");
        assert!(s.toggle("/p/one", "  One  "));
        assert_eq!(s.get("/p/one/").map(|f| f.label.as_str()), Some("One"));
        assert!(!s.toggle("/p/one", "x"));
        assert_eq!(s.add("/p/two", &json!(null)), AddResult::Ok);
        assert_eq!(s.get("/p/two").map(|f| f.label.as_str()), Some("two"));
        assert_eq!(s.add("/p/two", &json!("x")), AddResult::Exists);
        assert!(s.remove("/p/two"));
        assert!(!s.remove("/p/two"));
        for i in 0..MAX_FAVORITES {
            s.add(&format!("/q/{i}"), &json!(null));
        }
        assert_eq!(s.list().len(), MAX_FAVORITES);
        assert_eq!(s.add("/q/extra", &json!(null)), AddResult::Limit);
        assert!(!s.toggle("/q/extra", "x"));
        assert!(!s.rename("/q/none", &json!("x")));
        assert!(!s.rename("/q/0", &json!("0")));
    }

    #[test]
    fn skips_relative_and_malformed_rows() {
        let root = temp_dir("fav-bad");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("favorites.json");
        fs::write(&file, r#"[null, 5, {"dir":"rel"}, {"dir":"/ok","order":"x"}, {"dir":"/ok2","label":"  ","addedAt":3}]"#).unwrap();
        let mut s = FavoriteStore::new(Some(file), Platform::Darwin).with_style(Style::Posix, "/cwd");
        s.load();
        let list = s.list();
        assert_eq!(list.iter().map(|f| f.label.as_str()).collect::<Vec<_>>(), ["ok", "ok2"]);
        assert_eq!(list[1].added_at, "1970-01-01T00:00:00.000Z");
        let _ = fs::remove_dir_all(&root);
    }
}
