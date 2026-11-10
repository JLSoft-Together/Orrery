use crate::claude::{is_absolute, real};
use crate::{arg, js, util, Desk, DeskEnv};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const MODULE: &str = "layouts";
pub const LAYOUTS_FILE: &str = "layouts.json";
pub const MAX_PER_PROJECT: usize = 20;
const MAX_PANES: usize = 32;
const MAX_TABS: usize = 16;
const MAX_NAME: usize = 80;
const MAX_TEXT: usize = 4096;
const PRESETS: [&str; 4] = ["claude", "codex", "deepseek", "shell"];

fn text(v: &Value, max: usize) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && core_transcript::jsstr::units(s) <= max)
}

fn abs(v: &Value) -> Option<&str> {
    text(v, MAX_TEXT).filter(|s| is_absolute(s))
}

fn is_session_id(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        let b = s.as_bytes();
        b.len() == 36 && b.iter().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { *c == b'-' } else { c.is_ascii_hexdigit() })
    })
}

fn put(m: &mut Map<String, Value>, k: &str, v: Option<Value>) {
    if let Some(v) = v {
        m.insert(k.into(), v);
    }
}

fn clean_pane(v: &Value) -> Option<Value> {
    let p = v.as_object()?;
    let cwd = abs(p.get("cwd").unwrap_or(&Value::Null)).map(|s| json!(s));
    let mut out = Map::new();
    let kind = p.get("kind").and_then(Value::as_str)?;
    match kind {
        "term" => {
            let preset = p.get("preset").and_then(Value::as_str).filter(|s| PRESETS.contains(s))?;
            out.insert("kind".into(), json!("term"));
            out.insert("preset".into(), json!(preset));
            put(&mut out, "cwd", cwd);
            let resume = p.get("resume").filter(|r| preset != "shell" && is_session_id(r)).cloned();
            put(&mut out, "resume", resume);
        }
        "browser" => {
            let url = p.get("url").unwrap_or(&Value::Null);
            let empty = p.get("url").is_none() || url == "";
            if !empty && !text(url, MAX_TEXT).is_some_and(|u| u.get(..7).is_some_and(|h| h.eq_ignore_ascii_case("http://")) || u.get(..8).is_some_and(|h| h.eq_ignore_ascii_case("https://")) || is_absolute(u)) {
                return None;
            }
            out.insert("kind".into(), json!("browser"));
            out.insert("url".into(), json!(url.as_str().unwrap_or("")));
            put(&mut out, "cwd", cwd);
        }
        "file" => {
            let path = abs(p.get("path").unwrap_or(&Value::Null))?;
            out.insert("kind".into(), json!("file"));
            out.insert("path".into(), json!(path));
            let line = p.get("line").and_then(Value::as_f64).filter(|l| *l > 0.0).map(|l| Value::from(l.floor()));
            put(&mut out, "line", line);
            put(&mut out, "cwd", cwd);
        }
        "changes" | "log" | "run" => {
            out.insert("kind".into(), json!(kind));
            out.insert("cwd".into(), cwd?);
        }
        _ => return None,
    }
    Some(Value::Object(out))
}

fn clean_tree(n: &Value, panes: &Map<String, Value>, depth: usize) -> Option<Value> {
    let node = n.as_object().filter(|_| depth <= 12)?;
    if node.contains_key("pane") {
        let key = text(&node["pane"], 64).filter(|k| panes.contains_key(*k))?;
        return Some(json!({"pane": key}));
    }
    let dir = node.get("dir").and_then(Value::as_str).filter(|d| *d == "row" || *d == "col")?;
    let a = clean_tree(node.get("a").unwrap_or(&Value::Null), panes, depth + 1);
    let b = clean_tree(node.get("b").unwrap_or(&Value::Null), panes, depth + 1);
    let ratio = node.get("ratio").and_then(Value::as_f64).filter(|r| r.is_finite()).map(|r| r.clamp(0.15, 0.85)).unwrap_or(0.5);
    match (a, b) {
        (Some(a), Some(b)) => Some(json!({"dir": dir, "ratio": ratio, "a": a, "b": b})),
        (a, b) => a.or(b),
    }
}

fn leaves(n: &Value, out: &mut Vec<String>) {
    match n.get("pane").and_then(Value::as_str) {
        Some(p) => out.push(p.into()),
        None => {
            leaves(&n["a"], out);
            leaves(&n["b"], out);
        }
    }
}

