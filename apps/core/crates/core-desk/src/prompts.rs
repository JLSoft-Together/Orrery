use crate::{arg, js, util, Ctx, Desk, DeskEnv};
use serde_json::{json, Map, Value};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const MODULE: &str = "prompts";
pub const FILE: &str = "prompts.json";
pub const BACKUP: &str = "prompts.json.bak";
pub const CHANGED: &str = "ccm:prompts-changed";
pub const VERSION: u64 = 1;
pub const MAX_ITEMS: usize = 200;
pub const MAX_TITLE: usize = 80;
pub const MAX_BODY: usize = 4000;
pub const MAX_ID: usize = 64;
pub const MAX_SCOPE: usize = 4096;
pub const SCOPE_ALL: &str = "all";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    Op,
    Id,
    Title,
    Body,
    Scope,
    Full,
    NotFound,
    Io,
}

impl PromptError {
    pub fn code(&self) -> &'static str {
        match self {
            PromptError::Op => "invalid-op",
            PromptError::Id => "invalid-id",
            PromptError::Title => "invalid-title",
            PromptError::Body => "invalid-body",
            PromptError::Scope => "invalid-scope",
            PromptError::Full => "too-many",
            PromptError::NotFound => "not-found",
            PromptError::Io => "io",
        }
    }
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PromptError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub body: String,
    pub scope: String,
    pub updated_at: u64,
}

impl Item {
    pub fn to_json(&self) -> Value {
        json!({"id": self.id, "title": self.title, "body": self.body, "scope": self.scope, "updatedAt": self.updated_at})
    }
}

pub fn clean_id(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let ok = !s.is_empty() && s.len() <= MAX_ID && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then(|| s.to_owned())
}

pub fn clean_title(v: &Value) -> Option<String> {
    let s = v.as_str()?.trim();
    let ok = !s.is_empty() && js::len(s) <= MAX_TITLE && !s.chars().any(char::is_control);
    ok.then(|| s.to_owned())
}

pub fn clean_body(v: &Value) -> Option<String> {
    let s = v.as_str()?.replace("\r\n", "\n").replace('\r', "\n");
    let ok = !s.trim().is_empty() && js::len(&s) <= MAX_BODY && !s.contains('\0');
    ok.then_some(s)
}

pub fn clean_scope(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    if s == SCOPE_ALL {
        return Some(SCOPE_ALL.to_owned());
    }
    let ok = s.len() <= MAX_SCOPE && !s.chars().any(char::is_control) && crate::claude::is_absolute(s);
    ok.then(|| s.to_owned())
}

fn at_of(v: &Value) -> Option<u64> {
    let f = v.as_f64()?;
    (f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f < 9.0e15).then_some(f as u64)
}

fn item_from(v: &Value) -> Option<Item> {
    Some(Item {
        id: clean_id(v.get("id")?)?,
        title: clean_title(v.get("title")?)?,
        body: clean_body(v.get("body")?)?,
        scope: clean_scope(v.get("scope")?)?,
        updated_at: v.get("updatedAt").and_then(at_of).unwrap_or(0),
    })
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Library {
    pub items: Vec<Item>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Loaded {
    pub lib: Library,
    pub readable: bool,
    pub recovered: bool,
}

pub fn file_for(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn parse(bytes: &[u8]) -> Option<Library> {
    let text = String::from_utf8_lossy(bytes);
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let items = v.get("items")?.as_array()?;
    let mut lib = Library::default();
    for raw in items {
        let Some(item) = item_from(raw) else { continue };
        if lib.items.len() < MAX_ITEMS && !lib.items.iter().any(|x| x.id == item.id) {
            lib.items.push(item);
        }
    }
    Some(lib)
}

fn newer_than_known(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))
        .ok()
        .and_then(|v| v.get("version").and_then(Value::as_u64))
        .is_some_and(|n| n > VERSION)
}

pub fn load(file: &Path) -> Loaded {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded { readable: true, ..Loaded::default() },
        Err(_) => return Loaded::default(),
    };
    if newer_than_known(&bytes) {
        return Loaded::default();
    }
    match parse(&bytes) {
        Some(lib) => Loaded { lib, readable: true, recovered: false },
        None => {
            let moved = std::fs::rename(file, file.with_file_name(BACKUP)).is_ok();
            Loaded { lib: Library::default(), readable: moved, recovered: moved }
        }
    }
}

impl Library {
    pub fn to_json(&self) -> Value {
        json!({"version": VERSION, "items": self.items.iter().map(Item::to_json).collect::<Vec<_>>()})
    }

    pub fn save(&self, file: &Path) -> Result<(), PromptError> {
        let text = serde_json::to_string_pretty(&self.to_json()).map_err(|_| PromptError::Io)?;
        util::write_atomic(file, &text).map_err(|_| PromptError::Io)
    }

