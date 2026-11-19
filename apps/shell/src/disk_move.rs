use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::disk_guard::{self as guard, GuardContext, Plat, RealIo};
use crate::disk_move_text::{move_box, MoveAsk};
use crate::disk_move_tree as tree;
use crate::state::{lock, Shell};

pub const PROGRESS: &str = "ccm:diskmove-progress";
pub const MOVE_SUFFIX: &str = ".orrery-moving";
pub const MOVE_BACK_SUFFIX: &str = ".orrery-back";
pub const MOVE_DIR: &str = "OrreryMoved";

const CHANNELS: [&str; 10] = [
    "ccm:diskmove-list",
    "ccm:diskmove-pick",
    "ccm:diskmove-from-clean",
    "ccm:diskmove-volumes",
    "ccm:diskmove-plan",
    "ccm:diskmove-run",
    "ccm:diskmove-back",
    "ccm:diskmove-cancel",
    "ccm:diskmove-state",
    "ccm:diskmove-recover",
];

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcInfo {
    pub name: String,
    pub cmd: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolumeInfo {
    pub mount: String,
    pub label: String,
    pub network: bool,
    pub total: u64,
    pub free: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub kind: &'static str,
    pub path: String,
    pub guide: bool,
}

fn obj(raw: &Value) -> Option<&Map<String, Value>> {
    raw.as_object()
}

fn id_ok(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    (!s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')).then(|| s.to_owned())
}

fn text_ok(v: &Value, max: usize) -> Option<String> {
    let s = v.as_str()?;
    let n = s.encode_utf16().count();
    (n > 0 && n <= max && !s.chars().any(|c| (c as u32) < 0x20)).then(|| s.to_owned())
}

pub fn parse_plan(raw: &Value) -> Option<(String, String)> {
    let o = obj(raw)?;
    Some((id_ok(o.get("candidate")?)?, text_ok(o.get("volume")?, 1024)?))
}

pub fn parse_id(raw: &Value, key: &str) -> Option<String> {
    id_ok(obj(raw)?.get(key)?)
}

pub fn parse_recover(raw: &Value) -> Option<(String, &'static str)> {
    let o = obj(raw)?;
    let id = id_ok(o.get("pending")?)?;
    let action = match o.get("action")?.as_str()? {
        "restore" => "restore",
        "finish" => "finish",
        _ => return None,
    };
    Some((id, action))
}

pub fn parse_from_clean(raw: &Value) -> Option<(String, bool)> {
    let o = obj(raw)?;
    Some((text_ok(o.get("path")?, 4096)?, o.get("open") == Some(&Value::Bool(true))))
}

pub fn move_need(bytes: u64) -> u64 {
    ((bytes as f64) * 1.1).ceil() as u64
}

const TOOLS: [&str; 18] = ["java", "javaw", "gradle", "gradlew", "adb", "emulator", "docker", "dockerd", "node", "dart", "dartaotruntime", "flutter", "studio", "studio64", "idea", "idea64", "kotlin-daemon", "pub"];

fn tool_stem(n: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    if TOOLS.contains(&n) {
        return true;
    }
    if let Some(rest) = n.strip_prefix("qemu-system") {
        return rest.chars().all(|c| word(c) || c == '-');
    }
    if let Some(rest) = n.strip_prefix("com.docker.") {
        return !rest.is_empty() && rest.chars().all(|c| word(c) || c == '.' || c == '-');
    }
    false
}

pub fn is_tool(name: &str) -> bool {
    let n = name.to_lowercase();
    tool_stem(&n) || n.strip_suffix(".exe").is_some_and(tool_stem)
}

pub fn holder_of(source: &str, procs: &[ProcInfo], plat: Plat) -> Option<String> {
    let fold = |s: &str| {
        let n = s.replace('\\', "/");
        let n = n.trim_end_matches('/');
        if plat == Plat::Linux {
            n.to_owned()
        } else {
            n.to_lowercase()
        }
    };
    let needle = fold(source);
    if needle.is_empty() {
        return None;
    }
    for p in procs {
        if !is_tool(&p.name) {
            continue;
        }
        let cmd = fold(&p.cmd);
        let Some(at) = cmd.find(&needle) else { continue };
        match cmd[at + needle.len()..].chars().next() {
            None => return Some(p.name.clone()),
            Some(c) if c == '/' || c.is_whitespace() || matches!(c, '"' | '\'' | ';' | ':') => return Some(p.name.clone()),
            _ => {}
        }
    }
    None
}

pub fn parse_proc_lines(text: &str) -> Vec<ProcInfo> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| match l.find('\t') {
            Some(i) => ProcInfo { name: l[..i].to_lowercase(), cmd: l[i + 1..].to_owned() },
            None => ProcInfo { name: l.to_lowercase(), cmd: String::new() },
        })
        .collect()
}

pub fn sdk_spots(plat: Plat, home: &str, env: &dyn Fn(&str) -> Option<String>) -> (String, String) {
    let j = |parts: &[&str]| parts.iter().skip(1).fold(parts[0].to_owned(), |acc, p| guard::join(&acc, p, plat));
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let local = get("LOCALAPPDATA").unwrap_or_else(|| j(&[home, "AppData", "Local"]));
    let sdk = get("ANDROID_HOME").or_else(|| get("ANDROID_SDK_ROOT")).unwrap_or_else(|| match plat {
        Plat::Win => j(&[&local, "Android", "Sdk"]),
        Plat::Mac => j(&[home, "Library", "Android", "sdk"]),
        Plat::Linux => j(&[home, "Android", "Sdk"]),
    });
    let user = get("ANDROID_USER_HOME").unwrap_or_else(|| get("ANDROID_SDK_HOME").map_or_else(|| j(&[home, ".android"]), |h| j(&[&h, ".android"])));
    let avd = get("ANDROID_AVD_HOME").unwrap_or_else(|| j(&[&user, "avd"]));
    (sdk, avd)
}

