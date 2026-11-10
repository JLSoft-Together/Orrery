use super::rules::{is_build_rel, CleanRequest};
use super::scan::{self, find_build_dirs, git_ignored_only, is_git_work_tree, refused_root};
use crate::fs_tree::{remove_tree, tree_size, Cancelled, Signal};
use crate::git::GitResult;
use crate::util::{self, fmt_bytes, is_inside_or_same};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const EVENT: &str = "ccm:build-event";
const LINES_MAX: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Dir {
    pub rel: String,
    pub name: String,
    pub kind: &'static str,
    pub bytes: f64,
    pub files: f64,
    pub newest_ms: Option<f64>,
    pub reinstall: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub root: String,
    pub git: bool,
    pub dirs: Vec<Dir>,
    pub bytes: f64,
    pub error: Option<&'static str>,
}

impl Project {
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("root".into(), json!(self.root));
        m.insert("git".into(), json!(self.git));
        let dirs: Vec<Value> = self
            .dirs
            .iter()
            .map(|d| json!({"rel": d.rel, "name": d.name, "kind": d.kind, "bytes": d.bytes, "files": d.files, "newestMs": d.newest_ms, "reinstall": d.reinstall}))
            .collect();
        m.insert("dirs".into(), Value::Array(dirs));
        m.insert("bytes".into(), json!(self.bytes));
        if let Some(e) = self.error {
            m.insert("error".into(), json!(e));
        }
        Value::Object(m)
    }
}

pub trait CleanHost {
    fn git(&self, cwd: &str, args: &[String]) -> GitResult;
    fn emit(&self, progress: Value);
    fn running(&self, root: &str) -> Vec<String>;
    fn daemons(&self) -> Vec<Value>;
    fn trash(&self, path: &str) -> bool;
    fn confirm(&self, spec: Value) -> i64;
}

#[derive(Default)]
pub struct BuildCleanService {
    busy: AtomicBool,
    abort: Mutex<Option<Arc<Signal>>>,
    last_emit: Mutex<f64>,
    scanned: Mutex<HashMap<String, Project>>,
}

fn key(root: &str) -> String {
    let r = util::path_str(&util::resolve(root));
    if cfg!(windows) {
        r.to_lowercase()
    } else {
        r
    }
}

struct Live {
    key: String,
    root: String,
    dir: Dir,
    abs: String,
}

