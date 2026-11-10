use crate::claude::{fold, resolve};
use crate::{js, util};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

pub const GH_ACCOUNTS_FILE: &str = "github-accounts.json";
pub const MAX_ACCOUNTS: usize = 20;
const MAX_REPOS: usize = 500;

pub fn is_choice(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.is_empty() && js::len(s) <= 64)
}

pub fn root_key(root: &str) -> String {
    fold(&resolve(root), cfg!(windows))
}

pub struct Store {
    file: PathBuf,
    pub accounts: Vec<Map<String, Value>>,
    pub default_id: Option<String>,
    pub repos: Map<String, Value>,
}

fn str_of<'a>(a: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

impl Store {
    pub fn load(file: &Path) -> Store {
        let mut st = Store { file: file.to_path_buf(), accounts: Vec::new(), default_id: None, repos: Map::new() };
        let Some(raw) = std::fs::read_to_string(file).ok().and_then(|t| js::parse(&t)) else { return st };
        let Some(obj) = raw.as_object() else { return st };
        let accounts: Vec<Map<String, Value>> = match obj.get("accounts") {
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|v| v.as_object())
                .filter(|a| str_of(a, "id").is_some() && str_of(a, "login").is_some() && str_of(a, "token").is_some())
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        if let Some(Value::Object(r)) = obj.get("repos") {
            for (k, v) in js::entries(r) {
                if is_choice(v) {
                    st.repos.insert(k.clone(), v.clone());
                }
            }
        }
        let raw_default = obj.get("defaultId");
        let default_id = if accounts.iter().any(|a| a.get("id") == raw_default) {
            raw_default.and_then(Value::as_str).map(str::to_string)
        } else {
            accounts.first().and_then(|a| str_of(a, "id")).map(str::to_string)
        };
        st.accounts = accounts.into_iter().take(MAX_ACCOUNTS).collect();
        st.default_id = default_id;
        st
    }

    pub fn id_of(a: &Map<String, Value>) -> &str {
        str_of(a, "id").unwrap_or("")
    }

    pub fn has(&self, id: &str) -> bool {
        self.accounts.iter().any(|a| Self::id_of(a) == id)
    }

    pub fn find(&self, id: &str) -> Option<&Map<String, Value>> {
        self.accounts.iter().find(|a| Self::id_of(a) == id)
    }

    pub fn list(&self) -> Value {
        Value::Array(
            self.accounts
                .iter()
                .map(|a| {
                    let mut m = Map::new();
                    for k in ["id", "login", "name", "scopes", "addedAt"] {
                        if let Some(v) = a.get(k) {
                            m.insert(k.into(), v.clone());
                        }
                    }
                    m.insert("isDefault".into(), json!(self.default_id.as_deref() == Some(Self::id_of(a))));
                    Value::Object(m)
                })
                .collect(),
        )
    }

    pub fn put(&mut self, entry: Map<String, Value>, same: Option<&str>) {
        let id = Self::id_of(&entry).to_string();
        match same {
            Some(s) => {
                for a in self.accounts.iter_mut() {
                    if Self::id_of(a) == s {
                        *a = entry.clone();
                    }
                }
            }
            None => self.accounts.push(entry),
        }
        if self.default_id.is_none() {
            self.default_id = Some(id);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.accounts.retain(|a| Self::id_of(a) != id);
        if self.default_id.as_deref() == Some(id) {
            self.default_id = self.accounts.first().map(|a| Self::id_of(a).to_string());
        }
        let gone: Vec<String> = self.repos.iter().filter(|(_, v)| v.as_str() == Some(id)).map(|(k, _)| k.clone()).collect();
        for k in gone {
            self.repos.shift_remove(&k);
        }
    }

    pub fn choice_of(&self, root: &str) -> String {
        match self.repos.get(&root_key(root)).and_then(Value::as_str) {
            Some(c) if c == "system" || (!c.is_empty() && self.has(c)) => c.to_string(),
            _ => "auto".into(),
        }
    }

    pub fn set_choice(&mut self, root: &str, choice: &Value) -> bool {
        let Some(c) = choice.as_str().filter(|_| is_choice(choice)) else { return false };
        if c != "auto" && c != "system" && !self.has(c) {
            return false;
        }
        let key = root_key(root);
        self.repos.shift_remove(&key);
        if c != "auto" {
            self.repos.insert(key, json!(c));
        }
        let extra = self.repos.len().saturating_sub(MAX_REPOS);
        let old: Vec<String> = self.repos.keys().take(extra).cloned().collect();
        for k in old {
            self.repos.shift_remove(&k);
        }
        true
    }

    pub fn save(&self) {
        let mut m = Map::new();
        m.insert("accounts".into(), Value::Array(self.accounts.iter().cloned().map(Value::Object).collect()));
        m.insert("defaultId".into(), self.default_id.as_ref().map(|s| json!(s)).unwrap_or(Value::Null));
        m.insert("repos".into(), Value::Object(self.repos.clone()));
        let text = js::stringify_indent(&Value::Object(m), "  ");
        if util::write_atomic(&self.file, &text).is_ok() {
            restrict(&self.file);
        }
    }
}

#[cfg(unix)]
fn restrict(file: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_file: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_filters_and_saves_in_order() {
        let dir = std::env::temp_dir().join(format!("ccm-gh-store-{}", util::uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(GH_ACCOUNTS_FILE);
        std::fs::write(&file, r#"{"accounts":[{"id":"a","login":"A","token":"x","extra":1},{"id":"b"}],"defaultId":"zz","repos":{"/r":"a","/q":""}}"#).unwrap();
        let mut st = Store::load(&file);
        assert_eq!(st.accounts.len(), 1);
        assert_eq!(st.default_id.as_deref(), Some("a"));
        assert_eq!(st.repos.len(), 1);
        assert!(st.set_choice("/x", &json!("system")));
        assert!(!st.set_choice("/x", &json!("nobody")));
        st.remove("a");
        st.save();
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(text, "{\n  \"accounts\": [],\n  \"defaultId\": null,\n  \"repos\": {\n    \"/x\": \"system\"\n  }\n}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