pub fn move_suggestions(plat: Plat, home: &str, env: &dyn Fn(&str) -> Option<String>) -> Vec<Suggestion> {
    let j = |parts: &[&str]| parts.iter().skip(1).fold(parts[0].to_owned(), |acc, p| guard::join(&acc, p, plat));
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let local = get("LOCALAPPDATA").unwrap_or_else(|| j(&[home, "AppData", "Local"]));
    let (sdk, avd) = sdk_spots(plat, home, env);
    let s = |kind: &'static str, path: String| Suggestion { kind, path, guide: false };
    vec![
        s("gradle", get("GRADLE_USER_HOME").unwrap_or_else(|| j(&[home, ".gradle"]))),
        s("pub", get("PUB_CACHE").unwrap_or_else(|| if plat == Plat::Win { j(&[&local, "Pub", "Cache"]) } else { j(&[home, ".pub-cache"]) })),
        s("avd", avd),
        s("sdk", sdk),
        s("npm", get("npm_config_cache").unwrap_or_else(|| if plat == Plat::Win { j(&[&local, "npm-cache"]) } else { j(&[home, ".npm"]) })),
        Suggestion {
            kind: "docker",
            path: match plat {
                Plat::Win => j(&[&local, "Docker", "wsl"]),
                Plat::Mac => j(&[home, "Library", "Containers", "com.docker.docker"]),
                Plat::Linux => "/var/lib/docker".into(),
            },
            guide: true,
        },
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveEntry {
    pub id: String,
    pub kind: &'static str,
    pub source: String,
    pub target: String,
    pub bytes: u64,
    pub files: u64,
    pub at: u64,
    pub stage: String,
    pub leftover: Option<String>,
}

pub struct Journal {
    pub file: PathBuf,
    gate: Mutex<()>,
}

fn num(v: Option<&Value>) -> u64 {
    v.and_then(Value::as_f64).filter(|f| f.is_finite() && *f >= 0.0).map_or(0, |f| f as u64)
}

impl Journal {
    pub fn new(file: PathBuf) -> Self {
        Journal { file, gate: Mutex::new(()) }
    }

    pub fn append(&self, e: &Value) {
        let _g = lock(&self.gate);
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.file) {
            let _ = writeln!(f, "{e}");
        }
    }

    pub fn entries(&self) -> Vec<MoveEntry> {
        let text = {
            let _g = lock(&self.gate);
            std::fs::read_to_string(&self.file).unwrap_or_default()
        };
        let mut list: Vec<MoveEntry> = Vec::new();
        let mut at: HashMap<String, usize> = HashMap::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(e) = serde_json::from_str::<Value>(line) else { continue };
            let (Some(id), Some(stage)) = (e.get("id").and_then(Value::as_str), e.get("stage").and_then(Value::as_str)) else { continue };
            let leftover = e.get("leftover").and_then(Value::as_str);
            if let Some(&i) = at.get(id) {
                let cur = &mut list[i];
                cur.stage = stage.to_owned();
                if stage == "done" {
                    cur.at = num(e.get("at"));
                }
                if let Some(l) = leftover {
                    cur.leftover = (!l.is_empty()).then(|| l.to_owned());
                }
                if stage == "cleaned" {
                    cur.leftover = None;
                }
                continue;
            }
            let (Some(source), Some(target)) = (e.get("source").and_then(Value::as_str), e.get("target").and_then(Value::as_str)) else { continue };
            at.insert(id.to_owned(), list.len());
            list.push(MoveEntry {
                id: id.to_owned(),
                kind: if e.get("kind").and_then(Value::as_str) == Some("back") { "back" } else { "move" },
                source: source.to_owned(),
                target: target.to_owned(),
                bytes: num(e.get("bytes")),
                files: num(e.get("files")),
                at: num(e.get("at")),
                stage: stage.to_owned(),
                leftover: leftover.filter(|l| !l.is_empty()).map(str::to_owned),
            });
        }
        list
    }
}

pub fn is_open(e: &MoveEntry) -> bool {
    e.leftover.is_some() || !matches!(e.stage.as_str(), "done" | "rolledBack" | "cleaned" | "returned")
}

fn record_of(e: &MoveEntry) -> Value {
    let mut v = json!({ "id": e.id, "source": e.source, "target": e.target, "at": e.at, "bytes": e.bytes, "files": e.files, "state": if e.kind == "back" { "back" } else { "moved" } });
    if let Some(l) = &e.leftover {
        v["leftover"] = json!(l);
    }
    v
}

fn pending_of(e: &MoveEntry, stage: &str) -> Value {
    json!({ "id": e.id, "kind": e.kind, "source": e.source, "target": e.target, "stage": stage })
}

pub trait Deps: Send + Sync {
    fn plat(&self) -> Plat;
    fn home(&self) -> String;
    fn data_dir(&self) -> PathBuf;
    fn keep(&self) -> Vec<String>;
    fn self_dirs(&self) -> Vec<String>;
    fn volumes(&self) -> Vec<VolumeInfo>;
    fn suggest(&self) -> Vec<Suggestion>;
    fn holders(&self) -> Vec<ProcInfo>;
    fn confirm(&self, ask: &MoveAsk) -> bool;
    fn trash(&self, p: &str) -> Result<(), String>;
    fn record(&self, e: Value);
    fn emit(&self, p: Value);
    fn index_size(&self, p: &str) -> Option<u64>;
    fn clean_known(&self, p: &str) -> Option<String>;
    fn pick(&self) -> Option<String>;
    fn free_of(&self, p: &str) -> Option<u64>;
    fn dev_of(&self, p: &str) -> Option<String>;
    fn now(&self) -> u64;
    fn new_id(&self) -> String {
        crate::config::uuid()
    }
}

#[derive(Clone, Debug)]
struct Candidate {
    id: String,
    path: String,
    kind: &'static str,
    size: Option<u64>,
    guide: bool,
    block: Option<&'static str>,
}

#[derive(Clone, Debug)]
struct Plan {
    id: String,
    candidate: String,
    source: String,
    target: String,
    bytes: u64,
    files: u64,
    volume: String,
    need: u64,
    free: u64,
}

#[derive(Default)]
struct Inner {
    candidates: Vec<Candidate>,
    plans: HashMap<String, Plan>,
    abort: Option<Arc<AtomicBool>>,
    running: Option<String>,
}

pub struct Service<D: Deps> {
    pub deps: D,
    pub journal: Journal,
    inner: Mutex<Inner>,
}

fn fail(error: &str) -> Value {
    json!({ "ok": false, "error": error })
}