pub fn clean_layout(v: &Value) -> Option<Map<String, Value>> {
    let l = v.as_object()?;
    let project = abs(l.get("project").unwrap_or(&Value::Null))?;
    let name = js::slice(core_transcript::jsstr::js_trim(l.get("name")?.as_str()?), MAX_NAME);
    if name.is_empty() {
        return None;
    }
    let mut panes = Map::new();
    if let Some(Value::Object(raw)) = l.get("panes") {
        for (k, p) in js::entries(raw).into_iter().take(MAX_PANES) {
            if let (Some(pane), true) = (clean_pane(p), text(&Value::String(k.clone()), 64).is_some()) {
                panes.insert(k.clone(), pane);
            }
        }
    } else if let Some(Value::Array(raw)) = l.get("panes") {
        for (i, p) in raw.iter().enumerate().take(MAX_PANES) {
            if let Some(pane) = clean_pane(p) {
                panes.insert(i.to_string(), pane);
            }
        }
    }
    let mut tabs = Vec::new();
    let mut used: Vec<String> = Vec::new();
    for t in l.get("tabs").and_then(Value::as_array).map(|a| &a[..a.len().min(MAX_TABS)]).unwrap_or(&[]) {
        let Some(root) = clean_tree(t.get("root").unwrap_or(&Value::Null), &panes, 0) else { continue };
        let mut keys = Vec::new();
        leaves(&root, &mut keys);
        if keys.iter().any(|k| used.contains(k)) {
            continue;
        }
        used.extend(keys.iter().cloned());
        let focus = t.get("focus").and_then(Value::as_str).filter(|f| keys.iter().any(|k| k == f)).map(str::to_string).unwrap_or_else(|| keys[0].clone());
        tabs.push(json!({"root": root, "focus": focus}));
    }
    if tabs.is_empty() {
        return None;
    }
    panes.retain(|k, _| used.contains(k));
    let mut out = Map::new();
    out.insert("id".into(), text(l.get("id").unwrap_or(&Value::Null), 64).map(|s| json!(s)).unwrap_or(Value::Null));
    out.insert("project".into(), json!(project));
    out.insert("name".into(), json!(name));
    let open = l.get("openOnStart") == Some(&Value::Bool(true));
    out.insert("openOnStart".into(), if open { Value::Bool(true) } else { Value::Null });
    out.insert("panes".into(), Value::Object(panes));
    out.insert("tabs".into(), Value::Array(tabs));
    Some(out)
}

pub fn project_key(project: &str) -> String {
    let p = real(project).unwrap_or_else(|| project.to_string());
    let r = crate::claude::resolve(&p);
    if cfg!(windows) {
        r.to_lowercase()
    } else {
        r
    }
}

fn shown(l: &Map<String, Value>) -> Value {
    Value::Object(l.iter().filter(|(k, v)| !(v.is_null() && (*k == "id" || *k == "openOnStart"))).map(|(k, v)| (k.clone(), v.clone())).collect())
}

pub struct LayoutStore {
    file: Option<PathBuf>,
    list: Vec<Map<String, Value>>,
}

fn key_of(l: &Map<String, Value>) -> String {
    project_key(l["project"].as_str().unwrap_or(""))
}

