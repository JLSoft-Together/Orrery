use crate::Ctx;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub fn denied() -> Value {
    json!({"ok": false, "error": "denied"})
}

fn call(ctx: &Ctx, method: &str, params: Value) -> Value {
    ctx.host(method, params).unwrap_or(Value::Null)
}

fn text(v: Value) -> Option<String> {
    v.as_str().map(String::from)
}

pub fn repo_of(ctx: &Ctx, root: &Value) -> Option<String> {
    text(call(ctx, "preview.repoOf", json!({"root": root})))
}

pub fn register_repo(ctx: &Ctx, dir: &Value) -> Result<String, String> {
    match call(ctx, "preview.registerRepo", json!({"dir": dir})) {
        Value::String(s) => Ok(s),
        v => Err(v.get("error").and_then(Value::as_str).unwrap_or("denied").to_string()),
    }
}

pub fn add_repo(ctx: &Ctx, root: &str, from: &str) {
    call(ctx, "preview.addRepo", json!({"root": root, "from": from}));
}

pub struct Allowed {
    pub real: String,
    pub roots: Vec<String>,
}

pub fn allowed(ctx: &Ctx, path: &Value, fresh: bool) -> Option<Allowed> {
    let v = call(ctx, "preview.allowed", json!({"path": path, "fresh": fresh}));
    let real = v.get("real")?.as_str()?.to_string();
    let roots = v.get("roots").and_then(Value::as_array).map(|a| a.iter().filter_map(|r| r.as_str().map(String::from)).collect()).unwrap_or_default();
    Some(Allowed { real, roots })
}

pub fn check_dir(ctx: &Ctx, dir: &Value) -> Option<String> {
    text(call(ctx, "preview.checkDir", json!({"dir": dir})))
}

pub fn editable_file(ctx: &Ctx, file: &Value) -> Option<(String, String)> {
    let v = call(ctx, "preview.editableFile", json!({"file": file}));
    Some((v.get("real")?.as_str()?.to_string(), v.get("root")?.as_str()?.to_string()))
}

pub fn caps(ctx: &Ctx) -> Value {
    call(ctx, "preview.caps", json!({}))
}

pub fn has_cap(caps: &Value, name: &str) -> bool {
    caps.get(name) == Some(&json!(true))
}

pub fn sender(ctx: &Ctx) -> u64 {
    call(ctx, "preview.sender", json!({})).as_u64().unwrap_or(0)
}

pub fn invalidate(ctx: &Ctx, root: &str) {
    call(ctx, "preview.invalidate", json!({"root": root}));
}

pub fn trash(ctx: &Ctx, path: &str) -> Result<(), String> {
    match ctx.host("preview.trash", json!({"path": path})) {
        Ok(Value::Bool(true)) => Ok(()),
        Ok(v) => Err(v.get("error").and_then(Value::as_str).unwrap_or("failed").to_string()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn confirm(ctx: &Ctx, kind: &str, req: Value) -> bool {
    matches!(ctx.host("preview.confirm", json!({"kind": kind, "req": req})), Ok(Value::Bool(true)))
}

pub fn gh_env(ctx: &Ctx, repo: &str, url: &str) -> Option<Vec<(String, String)>> {
    let v = call(ctx, "preview.ghEnv", json!({"repo": repo, "url": url}));
    let m = v.as_object()?;
    Some(m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
}

pub fn gh_resolve(ctx: &Ctx, repo: &str, url: &str) -> Value {
    match call(ctx, "preview.ghResolve", json!({"repo": repo, "url": url})) {
        Value::Null => json!({"github": false}),
        v => v,
    }
}

pub fn gh_set_choice(ctx: &Ctx, repo: &str, choice: &Value) -> bool {
    call(ctx, "preview.ghSetChoice", json!({"repo": repo, "choice": choice})) == json!(true)
}

#[derive(Default)]
pub struct Latest {
    map: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Latest {
    pub fn start(&self, key: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.map.lock().unwrap().insert(key.to_string(), flag.clone()) {
            old.store(true, Ordering::SeqCst);
        }
        flag
    }

    pub fn try_start(&self, key: &str) -> Option<Arc<AtomicBool>> {
        let mut m = self.map.lock().unwrap();
        if m.contains_key(key) {
            return None;
        }
        let flag = Arc::new(AtomicBool::new(false));
        m.insert(key.to_string(), flag.clone());
        Some(flag)
    }

    pub fn finish(&self, key: &str, flag: &Arc<AtomicBool>) {
        let mut m = self.map.lock().unwrap();
        if m.get(key).map(|f| Arc::ptr_eq(f, flag)).unwrap_or(false) {
            m.remove(key);
        }
    }

    pub fn cancel(&self, key: &str) -> bool {
        match self.map.lock().unwrap().get(key) {
            Some(f) => {
                f.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.lock().unwrap().contains_key(key)
    }
}

pub struct Busy(AtomicBool);

impl Default for Busy {
    fn default() -> Self {
        Busy(AtomicBool::new(false))
    }
}

impl Busy {
    pub fn enter(&self) -> Option<BusyGuard<'_>> {
        if self.0.swap(true, Ordering::SeqCst) {
            None
        } else {
            Some(BusyGuard(&self.0))
        }
    }
}

pub struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