impl BuildCleanService {
    pub fn cancel(&self) -> bool {
        match self.abort.lock().unwrap().as_ref() {
            Some(s) => {
                s.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    fn progress(&self, host: &dyn CleanHost, p: Value, force: bool) {
        let now = util::now_ms();
        let mut last = self.last_emit.lock().unwrap();
        if !force && now - *last < 200.0 {
            return;
        }
        *last = now;
        drop(last);
        host.emit(p);
    }

    fn run<T>(&self, busy_value: T, f: impl FnOnce(&Signal) -> T) -> T {
        if self.busy.swap(true, Ordering::SeqCst) {
            return busy_value;
        }
        let signal = Arc::new(AtomicBool::new(false));
        *self.abort.lock().unwrap() = Some(signal.clone());
        let out = f(&signal);
        *self.abort.lock().unwrap() = None;
        self.busy.store(false, Ordering::SeqCst);
        out
    }

    pub fn scan(&self, roots: &[String], host: &dyn CleanHost) -> Option<Vec<Project>> {
        self.run(None, |signal| self.scan_all(roots, signal, host).ok())
    }

    fn scan_all(&self, roots: &[String], signal: &Signal, host: &dyn CleanHost) -> Result<Vec<Project>, Cancelled> {
        let mut out = Vec::new();
        let mut dirs = 0.0;
        let mut bytes = 0.0;
        let home = scan::home_dir();
        let git = |cwd: &str, args: &[String]| host.git(cwd, args);
        for raw in roots {
            let root = util::path_str(&util::resolve(raw));
            let mut project = Project { root: root.clone(), git: false, dirs: Vec::new(), bytes: 0.0, error: None };
            let is_dir = Path::new(raw).is_absolute() && std::fs::metadata(&root).map(|m| m.is_dir()).unwrap_or(false);
            if !is_dir {
                project.error = Some("missing");
                out.push(project);
                continue;
            }
            if refused_root(&root, &home) {
                project.error = Some("refused");
                out.push(project);
                continue;
            }
            project.git = is_git_work_tree(&root, &git);
            let mut cands = find_build_dirs(Path::new(&root), project.git, Some(signal))?;
            if project.git && !cands.is_empty() {
                cands = git_ignored_only(&root, &cands, &git);
            }
            for c in cands {
                let mut base = Path::new(&root).to_path_buf();
                for part in c.rel.split('/') {
                    base.push(part);
                }
                let size = tree_size(&base, Some(signal), |n| {
                    bytes += n;
                    self.progress(host, json!({"op": "scan", "root": root, "dirs": dirs, "bytes": bytes}), false);
                })?;
                dirs += 1.0;
                let reinstall = scan::reinstall_of(&c);
                project.dirs.push(Dir { rel: c.rel, name: c.name, kind: c.kind, bytes: size.bytes, files: size.files, newest_ms: size.newest_ms, reinstall });
            }
            project.dirs.sort_by(|a, b| b.bytes.partial_cmp(&a.bytes).unwrap_or(std::cmp::Ordering::Equal));
            project.bytes = project.dirs.iter().map(|d| d.bytes).sum();
            self.scanned.lock().unwrap().insert(key(&root), project.clone());
            out.push(project);
        }
        self.progress(host, json!({"op": "scan", "dirs": dirs, "bytes": bytes}), true);
        Ok(out)
    }

    pub fn clean(&self, req: &CleanRequest, host: &dyn CleanHost) -> Value {
        self.run(json!({"ok": false, "error": "busy"}), |signal| {
            self.do_clean(req, signal, host).unwrap_or_else(|_| json!({"ok": false, "error": "cancelled"}))
        })
    }

    fn do_clean(&self, req: &CleanRequest, signal: &Signal, host: &dyn CleanHost) -> Result<Value, Cancelled> {
        let invalid = Ok(json!({"ok": false, "error": "invalid"}));
        let mut picked: Vec<Live> = Vec::new();
        {
            let scanned = self.scanned.lock().unwrap();
            for item in &req.items {
                let k = key(&item.root);
                let Some(project) = scanned.get(&k) else { return invalid };
                let Some(dir) = project.dirs.iter().find(|d| d.rel == item.rel) else { return invalid };
                if !is_build_rel(&item.rel) {
                    return invalid;
                }
                let mut abs = Path::new(&project.root).to_path_buf();
                for part in item.rel.split('/') {
                    abs.push(part);
                }
                let abs = util::path_str(&abs);
                if !is_inside_or_same(&project.root, &abs) || abs == project.root {
                    return invalid;
                }
                if !picked.iter().any(|p| p.abs == abs) {
                    picked.push(Live { key: k, root: project.root.clone(), dir: dir.clone(), abs });
                }
            }
        }
        let live: Vec<Live> = picked
            .into_iter()
            .filter(|p| std::fs::symlink_metadata(&p.abs).map(|m| m.is_dir() && !m.file_type().is_symlink()).unwrap_or(false))
            .collect();
        if live.is_empty() {
            return Ok(json!({"ok": false, "error": "nothing"}));
        }
        let mut roots: Vec<&str> = Vec::new();
        for p in &live {
            if !roots.contains(&p.root.as_str()) {
                roots.push(&p.root);
            }
        }
        let mut holders: Vec<String> = Vec::new();
        let mut hold = |h: String| {
            if !holders.contains(&h) {
                holders.push(h);
            }
        };
        for root in &roots {
            host.running(root).into_iter().for_each(&mut hold);
            for b in &req.busy {
                if is_inside_or_same(root, &b.cwd) {
                    hold(b.name.clone());
                }
            }
        }
        if !holders.is_empty() {
            holders.truncate(20);
            return Ok(json!({"ok": false, "error": "running", "holders": holders}));
        }
        if !req.ignore_daemons && live.iter().any(|p| p.dir.kind == "gradle") {
            let daemons = host.daemons();
            if !daemons.is_empty() {
                return Ok(json!({"ok": false, "error": "daemons", "daemons": daemons}));
            }
        }
        let total: f64 = live.iter().map(|p| p.dir.bytes).sum();
        let several = roots.len() > 1;
        let lines: Vec<String> = live
            .iter()
            .take(LINES_MAX)
            .map(|p| format!("• {}{} — {}", if several { format!("{}/", util::basename(&p.root)) } else { String::new() }, p.dir.rel, fmt_bytes(p.dir.bytes)))
            .collect();
        let spec = json!({
            "dirs": live.len(),
            "size": fmt_bytes(total),
            "more": live.len() - lines.len(),
            "lines": lines,
            "reinstall": live.iter().any(|p| p.dir.reinstall),
            "trash": req.trash,
        });
        if host.confirm(spec) != 0 {
            return Ok(json!({"ok": false, "error": "cancelled"}));
        }
        let (mut dirs, mut bytes, mut failed, mut done) = (0.0, 0.0, 0.0, 0.0);
        let mut removed: Vec<Value> = Vec::new();
        for p in &live {
            if signal.load(Ordering::SeqCst) {
                break;
            }
            if req.trash {
                if host.trash(&p.abs) {
                    bytes += p.dir.bytes;
                    done += p.dir.bytes;
                } else {
                    failed += 1.0;
                }
            } else {
                let r = remove_tree(Path::new(&p.abs), Some(signal), |n| {
                    done += n;
                    self.progress(host, json!({"op": "clean", "root": p.root, "dirs": dirs, "bytes": done, "totalBytes": total}), false);
                })?;
                bytes += r.bytes;
                failed += r.failed;
            }
            if !Path::new(&p.abs).exists() {
                dirs += 1.0;
                removed.push(json!({"root": p.root, "rel": p.dir.rel}));
                if let Some(project) = self.scanned.lock().unwrap().get_mut(&p.key) {
                    project.dirs.retain(|d| d.rel != p.dir.rel);
                    project.bytes = project.dirs.iter().map(|d| d.bytes).sum();
                }
            }
            self.progress(host, json!({"op": "clean", "root": p.root, "dirs": dirs, "bytes": done, "totalBytes": total}), true);
        }
        let mut m = Map::new();
        m.insert("ok".into(), json!(dirs > 0.0 || failed == 0.0));
        if dirs == 0.0 && failed > 0.0 {
            m.insert("error".into(), json!("failed"));
        }
        m.insert("dirs".into(), json!(dirs));
        m.insert("bytes".into(), json!(bytes));
        m.insert("failed".into(), json!(failed));
        m.insert("removed".into(), Value::Array(removed));
        Ok(Value::Object(m))
    }
}
