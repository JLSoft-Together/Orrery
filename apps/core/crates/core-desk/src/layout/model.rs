use super::gradle_graph::{reaches, script_graph, Module, Project};
use super::project_model::{parse_project_model_text, Parsed, Payload};
use super::source_model::SourceModel;
use super::LayoutError;
use crate::preview::paths::root_key;
use crate::run::android::detect::analyze_gradle_root;
use crate::run::android::pathx;
use crate::run::settings::SettingsStore;
use crate::run::types::Env;
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

pub const KIT_KEY: &str = "loose-kit";
pub const SOURCE_PREFIX: &str = "source:";
const PREFS_FILE: &str = "preferred-apps.json";
const MAX_UP: usize = 16;
const MAX_THEMES: usize = 20;

pub type LogFn = Arc<dyn Fn(&str, &str) + Send + Sync>;
pub type EmitFn = Arc<dyn Fn(Value) + Send + Sync>;

pub trait ModelDaemon: Send + Sync {
    fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, LayoutError>;
    fn request_restart(&self);
}

impl ModelDaemon for super::daemon::LayoutDaemon {
    fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, LayoutError> {
        super::daemon::LayoutDaemon::request(self, method, params, timeout)
    }

    fn request_restart(&self) {
        super::daemon::LayoutDaemon::request_restart(self)
    }
}

#[derive(Clone)]
pub struct Timing {
    pub refresh: Duration,
    pub compile: Duration,
    pub debounce: Duration,
    pub gen_debounce: Duration,
    pub load: Duration,
    pub resolve_ttl: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { refresh: Duration::from_secs(300), compile: Duration::from_secs(900), debounce: Duration::from_millis(300), gen_debounce: Duration::from_millis(1500), load: Duration::from_secs(120), resolve_ttl: Duration::from_secs(10) }
    }
}

pub struct ModelCfg {
    pub models_dir: String,
    pub init_script: String,
    pub win: bool,
    pub env: Env,
    pub home: String,
    pub settings: Arc<SettingsStore>,
    pub loose_kit_dir: Option<String>,
    pub gradle_home: String,
    pub aar_cache_dir: String,
    pub daemon: Arc<dyn ModelDaemon>,
    pub emit: EmitFn,
    pub log: LogFn,
    pub can_run: Arc<dyn Fn() -> bool + Send + Sync>,
    pub timing: Timing,
}

#[derive(Clone)]
pub struct Library {
    pub module: String,
    pub candidates: Vec<String>,
    pub linked: bool,
}

