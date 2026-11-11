use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::{json, Value};

struct Scope {
    accept: Vec<String>,
    need: Option<Value>,
}

thread_local! {
    static SCOPE: RefCell<Option<Scope>> = const { RefCell::new(None) };
}

pub struct Guard;

pub fn enter(accept: &[String]) -> Guard {
    SCOPE.with(|s| *s.borrow_mut() = Some(Scope { accept: accept.to_vec(), need: None }));
    Guard
}

impl Guard {
    pub fn finish(self) -> Option<Value> {
        SCOPE.with(|s| s.borrow_mut().take()).and_then(|s| s.need)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        SCOPE.with(|s| *s.borrow_mut() = None);
    }
}

pub fn active() -> bool {
    SCOPE.with(|s| s.borrow().is_some())
}

pub fn key_of(title: &str, detail: &str) -> String {
    let mut h = DefaultHasher::new();
    title.hash(&mut h);
    detail.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn confirm(title: &str, detail: &str, ok: &str, cancel: &str) -> Option<bool> {
    SCOPE.with(|s| {
        let mut slot = s.borrow_mut();
        let scope = slot.as_mut()?;
        let key = key_of(title, detail);
        if scope.accept.contains(&key) {
            return Some(true);
        }
        if scope.need.is_none() {
            scope.need = Some(json!({ "title": title, "detail": detail, "ok": ok, "cancel": cancel, "key": key }));
        }
        Some(false)
    })
}
