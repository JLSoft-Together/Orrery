use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::disk_guard::{self as guard, GuardContext, Plat, RealIo};
use crate::disk_host::DiskHost;
use crate::disk_pick::{restrict, Picked};
use crate::disk_snap::DiskSnap;
use crate::disk_text::{delete_box, Ask};
use crate::state::{lock, Shell};

pub const PROGRESS: &str = "ccm:diskmap-progress";
pub const DONE: &str = "ccm:diskmap-done";
pub const DUPES_PROGRESS: &str = "ccm:diskmap-dupes-progress";
pub const DUPES_DONE: &str = "ccm:diskmap-dupes-done";

const CHANNELS: [&str; 19] = [
    "ccm:diskmap-volumes",
    "ccm:diskmap-status",
    "ccm:diskmap-scan",
    "ccm:diskmap-pause",
    "ccm:diskmap-resume",
    "ccm:diskmap-cancel",
    "ccm:diskmap-children",
    "ccm:diskmap-search",
    "ccm:diskmap-delete",
    "ccm:diskmap-reveal",
    "ccm:diskmap-history",
    "ccm:diskmap-fastscan",
    "ccm:diskmap-sizes",
    "ccm:diskmap-dupes",
    "ccm:diskmap-dupes-cancel",
    "ccm:diskmap-snap-list",
    "ccm:diskmap-snap-compare",
    "ccm:diskmap-schedule-get",
    "ccm:diskmap-schedule-set",
];

const MAX_ROOTS: usize = 32;
const MAX_EXCLUDES: usize = 500;
const MAX_LIMIT: u64 = 10_000;
pub const MAX_DELETE: usize = 5_000;
const MAX_SIZES: usize = 2_000;
const MIN_DUPES_SIZE: f64 = 1024.0;
const MAX_DUPE_GROUPS: usize = 5_000;
const MAX_DUPE_FILES: usize = 60_000;
const HISTORY_FILE: &str = "history.jsonl";
const HISTORY_PATHS: usize = 50;
const HISTORY_MAX_BYTES: u64 = 1024 * 1024;
const HISTORY_KEEP: usize = 1000;
const LEVELS: [&str; 3] = ["light", "balanced", "max"];
const SORTS: [&str; 3] = ["size", "name", "mtime"];
const KINDS: [&str; 7] = ["image", "video", "audio", "doc", "archive", "installer", "code"];

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

fn obj(v: &Value) -> Option<&Map<String, Value>> {
    v.as_object()
}

fn nv(v: Option<&Value>) -> Value {
    v.filter(|n| n.is_number()).cloned().unwrap_or(json!(0))
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_owned()
}

fn abs(v: &Value) -> bool {
    v.as_str().is_some_and(|p| guard::is_absolute(p, Plat::host()))
}

fn unique(list: &[Value]) -> Vec<String> {
    let mut seen = HashSet::new();
    list.iter().filter_map(Value::as_str).filter(|s| seen.insert(s.to_string())).map(str::to_owned).collect()
}

fn excludes_ok(ex: &[Value]) -> bool {
    ex.len() <= MAX_EXCLUDES && ex.iter().all(|e| e.as_str().is_some_and(|s| !s.is_empty() && s.chars().count() <= 1024 && !s.contains('\0')))
}

pub fn clean_scan(v: &Value) -> Option<Value> {
    let o = obj(v)?;
    let roots = o.get("roots")?.as_array()?;
    if roots.is_empty() || roots.len() > MAX_ROOTS || !roots.iter().all(abs) {
        return None;
    }
    let level = o.get("level").and_then(Value::as_str).filter(|l| LEVELS.contains(l)).unwrap_or("balanced");
    let ex = match o.get("excludes") {
        None => Vec::new(),
        Some(Value::Array(a)) if excludes_ok(a) => a.clone(),
        Some(_) => return None,
    };
    Some(json!({ "roots": unique(roots), "level": level, "excludes": ex }))
}

pub fn clean_fast_scan(v: &Value) -> Option<String> {
    if Plat::host() != Plat::Win {
        return None;
    }
    let d = v.get("drive")?.as_str()?;
    let mut chars = d.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    let rest: String = chars.collect();
    matches!(rest.as_str(), "" | ":" | ":\\" | ":/").then(|| letter.to_ascii_uppercase().to_string())
}

fn limit_of(v: Option<&Value>, fallback: u64) -> u64 {
    match v {
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f >= 0.0 && f.fract() == 0.0) => (n.as_f64().unwrap_or(0.0) as u64).min(MAX_LIMIT),
        _ => fallback,
    }
}

fn sort_of(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| SORTS.contains(s))
}