impl LayoutStore {
    pub fn open(file: Option<PathBuf>) -> Self {
        let mut list = Vec::new();
        if let Some(f) = &file {
            if let Some(Value::Array(raw)) = std::fs::read(f).ok().and_then(|b| serde_json::from_str::<Value>(&String::from_utf8_lossy(&b)).ok()) {
                for item in raw {
                    let Some(mut clean) = clean_layout(&item).filter(|c| c.get("id").is_some_and(Value::is_string)) else { continue };
                    let at = item.get("updatedAt").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| util::iso_string(0.0));
                    clean.insert("updatedAt".into(), json!(at));
                    list.push(clean);
                }
            }
        }
        LayoutStore { file, list }
    }

    pub fn for_project(&self, project: &str) -> Value {
        let key = project_key(project);
        let mut hits: Vec<&Map<String, Value>> = self.list.iter().filter(|l| key_of(l) == key).collect();
        hits.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(b["updatedAt"].as_str().unwrap_or(""), a["updatedAt"].as_str().unwrap_or("")));
        Value::Array(hits.into_iter().map(shown).collect())
    }

    pub fn save(&mut self, input: &Value) -> Value {
        let Some(clean) = clean_layout(input) else { return json!({"ok": false, "error": "invalid"}) };
        let key = key_of(&clean);
        let same: Vec<usize> = (0..self.list.len()).filter(|i| key_of(&self.list[*i]) == key).collect();
        let existing = clean.get("id").filter(|v| v.is_string()).and_then(|id| same.iter().copied().find(|i| self.list[*i].get("id") == Some(id)));
        if existing.is_none() && same.len() >= MAX_PER_PROJECT {
            return json!({"ok": false, "error": "limit"});
        }
        let id = existing.map(|i| self.list[i]["id"].clone()).unwrap_or_else(|| json!(util::uuid()));
        let mut ordered = clean;
        ordered.insert("id".into(), id.clone());
        ordered.insert("updatedAt".into(), json!(util::iso_string(util::now_ms())));
        if ordered.get("openOnStart") == Some(&Value::Bool(true)) {
            for i in &same {
                if self.list[*i].get("id") != Some(&id) {
                    self.list[*i].shift_remove("openOnStart");
                }
            }
        }
        match existing {
            Some(i) => self.list[i] = ordered.clone(),
            None => self.list.push(ordered.clone()),
        }
        self.write();
        json!({"ok": true, "layout": shown(&ordered)})
    }

    pub fn set_open_on_start(&mut self, project: &str, id: Option<&str>) -> bool {
        let key = project_key(project);
        let mut found = id.is_none();
        for l in self.list.iter_mut() {
            if key_of(l) != key {
                continue;
            }
            if id.is_some() && l.get("id").and_then(Value::as_str) == id {
                l.insert("openOnStart".into(), Value::Bool(true));
                found = true;
            } else {
                l.shift_remove("openOnStart");
            }
        }
        if found {
            self.write();
        }
        found
    }

    pub fn delete(&mut self, project: &str, id: &str) -> bool {
        let key = project_key(project);
        let before = self.list.len();
        self.list.retain(|l| !(l.get("id").and_then(Value::as_str) == Some(id) && key_of(l) == key));
        if self.list.len() == before {
            return false;
        }
        self.write();
        true
    }

    fn write(&self) {
        let Some(file) = &self.file else { return };
        let text = js::stringify_indent(&Value::Array(self.list.iter().map(shown).collect()), "  ");
        let tmp = PathBuf::from(format!("{}.tmp", file.display()));
        let _ = file.parent().map(std::fs::create_dir_all);
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, file);
        }
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let store = Arc::new(Mutex::new(None::<LayoutStore>));
    let file = env.data_dir.join(LAYOUTS_FILE);
    let with = move |f: &dyn Fn(&mut LayoutStore) -> Value| {
        let mut g = store.lock().unwrap();
        let s = g.get_or_insert_with(|| LayoutStore::open(Some(file.clone())));
        f(s)
    };
    let with = Arc::new(with);
    let w = with.clone();
    desk.on(MODULE, "ccm:layouts-list", move |_, args| Ok(match abs(arg(args, 0)) {
        Some(p) => w(&|s| s.for_project(p)),
        None => json!([]),
    }));
    let w = with.clone();
    desk.on(MODULE, "ccm:layouts-save", move |_, args| Ok(w(&|s| s.save(arg(args, 0)))));
    let w = with.clone();
    desk.on(MODULE, "ccm:layouts-delete", move |_, args| {
        let (Some(p), Some(id)) = (abs(arg(args, 0)), text(arg(args, 1), 64)) else { return Ok(Value::Bool(false)) };
        Ok(w(&|s| Value::Bool(s.delete(p, id))))
    });
    let w = with;
    desk.on(MODULE, "ccm:layouts-start", move |_, args| {
        let id = arg(args, 1);
        let Some(p) = abs(arg(args, 0)).filter(|_| args.len() >= 2) else { return Ok(Value::Bool(false)) };
        if !id.is_null() && text(id, 64).is_none() {
            return Ok(Value::Bool(false));
        }
        Ok(w(&|s| Value::Bool(s.set_open_on_start(p, id.as_str()))))
    });
}
