use super::configs::{clean_run_config, drop_blocked_env, is_run_config_id};
use super::settings::{read_json, write_json};
use crate::preview::paths::{is_absolute, relative, resolve_str};
use core_transcript::collate::locale_cmp;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const RUN_CONFIGS_DIR: &str = "run-configs";
pub const RUN_SHARED_DIR: [&str; 2] = [".agent-monitor", "run"];
pub const TRUST_FILE: &str = "run-trust.json";
const MAX_HASHES: usize = 500;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn canonical(v: &Value) -> Value {
    match v {
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        Value::Object(o) => {
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            Value::Object(keys.into_iter().map(|k| (k.clone(), canonical(&o[k]))).collect())
        }
        other => other.clone(),
    }
}

pub fn config_hash(cfg: &Value) -> String {
    hex(&Sha256::digest(crate::js::stringify(&canonical(cfg)).as_bytes()))
}

pub struct TrustStore {
    dir: PathBuf,
    cache: Mutex<Option<Vec<String>>>,
}

impl TrustStore {
    pub fn new(dir: &Path) -> Self {
        TrustStore { dir: dir.to_path_buf(), cache: Mutex::new(None) }
    }

    fn load(&self) -> Vec<String> {
        let mut g = self.cache.lock().unwrap();
        if let Some(c) = &*g {
            return c.clone();
        }
        let list: Vec<String> = read_json(&self.dir.join(TRUST_FILE))
            .and_then(|v| v.get("hashes").and_then(Value::as_array).cloned())
            .map(|a| a.iter().filter_map(|h| h.as_str().filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))).map(String::from)).collect())
            .unwrap_or_default();
        *g = Some(list.clone());
        list
    }

    pub fn has(&self, hash: &str) -> bool {
        self.load().iter().any(|h| h == hash)
    }

    pub fn add(&self, hash: &str) {
        let mut list: Vec<String> = self.load().into_iter().filter(|h| h != hash).collect();
        list.push(hash.to_string());
        let skip = list.len().saturating_sub(MAX_HASHES);
        let list: Vec<String> = list.into_iter().skip(skip).collect();
        *self.cache.lock().unwrap() = Some(list.clone());
        let _ = write_json(&self.dir.join(TRUST_FILE), &json!({"hashes": list}));
    }
}

pub struct RunConfigStore {
    dir: PathBuf,
}

fn sort_by_name(list: &mut [Value]) {
    list.sort_by(|a, b| {
        let (an, bn) = (a["name"].as_str().unwrap_or(""), b["name"].as_str().unwrap_or(""));
        locale_cmp(an, bn).then_with(|| locale_cmp(a["id"].as_str().unwrap_or(""), b["id"].as_str().unwrap_or("")))
    });
}

impl RunConfigStore {
    pub fn new(dir: &Path) -> Self {
        RunConfigStore { dir: dir.to_path_buf() }
    }

    pub fn personal_file(&self, root: &str) -> PathBuf {
        let digest = hex(&Sha256::digest(crate::layouts::project_key(root).as_bytes()));
        self.dir.join(RUN_CONFIGS_DIR).join(format!("{}.json", &digest[..24]))
    }

    fn shared_dir(root: &str) -> PathBuf {
        Path::new(root).join(RUN_SHARED_DIR[0]).join(RUN_SHARED_DIR[1])
    }

    fn shared_file(root: &str, id: &str) -> PathBuf {
        Self::shared_dir(root).join(format!("{id}.json"))
    }

    pub fn list(&self, root: &str) -> Vec<Value> {
        let mut by_id: Vec<Value> = Vec::new();
        for c in self.personal(root).into_iter().chain(self.shared(root)) {
            match by_id.iter().position(|x| x["id"] == c["id"]) {
                Some(i) => by_id[i] = c,
                None => by_id.push(c),
            }
        }
        sort_by_name(&mut by_id);
        by_id
    }

    pub fn save(&self, root: &str, raw: &Value) -> Option<Value> {
        let mut input = raw.clone();
        if let Some(o) = input.as_object_mut() {
            if !o.contains_key("id") {
                o.insert("id".into(), json!(crate::util::uuid()));
            }
        }
        let clean = clean_run_config(&input, root)?;
        let shared = clean["shared"] == true;
        let mut cfg = clean;
        if shared {
            cfg["env"] = drop_blocked_env(&cfg["env"]);
        }
        let id = cfg["id"].as_str()?.to_string();
        let personal = self.personal(root);
        let rest: Vec<Value> = personal.iter().filter(|c| c["id"] != cfg["id"]).cloned().collect();
        let ok = if shared {
            let wrote = write_json(&Self::shared_file(root, &id), &to_shared(root, &cfg)).is_ok();
            if wrote && rest.len() != personal.len() {
                self.write_personal(root, &rest).is_ok()
            } else {
                wrote
            }
        } else {
            let mut next = rest;
            next.push(cfg.clone());
            let wrote = self.write_personal(root, &next).is_ok();
            let _ = std::fs::remove_file(Self::shared_file(root, &id));
            wrote
        };
        ok.then_some(cfg)
    }