pub fn clean_children(v: &Value) -> Option<Value> {
    let o = obj(v)?;
    let p = match o.get("path") {
        None => String::new(),
        Some(Value::String(s)) if s.is_empty() => String::new(),
        Some(p) if abs(p) => p.as_str().unwrap_or("").to_owned(),
        Some(_) => return None,
    };
    Some(json!({ "path": p, "sort": sort_of(o.get("sort")).unwrap_or("size"), "limit": limit_of(o.get("limit"), 500) }))
}

pub fn clean_search(v: &Value) -> Option<Value> {
    let empty = Map::new();
    let o = obj(v).unwrap_or(&empty);
    let mut q = Map::new();
    if let Some(query) = o.get("query") {
        let s = query.as_str().filter(|s| s.chars().count() <= 1024)?;
        q.insert("query".into(), json!(s));
    }
    for k in ["minSize", "maxSize", "after", "before"] {
        match o.get(k) {
            None | Some(Value::Null) => {}
            Some(n) => {
                let f = n.as_f64().filter(|f| f.is_finite() && *f >= 0.0)?;
                q.insert(k.into(), json!(f));
            }
        }
    }
    match o.get("kind") {
        None => {}
        Some(Value::String(s)) if s.is_empty() => {}
        Some(k) => {
            let s = k.as_str().filter(|s| KINDS.contains(s))?;
            q.insert("kind".into(), json!(s));
        }
    }
    if let Some(only) = o.get("only") {
        let s = only.as_str().filter(|s| *s == "file" || *s == "dir")?;
        q.insert("only".into(), json!(s));
    }
    match o.get("under") {
        None => {}
        Some(Value::String(s)) if s.is_empty() => {}
        Some(u) if abs(u) => {
            q.insert("under".into(), u.clone());
        }
        Some(_) => return None,
    }
    if let Some(sort) = o.get("sort") {
        q.insert("sort".into(), json!(sort_of(Some(sort))?));
    }
    q.insert("limit".into(), json!(limit_of(o.get("limit"), 100)));
    Some(Value::Object(q))
}

pub fn clean_sizes(v: &Value) -> Option<Vec<String>> {
    let paths = obj(v)?.get("paths")?.as_array()?;
    if paths.is_empty() || paths.len() > MAX_SIZES || !paths.iter().all(abs) {
        return None;
    }
    Some(unique(paths))
}

pub fn clean_dupes(v: &Value) -> Option<Map<String, Value>> {
    let empty = Map::new();
    let o = obj(v).unwrap_or(&empty);
    let mut out = Map::new();
    if let Some(under) = o.get("under") {
        let list = under.as_array()?;
        if list.len() > MAX_ROOTS || !list.iter().all(abs) {
            return None;
        }
        if !list.is_empty() {
            out.insert("under".into(), json!(unique(list)));
        }
    }
    if let Some(m) = o.get("minSize") {
        let f = m.as_f64().filter(|f| f.is_finite() && *f >= 0.0)?;
        out.insert("minSize".into(), json!(f.floor().max(MIN_DUPES_SIZE) as u64));
    }
    if let Some(ex) = o.get("excludes") {
        let a = ex.as_array().filter(|a| excludes_ok(a))?;
        out.insert("excludes".into(), Value::Array(a.clone()));
    }
    if let Some(l) = o.get("level").and_then(Value::as_str).filter(|l| LEVELS.contains(l)) {
        out.insert("level".into(), json!(l));
    }
    Some(out)
}

pub fn history_limit(v: &Value) -> usize {
    v.get("limit").and_then(Value::as_u64).filter(|n| *n > 0).map_or(50, |n| n.min(1000) as usize)
}

pub fn to_entry(v: &Value) -> Option<Value> {
    let o = obj(v)?;
    let path = o.get("path")?.as_str()?;
    let dir = o.get("dir") == Some(&Value::Bool(true));
    let mut e = json!({ "path": path, "name": o.get("name").and_then(Value::as_str).unwrap_or(path), "size": nv(o.get("size")), "dir": dir, "mtime": nv(o.get("mtime")) });
    if o.get("cloud") == Some(&Value::Bool(true)) {
        e["cloud"] = json!(true);
    }
    if let Some(kind) = o.get("kind").and_then(Value::as_str).filter(|k| !dir && *k != "other") {
        e["kind"] = json!(kind);
    }
    Some(e)
}

fn entries(v: Option<&Value>) -> Vec<Value> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(to_entry).collect()).unwrap_or_default()
}

pub fn to_children(v: &Value) -> Option<Value> {
    let o = obj(v)?;
    Some(json!({ "path": text(o.get("path")), "size": nv(o.get("size")), "total": nv(o.get("total")), "items": entries(o.get("items")) }))
}