fn failed_with(error: &str, rolled_back: bool) -> Value {
    let mut v = json!({ "ok": false, "error": error, "rolledBack": rolled_back });
    if error == tree::CANCELLED {
        v["cancelled"] = json!(true);
    }
    v
}

struct Throttle {
    last: Option<Instant>,
}

impl Throttle {
    fn ready(&mut self) -> bool {
        let now = Instant::now();
        if self.last.is_some_and(|l| now.duration_since(l).as_millis() < 200) {
            return false;
        }
        self.last = Some(now);
        true
    }
}

impl<D: Deps> Service<D> {
    pub fn new(deps: D) -> Self {
        let journal = Journal::new(deps.data_dir().join("disk").join("moves.jsonl"));
        Service { deps, journal, inner: Mutex::new(Inner::default()) }
    }

    fn plat(&self) -> Plat {
        self.deps.plat()
    }

    fn key(&self, p: &str) -> String {
        guard::path_key(&guard::normalize(p, self.plat()), self.plat())
    }

    fn emit(&self, id: &str, phase: &str, bytes: u64, total: u64, files: u64) {
        self.deps.emit(json!({ "id": id, "phase": phase, "bytes": bytes, "total": total, "files": files }));
    }

    fn busy(&self) -> bool {
        let i = lock(&self.inner);
        i.running.is_some() || i.abort.is_some()
    }

    fn ctx(&self, root: &str) -> GuardContext {
        GuardContext { home: self.deps.home(), keep: self.deps.keep(), roots: vec![root.to_owned()], volumes: Vec::new() }
    }

