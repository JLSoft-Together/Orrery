use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{json, Value};

use crate::disk_guard::{self as guard, GuardContext, Plat, RealIo};
use crate::disk_organize_fs as ofs;
use crate::disk_organize_log::{hex, new_run_id, RunHeader, RunStore, RunWriter};
use crate::disk_organize_rules::{self as rules, clean_folder, clean_patch, Rule};
use crate::state::lock;

pub const CHANGED: &str = "ccm:organize-changed";

const CHANNELS: [&str; 8] = [
    "ccm:organize-state",
    "ccm:organize-settings",
    "ccm:organize-add-source",
    "ccm:organize-remove-source",
    "ccm:organize-plan",
    "ccm:organize-run",
    "ccm:organize-undo",
    "ccm:organize-junk",
];

const PLAN_TTL: f64 = 30.0 * 60_000.0;
const MAX_PLANS: usize = 5;
const MAX_SOURCES: usize = 12;
const MAX_PLAN_MOVES: usize = 5000;
const RETRY_MS: u64 = 30_000;
const LOCKED: [&str; 3] = ["EBUSY", "EPERM", "EACCES"];

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

pub struct Config {
    pub data_dir: PathBuf,
    pub plat: Plat,
    pub home: String,
    pub downloads: String,
    pub desktop: String,
    pub app_dirs: Vec<String>,
    pub stable_ms: u64,
    pub locale: Box<dyn Fn() -> String + Send + Sync>,
    pub broadcast: Box<dyn Fn(Value) + Send + Sync>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stored {
    pub sources: Vec<String>,
    pub source: String,
    pub mode: String,
    pub kinds: Vec<String>,
    pub rules: Vec<Rule>,
    pub auto: bool,
    pub since: HashMap<String, f64>,
}

impl Stored {
    fn settings_json(&self) -> Value {
        json!({ "sources": self.sources, "source": self.source, "mode": self.mode, "kinds": self.kinds, "rules": self.rules.iter().map(Rule::to_json).collect::<Vec<_>>(), "auto": self.auto })
    }

