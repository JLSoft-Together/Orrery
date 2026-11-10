use super::moves::{recover_moves, MoveStore};
use super::procs::AgentProc;
use super::rules::{classify_entry, Agent, DAY};
use super::strip::{count_file, running_sessions, strip_candidates, strip_file, StripCandidate};
use super::tree::{aborted, link_target, Halt};
use crate::fs_tree::{read_names, tree_size, Cancelled, Signal};
use crate::util::{fmt_bytes, is_inside_or_same, iso_string, mtime_ms, now_ms, path_str};
use crate::js;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub const EVENT: &str = "ccm:agentdata-event";

#[derive(Debug, Clone, PartialEq)]
pub struct Roots {
    pub claude: String,
    pub codex: String,
    pub claude_app: Option<String>,
}

impl Roots {
    pub fn of(&self, agent: Agent) -> &str {
        match agent {
            Agent::Claude => &self.claude,
            Agent::Codex => &self.codex,
        }
    }
}

pub trait AgentHost {
    fn roots(&self) -> Roots;
    fn procs(&self, agent: Agent) -> Vec<AgentProc>;
    fn confirm(&self, spec: Value) -> Option<i64>;
    fn pick_folder(&self, purpose: &str) -> Option<String>;
    fn stop_collector(&self);
    fn start_collector(&self);
    fn emit(&self, progress: Value);
    fn is_local(&self, dir: &str) -> bool;
    fn free_bytes(&self, dir: &str) -> Option<f64>;
    fn alive(&self, pid: f64) -> bool;
    fn now(&self) -> f64;
}

pub type Out = Result<Value, String>;

pub fn fail(error: &str) -> Value {
    json!({"ok": false, "error": error})
}

pub fn fail_with(error: &str, extra: &[(&str, Value)]) -> Value {
    let mut m = Map::new();
    m.insert("ok".into(), json!(false));
    m.insert("error".into(), json!(error));
    for (k, v) in extra {
        m.insert((*k).into(), v.clone());
    }
    Value::Object(m)
}

pub fn procs_value(procs: &[AgentProc]) -> Value {
    Value::Array(procs.iter().map(AgentProc::to_value).collect())
}

pub fn io_err(e: std::io::Error) -> String {
    format!("agent data store: {e}")
}

pub fn join(parent: &str, name: &str) -> PathBuf {
    let p = Path::new(parent).join(name);
    if p.is_absolute() {
        crate::util::resolve(&path_str(&p))
    } else {
        p
    }
}

pub fn dirname(p: &str) -> PathBuf {
    Path::new(p).parent().map(Path::to_path_buf).unwrap_or_default()
}

pub fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

struct Group {
    id: &'static str,
    kind: &'static str,
    cleanable: bool,
    idle: bool,
    files: f64,
    bytes: f64,
    entries: Vec<(f64, Value)>,
}

impl Group {
    fn into_value(mut self) -> Value {
        self.entries.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
        let entries: Vec<Value> = self.entries.into_iter().map(|e| e.1).collect();
        json!({"id": self.id, "kind": self.kind, "cleanable": self.cleanable, "idleOnly": self.idle, "files": self.files, "bytes": self.bytes, "entries": entries})
    }
}

struct BusyGuard<'a>(&'a AgentDataService);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        *self.0.abort.lock().unwrap() = None;
        *self.0.busy.lock().unwrap() = None;
    }
}

pub struct AgentDataService {
    store: Mutex<MoveStore>,
    loaded: AtomicBool,
    busy: Mutex<Option<&'static str>>,
    abort: Mutex<Option<Arc<Signal>>>,
    pub(super) recovered: Mutex<Vec<Agent>>,
    last_emit: Mutex<f64>,
}

impl AgentDataService {
    pub fn new(file: PathBuf) -> Self {
        AgentDataService {
            store: Mutex::new(MoveStore::new(file)),
            loaded: AtomicBool::new(false),
            busy: Mutex::new(None),
            abort: Mutex::new(None),
            recovered: Mutex::new(Vec::new()),
            last_emit: Mutex::new(0.0),
        }
    }