    fn block(&self, p: &str) -> Option<&'static str> {
        let plat = self.plat();
        let Some(st) = tree::lstat(Path::new(p)) else { return Some("missing") };
        if st.file_type().is_symlink() {
            let moved = self.journal.entries().iter().any(|e| e.kind == "move" && (e.stage == "done" || e.stage == "cleaned") && self.key(&e.source) == self.key(p));
            return Some(if moved { "moved" } else { "link" });
        }
        if !st.is_dir() {
            return Some("notDir");
        }
        if guard::resolve_deletable(p, &self.ctx(p), plat, &RealIo).is_err() {
            return Some("guard");
        }
        let data = self.deps.data_dir().to_string_lossy().into_owned();
        let mine = self.deps.self_dirs().iter().any(|s| !s.is_empty() && (guard::is_under(p, s, plat) || guard::is_under(s, p, plat)));
        if mine || guard::is_under(p, &data, plat) || guard::is_under(&data, p, plat) {
            return Some("self");
        }
        if tree::lstat(Path::new(&format!("{p}{MOVE_SUFFIX}"))).is_some() || tree::lstat(Path::new(&format!("{p}{MOVE_BACK_SUFFIX}"))).is_some() {
            return Some("pending");
        }
        None
    }

    fn add_candidate(&self, kind: &'static str, p: &str, guide: bool) -> Candidate {
        let abs = guard::normalize(p, self.plat());
        let existing = lock(&self.inner).candidates.iter().find(|c| self.key(&c.path) == self.key(&abs)).cloned();
        let block = if guide { None } else { self.block(&abs) };
        let size = if block.is_some() { None } else { self.deps.index_size(&abs) };
        let c = Candidate {
            id: existing.as_ref().map_or_else(|| self.deps.new_id(), |e| e.id.clone()),
            path: abs,
            kind: match &existing {
                Some(e) if e.kind != "picked" => e.kind,
                _ => kind,
            },
            size,
            guide,
            block,
        };
        let mut i = lock(&self.inner);
        match i.candidates.iter_mut().find(|x| x.id == c.id) {
            Some(slot) => *slot = c.clone(),
            None => i.candidates.push(c.clone()),
        }
        c
    }

    fn view(c: &Candidate) -> Value {
        let mut v = json!({ "id": c.id, "path": c.path, "kind": c.kind });
        if c.guide {
            v["guide"] = json!(true);
        }
        if let Some(b) = c.block {
            v["block"] = json!(b);
        }
        if let Some(s) = c.size {
            v["size"] = json!(s);
        }
        v
    }

    fn candidate(&self, id: &str) -> Option<Candidate> {
        lock(&self.inner).candidates.iter().find(|c| c.id == id).cloned()
    }

    pub fn list(&self) -> Value {
        for s in self.deps.suggest() {
            if !s.path.is_empty() {
                self.add_candidate(s.kind, &s.path, s.guide);
            }
        }
        let again: Vec<(&'static str, String)> = lock(&self.inner).candidates.iter().filter(|c| c.kind == "picked" || c.kind == "clean").map(|c| (c.kind, c.path.clone())).collect();
        for (kind, path) in again {
            self.add_candidate(kind, &path, false);
        }
        Value::Array(lock(&self.inner).candidates.iter().map(Self::view).collect())
    }

    pub fn pick(&self) -> Value {
        match self.deps.pick().filter(|p| guard::is_absolute(p, self.plat())) {
            Some(p) => Self::view(&self.add_candidate("picked", &p, false)),
            None => Value::Null,
        }
    }

    pub fn from_clean(&self, p: &str, open: bool) -> Value {
        let known = if open { guard::is_absolute(p, self.plat()).then(|| p.to_owned()) } else { self.deps.clean_known(p) };
        match known {
            Some(k) => Self::view(&self.add_candidate("clean", &k, false)),
            None => Value::Null,
        }
    }

    fn volume_list(&self, c: &Candidate) -> Vec<(String, String, u64, u64, Option<&'static str>)> {
        let plat = self.plat();
        let src_dev = self.deps.dev_of(&guard::parent(&c.path, plat));
        let mut out = Vec::new();
        for v in self.deps.volumes() {
            if v.mount.is_empty() {
                continue;
            }
            let dev = self.deps.dev_of(&v.mount);
            let free = self.deps.free_of(&v.mount).unwrap_or(v.free);
            let same = dev.is_some() && dev == src_dev;
            let inside = guard::is_under(&v.mount, &c.path, plat);
            let reason = if dev.is_none() {
                Some("missing")
            } else if v.network {
                Some("network")
            } else if same || inside {
                Some("same")
            } else if c.size.is_some_and(|s| free < move_need(s)) {
                Some("space")
            } else {
                None
            };
            out.push((v.mount, v.label, free, v.total, reason));
        }
        out
    }

    pub fn volumes(&self, candidate: &str) -> Value {
        let Some(c) = self.candidate(candidate) else { return json!([]) };
        Value::Array(
            self.volume_list(&c)
                .into_iter()
                .map(|(mount, label, free, total, reason)| {
                    let mut v = json!({ "mount": mount, "label": label, "free": free, "total": total, "ok": reason.is_none() });
                    if let Some(r) = reason {
                        v["reason"] = json!(r);
                    }
                    v
                })
                .collect(),
        )
    }

    pub fn cancel(&self) -> bool {
        match &lock(&self.inner).abort {
            Some(a) => {
                a.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    fn target_for(&self, source: &str, mount: &str) -> Option<String> {
        let plat = self.plat();
        let base = guard::join(mount, MOVE_DIR, plat);
        let name = guard::base_name(source, plat);
        (1..100).map(|i| guard::join(&base, &if i == 1 { name.clone() } else { format!("{name}-{i}") }, plat)).find(|t| tree::lstat(Path::new(t)).is_none())
    }

    pub fn lock_of(&self, source: &str) -> Option<String> {
        if let Some(h) = holder_of(source, &self.deps.holders(), self.plat()) {
            return Some(h);
        }
        let probe = format!("{source}.orrery-probe");
        if let Err(c) = tree::rename(Path::new(source), Path::new(&probe)) {
            return tree::lock_code(&c).then(|| "probe".to_owned());
        }
        let _ = tree::retry(|| tree::rename(Path::new(&probe), Path::new(source)));
        None
    }

    fn begin_abort(&self) -> Option<Arc<AtomicBool>> {
        let mut i = lock(&self.inner);
        if i.running.is_some() || i.abort.is_some() {
            return None;
        }
        let a = Arc::new(AtomicBool::new(false));
        i.abort = Some(a.clone());
        Some(a)
    }

    fn end_abort(&self) {
        lock(&self.inner).abort = None;
    }

    pub fn plan(&self, candidate: &str, mount: &str) -> Value {
        if self.busy() {
            return json!({ "error": "busy" });
        }
        let Some(c) = self.candidate(candidate).filter(|c| !c.guide) else { return json!({ "error": "invalid" }) };
        let Some(vol) = self.volume_list(&c).into_iter().find(|v| self.key(&v.0) == self.key(mount)) else { return json!({ "error": "invalid" }) };
        if let Some(r) = vol.4.filter(|r| *r != "space") {
            return json!({ "error": r });
        }
        if let Some(b) = self.block(&c.path) {
            return json!({ "error": b });
        }
        if let Some(h) = self.lock_of(&c.path) {
            return json!({ "error": "locked", "holder": h });
        }
        let Some(abort) = self.begin_abort() else { return json!({ "error": "busy" }) };
        let out = self.plan_inner(&c, &vol.0, vol.2, &abort);
        self.end_abort();
        out
    }

    fn plan_inner(&self, c: &Candidate, mount: &str, vol_free: u64, abort: &AtomicBool) -> Value {
        let plat = self.plat();
        self.emit(&c.id, "check", 0, 0, 0);
        let st = match tree::walk_tree(Path::new(&c.path), Some(abort), &mut |_, _| {}) {
            Ok(s) => s,
            Err(e) => return json!({ "error": e }),
        };
        if let Some(slot) = lock(&self.inner).candidates.iter_mut().find(|x| x.id == c.id) {
            slot.size = Some(st.bytes);
        }
        let free = self.deps.free_of(mount).unwrap_or(vol_free);
        let need = move_need(st.bytes);
        if free < need {
            return json!({ "error": "space" });
        }
        let Some(target) = self.target_for(&c.path, mount) else { return json!({ "error": "target" }) };
        if guard::is_under(&target, &c.path, plat) || guard::is_under(&c.path, &target, plat) {
            return json!({ "error": "same" });
        }
        let plan = Plan { id: self.deps.new_id(), candidate: c.id.clone(), source: c.path.clone(), target, bytes: st.bytes, files: st.files, volume: mount.to_owned(), need, free };
        let view = json!({ "id": plan.id, "candidate": plan.candidate, "source": plan.source, "target": plan.target, "bytes": plan.bytes, "files": plan.files, "volume": plan.volume, "need": plan.need, "free": plan.free });
        lock(&self.inner).plans.insert(plan.id.clone(), plan);
        json!({ "plan": view })
    }

    fn log(&self, e: Value) {
        self.journal.append(&e);
    }

    fn history(&self, action: &str, freed: u64, paths: &[&str]) {
        self.deps.record(json!({ "at": self.deps.now(), "action": action, "count": 1, "freed": freed, "paths": paths, "failed": 0 }));
    }

    fn drop_old(&self, source: &str, old: &str, bytes: u64) -> Option<String> {
        if guard::resolve_deletable(old, &self.ctx(old), self.plat(), &RealIo).is_err() {
            return Some(old.to_owned());
        }
        if self.deps.trash(old).is_ok() {
            self.history("trash", bytes, &[old]);
            return None;
        }
        if !self.deps.confirm(&MoveAsk { kind: "permanent", source: source.to_owned(), target: old.to_owned(), bytes }) {
            return Some(old.to_owned());
        }
        match tree::remove_tree(Path::new(old)) {
            Ok(()) => {
                self.history("delete", bytes, &[old]);
                None
            }
            Err(_) => Some(old.to_owned()),
        }
    }

    fn begin_run(&self, id: &str) -> Option<Arc<AtomicBool>> {
        let mut i = lock(&self.inner);
        if i.running.is_some() || i.abort.is_some() {
            return None;
        }
        let a = Arc::new(AtomicBool::new(false));
        i.abort = Some(a.clone());
        i.running = Some(id.to_owned());
        Some(a)
    }

    fn end_run(&self) {
        let mut i = lock(&self.inner);
        i.abort = None;
        i.running = None;
    }

    pub fn run(&self, plan_id: &str) -> Value {
        if self.busy() {
            return fail("busy");
        }
        let Some(plan) = lock(&self.inner).plans.remove(plan_id) else { return fail("invalid") };
        let (source, target) = (plan.source.clone(), plan.target.clone());
        if let Some(b) = self.block(&source) {
            return fail(b);
        }
        if !self.deps.confirm(&MoveAsk { kind: "move", source: source.clone(), target: target.clone(), bytes: plan.bytes }) {
            return json!({ "ok": false, "cancelled": true });
        }
        if let Some(h) = self.lock_of(&source) {
            return json!({ "ok": false, "error": "locked", "holder": h });
        }
        if self.deps.free_of(&plan.volume).unwrap_or(0) < move_need(plan.bytes) {
            return fail("space");
        }
        if tree::lstat(Path::new(&target)).is_some() {
            return fail("target");
        }
        let id = self.deps.new_id();
        let Some(abort) = self.begin_run(&id) else { return fail("busy") };
        let out = self.run_inner(&id, &plan, &abort);
        self.end_run();
        out
    }

    fn run_inner(&self, id: &str, plan: &Plan, abort: &AtomicBool) -> Value {
        let (source, target) = (plan.source.as_str(), plan.target.as_str());
        let moving = format!("{source}{MOVE_SUFFIX}");
        let mut stage = "copy";
        let mut made = false;
        let r = (|| -> Result<Value, String> {
            self.log(json!({ "id": id, "stage": "copying", "at": self.deps.now(), "kind": "move", "source": source, "target": target, "bytes": plan.bytes, "files": plan.files }));
            std::fs::create_dir_all(guard::parent(target, self.plat())).map_err(|e| tree::code_of(&e))?;
            made = true;
            let mut th = Throttle { last: None };
            tree::copy_tree(Path::new(source), Path::new(target), Some(abort), &mut |bytes, files| {
                if th.ready() {
                    self.emit(id, "copy", bytes, plan.bytes, files);
                }
            })?;
            self.emit(id, "verify", plan.bytes, plan.bytes, plan.files);
            if let Some(reason) = tree::verify_copy(Path::new(source), Path::new(target), Some(abort))? {
                return Err(format!("verify:{reason}"));
            }
            self.emit(id, "switch", plan.bytes, plan.bytes, plan.files);
            tree::rename(Path::new(source), Path::new(&moving)).map_err(|c| if tree::lock_code(&c) { "locked".to_owned() } else { c })?;
            stage = "renamed";
            self.log(json!({ "id": id, "stage": "renamed", "at": self.deps.now() }));
            tree::make_dir_link(Path::new(target), Path::new(source)).map_err(|e| tree::code_of(&e))?;
            stage = "linked";
            let real = tree::realpath(Path::new(source));
            let real_target = tree::realpath(Path::new(target));
            if real.is_none() || real.as_deref().map(|r| self.key(r)) != real_target.as_deref().map(|r| self.key(r)) {
                return Err("linkCheck".into());
            }
            std::fs::read_dir(source).map_err(|e| tree::code_of(&e))?;
            self.log(json!({ "id": id, "stage": "linked", "at": self.deps.now() }));
            self.emit(id, "cleanup", plan.bytes, plan.bytes, plan.files);
            let leftover = self.drop_old(source, &moving, plan.bytes);
            let mut done = json!({ "id": id, "stage": "done", "at": self.deps.now() });
            if let Some(l) = &leftover {
                done["leftover"] = json!(l);
            }
            self.log(done);
            self.emit(id, "done", plan.bytes, plan.bytes, plan.files);
            self.history("move", plan.bytes, &[source, target]);
            let mut record = json!({ "id": id, "source": source, "target": target, "at": self.deps.now(), "bytes": plan.bytes, "files": plan.files, "state": "moved" });
            let mut out = json!({ "ok": true });
            if let Some(l) = &leftover {
                record["leftover"] = json!(l);
                out["leftover"] = json!(l);
            }
            out["record"] = record;
            Ok(out)
        })();
        match r {
            Ok(v) => v,
            Err(error) => {
                self.emit(id, "rollback", 0, 0, 0);
                let back = self.rollback_move(stage, source, &moving, target, made);
                self.log(json!({ "id": id, "stage": if back { "rolledBack" } else { "failed" }, "at": self.deps.now(), "error": error }));
                failed_with(&error, back)
            }
        }
    }

    fn rollback_move(&self, stage: &str, source: &str, moving: &str, target: &str, made: bool) -> bool {
        let r = (|| -> Result<bool, String> {
            if stage == "linked" || stage == "renamed" {
                if tree::lstat(Path::new(moving)).is_none() {
                    return Ok(false);
                }
                match tree::lstat(Path::new(source)) {
                    Some(st) if st.file_type().is_symlink() => tree::retry(|| tree::unlink(Path::new(source)))?,
                    Some(_) => return Ok(false),
                    None => {}
                }
                tree::retry(|| tree::rename(Path::new(moving), Path::new(source)))?;
            }
            if made {
                tree::remove_tree(Path::new(target))?;
            }
            Ok(true)
        })();
        r.unwrap_or(false)
    }

    fn find_move(&self, id: &str) -> Option<MoveEntry> {
        self.journal.entries().into_iter().find(|e| e.id == id && e.kind == "move" && (e.stage == "done" || e.stage == "cleaned"))
    }

    pub fn back(&self, move_id: &str) -> Value {
        if self.busy() {
            return fail("busy");
        }
        let Some(m) = self.find_move(move_id) else { return fail("invalid") };
        let (source, target) = (m.source.clone(), m.target.clone());
        if m.leftover.as_ref().is_some_and(|l| tree::lstat(Path::new(l)).is_some()) {
            return fail("leftover");
        }
        if !tree::is_link(Path::new(&source)) {
            return fail("notLink");
        }
        if !tree::lstat(Path::new(&target)).is_some_and(|m| m.is_dir()) {
            return fail("targetMissing");
        }
        let Some(real_target) = tree::realpath(Path::new(&target)) else { return fail("invalid") };
        match tree::realpath(Path::new(&source)) {
            Some(r) if self.key(&r) == self.key(&real_target) => {}
            _ => return fail("notLink"),
        }
        let tmp = format!("{source}{MOVE_BACK_SUFFIX}");
        if tree::lstat(Path::new(&tmp)).is_some() {
            return fail("pending");
        }
        let holder = self.lock_of(&target).or_else(|| holder_of(&source, &self.deps.holders(), self.plat()));
        if let Some(h) = holder {
            return json!({ "ok": false, "error": "locked", "holder": h });
        }
        let Some(abort) = self.begin_abort() else { return fail("busy") };
        let size = tree::walk_tree(Path::new(&target), Some(&abort), &mut |_, _| {});
        self.end_abort();
        let size = match size {
            Ok(s) => s,
            Err(e) => return fail(&e),
        };
        if self.deps.free_of(&guard::parent(&source, self.plat())).unwrap_or(0) < move_need(size.bytes) {
            return fail("space");
        }
        if !self.deps.confirm(&MoveAsk { kind: "back", source: source.clone(), target: target.clone(), bytes: size.bytes }) {
            return json!({ "ok": false, "cancelled": true });
        }
        let id = self.deps.new_id();
        let Some(abort) = self.begin_run(&id) else { return fail("busy") };
        let out = self.back_inner(&id, &m, &tmp, size, &abort);
        self.end_run();
        out
    }

    fn back_inner(&self, id: &str, m: &MoveEntry, tmp: &str, size: tree::TreeStat, abort: &AtomicBool) -> Value {
        let (source, target) = (m.source.as_str(), m.target.as_str());
        let mut unlinked = false;
        let r = (|| -> Result<Value, String> {
            self.log(json!({ "id": id, "stage": "copying", "at": self.deps.now(), "kind": "back", "source": source, "target": target, "bytes": size.bytes, "files": size.files }));
            let mut th = Throttle { last: None };
            tree::copy_tree(Path::new(target), Path::new(tmp), Some(abort), &mut |bytes, files| {
                if th.ready() {
                    self.emit(id, "copy", bytes, size.bytes, files);
                }
            })?;
            self.emit(id, "verify", size.bytes, size.bytes, size.files);
            if let Some(reason) = tree::verify_copy(Path::new(target), Path::new(tmp), Some(abort))? {
                return Err(format!("verify:{reason}"));
            }
            self.emit(id, "switch", size.bytes, size.bytes, size.files);
            tree::retry(|| tree::unlink(Path::new(source)))?;
            unlinked = true;
            self.log(json!({ "id": id, "stage": "renamed", "at": self.deps.now() }));
            tree::retry(|| tree::rename(Path::new(tmp), Path::new(source)))?;
            unlinked = false;
            self.log(json!({ "id": m.id, "stage": "returned", "at": self.deps.now() }));
            self.emit(id, "cleanup", size.bytes, size.bytes, size.files);
            let leftover = match tree::remove_tree(Path::new(target)) {
                Ok(()) => {
                    self.history("delete", size.bytes, &[target]);
                    None
                }
                Err(_) => Some(target.to_owned()),
            };
            let mut done = json!({ "id": id, "stage": "done", "at": self.deps.now() });
            let mut record = json!({ "id": id, "source": source, "target": target, "at": self.deps.now(), "bytes": size.bytes, "files": size.files, "state": "back" });
            let mut out = json!({ "ok": true });
            if let Some(l) = &leftover {
                done["leftover"] = json!(l);
                record["leftover"] = json!(l);
                out["leftover"] = json!(l);
            }
            self.log(done);
            self.emit(id, "done", size.bytes, size.bytes, size.files);
            out["record"] = record;
            Ok(out)
        })();
        match r {
            Ok(v) => v,
            Err(error) => {
                self.emit(id, "rollback", 0, 0, 0);
                let mut ok = true;
                if unlinked && tree::lstat(Path::new(source)).is_none() && tree::make_dir_link(Path::new(target), Path::new(source)).is_err() {
                    ok = false;
                }
                if ok && tree::remove_tree(Path::new(tmp)).is_err() {
                    ok = false;
                }
                self.log(json!({ "id": id, "stage": if ok { "rolledBack" } else { "failed" }, "at": self.deps.now(), "error": error }));
                failed_with(&error, ok)
            }
        }
    }

    fn pending_stage(&self, e: &MoveEntry) -> Option<Value> {
        let lst = |p: &str| tree::lstat(Path::new(p));
        let src = lst(&e.source);
        let src_link = src.as_ref().is_some_and(|m| m.file_type().is_symlink());
        let src_dir = src.as_ref().is_some_and(|m| m.is_dir() && !m.file_type().is_symlink());
        if e.kind == "move" {
            let moving = lst(&format!("{}{MOVE_SUFFIX}", e.source)).is_some();
            if moving && src_link {
                return Some(pending_of(e, "linked"));
            }
            if moving && src.is_none() {
                return Some(pending_of(e, "renamed"));
            }
            if !moving && src_dir && lst(&e.target).is_some() && e.stage != "done" && e.stage != "cleaned" {
                return Some(pending_of(e, "copying"));
            }
            return None;
        }
        let tmp = lst(&format!("{}{MOVE_BACK_SUFFIX}", e.source)).is_some();
        if tmp && src_link {
            return Some(pending_of(e, "copying"));
        }
        if tmp && src.is_none() {
            return Some(pending_of(e, "renamed"));
        }
        if !tmp && e.leftover.as_ref().is_some_and(|l| lst(l).is_some()) && src_dir {
            return Some(pending_of(e, "linked"));
        }
        None
    }

    pub fn state(&self) -> Value {
        let running = lock(&self.inner).running.clone();
        let mut moves: Vec<Value> = Vec::new();
        let mut pending: Vec<Value> = Vec::new();
        for e in self.journal.entries() {
            if Some(&e.id) == running.as_ref() {
                continue;
            }
            if e.kind == "move" && (e.stage == "done" || e.stage == "cleaned") && tree::is_link(Path::new(&e.source)) {
                let mut r = record_of(&e);
                if e.leftover.as_ref().is_some_and(|l| tree::lstat(Path::new(l)).is_none()) {
                    if let Some(o) = r.as_object_mut() {
                        o.remove("leftover");
                    }
                }
                moves.push(r);
            }
            if is_open(&e) {
                if let Some(p) = self.pending_stage(&e) {
                    pending.push(p);
                }
            }
        }
        moves.sort_by(|a, b| b["at"].as_u64().unwrap_or(0).cmp(&a["at"].as_u64().unwrap_or(0)));
        json!({ "moves": moves, "pending": pending, "running": running })
    }

    pub fn recover(&self, pending_id: &str, action: &'static str) -> Value {
        if self.busy() {
            return fail("busy");
        }
        let entry = self.journal.entries().into_iter().find(|x| x.id == pending_id);
        let Some((e, p)) = entry.and_then(|e| if is_open(&e) { self.pending_stage(&e).map(|p| (e, p)) } else { None }) else { return fail("invalid") };
        if !self.deps.confirm(&MoveAsk { kind: action, source: e.source.clone(), target: e.target.clone(), bytes: e.bytes }) {
            return json!({ "ok": false, "cancelled": true });
        }
        {
            let mut i = lock(&self.inner);
            if i.running.is_some() || i.abort.is_some() {
                return fail("busy");
            }
            i.running = Some(e.id.clone());
        }
        let out = self.recover_inner(&e, p["stage"].as_str().unwrap_or(""), action).unwrap_or_else(|c| fail(&c));
        lock(&self.inner).running = None;
        out
    }

    fn recover_inner(&self, e: &MoveEntry, stage: &str, action: &str) -> Result<Value, String> {
        let moving = format!("{}{MOVE_SUFFIX}", e.source);
        let tmp = format!("{}{MOVE_BACK_SUFFIX}", e.source);
        if e.kind == "move" {
            if action == "finish" {
                if stage != "linked" {
                    return Ok(fail("invalid"));
                }
                if let Some(l) = self.drop_old(&e.source, &moving, e.bytes) {
                    return Ok(json!({ "ok": false, "error": "leftover", "leftover": l }));
                }
                self.log(json!({ "id": e.id, "stage": "cleaned", "at": self.deps.now(), "leftover": "" }));
                return Ok(json!({ "ok": true }));
            }
            let ok = self.rollback_move(if stage == "copying" { "copy" } else { "linked" }, &e.source, &moving, &e.target, true);
            self.log(json!({ "id": e.id, "stage": if ok { "rolledBack" } else { "failed" }, "at": self.deps.now() }));
            return Ok(if ok { json!({ "ok": true, "rolledBack": true }) } else { fail("restore") });
        }
        if stage == "linked" {
            if action != "finish" {
                return Ok(fail("invalid"));
            }
            tree::remove_tree(Path::new(&e.target))?;
            self.log(json!({ "id": e.id, "stage": "cleaned", "at": self.deps.now(), "leftover": "" }));
            return Ok(json!({ "ok": true }));
        }
        if stage == "renamed" && action == "finish" {
            tree::retry(|| tree::rename(Path::new(&tmp), Path::new(&e.source)))?;
            self.log(json!({ "id": e.id, "stage": "done", "at": self.deps.now(), "leftover": e.target }));
            return Ok(json!({ "ok": true }));
        }
        if stage == "renamed" {
            tree::make_dir_link(Path::new(&e.target), Path::new(&e.source)).map_err(|x| tree::code_of(&x))?;
        }
        tree::remove_tree(Path::new(&tmp))?;
        self.log(json!({ "id": e.id, "stage": "rolledBack", "at": self.deps.now() }));
        Ok(json!({ "ok": true, "rolledBack": true }))
    }

    pub fn handle(&self, channel: &str, a: &Value) -> Value {
        let bad = || fail("invalid");
        match channel {
            "ccm:diskmove-list" => self.list(),
            "ccm:diskmove-pick" => self.pick(),
            "ccm:diskmove-from-clean" => parse_from_clean(a).map_or(Value::Null, |(p, open)| self.from_clean(&p, open)),
            "ccm:diskmove-volumes" => parse_id(a, "candidate").map_or(json!([]), |id| self.volumes(&id)),
            "ccm:diskmove-plan" => parse_plan(a).map_or(json!({ "error": "invalid" }), |(c, v)| self.plan(&c, &v)),
            "ccm:diskmove-run" => parse_id(a, "plan").map_or_else(bad, |id| self.run(&id)),
            "ccm:diskmove-back" => parse_id(a, "move").map_or_else(bad, |id| self.back(&id)),
            "ccm:diskmove-cancel" => json!(self.cancel()),
            "ccm:diskmove-state" => self.state(),
            "ccm:diskmove-recover" => parse_recover(a).map_or_else(bad, |(id, action)| self.recover(&id, action)),
            _ => Value::Null,
        }
    }
}

fn e2e_on() -> bool {
    cfg!(debug_assertions) || crate::config::e2e_isolated()
}

fn auto_confirm() -> bool {
    std::env::var("CCM_E2E_DISKMAP_CONFIRM").is_ok_and(|v| v == "1") && e2e_on()
}

fn fake_root() -> Option<PathBuf> {
    std::env::var("CCM_E2E_DISKMOVE").ok().filter(|v| !v.is_empty() && e2e_on()).map(PathBuf::from)
}

pub fn fake_suggestions(fake: &Path) -> Vec<Suggestion> {
    let h = fake.join("home");
    let p = |rel: &[&str]| rel.iter().fold(h.clone(), |acc, s| acc.join(s)).to_string_lossy().into_owned();
    vec![
        Suggestion { kind: "gradle", path: p(&[".gradle"]), guide: false },
        Suggestion { kind: "avd", path: p(&[".android", "avd"]), guide: false },
        Suggestion { kind: "npm", path: p(&["npm-cache"]), guide: false },
        Suggestion { kind: "docker", path: p(&["Docker"]), guide: true },
    ]
}

pub fn fake_volumes(fake: &Path) -> Vec<VolumeInfo> {
    let base = fake.join("vols");
    let mut names: Vec<String> = std::fs::read_dir(&base).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let mount = base.join(&n).to_string_lossy().into_owned();
            let (free, total) = tree::space_of(&mount).unwrap_or((0, 0));
            VolumeInfo { network: n.starts_with("net"), label: n, mount, total, free }
        })
        .collect()
}

pub fn fake_dev(fake: &Path, p: &str, plat: Plat) -> Option<String> {
    tree::lstat(Path::new(p))?;
    let base = fake.join("vols").to_string_lossy().to_lowercase();
    let lower = p.to_lowercase();
    if !guard::is_under(&lower, &base, plat) || guard::path_key(&guard::normalize(&lower, plat), plat) == guard::path_key(&guard::normalize(&base, plat), plat) {
        return Some("1".into());
    }
    let rest = guard::normalize(&lower, plat)[guard::normalize(&base, plat).len()..].trim_start_matches(['\\', '/']).to_owned();
    let first = rest.split(['\\', '/']).next().unwrap_or("");
    let mut h: u32 = 7;
    for c in first.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(c));
    }
    Some((100 + h % 100_000).to_string())
}