    fn file_json(&self) -> Value {
        let mut v = self.settings_json();
        v["since"] = json!(self.since);
        v
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedMove {
    pub name: String,
    pub folder: String,
    pub target: String,
    pub size: u64,
    pub mtime: f64,
    pub kind: &'static str,
    pub touched: f64,
    pub rule: Option<usize>,
}

#[derive(Debug, Default)]
pub struct PlanOutcome {
    pub moves: Vec<PlannedMove>,
    pub skipped: Vec<(String, String)>,
    pub wait_until: Option<f64>,
    pub pending_min: Option<f64>,
}

pub struct PlanOpts<'a> {
    pub mode: &'a str,
    pub kinds: &'a [String],
    pub rules: &'a [Rule],
    pub locale: &'a str,
    pub plat: Plat,
    pub now: f64,
    pub stable_ms: f64,
    pub since: Option<f64>,
    pub max: usize,
}

fn skips(list: &[(String, String)]) -> Value {
    json!(list.iter().map(|(name, reason)| json!({ "name": name, "reason": reason })).collect::<Vec<_>>())
}

pub fn key_of(p: &str, plat: Plat) -> String {
    guard::path_key(&guard::normalize(p, plat), plat)
}

fn name_key(name: &str, plat: Plat) -> String {
    if plat == Plat::Linux {
        name.to_owned()
    } else {
        name.to_lowercase()
    }
}

pub fn free_name(dir: &str, name: &str, taken: &mut HashSet<String>, plat: Plat) -> Option<String> {
    for n in 1..10_000 {
        let candidate = rules::with_suffix(name, n);
        let full = guard::join(dir, &candidate, plat);
        let key = key_of(&full, plat);
        if taken.contains(&key) {
            continue;
        }
        let used = match std::fs::symlink_metadata(&full) {
            Ok(_) => true,
            Err(e) => ofs::code_of(&e) != "ENOENT",
        };
        if !used {
            taken.insert(key);
            return Some(candidate);
        }
    }
    None
}

pub fn build_plan(source: &str, o: &PlanOpts) -> std::io::Result<PlanOutcome> {
    let mut out = PlanOutcome::default();
    let mut entries: Vec<(String, std::fs::FileType)> = Vec::new();
    for e in std::fs::read_dir(source)? {
        let e = e?;
        if let Ok(ft) = e.file_type() {
            entries.push((e.file_name().to_string_lossy().into_owned(), ft));
        }
    }
    let names: HashSet<String> = entries.iter().map(|(n, _)| name_key(n, o.plat)).collect();
    let mut taken: HashSet<String> = HashSet::new();
    let mut folder_state: HashMap<String, bool> = HashMap::new();
    let pending = |out: &mut PlanOutcome, t: f64| out.pending_min = Some(out.pending_min.map_or(t, |p| p.min(t)));
    for (name, ft) in &entries {
        if out.moves.len() >= o.max {
            out.skipped.push((name.clone(), "limit".into()));
            continue;
        }
        if ft.is_symlink() {
            out.skipped.push((name.clone(), "link".into()));
            continue;
        }
        if ft.is_dir() || !ft.is_file() {
            continue;
        }
        if rules::is_hidden_name(name) {
            out.skipped.push((name.clone(), "hidden".into()));
            continue;
        }
        let full = guard::join(source, name, o.plat);
        let Ok(st) = ofs::lstat(&full) else {
            out.skipped.push((name.clone(), "unreadable".into()));
            continue;
        };
        if !st.file || st.link {
            continue;
        }
        let t = st.touched;
        if o.since.is_some_and(|s| t <= s) {
            continue;
        }
        if rules::is_partial_name(name) || rules::PARTIAL_EXT.iter().any(|x| names.contains(&name_key(&format!("{name}.{x}"), o.plat))) {
            out.skipped.push((name.clone(), "downloading".into()));
            pending(&mut out, t);
            continue;
        }
        if o.now - t < o.stable_ms {
            out.skipped.push((name.clone(), "recent".into()));
            let ready = t + o.stable_ms;
            out.wait_until = Some(out.wait_until.map_or(ready, |w| w.min(ready)));
            pending(&mut out, t);
            continue;
        }
        let Some(c) = rules::classify(name, st.size, st.mtime, o.mode, o.kinds, o.rules, o.locale) else {
            out.skipped.push((name.clone(), "kind".into()));
            continue;
        };
        let dir = guard::join(source, &c.folder, o.plat);
        let ok = *folder_state.entry(c.folder.clone()).or_insert_with(|| match std::fs::symlink_metadata(&dir) {
            Ok(m) => m.is_dir() && !m.file_type().is_symlink(),
            Err(e) => ofs::code_of(&e) == "ENOENT",
        });
        if !ok {
            out.skipped.push((name.clone(), "blocked".into()));
            continue;
        }
        let Some(target) = free_name(&dir, name, &mut taken, o.plat) else {
            out.skipped.push((name.clone(), "blocked".into()));
            continue;
        };
        out.moves.push(PlannedMove { name: name.clone(), folder: c.folder, target, size: st.size, mtime: st.mtime, kind: c.kind, touched: t, rule: c.rule });
    }
    Ok(out)
}

struct PlanEntry {
    id: String,
    source: String,
    mode: String,
    moves: Vec<(String, PlannedMove)>,
    at: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Act {
    Pass,
    Open,
}

struct Slot {
    source: String,
    watcher: Option<RecommendedWatcher>,
}

#[derive(Default)]
struct Timers {
    due: HashMap<(String, Act), (String, Instant)>,
    running: bool,
}

struct Executed {
    run: Option<String>,
    moved: usize,
    bytes: u64,
    failed: Vec<(String, String)>,
    retry_at: Option<f64>,
}

pub struct Organize {
    cfg: Config,
    me: Weak<Organize>,
    settings: Mutex<Option<Stored>>,
    plans: Mutex<Vec<PlanEntry>>,
    serial: Mutex<()>,
    slots: Mutex<HashMap<String, Slot>>,
    timers: Mutex<Timers>,
    wake: Condvar,
    disposed: AtomicBool,
    runs: RunStore,
}

impl Organize {
    pub fn new(cfg: Config) -> Arc<Organize> {
        let runs = RunStore::new(&cfg.data_dir.join("organize"));
        Arc::new_cyclic(|me| Organize { cfg, me: me.clone(), settings: Mutex::new(None), plans: Mutex::new(Vec::new()), serial: Mutex::new(()), slots: Mutex::new(HashMap::new()), timers: Mutex::new(Timers::default()), wake: Condvar::new(), disposed: AtomicBool::new(false), runs })
    }

    fn now(&self) -> f64 {
        crate::disk_map::now_ms() as f64
    }

    fn plat(&self) -> Plat {
        self.cfg.plat
    }

    fn key(&self, p: &str) -> String {
        key_of(p, self.plat())
    }

    fn same(&self, a: &str, b: &str) -> bool {
        self.key(a) == self.key(b)
    }

    fn settings_file(&self) -> PathBuf {
        self.cfg.data_dir.join("organize").join("settings.json")
    }