    pub fn delete(&self, root: &str, id: &str) -> bool {
        if !is_run_config_id(&json!(id)) {
            return false;
        }
        let mut removed = false;
        let personal = self.personal(root);
        let rest: Vec<Value> = personal.iter().filter(|c| c["id"] != id).cloned().collect();
        if rest.len() != personal.len() && self.write_personal(root, &rest).is_ok() {
            removed = true;
        }
        let file = Self::shared_file(root, id);
        if file.exists() && std::fs::remove_file(&file).is_ok() {
            removed = true;
        }
        removed
    }

    fn write_personal(&self, root: &str, list: &[Value]) -> std::io::Result<()> {
        write_json(&self.personal_file(root), &Value::Array(list.to_vec()))
    }

    fn personal(&self, root: &str) -> Vec<Value> {
        let Some(Value::Array(items)) = read_json(&self.personal_file(root)) else { return Vec::new() };
        items
            .iter()
            .filter_map(|i| clean_run_config(i, root))
            .map(|mut c| {
                c["shared"] = json!(false);
                c
            })
            .collect()
    }

    fn shared(&self, root: &str) -> Vec<Value> {
        let Ok(rd) = std::fs::read_dir(Self::shared_dir(root)) else { return Vec::new() };
        let mut out = Vec::new();
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(id) = name.strip_suffix(".json") else { continue };
            if !is_run_config_id(&json!(id)) {
                continue;
            }
            let Some(Value::Object(mut raw)) = read_json(&Self::shared_file(root, id)) else { continue };
            let cwd = match raw.get("cwd") {
                Some(Value::String(rel)) if !is_absolute(rel) => json!(resolve_str(&format!("{root}/{rel}"))),
                Some(other) => other.clone(),
                None => Value::Null,
            };
            raw.insert("id".into(), json!(id));
            raw.insert("cwd".into(), cwd);
            if let Some(mut c) = clean_run_config(&Value::Object(raw), root) {
                c["shared"] = json!(true);
                c["env"] = drop_blocked_env(&c["env"]);
                out.push(c);
            }
        }
        out
    }
}

fn to_shared(root: &str, cfg: &Value) -> Value {
    let mut out = Map::new();
    for (k, v) in cfg.as_object().into_iter().flatten() {
        if k == "id" || k == "shared" {
            continue;
        }
        out.insert(k.clone(), v.clone());
    }
    let rel = relative(root, cfg["cwd"].as_str().unwrap_or("")).replace('\\', "/");
    out.insert("cwd".into(), json!(if rel.is_empty() { ".".to_string() } else { rel }));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ccm-d4-{tag}-{}-{}", std::process::id(), crate::util::uuid()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn cfg(id: &str, shared: bool, root: &str) -> Value {
        json!({"id": id, "name": id, "cwd": format!("{root}/app"), "type": "custom", "command": "echo hi", "shared": shared, "env": {"PATH": "x", "K": "v"}})
    }

    #[test]
    fn personal_and_shared_roundtrip() {
        let data = tmp("data");
        let root = tmp("root");
        let r = root.to_string_lossy().to_string();
        let store = RunConfigStore::new(&data);
        let a = store.save(&r, &cfg("a", false, &r)).unwrap();
        assert_eq!(a["env"], json!({"PATH": "x", "K": "v"}));
        let b = store.save(&r, &cfg("b", true, &r)).unwrap();
        assert_eq!(b["env"], json!({"K": "v"}));
        let file = std::fs::read_to_string(root.join(".agent-monitor/run/b.json")).unwrap();
        assert!(file.contains("\"cwd\": \"app\"") && !file.contains("\"id\""));
        let list = store.list(&r);
        assert_eq!(list.iter().map(|c| c["id"].as_str().unwrap()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(list[1]["shared"], true);
        assert!(store.delete(&r, "a") && store.delete(&r, "b") && !store.delete(&r, "b"));
        assert!(store.list(&r).is_empty());
    }

    #[test]
    fn trust_store_persists() {
        let data = tmp("trust");
        let t = TrustStore::new(&data);
        let h = config_hash(&json!({"b": 1, "a": [1, {"z": 1, "y": 2}]}));
        assert_eq!(h, config_hash(&json!({"a": [1, {"y": 2, "z": 1}], "b": 1})));
        assert!(!t.has(&h));
        t.add(&h);
        assert!(TrustStore::new(&data).has(&h));
    }
}