pub struct AppDeps {
    app: AppHandle,
    fake: Option<PathBuf>,
}

impl AppDeps {
    fn disk(&self) -> Option<Arc<crate::disk_map::DiskMap>> {
        self.app.try_state::<crate::disk_map::DiskState>().map(|s| s.0.clone())
    }
}

impl Deps for AppDeps {
    fn plat(&self) -> Plat {
        Plat::host()
    }

    fn home(&self) -> String {
        crate::disk_map::home_of(&self.app)
    }

    fn data_dir(&self) -> PathBuf {
        self.app.state::<Shell>().data_dir.clone()
    }

    fn keep(&self) -> Vec<String> {
        crate::disk_map::keep_of(&self.app)
    }

    fn self_dirs(&self) -> Vec<String> {
        let p = self.app.path();
        let shell = self.app.state::<Shell>();
        let mut out: Vec<PathBuf> = [p.app_data_dir(), p.app_local_data_dir()].into_iter().flatten().collect();
        out.push(shell.data_dir.clone());
        out.push(shell.webview_dir.clone());
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            out.push(dir);
        }
        if cfg!(debug_assertions) {
            if let Some(repo) = Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent) {
                out.push(repo.to_path_buf());
            }
        }
        out.into_iter().map(|d| d.to_string_lossy().into_owned()).filter(|s| !s.is_empty()).collect()
    }

    fn volumes(&self) -> Vec<VolumeInfo> {
        if let Some(f) = &self.fake {
            return fake_volumes(f);
        }
        let Some(m) = self.disk() else { return Vec::new() };
        m.volumes()
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|v| VolumeInfo {
                        mount: v["mount"].as_str().unwrap_or("").to_owned(),
                        label: v["label"].as_str().unwrap_or("").to_owned(),
                        network: v["network"] == json!(true),
                        total: num(v.get("total")),
                        free: num(v.get("free")),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn suggest(&self) -> Vec<Suggestion> {
        match &self.fake {
            Some(f) => fake_suggestions(f),
            None => move_suggestions(Plat::host(), &self.home(), &|k| std::env::var(k).ok()),
        }
    }

    fn holders(&self) -> Vec<ProcInfo> {
        if let Some(f) = &self.fake {
            return parse_proc_lines(&std::fs::read_to_string(f.join("procs.txt")).unwrap_or_default());
        }
        let Some(rows) = core_procs::table() else { return Vec::new() };
        let tools: Vec<(u32, String)> = rows.into_iter().filter(|r| is_tool(&r.name)).map(|r| (r.pid, r.name.to_lowercase())).collect();
        if tools.is_empty() {
            return Vec::new();
        }
        let pids: Vec<u32> = tools.iter().map(|t| t.0).collect();
        let cmds: HashMap<u32, String> = core_procs::cmdline(&pids).into_iter().map(|c| (c.pid, c.cmd)).collect();
        tools.into_iter().map(|(pid, name)| ProcInfo { name, cmd: cmds.get(&pid).cloned().unwrap_or_default() }).collect()
    }

    fn confirm(&self, ask: &MoveAsk) -> bool {
        if auto_confirm() {
            return true;
        }
        let b = move_box(ask, &crate::hosts::locale(&self.app));
        crate::hosts::confirm(&self.app, &b.message, &b.detail, &b.ok, &b.cancel)
    }

    fn trash(&self, p: &str) -> Result<(), String> {
        if tree::lstat(Path::new(p)).is_none() {
            return Err("ENOENT".into());
        }
        trash::delete(p).map_err(|e| e.to_string())
    }

    fn record(&self, e: Value) {
        if let Some(m) = self.disk() {
            m.history.append(&e);
        }
    }

    fn emit(&self, p: Value) {
        crate::desk::emit_app(&self.app, PROGRESS, p);
    }

    fn index_size(&self, p: &str) -> Option<u64> {
        let m = self.disk()?;
        if !m.host.running() {
            return None;
        }
        let st = crate::disk_map::to_status(&m.host.call("disk.status", json!({})).ok()?);
        if st["indexed"] != json!(true) || st["scanning"] == json!(true) {
            return None;
        }
        if !crate::disk_map::status_roots(&st).iter().any(|r| guard::is_under(p, r, Plat::host())) {
            return None;
        }
        m.size_of(p)
    }

    fn clean_known(&self, p: &str) -> Option<String> {
        crate::disk_clean::known_plain(&self.app, p)
    }

    fn pick(&self) -> Option<String> {
        if let Some(f) = &self.fake {
            return std::fs::read_to_string(f.join("pick.txt")).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        }
        self.app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
    }

    fn free_of(&self, p: &str) -> Option<u64> {
        tree::space_of(p).map(|s| s.0)
    }

    fn dev_of(&self, p: &str) -> Option<String> {
        match &self.fake {
            Some(f) => fake_dev(f, p, Plat::host()),
            None => tree::dev_of(p),
        }
    }

    fn now(&self) -> u64 {
        crate::disk_map::now_ms()
    }
}

pub struct DiskMoveState(pub Arc<Service<AppDeps>>);

fn service(app: &AppHandle) -> Arc<Service<AppDeps>> {
    if let Some(s) = app.try_state::<DiskMoveState>() {
        return s.0.clone();
    }
    let svc = Arc::new(Service::new(AppDeps { app: app.clone(), fake: fake_root() }));
    app.manage(DiskMoveState(svc.clone()));
    app.try_state::<DiskMoveState>().map_or(svc, |s| s.0.clone())
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    if !handles(channel) {
        return Err(format!("not ported: {channel}"));
    }
    let a = args.first().cloned().unwrap_or(Value::Null);
    Ok(service(app).handle(channel, &a))
}

#[cfg(test)]
#[path = "disk_move_tests.rs"]
mod tests;