    fn ctx(&self, source: &str) -> GuardContext {
        GuardContext { home: self.cfg.home.clone(), keep: Vec::new(), roots: vec![source.to_owned()], volumes: Vec::new() }
    }

    fn in_tree(&self, p: &str) -> bool {
        let plat = self.plat();
        let n = guard::normalize(p, plat);
        self.cfg.app_dirs.iter().filter(|d| !d.is_empty()).any(|d| guard::is_under(&n, d, plat) || guard::is_under(d, &n, plat))
    }

    fn resolve(&self, p: &str, source: &str) -> Result<guard::Canonical, String> {
        if self.in_tree(p) {
            return Err("protected".into());
        }
        let c = guard::resolve_deletable(p, &self.ctx(source), self.plat(), &RealIo).map_err(str::to_owned)?;
        if self.in_tree(&c.path) {
            return Err("protected".into());
        }
        Ok(c)
    }

    pub fn load(&self) -> Stored {
        let mut slot = lock(&self.settings);
        if let Some(s) = slot.as_ref() {
            return s.clone();
        }
        let raw: Value = std::fs::read_to_string(self.settings_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
        let patch = clean_patch(&raw);
        let plat = self.plat();
        let sources: Vec<String> = match raw.get("sources").and_then(Value::as_array) {
            Some(a) => a.iter().filter_map(Value::as_str).filter(|s| guard::is_absolute(s, plat)).take(MAX_SOURCES).map(str::to_owned).collect(),
            None if !self.cfg.downloads.is_empty() => vec![self.cfg.downloads.clone()],
            None => Vec::new(),
        };
        let since: HashMap<String, f64> = raw.get("since").and_then(Value::as_object).map(|o| o.iter().filter_map(|(k, v)| v.as_f64().filter(|n| n.is_finite()).map(|n| (k.clone(), n))).collect()).unwrap_or_default();
        let source = match patch.source.as_ref() {
            Some(s) if sources.iter().any(|x| self.same(x, s)) => s.clone(),
            _ => sources.first().cloned().unwrap_or_default(),
        };
        let s = Stored { sources, source, mode: patch.mode.unwrap_or_else(|| "type".into()), kinds: patch.kinds.unwrap_or_else(|| rules::KINDS.iter().map(|k| (*k).to_owned()).collect()), rules: patch.rules.unwrap_or_default(), auto: patch.auto.unwrap_or(false), since };
        *slot = Some(s.clone());
        s
    }

    fn store(&self, s: Stored) {
        let file = self.settings_file();
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = file.with_file_name(format!("settings.json.{}.tmp", hex(3)));
        if std::fs::write(&tmp, serde_json::to_string_pretty(&s.file_json()).unwrap_or_default()).is_ok() && std::fs::rename(&tmp, &file).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        *lock(&self.settings) = Some(s);
    }

    pub fn check_source(&self, p: &str) -> Result<String, String> {
        let c = self.resolve(p, p)?;
        if c.missing {
            return Err("missing".into());
        }
        if c.link {
            return Err("link".into());
        }
        match std::fs::metadata(&c.path) {
            Ok(m) if m.is_dir() => Ok(c.path),
            Ok(_) => Err("notdir".into()),
            Err(_) => Err("missing".into()),
        }
    }

    fn label_of(&self, p: &str) -> &'static str {
        if !self.cfg.downloads.is_empty() && self.same(p, &self.cfg.downloads) {
            "downloads"
        } else if !self.cfg.desktop.is_empty() && self.same(p, &self.cfg.desktop) {
            "desktop"
        } else {
            "custom"
        }
    }

    pub fn state(&self) -> Value {
        let s = self.load();
        let sources: Vec<Value> = s
            .sources
            .iter()
            .map(|p| match self.check_source(p) {
                Ok(_) => json!({ "path": p, "label": self.label_of(p), "ok": true }),
                Err(reason) => json!({ "path": p, "label": self.label_of(p), "ok": false, "reason": reason }),
            })
            .collect();
        let watching: Vec<String> = lock(&self.slots).values().filter(|w| w.watcher.is_some()).map(|w| w.source.clone()).collect();
        json!({ "settings": s.settings_json(), "sources": sources, "runs": self.runs.list(50), "watching": watching, "stableMs": self.cfg.stable_ms })
    }

    pub fn patch(&self, v: &Value) -> Value {
        let mut s = self.load();
        let p = clean_patch(v);
        if let Some(src) = &p.source {
            if let Some(hit) = s.sources.iter().find(|x| self.same(x, src)).cloned() {
                s.source = hit;
            }
        }
        if let Some(m) = p.mode {
            s.mode = m;
        }
        if let Some(k) = p.kinds {
            s.kinds = k;
        }
        if let Some(r) = p.rules {
            s.rules = r;
        }
        if let Some(auto) = p.auto.filter(|a| *a != s.auto) {
            s.auto = auto;
            if auto {
                let now = self.now();
                for src in s.sources.clone() {
                    s.since.insert(self.key(&src), now);
                }
            }
        }
        self.store(s);
        self.sync_watchers();
        self.state()
    }