pub fn to_search(v: &Value) -> Value {
    json!({ "total": nv(v.get("total")), "ms": nv(v.get("ms")), "items": entries(v.get("items")) })
}

pub fn to_status(v: &Value) -> Value {
    let ix = v.get("index").filter(|i| i.is_object());
    let roots: Vec<Value> = ix.and_then(|i| i.get("roots")).and_then(Value::as_array).map(|a| a.iter().filter(|r| r.is_string()).cloned().collect()).unwrap_or_default();
    let mut s = json!({ "scanning": v.get("scanning") == Some(&Value::Bool(true)), "paused": v.get("paused") == Some(&Value::Bool(true)), "indexed": ix.is_some(), "roots": roots });
    if let Some(i) = ix {
        if let Some(at) = i.get("scannedAt").filter(|n| n.as_f64().is_some_and(|f| f > 0.0)) {
            s["scannedAt"] = at.clone();
        }
        if let Some(n) = i.get("entries").filter(|n| n.is_number()) {
            s["entries"] = n.clone();
        }
    }
    s
}

pub fn status_roots(s: &Value) -> Vec<String> {
    s["roots"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default()
}

pub fn to_progress(v: &Value) -> Value {
    json!({ "files": nv(v.get("files")), "bytes": nv(v.get("bytes")), "current": text(v.get("current")), "elapsedMs": nv(v.get("elapsedMs")) })
}

fn clip(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn to_done(v: &Value) -> Value {
    let mut d = json!({ "ok": v.get("ok") == Some(&Value::Bool(true)) });
    if v.get("cancelled") == Some(&Value::Bool(true)) {
        d["cancelled"] = json!(true);
    }
    if let Some(e) = v.get("error").and_then(Value::as_str) {
        d["error"] = json!(clip(e, 300));
    }
    for k in ["entries", "bytes"] {
        if let Some(n) = v.get(k).filter(|n| n.is_number()) {
            d[k] = n.clone();
        }
    }
    if let Some(r) = v.get("roots").and_then(Value::as_array) {
        d["roots"] = Value::Array(r.iter().filter(|x| x.is_string()).cloned().collect());
    }
    if v.get("mft") == Some(&Value::Bool(true)) {
        d["mft"] = json!(true);
    }
    d
}

pub fn to_volumes(v: &Value) -> Value {
    let list = v.as_array().map(|a| {
        a.iter()
            .filter_map(|x| {
                let o = obj(x)?;
                let mount = o.get("mount")?.as_str()?;
                Some(json!({ "mount": mount, "label": o.get("label").and_then(Value::as_str).unwrap_or(mount), "fs": text(o.get("fs")), "kind": o.get("kind").and_then(Value::as_str).unwrap_or("unknown"), "network": o.get("network") == Some(&Value::Bool(true)), "total": nv(o.get("total")), "free": nv(o.get("free")) }))
            })
            .collect::<Vec<_>>()
    });
    Value::Array(list.unwrap_or_default())
}

pub fn to_sizes(v: &Value) -> Option<Value> {
    let version = v.get("version")?.as_str()?;
    let sizes = v.get("sizes")?.as_object()?;
    let out: Map<String, Value> = sizes.iter().map(|(k, n)| (k.clone(), n.as_f64().filter(|f| f.is_finite() && *f >= 0.0).map_or(Value::Null, |f| json!(f)))).collect();
    Some(json!({ "version": version, "sizes": out }))
}

const DUPE_STAGES: [&str; 4] = ["idle", "size", "quick", "full"];

pub fn to_dupes_progress(v: &Value) -> Value {
    let stage = v.get("stage").and_then(Value::as_str).filter(|s| DUPE_STAGES.contains(s)).unwrap_or("idle");
    let mut p = json!({ "stage": stage, "current": text(v.get("current")), "paused": v.get("paused") == Some(&Value::Bool(true)) });
    for k in ["candidates", "files", "filesDone", "bytes", "bytesRead", "groups", "skipped", "elapsedMs"] {
        p[k] = nv(v.get(k));
    }
    p
}

pub fn to_dupes_done(v: &Value) -> Value {
    let sk = v.get("skipped");
    let skipped = json!({ "stale": nv(sk.and_then(|s| s.get("stale"))), "cloud": nv(sk.and_then(|s| s.get("cloud"))), "locked": nv(sk.and_then(|s| s.get("locked"))), "changed": nv(sk.and_then(|s| s.get("changed"))), "gone": nv(sk.and_then(|s| s.get("gone"))) });
    let mut groups = Vec::new();
    let mut budget = MAX_DUPE_FILES;
    for g in v.get("groups").and_then(Value::as_array).into_iter().flatten() {
        if groups.len() >= MAX_DUPE_GROUPS || budget < 2 {
            break;
        }
        let Some(files) = g.get("files").and_then(Value::as_array) else { continue };
        let mut list: Vec<Value> = files.iter().filter_map(|f| f.get("path").and_then(Value::as_str).map(|p| json!({ "path": p, "mtime": nv(f.get("mtime")) }))).collect();
        if list.len() < 2 {
            continue;
        }
        let count = (list.len() as f64).max(g.get("count").and_then(Value::as_f64).filter(|n| n.is_finite()).unwrap_or(list.len() as f64));
        list.truncate(budget);
        budget -= list.len();
        groups.push(json!({ "size": nv(g.get("size")), "hash": text(g.get("hash")), "count": count, "files": list }));
    }
    let mut d = json!({ "ok": v.get("ok") == Some(&Value::Bool(true)), "groups": groups, "skipped": skipped });
    for k in ["totalGroups", "wasted", "candidates", "hashedFiles", "hashedBytes", "ms"] {
        d[k] = nv(v.get(k));
    }
    if v.get("cancelled") == Some(&Value::Bool(true)) {
        d["cancelled"] = json!(true);
    }
    if let Some(e) = v.get("error").and_then(Value::as_str) {
        d["error"] = json!(clip(e, 300));
    }
    d
}

pub fn clean_history_entry(v: &Value) -> Option<Value> {
    let n = |k: &str| v.get(k).and_then(Value::as_f64).filter(|f| f.is_finite() && *f >= 0.0);
    let (at, count, freed, failed) = (n("at")?, n("count")?, n("freed")?, n("failed")?);
    let action = v.get("action")?.as_str().filter(|a| matches!(*a, "trash" | "delete" | "move"))?;
    let paths: Vec<Value> = v.get("paths").and_then(Value::as_array).map(|a| a.iter().filter(|p| p.is_string()).take(HISTORY_PATHS).cloned().collect()).unwrap_or_default();
    Some(json!({ "at": at, "action": action, "count": count, "freed": freed, "paths": paths, "failed": failed }))
}

pub struct History {
    file: PathBuf,
    gate: Mutex<()>,
}

impl History {
    pub fn new(dir: &Path) -> Self {
        History { file: dir.join(HISTORY_FILE), gate: Mutex::new(()) }
    }

    pub fn append(&self, entry: &Value) {
        let Some(clean) = clean_history_entry(entry) else { return };
        let _g = lock(&self.gate);
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.file) else { return };
        let _ = writeln!(f, "{clean}");
        drop(f);
        if std::fs::metadata(&self.file).is_ok_and(|m| m.len() > HISTORY_MAX_BYTES) {
            let Ok(body) = std::fs::read_to_string(&self.file) else { return };
            let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
            let keep = &lines[lines.len().saturating_sub(HISTORY_KEEP)..];
            let tmp = self.file.with_extension("jsonl.tmp");
            if std::fs::write(&tmp, format!("{}\n", keep.join("\n"))).is_ok() {
                let _ = std::fs::rename(&tmp, &self.file);
            }
        }
    }

    pub fn read(&self, limit: usize) -> Vec<Value> {
        let _g = lock(&self.gate);
        let Ok(body) = std::fs::read_to_string(&self.file) else { return Vec::new() };
        body.lines()
            .rev()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
            .filter_map(|v| clean_history_entry(&v))
            .take(limit)
            .collect()
    }
}

pub struct DiskMap {
    pub host: DiskHost,
    pub history: History,
    data_dir: PathBuf,
    picked: Picked,
    loaded: Mutex<bool>,
    deleting: AtomicBool,
    pub dupes_running: AtomicBool,
    pub snap: DiskSnap,
}

fn e2e_auto_confirm() -> bool {
    std::env::var("CCM_E2E_DISKMAP_CONFIRM").is_ok_and(|v| v == "1") && (cfg!(debug_assertions) || crate::config::e2e_isolated())
}

fn idle_status() -> Value {
    json!({ "scanning": false, "paused": false, "indexed": false, "roots": [] })
}

fn empty_search() -> Value {
    json!({ "total": 0, "ms": 0, "items": [] })
}

fn cancelled_delete(permanent: bool) -> Value {
    json!({ "done": [], "failed": [], "freed": 0, "permanent": permanent, "cancelled": true })
}

impl DiskMap {
    fn ensure_loaded(&self) -> Result<(), String> {
        let mut loaded = lock(&self.loaded);
        if *loaded {
            return Ok(());
        }
        let st = to_status(&self.host.call("disk.status", json!({}))?);
        if st["indexed"] != json!(true) && st["scanning"] != json!(true) {
            let _ = self.host.call("disk.load", json!({}));
        }
        *loaded = true;
        Ok(())
    }

    pub fn on_exit(&self) {
        *lock(&self.loaded) = false;
    }

    pub fn volumes(&self) -> Value {
        to_volumes(&self.host.call("disk.volumes", json!({})).unwrap_or(json!([])))
    }

    pub fn scheduled_roots(&self, indexed: &[String]) -> Vec<String> {
        restrict(indexed, self.picked.load().as_deref(), Plat::host())
    }

    pub fn status(&self) -> Result<Value, String> {
        self.ensure_loaded()?;
        Ok(to_status(&self.host.call("disk.status", json!({}))?))
    }

    pub fn scan(&self, req: &Value) -> Value {
        let Some(clean) = clean_scan(req) else { return json!({ "started": false }) };
        self.snap.before_scan(self);
        if self.ensure_loaded().is_err() {
            return json!({ "started": false });
        }
        let picked: Vec<String> = clean["roots"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
        let mut args = clean;
        args["save"] = json!(true);
        match self.host.call("disk.scan", args) {
            Ok(_) => {
                self.picked.save(&picked);
                self.host.mark_scanning(true);
                json!({ "started": true })
            }
            Err(_) => json!({ "started": false }),
        }
    }

    fn fast_scan(&self, req: &Value) -> Value {
        let Some(drive) = clean_fast_scan(req) else { return json!({ "started": false, "error": "unsupported" }) };
        self.snap.before_scan(self);
        if self.ensure_loaded().is_err() {
            return json!({ "started": false, "error": "failed" });
        }
        match self.host.call("disk.mft", json!({ "drive": drive, "save": true })) {
            Ok(_) => {
                self.picked.save(&[format!("{}:\\", drive.chars().next().unwrap_or('C').to_ascii_uppercase())]);
                self.host.mark_scanning(true);
                json!({ "started": true })
            }
            Err(e) => json!({ "started": false, "error": if e == "unsupported" || e == "busy" { e.as_str() } else { "failed" } }),
        }
    }

    fn dupes(&self, req: &Value, home: &str) -> Value {
        let Some(clean) = clean_dupes(req) else { return json!({ "started": false, "error": "invalid" }) };
        if self.ensure_loaded().is_err() {
            return json!({ "started": false, "error": "failed" });
        }
        let base = json!({ "minSize": clean.get("minSize").cloned().unwrap_or(Value::Null), "excludes": clean.get("excludes").cloned().unwrap_or(json!([])), "level": clean.get("level").cloned().unwrap_or(Value::Null) });
        let attempt = |under: Value| {
            let mut a = base.clone();
            if a["minSize"].is_null() {
                a.as_object_mut().map(|m| m.remove("minSize"));
            }
            if a["level"].is_null() {
                a.as_object_mut().map(|m| m.remove("level"));
            }
            a["under"] = under;
            self.host.call("disk.dupes", a).map(|_| ())
        };
        self.dupes_running.store(true, Ordering::SeqCst);
        self.host.mark_scanning(true);
        let r = match clean.get("under") {
            Some(under) => attempt(under.clone()),
            None => match attempt(json!([home])) {
                Err(e) if e.starts_with("not in index") => {
                    let roots = self.host.call("disk.status", json!({})).map(|s| status_roots(&to_status(&s))).unwrap_or_default();
                    if roots.is_empty() {
                        Err(e)
                    } else {
                        attempt(json!(roots))
                    }
                }
                other => other,
            },
        };
        match r {
            Ok(()) => json!({ "started": true }),
            Err(e) => {
                self.dupes_running.store(false, Ordering::SeqCst);
                if e != "busy" {
                    self.host.mark_scanning(false);
                }
                let error = if e.starts_with("not in index") {
                    "not-indexed"
                } else if e == "busy" {
                    "busy"
                } else {
                    "failed"
                };
                json!({ "started": false, "error": error })
            }
        }
    }

    pub fn control(&self, op: &str) -> bool {
        if !self.host.running() {
            return false;
        }
        self.host.call(&format!("disk.{op}"), json!({})).is_ok_and(|v| v["ok"] == json!(true))
    }

    fn children(&self, req: &Value) -> Value {
        let Some(clean) = clean_children(req) else { return Value::Null };
        if self.ensure_loaded().is_err() {
            return Value::Null;
        }
        self.host.call("disk.children", clean).ok().and_then(|v| to_children(&v)).unwrap_or(Value::Null)
    }

    fn sizes(&self, req: &Value) -> Value {
        let Some(paths) = clean_sizes(req) else { return Value::Null };
        if !self.host.running() && !self.data_dir.join("disk").join("index.bin").exists() {
            return Value::Null;
        }
        if self.ensure_loaded().is_err() {
            return Value::Null;
        }
        self.host.call("disk.sizes", json!({ "paths": paths })).ok().and_then(|v| to_sizes(&v)).unwrap_or(Value::Null)
    }

    fn search(&self, req: &Value) -> Value {
        let Some(q) = clean_search(req) else { return empty_search() };
        if self.ensure_loaded().is_err() {
            return empty_search();
        }
        self.host.call("disk.search", q).map(|v| to_search(&v)).unwrap_or_else(|_| empty_search())
    }

    pub fn size_of(&self, p: &str) -> Option<u64> {
        let v = self.host.call("disk.children", json!({ "path": p, "limit": 0 })).ok()?;
        to_children(&v).and_then(|c| c["size"].as_f64()).map(|f| f as u64)
    }

    fn known(&self, p: &str) -> bool {
        self.host.call("disk.children", json!({ "path": p, "limit": 0 })).ok().and_then(|v| to_children(&v)).is_some()
    }

    fn remove(&self, app: &AppHandle, req: &Value) -> Value {
        let permanent = req.get("permanent") == Some(&Value::Bool(true));
        let paths: Vec<Value> = req.get("paths").and_then(Value::as_array).cloned().unwrap_or_default();
        self.remove_with(app, &paths, permanent, &|ask, second| confirm_delete(app, ask, second), 3)
    }

    pub fn is_deleting(&self) -> bool {
        self.deleting.load(Ordering::SeqCst)
    }

    pub fn remove_with(&self, app: &AppHandle, paths: &[Value], permanent: bool, confirm: &dyn Fn(&Ask, bool) -> bool, first_count: usize) -> Value {
        if paths.is_empty() || paths.len() > MAX_DELETE {
            return cancelled_delete(permanent);
        }
        if self.deleting.swap(true, Ordering::SeqCst) {
            let failed: Vec<Value> = paths.iter().map(|p| json!({ "path": p.as_str().unwrap_or(""), "reason": "busy" })).collect();
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
        }
        let out = if self.ensure_loaded().is_ok() { self.run_delete(app, paths, permanent, confirm, first_count) } else { cancelled_delete(permanent) };
        self.deleting.store(false, Ordering::SeqCst);
        out
    }

    fn guard_context(&self, app: &AppHandle) -> GuardContext {
        let roots = self.host.call("disk.status", json!({})).map(|s| status_roots(&to_status(&s))).unwrap_or_default();
        let volumes = self.volumes().as_array().map(|a| a.iter().filter_map(|v| v["mount"].as_str()).filter(|m| !m.is_empty()).map(str::to_owned).collect()).unwrap_or_default();
        GuardContext { home: home_of(app), keep: keep_of(app), roots, volumes }
    }

    fn run_delete(&self, app: &AppHandle, paths: &[Value], permanent: bool, confirm: &dyn Fn(&Ask, bool) -> bool, first_count: usize) -> Value {
        let plat = Plat::host();
        let ctx = self.guard_context(app);
        let io = RealIo;
        let mut failed: Vec<Value> = Vec::new();
        let mut done: Vec<Value> = Vec::new();
        let mut freed: u64 = 0;
        let mut allowed: Vec<String> = Vec::new();
        let mut canon: Vec<(String, guard::Canonical)> = Vec::new();
        let key = |p: &str| guard::normalize(p, plat).to_lowercase();
        for raw in paths {
            let p = raw.as_str().unwrap_or("");
            match guard::resolve_deletable(p, &ctx, plat, &io) {
                Err(reason) => failed.push(json!({ "path": p, "reason": reason })),
                Ok(c) => {
                    if !self.known(p) {
                        failed.push(json!({ "path": p, "reason": "outside" }));
                        continue;
                    }
                    allowed.push(p.to_owned());
                    canon.push((key(p), c));
                }
            }
        }
        let targets = guard::top_level(&allowed, plat);
        if targets.is_empty() {
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
        }
        let sizes: Vec<Option<u64>> = targets.iter().map(|p| self.size_of(p)).collect();
        let first: Vec<String> = targets.iter().take(first_count).cloned().collect();
        let ask = Ask { count: targets.len(), bytes: sizes.iter().map(|s| s.unwrap_or(0)).sum(), first: &first, permanent };
        if !confirm(&ask, false) || (permanent && !confirm(&ask, true)) {
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent, "cancelled": true });
        }
        let mut gone: Vec<String> = Vec::new();
        let mut partial: Vec<String> = Vec::new();
        for (i, p) in targets.iter().enumerate() {
            let before = canon.iter().find(|(k, _)| *k == key(p)).map(|(_, c)| c.clone());
            let again = guard::resolve_deletable(p, &ctx, plat, &io);
            let target = match (&before, &again) {
                (Some(b), Ok(a)) if guard::same_canonical(b, a, plat) => a.path.clone(),
                (_, Err(reason)) => {
                    failed.push(json!({ "path": p, "reason": reason }));
                    continue;
                }
                _ => {
                    failed.push(json!({ "path": p, "reason": "changed" }));
                    continue;
                }
            };
            let size = sizes[i].unwrap_or(0);
            let r = if permanent { remove_path(&target) } else { trash_path(&target) };
            match r {
                Ok(()) => {
                    done.push(json!({ "path": p, "size": size }));
                    freed += size;
                    gone.push(p.clone());
                }
                Err(reason) => {
                    failed.push(json!({ "path": p, "reason": reason }));
                    if reason == "ENOENT" {
                        gone.push(p.clone());
                    } else if permanent && reason != "EACCES" && reason != "EPERM" {
                        gone.push(p.clone());
                        if let Some(rest) = measure_rest(&target) {
                            if size > rest {
                                freed += size - rest;
                                partial.push(p.clone());
                            }
                        }
                    }
                }
            }
        }
        if !gone.is_empty() {
            let _ = self.host.call("disk.forget", json!({ "paths": gone }));
            let host = self.host.clone();
            std::thread::spawn(move || {
                let _ = host.call("disk.save", json!({}));
            });
        }
        if !done.is_empty() || !failed.is_empty() {
            let mut list: Vec<Value> = done.iter().map(|d| d["path"].clone()).collect();
            list.extend(partial.iter().map(|p| json!(p)));
            self.history.append(&json!({ "at": now_ms(), "action": if permanent { "delete" } else { "trash" }, "count": done.len(), "freed": freed, "paths": list, "failed": failed.len() }));
        }
        json!({ "done": done, "failed": failed, "freed": freed, "permanent": permanent })
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn confirm_delete(app: &AppHandle, ask: &Ask, second: bool) -> bool {
    if e2e_auto_confirm() {
        return true;
    }
    let b = delete_box(ask, second, &crate::hosts::locale(app), cfg!(target_os = "macos"));
    crate::hosts::confirm(app, &b.message, &b.detail, &b.ok, &b.cancel)
}

fn io_reason(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT".into(),
        std::io::ErrorKind::PermissionDenied => "EPERM".into(),
        _ => clip(&e.to_string().split_whitespace().collect::<Vec<_>>().join(" "), 200),
    }
}

pub fn trash_path(p: &str) -> Result<(), String> {
    if std::fs::symlink_metadata(p).is_err() {
        return Err("ENOENT".into());
    }
    trash::delete(p).map_err(|e| {
        let msg = e.to_string();
        if msg.is_empty() {
            "failed".into()
        } else {
            clip(&msg.split_whitespace().collect::<Vec<_>>().join(" "), 200)
        }
    })
}

fn remove_once(p: &str) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(p)?;
    if meta.file_type().is_symlink() {
        return std::fs::remove_file(p).or_else(|_| std::fs::remove_dir(p));
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}

pub fn remove_path(p: &str) -> Result<(), String> {
    let mut last = None;
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        match remove_once(p) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && attempt == 0 => return Err("ENOENT".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => last = Some(e),
        }
    }
    Err(last.map_or("failed".into(), |e| io_reason(&e)))
}

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else { return 0 };
    rd.flatten()
        .map(|e| match e.path().symlink_metadata() {
            Ok(m) if m.file_type().is_symlink() => 0,
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn measure_rest(p: &str) -> Option<u64> {
    match std::fs::symlink_metadata(p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(0),
        Err(_) => None,
        Ok(m) if m.file_type().is_symlink() => Some(0),
        Ok(m) if m.is_dir() => Some(dir_size(Path::new(p))),
        Ok(m) => Some(m.len()),
    }
}

pub fn home_of(app: &AppHandle) -> String {
    app.path().home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn keep_of(app: &AppHandle) -> Vec<String> {
    let p = app.path();
    let shell = app.state::<Shell>();
    let mut out: Vec<PathBuf> = [p.document_dir(), p.desktop_dir(), p.download_dir(), p.picture_dir(), p.audio_dir(), p.video_dir(), p.app_data_dir(), p.app_local_data_dir()].into_iter().flatten().collect();
    if let Ok(cfg) = p.config_dir() {
        out.push(cfg.join("Agent Monitor"));
    }
    out.push(shell.data_dir.clone());
    out.push(shell.webview_dir.clone());
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        out.push(dir);
    }
    out.into_iter().map(|d| d.to_string_lossy().into_owned()).filter(|s| !s.is_empty()).collect()
}

pub fn manage(app: &AppHandle, core_bin: PathBuf) {
    let data_dir = app.state::<Shell>().data_dir.clone();
    let host = DiskHost::new(core_bin, data_dir.clone());
    let disk_dir = data_dir.join("disk");
    let map = Arc::new(DiskMap {
        host: host.clone(),
        history: History::new(&disk_dir),
        picked: Picked::new(&disk_dir),
        data_dir,
        loaded: Mutex::new(false),
        deleting: AtomicBool::new(false),
        dupes_running: AtomicBool::new(false),
        snap: DiskSnap::new(&disk_dir),
    });
    let ev_app = app.clone();
    let ev_map = Arc::downgrade(&map);
    let exit_app = app.clone();
    let exit_map = Arc::downgrade(&map);
    host.set_handlers(
        Arc::new(move |event, payload| {
            match event {
                "disk:progress" => crate::desk::emit_app(&ev_app, PROGRESS, to_progress(&payload)),
                "disk:done" => {
                    let done = to_done(&payload);
                    crate::desk::emit_app(&ev_app, DONE, done.clone());
                    if let Some(m) = ev_map.upgrade() {
                        let app = ev_app.clone();
                        std::thread::spawn(move || m.snap.on_done(&app, &m, &done));
                    }
                }
                "disk:dupes-progress" => crate::desk::emit_app(&ev_app, DUPES_PROGRESS, to_dupes_progress(&payload)),
                "disk:dupes-done" => {
                    if let Some(m) = ev_map.upgrade() {
                        m.dupes_running.store(false, Ordering::SeqCst);
                    }
                    crate::desk::emit_app(&ev_app, DUPES_DONE, to_dupes_done(&payload));
                }
                _ => {}
            }
        }),
        Arc::new(move |busy, reason| {
            let Some(m) = exit_map.upgrade() else { return };
            m.on_exit();
            if m.dupes_running.swap(false, Ordering::SeqCst) {
                if busy {
                    crate::desk::emit_app(&exit_app, DUPES_DONE, to_dupes_done(&json!({ "ok": false, "error": reason })));
                }
            } else if busy {
                let done = to_done(&json!({ "ok": false, "error": reason }));
                crate::desk::emit_app(&exit_app, DONE, done.clone());
                let app = exit_app.clone();
                std::thread::spawn(move || m.snap.on_done(&app, &m, &done));
            }
        }),
    );
    app.manage(DiskState(map.clone()));
    crate::disk_snap::start(app, Arc::downgrade(&map));
}

pub struct DiskState(pub Arc<DiskMap>);

pub fn stop(app: &AppHandle) {
    if let Some(s) = app.try_state::<DiskState>() {
        s.0.snap.stop();
        s.0.host.stop();
    }
}

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    let Some(state) = app.try_state::<DiskState>() else { return Err(format!("not ported: {channel}")) };
    let m = state.0.clone();
    let a = arg(args, 0);
    Ok(match channel {
        "ccm:diskmap-volumes" => m.volumes(),
        "ccm:diskmap-status" => m.status().unwrap_or_else(|_| idle_status()),
        "ccm:diskmap-scan" => m.scan(&a),
        "ccm:diskmap-pause" => json!(m.control("pause")),
        "ccm:diskmap-resume" => json!(m.control("resume")),
        "ccm:diskmap-cancel" | "ccm:diskmap-dupes-cancel" => json!(m.control("cancel")),
        "ccm:diskmap-children" => m.children(&a),
        "ccm:diskmap-search" => m.search(&a),
        "ccm:diskmap-delete" => m.remove(app, &a),
        "ccm:diskmap-reveal" => {
            if let Some(p) = a.as_str().filter(|p| guard::is_absolute(p, Plat::host())) {
                let _ = app.opener().reveal_item_in_dir(p);
            }
            Value::Null
        }
        "ccm:diskmap-history" => json!(m.history.read(history_limit(&a))),
        "ccm:diskmap-fastscan" => m.fast_scan(&a),
        "ccm:diskmap-sizes" => m.sizes(&a),
        "ccm:diskmap-dupes" => m.dupes(&a, &home_of(app)),
        "ccm:diskmap-snap-list" => json!(m.snap.list()),
        "ccm:diskmap-snap-compare" => m.snap.compare(&m, &a),
        "ccm:diskmap-schedule-get" => m.snap.schedule_state(),
        "ccm:diskmap-schedule-set" => m.snap.schedule_set(app, &m, &a),
        _ => return Err(format!("not ported: {channel}")),
    })
}

#[cfg(test)]
#[path = "disk_map_tests.rs"]
mod tests;