    pub(super) fn store(&self) -> MutexGuard<'_, MoveStore> {
        let mut s = self.store.lock().unwrap();
        if !self.loaded.swap(true, Ordering::SeqCst) {
            s.load();
            let rolled = recover_moves(&mut s);
            *self.recovered.lock().unwrap() = rolled;
        }
        s
    }

    pub fn recover(&self) -> Vec<Agent> {
        drop(self.store());
        self.recovered.lock().unwrap().clone()
    }

    pub fn cancel(&self) -> bool {
        match self.abort.lock().unwrap().as_ref() {
            Some(s) => {
                s.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub(super) fn progress(&self, host: &dyn AgentHost, p: Value, force: bool) {
        let now = now_ms();
        let mut last = self.last_emit.lock().unwrap();
        if !force && now - *last < 200.0 {
            return;
        }
        *last = now;
        drop(last);
        host.emit(p);
    }

    pub(super) fn run(&self, op: &'static str, busy_value: Value, f: impl FnOnce(&Signal) -> Out) -> Out {
        {
            let mut b = self.busy.lock().unwrap();
            if b.is_some() {
                return Ok(busy_value);
            }
            *b = Some(op);
        }
        let signal = Arc::new(AtomicBool::new(false));
        *self.abort.lock().unwrap() = Some(signal.clone());
        let _guard = BusyGuard(self);
        f(&signal)
    }

    pub fn state(&self, host: &dyn AgentHost) -> Value {
        let roots = host.roots();
        let store = self.store();
        let places: Vec<Value> = Agent::ALL
            .iter()
            .map(|a| {
                let root = roots.of(*a);
                json!({
                    "agent": a.as_str(),
                    "root": root,
                    "exists": exists(root),
                    "linkTarget": link_target(Path::new(root)),
                    "move": store.get(*a).map(|r| r.to_value()),
                })
            })
            .collect();
        drop(store);
        let busy = *self.busy.lock().unwrap();
        let recovered: Vec<&str> = self.recovered.lock().unwrap().iter().map(|a| a.as_str()).collect();
        json!({"places": places, "claudeApp": roots.claude_app, "busy": busy, "recovered": recovered})
    }

    pub fn scan(&self, host: &dyn AgentHost) -> Out {
        self.run("scan", Value::Null, |signal| Ok(self.scan_all(host, signal).map(Value::Array).unwrap_or(Value::Null)))
    }

    fn scan_all(&self, host: &dyn AgentHost, signal: &Signal) -> Result<Vec<Value>, Cancelled> {
        let roots = host.roots();
        let mut out = Vec::new();
        let (mut files, mut bytes) = (0.0, 0.0);
        let places: [(&str, Option<String>); 3] = [("claude", Some(roots.claude.clone())), ("codex", Some(roots.codex.clone())), ("claudeApp", roots.claude_app.clone())];
        for (label, root) in places {
            let Some(root) = root else { continue };
            let there = exists(&root);
            if !there {
                out.push(json!({"agent": label, "root": root, "exists": false, "files": 0, "bytes": 0, "groups": []}));
                continue;
            }
            let base = link_target(Path::new(&root)).unwrap_or_else(|| root.clone());
            let mut groups: Vec<Group> = Vec::new();
            for name in read_names(Path::new(&base)) {
                let (id, kind, cleanable, idle) = match Agent::parse(&json!(label)) {
                    None => ("desktopApp", "unknown", false, false),
                    Some(agent) => {
                        let r = classify_entry(agent, &name, cfg!(windows));
                        (r.id, r.kind, r.cleanable, r.idle_only)
                    }
                };
                let p = Path::new(&base).join(&name);
                let Ok(meta) = fs::symlink_metadata(&p) else { continue };
                let t = meta.file_type();
                let (mut ef, mut eb, mut en, mut link) = (0.0, 0.0, Value::Null, Value::Null);
                if t.is_symlink() {
                    link = json!(link_target(&p));
                } else if t.is_dir() {
                    let s = tree_size(&p, Some(signal), |n| {
                        files += 1.0;
                        bytes += n;
                        self.progress(host, json!({"op": "scan", "files": files, "bytes": bytes}), false);
                    })?;
                    (ef, eb, en) = (s.files, s.bytes, json!(s.newest_ms));
                } else if t.is_file() {
                    (ef, eb, en) = (1.0, meta.len() as f64, json!(mtime_ms(&meta)));
                    files += 1.0;
                    bytes += eb;
                    self.progress(host, json!({"op": "scan", "files": files, "bytes": bytes}), false);
                }
                let entry = json!({"name": name, "dir": t.is_dir(), "link": link, "files": ef, "bytes": eb, "newestMs": en});
                let idx = match groups.iter().position(|g| g.id == id) {
                    Some(i) => i,
                    None => {
                        groups.push(Group { id, kind, cleanable, idle, files: 0.0, bytes: 0.0, entries: Vec::new() });
                        groups.len() - 1
                    }
                };
                let g = &mut groups[idx];
                g.files += ef;
                g.bytes += eb;
                g.entries.push((eb, entry));
            }
            groups.sort_by(|a, b| b.bytes.partial_cmp(&a.bytes).unwrap_or(std::cmp::Ordering::Equal));
            let total_files: f64 = groups.iter().map(|g| g.files).sum();
            let total_bytes: f64 = groups.iter().map(|g| g.bytes).sum();
            let list: Vec<Value> = groups.into_iter().map(Group::into_value).collect();
            out.push(json!({"agent": label, "root": root, "exists": true, "files": total_files, "bytes": total_bytes, "groups": list}));
        }
        self.progress(host, json!({"op": "scan", "files": files, "bytes": bytes}), true);
        Ok(out)
    }

    pub fn strip(&self, min_days: f64, host: &dyn AgentHost) -> Out {
        self.run("strip", fail("busy"), |signal| Ok(self.do_strip(min_days, signal, host).unwrap_or_else(|_| fail("cancelled"))))
    }

    fn do_strip(&self, min_days: f64, signal: &Signal, host: &dyn AgentHost) -> Result<Value, Halt> {
        let root = host.roots().claude;
        let alive = |pid: f64| host.alive(pid);
        let running = running_sessions(Path::new(&root), &alive);
        let projects = Path::new(&root).join("projects");
        let now = host.now();
        let cands = strip_candidates(&projects, now - min_days * DAY, &running, Some(signal))?;
        let mut with_images: Vec<(StripCandidate, f64, f64)> = Vec::new();
        let mut scanned = 0.0;
        for c in cands {
            let n = match count_file(&c.file, Some(signal)) {
                Err(Halt::Cancelled) => return Err(Halt::Cancelled),
                Err(Halt::Failed) => Default::default(),
                Ok(n) => n,
            };
            scanned += c.size;
            self.progress(host, json!({"op": "strip", "agent": "claude", "files": with_images.len(), "bytes": scanned, "phase": "check"}), false);
            if n.bytes > 0.0 {
                with_images.push((c, n.images, n.bytes));
            }
        }
        if with_images.is_empty() {
            return Ok(fail("nothing"));
        }
        let Some(backup_root) = host.pick_folder("backup") else { return Ok(fail("cancelled")) };
        if is_inside_or_same(&root, &backup_root) || is_inside_or_same(&backup_root, &root) {
            return Ok(fail("target-inside"));
        }
        let need: f64 = with_images.iter().map(|c| c.0.size).sum();
        if host.free_bytes(&backup_root).is_some_and(|free| free < need + 64.0 * 1024.0 * 1024.0) {
            return Ok(fail("no-space"));
        }
        let images: f64 = with_images.iter().map(|c| c.1).sum();
        let chars: f64 = with_images.iter().map(|c| c.2).sum();
        let dest = join(&backup_root, &format!("orrery-transcripts-{}-{}", &iso_string(now)[..10], js::number(now)));
        let dest_text = path_str(&dest);
        let spec = json!({"kind": "strip", "files": with_images.len(), "images": images, "size": fmt_bytes(chars), "backup": dest_text});
        if host.confirm(spec).unwrap_or(1) != 0 {
            return Ok(fail("cancelled"));
        }
        let still = running_sessions(Path::new(&root), &alive);
        let (mut files, mut saved, mut skipped) = (0.0, 0.0, 0.0);
        for (c, _, _) in &with_images {
            if aborted(Some(signal)) {
                break;
            }
            if still.contains(&c.session_id) {
                skipped += 1.0;
                continue;
            }
            let r = match strip_file(&c.file, &dest.join(&c.rel), Some(signal)) {
                Err(Halt::Cancelled) => return Err(Halt::Cancelled),
                Err(Halt::Failed) => None,
                Ok(r) => Some(r),
            };
            match r {
                Some(r) if r.ok && r.count.bytes > 0.0 => {
                    files += 1.0;
                    saved += r.count.bytes;
                }
                Some(r) if r.ok => {}
                _ => skipped += 1.0,
            }
            self.progress(host, json!({"op": "strip", "agent": "claude", "files": files, "bytes": saved, "phase": "rewrite"}), false);
        }
        let mut m = Map::new();
        m.insert("ok".into(), json!(files > 0.0));
        if files == 0.0 {
            m.insert("error".into(), json!("failed"));
        }
        m.insert("files".into(), json!(files));
        m.insert("bytes".into(), json!(saved));
        m.insert("skipped".into(), json!(skipped));
        m.insert("path".into(), json!(dest_text));
        Ok(Value::Object(m))
    }
}