    pub fn add_source(&self, p: Option<String>) -> Value {
        let Some(p) = p.filter(|p| !p.is_empty()) else { return json!({ "state": self.state() }) };
        let checked = match self.check_source(&p) {
            Ok(c) => c,
            Err(reason) => return json!({ "state": self.state(), "reason": reason }),
        };
        let mut s = self.load();
        if !s.sources.iter().any(|x| self.same(x, &checked)) {
            if s.sources.len() >= MAX_SOURCES {
                return json!({ "state": self.state(), "reason": "limit" });
            }
            s.sources.push(checked.clone());
            s.since.insert(self.key(&checked), self.now());
        }
        if let Some(hit) = s.sources.iter().find(|x| self.same(x, &checked)).cloned() {
            s.source = hit;
        }
        self.store(s);
        self.sync_watchers();
        json!({ "state": self.state() })
    }

    pub fn remove_source(&self, v: &Value) -> Value {
        if let Some(p) = v.as_str() {
            let mut s = self.load();
            s.sources.retain(|x| !self.same(x, p));
            s.since.remove(&self.key(p));
            if !s.sources.iter().any(|x| self.same(x, &s.source)) {
                s.source = s.sources.first().cloned().unwrap_or_default();
            }
            self.store(s);
            self.sync_watchers();
        }
        self.state()
    }

    fn opts<'a>(&self, s: &'a Stored, locale: &'a str, now: f64, since: Option<f64>) -> PlanOpts<'a> {
        PlanOpts { mode: &s.mode, kinds: &s.kinds, rules: &s.rules, locale, plat: self.plat(), now, stable_ms: self.cfg.stable_ms as f64, since, max: MAX_PLAN_MOVES }
    }

    pub fn plan(&self, v: &Value) -> Value {
        let s = self.load();
        let want = v.get("source").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| s.source.clone());
        let id = hex(8);
        let empty = |source: &str, error: &str| json!({ "id": id, "source": source, "mode": s.mode, "items": [], "skipped": [], "error": error });
        let Some(listed) = s.sources.iter().find(|x| self.same(x, &want)).cloned() else { return empty(&want, "unknown") };
        let path = match self.check_source(&listed) {
            Ok(p) => p,
            Err(reason) => return empty(&listed, &reason),
        };
        let locale = (self.cfg.locale)();
        let Ok(outcome) = build_plan(&path, &self.opts(&s, &locale, self.now(), None)) else { return empty(&listed, "unreadable") };
        let mut moves = Vec::new();
        let items: Vec<Value> = outcome
            .moves
            .into_iter()
            .enumerate()
            .map(|(i, m)| {
                let mut item = json!({ "id": i.to_string(), "name": m.name, "folder": m.folder, "target": m.target, "size": m.size, "mtime": m.mtime, "kind": m.kind });
                if let Some(r) = m.rule {
                    item["rule"] = json!(r);
                }
                moves.push((i.to_string(), m));
                item
            })
            .collect();
        let plan = json!({ "id": id, "source": path, "mode": s.mode, "items": items, "skipped": skips(&outcome.skipped) });
        let mut plans = lock(&self.plans);
        let now = self.now();
        plans.retain(|e| now - e.at <= PLAN_TTL);
        plans.push(PlanEntry { id: id.clone(), source: path, mode: s.mode.clone(), moves, at: now });
        while plans.len() > MAX_PLANS {
            plans.remove(0);
        }
        plan
    }

    pub fn run(&self, v: &Value) -> Value {
        let fail = |error: &str| json!({ "run": null, "moved": 0, "bytes": 0, "failed": [], "error": error });
        let Some(plan_id) = v.get("plan").and_then(Value::as_str) else { return fail("unknown") };
        let entry = {
            let mut plans = lock(&self.plans);
            match plans.iter().position(|e| e.id == plan_id) {
                Some(i) => plans.remove(i),
                None => return fail("expired"),
            }
        };
        let exclude: HashSet<String> = v.get("exclude").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
        let moves: Vec<PlannedMove> = entry.moves.into_iter().filter(|(id, _)| !exclude.contains(id)).map(|(_, m)| m).collect();
        if moves.is_empty() {
            return fail("empty");
        }
        let _serial = lock(&self.serial);
        match self.check_source(&entry.source) {
            Ok(c) if self.same(&c, &entry.source) => {
                let r = self.execute(&c, &moves, &entry.mode, false);
                json!({ "run": r.run, "moved": r.moved, "bytes": r.bytes, "failed": skips(&r.failed) })
            }
            Ok(_) => fail("changed"),
            Err(reason) => fail(&reason),
        }
    }