#[derive(Clone)]
pub enum Resolved {
    Target { root: String, module: String, name: String, library: Option<Library> },
    Reason(&'static str),
}

#[derive(Clone)]
pub struct Entry {
    pub key: String,
    pub root: String,
    pub module: String,
    pub name: String,
    pub hash: String,
    pub state: Value,
    pub model: Option<Arc<Parsed>>,
    pub at: f64,
    pub variant: String,
    pub broken: bool,
    pub classes_changed: bool,
    pub disk_checked: bool,
    pub code_changed_at: i64,
    pub source_themes: Vec<String>,
}

#[derive(Clone)]
pub struct SourceHit {
    pub key: String,
    pub model: Arc<SourceModel>,
}

#[derive(Clone)]
pub struct LoadedSource {
    pub kit: bool,
    pub prebuilt_at: Option<f64>,
}

pub struct Job {
    pub done: Mutex<bool>,
    pub cv: Condvar,
}

#[derive(Default)]
pub struct St {
    pub entries: HashMap<String, Entry>,
    pub resolved: HashMap<String, (Instant, Resolved)>,
    pub loaded_key: Option<String>,
    pub version: u64,
    pub wanted: Option<String>,
    pub sources: HashMap<String, (Instant, Option<SourceHit>)>,
    pub source_failed: HashSet<String>,
    pub source_entries: HashMap<String, (String, String)>,
    pub auto_tried: HashSet<String>,
    pub loaded_source: Option<LoadedSource>,
    pub kit_failed: bool,
    pub kit: Option<Payload>,
    pub prefs: Option<Map<String, Value>>,
    pub watchers: Vec<notify::RecommendedWatcher>,
    pub watched_key: Option<String>,
    pub res_pending: Vec<String>,
    pub res_slow: bool,
    pub res_due: Option<Instant>,
    pub res_timer: bool,
    pub children: HashSet<u32>,
}

pub struct ModelSvc {
    pub cfg: ModelCfg,
    pub st: Mutex<St>,
    pub serial: Mutex<()>,
    pub jobs: Mutex<HashMap<String, Arc<Job>>>,
    pub me: Weak<ModelSvc>,
}

pub fn num(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9e15 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

pub fn msg(e: &LayoutError) -> String {
    match &e.detail {
        Some(d) => format!("{}: {d}", e.code),
        None => e.code.clone(),
    }
}

pub fn show(v: &Value) -> String {
    match v {
        Value::Null => "?".into(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_i64().map(|i| i.to_string()).unwrap_or_else(|| n.to_string()),
        other => other.to_string(),
    }
}

pub fn now_ms() -> i64 {
    crate::util::now_ms() as i64
}

pub fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

pub fn find_gradle_root(file: &str) -> Option<String> {
    let mut dir = Path::new(file).parent()?.to_path_buf();
    for _ in 0..MAX_UP {
        for name in ["settings.gradle.kts", "settings.gradle"] {
            if std::fs::metadata(dir.join(name)).is_ok_and(|m| m.is_file()) {
                return Some(dir.to_string_lossy().to_string());
            }
        }
        dir = dir.parent()?.to_path_buf();
    }
    None
}

pub fn pick_module(project: &Project, file: &str, win: bool, preferred: Option<&str>, model_deps: &dyn Fn(&str) -> Option<Vec<String>>) -> Option<(String, Option<Library>)> {
    let mut owner: Option<&Module> = None;
    for m in &project.modules {
        if pathx::is_within(&m.dir, file, win) && owner.is_none_or(|o| m.dir.len() > o.dir.len()) {
            owner = Some(m);
        }
    }
    let apps: Vec<&Module> = project.modules.iter().filter(|m| m.is_app()).collect();
    if let Some(o) = owner.filter(|o| o.is_app()) {
        return Some((o.path.clone(), None));
    }
    let first = apps.first()?;
    let preferred_app = apps.iter().find(|a| Some(a.path.as_str()) == preferred).map(|a| a.path.clone());
    let Some(owner) = owner else { return Some((preferred_app.unwrap_or_else(|| first.path.clone()), None)) };
    let lib = owner.path.clone();
    let graph = script_graph(project);
    let mut candidates: Vec<String> = Vec::new();
    for app in &apps {
        let deps = model_deps(&app.path).unwrap_or_default();
        if deps.contains(&lib) || reaches(&graph, &app.path, &lib) {
            candidates.push(app.path.clone());
        }
    }
    candidates.sort();
    if candidates.is_empty() {
        return Some((preferred_app.unwrap_or_else(|| first.path.clone()), Some(Library { module: lib, candidates, linked: false })));
    }
    let module = match preferred {
        Some(p) if candidates.iter().any(|c| c == p) => p.to_string(),
        _ => candidates[0].clone(),
    };
    Some((module, Some(Library { module: lib, candidates, linked: true })))
}

fn themes_of(model: &Parsed) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in model.payload.app_theme.iter().chain(model.payload.activity_themes.iter().map(|(_, t)| t)) {
        if !out.contains(t) {
            out.push(t.clone());
        }
    }
    out.truncate(MAX_THEMES);
    out
}

impl ModelSvc {
    pub fn new(cfg: ModelCfg) -> Arc<ModelSvc> {
        Arc::new_cyclic(|me| ModelSvc { cfg, st: Mutex::new(St::default()), serial: Mutex::new(()), jobs: Mutex::new(HashMap::new()), me: me.clone() })
    }

    pub fn log(&self, level: &str, m: &str) {
        (self.cfg.log)(level, m)
    }

    pub fn emit(&self, v: Value) {
        (self.cfg.emit)(v)
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, St> {
        self.st.lock().unwrap()
    }

    pub fn key_of(&self, root: &str, module: &str) -> String {
        format!("{}\u{0}{module}", root_key(root))
    }

    pub fn entry(&self, root: &str, module: &str, name: Option<&str>) -> String {
        let key = self.key_of(root, module);
        let mut st = self.lock();
        match st.entries.get_mut(&key) {
            Some(e) => {
                if let Some(n) = name {
                    e.name = n.to_string();
                }
            }
            None => {
                let hash: String = Sha1::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..16].to_string();
                let fallback = root.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_string();
                let e = Entry {
                    key: key.clone(),
                    root: root.into(),
                    module: module.into(),
                    name: name.map(String::from).unwrap_or(fallback),
                    hash,
                    state: json!({"state": "none"}),
                    model: None,
                    at: 0.0,
                    variant: "debug".into(),
                    broken: false,
                    classes_changed: false,
                    disk_checked: false,
                    code_changed_at: 0,
                    source_themes: Vec::new(),
                };
                st.entries.insert(key.clone(), e);
            }
        }
        key
    }

    pub fn get(&self, key: &str) -> Option<Entry> {
        self.lock().entries.get(key).cloned()
    }

    fn cache_file(&self, hash: &str) -> String {
        pathx::join(self.cfg.win, &[&self.cfg.models_dir, &format!("{hash}.json")])
    }

    pub fn ready_state(&self, e: &Entry) -> Value {
        let m = e.model.as_ref().expect("model");
        let p = &m.payload;
        json!({"state": "ready", "at": num(e.at), "resCount": p.res_dirs.len() + p.aar_res_dirs.len(), "classpathCount": p.classpath.len(), "warnings": m.warnings.len()})
    }

    pub fn set_state(&self, key: &str, state: Value) {
        let (root, module) = {
            let mut st = self.lock();
            let Some(e) = st.entries.get_mut(key) else { return };
            e.state = state.clone();
            (e.root.clone(), e.module.clone())
        };
        self.emit(json!({"type": "model", "root": root, "module": module, "state": state}));
    }

    fn prefs(&self) -> Map<String, Value> {
        if let Some(p) = &self.lock().prefs {
            return p.clone();
        }
        let mut out = Map::new();
        let file = pathx::join(self.cfg.win, &[&self.cfg.models_dir, PREFS_FILE]);
        if let Some(Value::Object(raw)) = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()) {
            for (k, v) in raw {
                if v.is_string() {
                    out.insert(k, v);
                }
            }
        }
        self.lock().prefs = Some(out.clone());
        out
    }

    pub fn prefer(&self, root: &str, module: &str) -> std::io::Result<()> {
        let mut prefs = self.prefs();
        let key = root_key(root);
        self.lock().resolved.clear();
        if prefs.get(&key).and_then(Value::as_str) == Some(module) {
            return Ok(());
        }
        prefs.insert(key, json!(module));
        std::fs::create_dir_all(&self.cfg.models_dir)?;
        std::fs::write(pathx::join(self.cfg.win, &[&self.cfg.models_dir, PREFS_FILE]), Value::Object(prefs.clone()).to_string())?;
        self.lock().prefs = Some(prefs);
        Ok(())
    }

    pub fn resolve(&self, file: &str) -> Resolved {
        let dir_key = root_key(&crate::preview::paths::dirname(file));
        if let Some((at, v)) = self.lock().resolved.get(&dir_key) {
            if at.elapsed() < self.cfg.timing.resolve_ttl {
                return v.clone();
            }
        }
        let root = find_gradle_root(file);
        let project = root.as_deref().and_then(analyze_gradle_root).map(|v| Project::from_value(&v));
        let value = match (root, project) {
            (Some(root), Some(project)) => {
                let preferred = self.prefs().get(&root_key(&root)).and_then(Value::as_str).map(String::from);
                let deps = |app: &str| -> Option<Vec<String>> {
                    let key = self.entry(&root, app, None);
                    self.load_disk(&key);
                    self.get(&key)?.model.map(|m| m.project_deps.clone())
                };
                match pick_module(&project, file, self.cfg.win, preferred.as_deref(), &deps) {
                    Some((module, library)) => Resolved::Target { root: root.clone(), module, name: project.name.clone(), library },
                    None => Resolved::Reason("no-app"),
                }
            }
            _ => Resolved::Reason("no-gradle"),
        };
        self.lock().resolved.insert(dir_key, (Instant::now(), value.clone()));
        value
    }

    pub fn load_disk(&self, key: &str) {
        let hash = {
            let mut st = self.lock();
            let Some(e) = st.entries.get_mut(key) else { return };
            if e.disk_checked {
                return;
            }
            e.disk_checked = true;
            if e.model.is_some() {
                return;
            }
            e.hash.clone()
        };
        let file = self.cache_file(&hash);
        let text = std::fs::read(&file).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
        let at = std::fs::metadata(&file).ok().map(|m| crate::util::mtime_ms(&m));
        let Some(text) = text else { return };
        if self.lock().entries.get(key).is_none_or(|e| e.model.is_some()) {
            return;
        }
        match parse_project_model_text(&text, &exists, self.cfg.win) {
            Ok(parsed) => {
                let ready = {
                    let mut st = self.lock();
                    let Some(e) = st.entries.get_mut(key) else { return };
                    e.variant = parsed.variant.clone();
                    e.model = Some(Arc::new(parsed));
                    e.at = at.unwrap_or_else(|| now_ms() as f64);
                    e.state["state"] == "none"
                };
                if ready {
                    let e = self.get(key).expect("entry");
                    self.lock().entries.get_mut(key).expect("entry").state = self.ready_state(&e);
                }
            }
            Err(err) => self.log("warn", &format!("project model cache unreadable: {err}")),
        }
    }

    pub fn entry_for(&self, file: &str) -> Option<String> {
        let Resolved::Target { root, module, name, .. } = self.resolve(file) else { return None };
        let key = self.entry(&root, &module, Some(&name));
        self.load_disk(&key);
        Some(key)
    }

    pub fn info(&self, file: &str) -> Value {
        let (root, module, name, library) = match self.resolve(file) {
            Resolved::Target { root, module, name, library } => (root, module, name, library),
            Resolved::Reason(r) => return json!({"target": null, "reason": r, "state": {"state": "none"}, "hasModel": false, "themes": []}),
        };
        let key = self.entry(&root, &module, Some(&name));
        self.load_disk(&key);
        let e = self.get(&key).expect("entry");
        let mut out = Map::new();
        out.insert("target".into(), json!({"root": e.root, "module": e.module, "name": e.name}));
        if let Some(l) = library {
            out.insert("library".into(), json!({"module": l.module, "candidates": l.candidates, "linked": l.linked}));
        }
        out.insert("state".into(), e.state.clone());
        out.insert("hasModel".into(), json!(e.model.is_some() && !e.broken));
        let themes = match &e.model {
            Some(m) => themes_of(m),
            None => e.source_themes.iter().take(MAX_THEMES).cloned().collect(),
        };
        out.insert("themes".into(), json!(themes));
        out.insert("codeChanged".into(), json!(e.model.is_some() && e.code_changed_at > 0));
        Value::Object(out)
    }

    pub fn pick_app(&self, file: &str, module: &str) -> Value {
        if let Resolved::Target { root, library: Some(l), .. } = self.resolve(file) {
            if l.candidates.iter().any(|c| c == module) {
                if let Err(err) = self.prefer(&root, module) {
                    self.log("warn", &format!("preferred app: {err}"));
                }
            }
        }
        self.info(file)
    }

    pub fn prepare(&self, file: &str) -> Value {
        let key = self.entry_for(file);
        self.lock().wanted = key.clone();
        self.flush_res(false);
        {
            let _g = self.serial.lock().unwrap();
            if let Err(err) = self.ensure_loaded(key.as_deref(), Some(file)) {
                self.log("error", &format!("project load: {}", msg(&err)));
            }
        }
        let st = self.lock();
        let Some(loaded) = st.loaded_key.clone() else { return json!({"key": "framework", "frameworkOnly": true, "builtinLibs": false, "sourceOnly": false}) };
        let kit = loaded == KIT_KEY;
        let source = loaded.starts_with(SOURCE_PREFIX);
        let s = if source { st.loaded_source.clone() } else { None };
        let mut out = json!({
            "key": format!("{loaded}#{}", st.version),
            "frameworkOnly": kit,
            "builtinLibs": kit || s.as_ref().is_some_and(|s| s.kit),
            "sourceOnly": source && s.as_ref().and_then(|s| s.prebuilt_at).is_none(),
        });
        if let Some(at) = s.and_then(|s| s.prebuilt_at) {
            out["prebuiltAt"] = num(at);
        }
        out
    }
}