    fn draft(p: &Value) -> Result<(String, String, String), PromptError> {
        let title = p.get("title").and_then(clean_title).ok_or(PromptError::Title)?;
        let body = p.get("body").and_then(clean_body).ok_or(PromptError::Body)?;
        let scope = p.get("scope").map_or(Some(SCOPE_ALL.to_owned()), clean_scope).ok_or(PromptError::Scope)?;
        Ok((title, body, scope))
    }

    pub fn upsert(&mut self, p: &Value, now: u64) -> Result<String, PromptError> {
        let given = match p.get("id") {
            None | Some(Value::Null) => None,
            Some(v) => Some(clean_id(v).ok_or(PromptError::Id)?),
        };
        let (title, body, scope) = Self::draft(p)?;
        if let Some(item) = given.as_ref().and_then(|id| self.items.iter_mut().find(|x| &x.id == id)) {
            (item.title, item.body, item.scope, item.updated_at) = (title, body, scope, now);
            return Ok(item.id.clone());
        }
        if self.items.len() >= MAX_ITEMS {
            return Err(PromptError::Full);
        }
        let id = given.unwrap_or_else(util::uuid);
        self.items.push(Item { id: id.clone(), title, body, scope, updated_at: now });
        Ok(id)
    }

    pub fn delete(&mut self, id: &Value) -> Result<(), PromptError> {
        let id = clean_id(id).ok_or(PromptError::Id)?;
        let before = self.items.len();
        self.items.retain(|x| x.id != id);
        (self.items.len() != before).then_some(()).ok_or(PromptError::NotFound)
    }

    pub fn reorder(&mut self, ids: &Value) -> Result<(), PromptError> {
        let ids = ids.as_array().ok_or(PromptError::Op)?;
        let mut rest = std::mem::take(&mut self.items);
        for id in ids.iter().filter_map(Value::as_str) {
            if let Some(at) = rest.iter().position(|x| x.id == id) {
                self.items.push(rest.remove(at));
            }
        }
        self.items.append(&mut rest);
        Ok(())
    }

    pub fn import(&mut self, list: &Value, now: u64) -> Result<(u64, u64), PromptError> {
        let list = list.as_array().ok_or(PromptError::Op)?;
        let (mut added, mut skipped) = (0, 0);
        for raw in list {
            let mut p = match raw {
                Value::Object(m) => m.clone(),
                _ => Map::new(),
            };
            p.remove("id");
            match self.upsert(&Value::Object(p), now) {
                Ok(_) => added += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok((added, skipped))
    }

    pub fn apply(&mut self, req: &Value, now: u64) -> Result<Value, PromptError> {
        let op = req.get("op").and_then(Value::as_str).ok_or(PromptError::Op)?;
        match op {
            "upsert" => {
                let id = self.upsert(req.get("item").ok_or(PromptError::Op)?, now)?;
                Ok(json!({"id": id}))
            }
            "delete" => self.delete(req.get("id").unwrap_or(&Value::Null)).map(|_| json!({})),
            "reorder" => self.reorder(req.get("ids").unwrap_or(&Value::Null)).map(|_| json!({})),
            "import" => self.import(req.get("items").unwrap_or(&Value::Null), now).map(|(a, s)| json!({"added": a, "skipped": s})),
            _ => Err(PromptError::Op),
        }
    }
}

fn reply(lib: &Library, extra: Value) -> Value {
    let mut out = json!({"ok": true, "version": VERSION, "items": lib.to_json()["items"]});
    if let (Some(o), Value::Object(e)) = (out.as_object_mut(), extra) {
        o.extend(e);
    }
    out
}

fn failure(e: &PromptError) -> Value {
    json!({"ok": false, "error": e.code()})
}

pub fn set(dir: &Path, lock: &Mutex<()>, req: &Value, ctx: Option<&Ctx>) -> Value {
    let _g = lock.lock().unwrap_or_else(|p| p.into_inner());
    let file = file_for(dir);
    let loaded = load(&file);
    if !loaded.readable {
        return failure(&PromptError::Io);
    }
    let mut lib = loaded.lib;
    let extra = match lib.apply(req, util::now_ms() as u64) {
        Ok(v) => v,
        Err(e) => return failure(&e),
    };
    if let Err(e) = lib.save(&file) {
        return failure(&e);
    }
    if let Some(ctx) = ctx {
        ctx.emit(CHANGED, json!({"count": lib.items.len()}));
    }
    reply(&lib, extra)
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let dir = env.data_dir.clone();
    let lock = Arc::new(Mutex::new(()));

    let (d, l) = (dir.clone(), lock.clone());
    desk.on(MODULE, "ccm:prompts-get", move |_, _| {
        let _g = l.lock().unwrap_or_else(|p| p.into_inner());
        let loaded = load(&file_for(&d));
        if !loaded.readable {
            return Ok(failure(&PromptError::Io));
        }
        Ok(reply(&loaded.lib, json!({"recovered": loaded.recovered})))
    });

    desk.on(MODULE, "ccm:prompts-set", move |ctx, args| Ok(set(&dir, &lock, arg(args, 0), Some(ctx))));
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