    fn dir_ok(&self, source: &str, dir: &str) -> &'static str {
        match std::fs::symlink_metadata(dir) {
            Err(e) if ofs::code_of(&e) == "ENOENT" => return "missing",
            Err(_) => return "blocked",
            Ok(m) if m.file_type().is_symlink() || !m.is_dir() => return "blocked",
            Ok(_) => {}
        }
        let plat = self.plat();
        match guard::canonical_of(dir, plat, &RealIo) {
            Some(c) if !c.link && !c.missing && self.same(&guard::parent(&c.path, plat), source) && !self.in_tree(&c.path) => "ok",
            _ => "blocked",
        }
    }

    fn from_ok(&self, source: &str, from: &str) -> bool {
        self.resolve(from, source).is_ok_and(|c| !c.link && !c.missing && self.same(&guard::parent(&c.path, self.plat()), source))
    }

    fn execute(&self, source: &str, moves: &[PlannedMove], mode: &str, auto: bool) -> Executed {
        let plat = self.plat();
        let stable = self.cfg.stable_ms as f64;
        let mut out = Executed { run: None, moved: 0, bytes: 0, failed: Vec::new(), retry_at: None };
        let mut writer: Option<RunWriter> = None;
        let mut taken: HashSet<String> = HashSet::new();
        let mut created: Vec<String> = Vec::new();
        let ensure = |writer: &mut Option<RunWriter>, run: &mut Option<String>| {
            if writer.is_none() {
                let id = new_run_id(self.now());
                *writer = Some(self.runs.writer(RunHeader { id: id.clone(), at: self.now(), source: source.to_owned(), mode: mode.to_owned(), auto }));
                *run = Some(id);
            }
        };
        for m in moves {
            let from = guard::join(source, &m.name, plat);
            let Ok(st) = ofs::lstat(&from) else {
                out.failed.push((m.name.clone(), "missing".into()));
                continue;
            };
            if !st.file || st.link || st.size != m.size || st.mtime != m.mtime {
                out.failed.push((m.name.clone(), "changed".into()));
                continue;
            }
            if self.now() - st.touched < stable {
                out.failed.push((m.name.clone(), "recent".into()));
                continue;
            }
            if clean_folder(&json!(m.folder)).is_none() || !self.from_ok(source, &from) {
                out.failed.push((m.name.clone(), "guard".into()));
                continue;
            }
            let dir = guard::join(source, &m.folder, plat);
            let mut state = self.dir_ok(source, &dir);
            if state == "missing" {
                match std::fs::create_dir(&dir) {
                    Ok(()) => {
                        created.push(dir.clone());
                        ensure(&mut writer, &mut out.run);
                        if let Some(w) = writer.as_mut() {
                            let _ = w.write(&json!({ "t": "dir", "path": dir }), true);
                        }
                    }
                    Err(e) if ofs::code_of(&e) == "EEXIST" => {}
                    Err(_) => {
                        out.failed.push((m.name.clone(), "blocked".into()));
                        continue;
                    }
                }
                state = self.dir_ok(source, &dir);
            }
            if state != "ok" {
                out.failed.push((m.name.clone(), "blocked".into()));
                continue;
            }
            let mut target = m.target.clone();
            let first = guard::join(&dir, &target, plat);
            if taken.contains(&self.key(&first)) || ofs::exists(&first).unwrap_or(true) {
                match free_name(&dir, &m.name, &mut taken, plat) {
                    Some(n) => target = n,
                    None => {
                        out.failed.push((m.name.clone(), "blocked".into()));
                        continue;
                    }
                }
            }
            let to = guard::join(&dir, &target, plat);
            taken.insert(self.key(&to));
            ensure(&mut writer, &mut out.run);
            let w = writer.as_mut().expect("writer");
            if w.write(&json!({ "t": "intent", "from": from, "to": to }), true).is_err() {
                out.failed.push((m.name.clone(), "failed".into()));
                continue;
            }
            match ofs::move_file(&from, &to).and_then(|_| ofs::lstat(&to).map_err(|e| ofs::code_of(&e))) {
                Ok(after) => {
                    let _ = w.write(&json!({ "t": "move", "from": from, "to": to, "size": after.size, "mtime": after.mtime }), false);
                    out.moved += 1;
                    out.bytes += after.size;
                }
                Err(code) => {
                    let _ = w.write(&json!({ "t": "fail", "to": to }), false);
                    let locked = LOCKED.contains(&code.as_str());
                    out.failed.push((m.name.clone(), if locked { "locked".into() } else { code }));
                    if locked {
                        out.retry_at = Some(self.now() + stable);
                    }
                }
            }
        }
        for d in &created {
            let _ = std::fs::remove_dir(d);
        }
        drop(writer);
        if out.moved > 0 && auto {
            (self.cfg.broadcast)(json!({ "run": out.run, "moved": out.moved }));
        }
        out
    }

