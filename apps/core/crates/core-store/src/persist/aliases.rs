use std::path::PathBuf;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::json_file::{file_label, read_json, write_json};
use super::jsstr::{collapse_spaces, slice_units, trim};

pub const MAX_ALIAS_LENGTH: usize = 80;
pub const MAX_NOTE_LENGTH: usize = 140;

pub fn sanitize_alias_str(value: &str, max: usize) -> Option<String> {
    let spaced: String = value.chars().map(|c| if c <= '\u{1f}' || c == '\u{7f}' { ' ' } else { c }).collect();
    let clean = collapse_spaces(&spaced);
    let clean = trim(&clean);
    (!clean.is_empty()).then(|| slice_units(clean, max))
}

pub fn sanitize_alias(value: &Value, max: usize) -> Option<String> {
    match value {
        Value::String(s) => sanitize_alias_str(s, max),
        _ => None,
    }
}

pub struct AliasStore {
    map: IndexMap<String, String>,
    file: Option<PathBuf>,
    max: usize,
}

impl AliasStore {
    pub fn new(file: Option<PathBuf>, max: usize) -> Self {
        Self { map: IndexMap::new(), file, max }
    }

    pub fn load(&mut self) {
        let Some(file) = &self.file else { return };
        let entries: Vec<(String, Value)> = match read_json(file) {
            Some(Value::Object(o)) => o.into_iter().collect(),
            Some(Value::Array(a)) => a.into_iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect(),
            _ => return,
        };
        for (id, value) in entries {
            if let Some(alias) = sanitize_alias(&value, self.max) {
                self.map.insert(id, alias);
            }
        }
    }

    pub fn get(&self, terminal_id: &str) -> Option<&str> {
        self.map.get(terminal_id).map(String::as_str)
    }

    pub fn entries(&self) -> Vec<(String, String)> {
        self.map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn set(&mut self, terminal_id: &str, alias: Option<&str>) -> bool {
        let prev = self.map.get(terminal_id).cloned();
        match alias.filter(|a| !a.is_empty()) {
            Some(a) => {
                self.map.insert(terminal_id.to_string(), a.to_string());
            }
            None => {
                self.map.shift_remove(terminal_id);
            }
        }
        if prev.as_deref() == alias.filter(|a| !a.is_empty()) {
            return false;
        }
        self.save();
        true
    }

    pub fn delete(&mut self, terminal_id: &str) {
        if self.map.shift_remove(terminal_id).is_some() {
            self.save();
        }
    }

    pub fn save(&self) {
        let Some(file) = &self.file else { return };
        let obj: Map<String, Value> = self.map.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();
        if let Err(err) = write_json(file, &Value::Object(obj), true) {
            eprintln!("[collector] cannot save {}: {err}", file_label(file));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use serde_json::json;
    use std::fs;

    #[test]
    fn sanitizes_like_the_node_helper() {
        assert_eq!(sanitize_alias(&json!("  a\u{0}\tb\n c "), 80).as_deref(), Some("a b c"));
        assert_eq!(sanitize_alias(&json!("   "), 80), None);
        assert_eq!(sanitize_alias(&json!(5), 80), None);
        assert_eq!(sanitize_alias(&json!("abcdef"), 3).as_deref(), Some("abc"));
    }

    #[test]
    fn persists_and_reloads_names() {
        let dir = temp_dir("aliases");
        let file = dir.join("aliases.json");
        let mut s = AliasStore::new(Some(file.clone()), MAX_ALIAS_LENGTH);
        assert!(s.set("t1", Some("Alpha")));
        assert!(!s.set("t1", Some("Alpha")));
        assert!(s.set("t2", Some("Beta")));
        assert!(s.set("t2", None));
        assert!(!s.set("t3", None));
        s.delete("t9");
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\n  \"t1\": \"Alpha\"\n}");
        let mut again = AliasStore::new(Some(file.clone()), 3);
        again.load();
        assert_eq!(again.get("t1"), Some("Alp"));
        fs::write(&file, r#"{"a":"x","b":7,"c":"  "}"#).unwrap();
        let mut odd = AliasStore::new(Some(file), MAX_ALIAS_LENGTH);
        odd.load();
        assert_eq!(odd.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
