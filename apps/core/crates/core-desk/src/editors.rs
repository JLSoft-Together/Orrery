use crate::{arg, js, Desk, DeskEnv};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

const MAX_FILES: usize = 200;

pub fn clean_dirty(raw: &Value) -> Vec<Value> {
    let Some(list) = raw.as_array() else { return Vec::new() };
    list.iter()
        .take(MAX_FILES)
        .filter_map(|f| {
            let path = f.get("path")?.as_str()?;
            let name = f.get("name")?.as_str()?;
            Some(json!({"name": js::slice(name, 200), "path": js::slice(path, 1024)}))
        })
        .collect()
}

#[derive(Default)]
pub struct EditorTracker {
    pages: Vec<(u64, Vec<Value>, bool)>,
}

impl EditorTracker {
    pub fn on_dirty(&mut self, page: u64, raw: &Value, auto_save: &Value) {
        let files = clean_dirty(raw);
        if files.is_empty() {
            self.forget(page);
            return;
        }
        let auto = auto_save == &Value::Bool(true);
        match self.pages.iter_mut().find(|(p, _, _)| *p == page) {
            Some(slot) => *slot = (page, files, auto),
            None => self.pages.push((page, files, auto)),
        }
    }

    pub fn forget(&mut self, page: u64) {
        self.pages.retain(|(p, _, _)| *p != page);
    }

    pub fn files(&self) -> Vec<Value> {
        self.pages.iter().flat_map(|(_, f, _)| f.iter().cloned()).collect()
    }

    pub fn auto_save(&self) -> bool {
        !self.pages.is_empty() && self.pages.iter().all(|(_, _, a)| *a)
    }

    pub fn pages(&self) -> Vec<u64> {
        self.pages.iter().map(|(p, _, _)| *p).collect()
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    let t = Arc::new(Mutex::new(EditorTracker::default()));
    let page = |v: &Value| v.as_u64().unwrap_or(0);
    let s = t.clone();
    desk.on("editors", "desk:editor-dirty", move |_, args| {
        s.lock().unwrap().on_dirty(page(arg(args, 0)), arg(args, 1), arg(args, 2));
        Ok(Value::Null)
    });
    let s = t.clone();
    desk.on("editors", "desk:editor-forget", move |_, args| {
        s.lock().unwrap().forget(page(arg(args, 0)));
        Ok(Value::Null)
    });
    let s = t;
    desk.on("editors", "desk:editor-state", move |_, _| {
        let g = s.lock().unwrap();
        Ok(json!({"files": g.files(), "autoSave": g.auto_save(), "pages": g.pages()}))
    });
}