    pub fn undo(&self, v: &Value) -> Value {
        let fail = |error: &str| json!({ "restored": 0, "conflicts": [], "dirs": 0, "error": error });
        let id = v.get("run").and_then(Value::as_str).unwrap_or("").to_owned();
        let _serial = lock(&self.serial);
        let Some(rec) = self.runs.read(&id) else { return fail("missing") };
        if rec.undo.is_some() {
            return fail("done");
        }
        let source = rec.header.source.clone();
        match self.check_source(&source) {
            Ok(c) if self.same(&c, &source) => {}
            Ok(_) => return fail("changed"),
            Err(reason) => return fail(&reason),
        }
        let plat = self.plat();
        let mut w = self.runs.appender(&rec.header.id);
        let mut conflicts: Vec<(String, String)> = Vec::new();
        let mut restored = 0usize;
        let mut dirs = 0usize;
        for m in rec.moves.iter().rev() {
            if rec.backs.contains(&m.to) {
                continue;
            }
            let name = guard::base_name(&m.from, plat);
            let to_dir = guard::parent(&m.to, plat);
            if !self.same(&guard::parent(&m.from, plat), &source) || !self.same(&guard::parent(&to_dir, plat), &source) {
                conflicts.push((name, "guard".into()));
                continue;
            }
            let st = match ofs::lstat(&m.to) {
                Ok(st) => st,
                Err(_) => {
                    if m.done {
                        conflicts.push((name, "missing".into()));
                    }
                    continue;
                }
            };
            if m.done && (!st.file || st.link || Some(st.size as f64) != m.size || (st.mtime - m.mtime.unwrap_or(-1.0)).abs() >= 1.0) {
                conflicts.push((name, "changed".into()));
                continue;
            }
            if !m.done && !st.file {
                continue;
            }
            if ofs::exists(&m.from).unwrap_or(true) {
                if m.done {
                    conflicts.push((name, "occupied".into()));
                }
                continue;
            }
            if self.dir_ok(&source, &to_dir) != "ok" {
                conflicts.push((name, "guard".into()));
                continue;
            }
            match ofs::move_file(&m.to, &m.from) {
                Ok(_) => {
                    let _ = w.write(&json!({ "t": "back", "from": m.from, "to": m.to }), true);
                    restored += 1;
                }
                Err(code) => conflicts.push((name, if LOCKED.contains(&code.as_str()) { "locked".into() } else { code })),
            }
        }
        for d in rec.dirs.iter().rev() {
            if !self.same(&guard::parent(d, plat), &source) {
                continue;
            }
            let empty = std::fs::read_dir(d).map(|mut r| r.next().is_none()).unwrap_or(false);
            if empty && self.dir_ok(&source, d) == "ok" && std::fs::remove_dir(d).is_ok() {
                dirs += 1;
            }
        }
        let _ = w.write(&json!({ "t": "undo", "at": self.now(), "restored": restored, "conflicts": conflicts.len() }), true);
        json!({ "restored": restored, "conflicts": skips(&conflicts), "dirs": dirs })
    }

    pub fn start(&self) {
        self.load();
        self.sync_watchers();
    }

    pub fn sync_watchers(&self) {
        if self.disposed.load(Ordering::SeqCst) {
            return;
        }
        let s = self.load();
        let want: Vec<String> = if s.auto { s.sources.clone() } else { Vec::new() };
        let drop_keys: Vec<String> = lock(&self.slots).keys().filter(|k| !want.iter().any(|x| self.key(x) == **k)).cloned().collect();
        let dropped: Vec<Slot> = {
            let mut slots = lock(&self.slots);
            drop_keys.iter().filter_map(|k| slots.remove(k)).collect()
        };
        drop(dropped);
        if !drop_keys.is_empty() {
            let mut t = lock(&self.timers);
            t.due.retain(|(k, _), _| !drop_keys.contains(k));
        }
        for p in want {
            let fresh = !lock(&self.slots).contains_key(&self.key(&p));
            if fresh {
                self.open(&p);
            }
        }
    }

    fn open(&self, source: &str) {
        let k = self.key(source);
        lock(&self.timers).due.remove(&(k.clone(), Act::Open));
        let me = self.me.clone();
        let src = source.to_owned();
        let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Some(o) = me.upgrade() else { return };
            match res {
                Ok(_) => o.schedule(&src, Act::Pass, o.cfg.stable_ms + 500),
                Err(_) => o.watch_failed(&src),
            }
        });
        let watcher = made.ok().and_then(|mut w| w.watch(Path::new(source), RecursiveMode::NonRecursive).is_ok().then_some(w));
        let ok = watcher.is_some();
        lock(&self.slots).insert(k, Slot { source: source.to_owned(), watcher });
        if ok {
            self.schedule(source, Act::Pass, 0);
        } else {
            self.watch_failed(source);
        }
    }

    fn watch_failed(&self, source: &str) {
        let k = self.key(source);
        let old = {
            let mut slots = lock(&self.slots);
            match slots.get_mut(&k) {
                Some(slot) => slot.watcher.take(),
                None => return,
            }
        };
        drop(old);
        if !self.disposed.load(Ordering::SeqCst) {
            self.schedule(source, Act::Open, RETRY_MS.max(self.cfg.stable_ms));
        }
    }

    fn schedule(&self, source: &str, act: Act, ms: u64) {
        if self.disposed.load(Ordering::SeqCst) || !lock(&self.slots).contains_key(&self.key(source)) {
            return;
        }
        let mut t = lock(&self.timers);
        t.due.insert((self.key(source), act), (source.to_owned(), Instant::now() + Duration::from_millis(ms)));
        if !t.running {
            t.running = true;
            let me = self.me.clone();
            std::thread::Builder::new().name("organize-timer".into()).spawn(move || timer_loop(me)).ok();
        }
        drop(t);
        self.wake.notify_all();
    }

    pub fn pass(&self, source: &str) -> usize {
        let _serial = lock(&self.serial);
        let s = self.load();
        if self.disposed.load(Ordering::SeqCst) || !s.auto || !s.sources.iter().any(|x| self.same(x, source)) {
            return 0;
        }
        let Ok(path) = self.check_source(source) else { return 0 };
        let k = self.key(source);
        let since = s.since.get(&k).copied().unwrap_or_else(|| self.now());
        let start = self.now();
        let locale = (self.cfg.locale)();
        let Ok(outcome) = build_plan(&path, &self.opts(&s, &locale, start, Some(since))) else { return 0 };
        let r = if outcome.moves.is_empty() { Executed { run: None, moved: 0, bytes: 0, failed: Vec::new(), retry_at: None } } else { self.execute(&path, &outcome.moves, &s.mode, true) };
        let mut next = start - self.cfg.stable_ms as f64;
        if let Some(p) = outcome.pending_min {
            next = next.min(p - 1.0);
        }
        for m in &outcome.moves {
            if r.failed.iter().any(|(n, reason)| *n == m.name && reason != "missing") {
                next = next.min(m.touched - 1.0);
            }
        }
        if next > since {
            let mut cur = self.load();
            cur.since.insert(k, next);
            self.store(cur);
        }
        let wakes: Vec<f64> = [outcome.wait_until, r.retry_at].into_iter().flatten().collect();
        if let Some(w) = wakes.into_iter().reduce(f64::min) {
            self.schedule(source, Act::Pass, (w - self.now() + 250.0).max(0.0) as u64);
        }
        r.moved
    }

    pub fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
        let old: Vec<Slot> = lock(&self.slots).drain().map(|(_, s)| s).collect();
        drop(old);
        lock(&self.timers).due.clear();
        self.wake.notify_all();
    }
}

fn timer_loop(me: Weak<Organize>) {
    loop {
        let Some(o) = me.upgrade() else { return };
        let mut t = lock(&o.timers);
        if o.disposed.load(Ordering::SeqCst) {
            t.running = false;
            return;
        }
        let now = Instant::now();
        let next = t.due.iter().min_by_key(|(_, (_, at))| *at).map(|(k, (src, at))| (k.clone(), src.clone(), *at));
        match next {
            None => {
                let (g, _) = o.wake.wait_timeout(t, Duration::from_secs(60)).unwrap_or_else(|e| e.into_inner());
                if g.due.is_empty() {
                    let mut g = g;
                    g.running = false;
                    return;
                }
            }
            Some((k, src, at)) if at <= now => {
                t.due.remove(&k);
                drop(t);
                match k.1 {
                    Act::Pass => {
                        o.pass(&src);
                    }
                    Act::Open => {
                        if lock(&o.slots).contains_key(&k.0) {
                            o.open(&src);
                        }
                    }
                }
            }
            Some((_, _, at)) => {
                let _ = o.wake.wait_timeout(t, at - now);
            }
        }
    }
}

fn e2e_on() -> bool {
    cfg!(debug_assertions) || crate::config::e2e_isolated()
}

fn e2e_var(name: &str) -> Option<String> {
    if !e2e_on() {
        return None;
    }
    std::env::var(format!("CCM_E2E_ORGANIZE_{name}")).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

pub struct OrganizeState(pub Arc<Organize>);

pub fn app_dirs(app: &tauri::AppHandle) -> Vec<String> {
    use tauri::Manager;
    let p = app.path();
    let shell = app.state::<crate::state::Shell>();
    let mut out: Vec<PathBuf> = [p.app_data_dir(), p.app_local_data_dir()].into_iter().flatten().collect();
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

fn service(app: &tauri::AppHandle) -> Arc<Organize> {
    use tauri::Manager;
    if let Some(s) = app.try_state::<OrganizeState>() {
        return s.0.clone();
    }
    let p = app.path();
    let dir_of = |r: tauri::Result<PathBuf>| r.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
    let stable = e2e_var("STABLE_MS").and_then(|v| v.parse::<u64>().ok()).unwrap_or(rules::STABLE_MS);
    let (la, ba) = (app.clone(), app.clone());
    let cfg = Config {
        data_dir: app.state::<crate::state::Shell>().data_dir.clone(),
        plat: Plat::host(),
        home: crate::disk_map::home_of(app),
        downloads: e2e_var("DOWNLOADS").unwrap_or_else(|| dir_of(p.download_dir())),
        desktop: e2e_var("DESKTOP").unwrap_or_else(|| dir_of(p.desktop_dir())),
        app_dirs: app_dirs(app),
        stable_ms: stable,
        locale: Box::new(move || crate::hosts::locale(&la)),
        broadcast: Box::new(move |payload| crate::desk::emit_app(&ba, CHANGED, payload)),
    };
    let svc = Organize::new(cfg);
    app.manage(OrganizeState(svc.clone()));
    app.try_state::<OrganizeState>().map_or(svc, |s| s.0.clone())
}

pub fn start(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || service(&app).start());
}

pub fn stop(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(s) = app.try_state::<OrganizeState>() {
        s.0.dispose();
    }
}

fn pick(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    if let Some(p) = e2e_var("PICK") {
        return Some(p);
    }
    app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

fn no_state(stable: u64) -> Value {
    json!({ "settings": { "sources": [], "source": "", "mode": "type", "kinds": [], "rules": [], "auto": false }, "sources": [], "runs": [], "watching": [], "stableMs": stable })
}

pub fn dispatch(app: &tauri::AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    if !handles(channel) {
        return Err(format!("not ported: {channel}"));
    }
    let o = service(app);
    let a = args.first().cloned().unwrap_or(Value::Null);
    let stable = o.cfg.stable_ms;
    let app2 = app.clone();
    let o2 = o.clone();
    let ch = channel.to_owned();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || match ch.as_str() {
        "ccm:organize-state" => o2.state(),
        "ccm:organize-settings" => o2.patch(&a),
        "ccm:organize-add-source" => {
            let p = match a.as_str() {
                Some("downloads") => Some(o2.cfg.downloads.clone()),
                Some("desktop") => Some(o2.cfg.desktop.clone()),
                Some("pick") => pick(&app2),
                _ => None,
            };
            o2.add_source(p)
        }
        "ccm:organize-remove-source" => o2.remove_source(&a),
        "ccm:organize-plan" => o2.plan(&a),
        "ccm:organize-run" => o2.run(&a),
        "ccm:organize-undo" => o2.undo(&a),
        _ => crate::disk_organize_junk::scan(&app2, &o2.cfg.app_dirs),
    }));
    Ok(r.unwrap_or_else(|_| match channel {
        "ccm:organize-add-source" => json!({ "state": no_state(stable), "reason": "failed" }),
        "ccm:organize-plan" => json!({ "id": "", "source": "", "mode": "type", "items": [], "skipped": [], "error": "failed" }),
        "ccm:organize-run" => json!({ "run": null, "moved": 0, "bytes": 0, "failed": [], "error": "failed" }),
        "ccm:organize-undo" => json!({ "restored": 0, "conflicts": [], "dirs": 0, "error": "failed" }),
        "ccm:organize-junk" => json!({ "indexed": false, "items": [] }),
        _ => no_state(stable),
    }))
}

#[cfg(test)]
#[path = "disk_organize_tests.rs"]
mod tests;
