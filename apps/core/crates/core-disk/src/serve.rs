use crate::fold::{group_id, GROUP_NAMES};
use crate::dupes::{self, DupeProgress};
use crate::index::{flag, Index, Sort};
use crate::mft::{self, MftError};
use crate::search::{search, Only, Query};
use crate::walk::{scan_with, Control, Level, Lister, Progress, ScanError, ScanOptions};
use crate::{store, volumes};
use serde_json::{json, Map, Value};
use std::io::{BufRead, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

pub const CHANNELS: [&str; 15] = ["disk.scan", "disk.pause", "disk.resume", "disk.cancel", "disk.status", "disk.children", "disk.search", "disk.save", "disk.load", "disk.volumes", "disk.clear", "disk.forget", "disk.mft", "disk.sizes", "disk.dupes"];
pub const EVENT_PROGRESS: &str = "disk:progress";
pub const EVENT_DONE: &str = "disk:done";
pub const EVENT_DUPES_PROGRESS: &str = "disk:dupes-progress";
pub const EVENT_DUPES_DONE: &str = "disk:dupes-done";
const TICK: Duration = Duration::from_millis(100);
const LIMIT_MAX: usize = 10_000;
const SIZES_MAX: usize = 2_000;
const DRAIN: Duration = Duration::from_secs(3);

pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

pub struct Engine {
    index: RwLock<Option<Arc<Index>>>,
    scan: Mutex<Option<Arc<Control>>>,
    data_dir: Option<PathBuf>,
    emit: Emit,
    lister: Lister,
    mft_fixture: Option<PathBuf>,
}

fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

fn u(a: &Value, k: &str) -> Option<u64> {
    a.get(k).and_then(|v| v.as_u64().or_else(|| v.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)))
}

fn b(a: &Value, k: &str) -> Option<bool> {
    a.get(k).and_then(Value::as_bool)
}

fn strings(a: &Value, k: &str) -> Vec<String> {
    a.get(k).and_then(Value::as_array).map(|v| v.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn secs(ms: Option<u64>) -> Option<u32> {
    ms.map(|m| (m / 1000).min(u32::MAX as u64) as u32)
}

pub fn item(ix: &Index, i: u32) -> Value {
    let n = &ix.nodes[i as usize];
    let f = |bit: u16| n.flags & bit != 0;
    json!({
        "path": ix.path(i),
        "name": ix.name(i),
        "dir": n.is_dir(),
        "size": n.size,
        "len": n.len,
        "mtime": n.mtime as u64 * 1000,
        "count": n.count,
        "kind": GROUP_NAMES[(n.group as usize).min(GROUP_NAMES.len() - 1)],
        "cloud": f(flag::CLOUD),
        "link": f(flag::LINK),
        "hardlink": f(flag::HARDLINK),
        "denied": f(flag::DENIED),
        "mount": f(flag::MOUNT),
        "hidden": f(flag::HIDDEN) || f(flag::SYSTEM),
    })
}

fn summary(ix: &Index) -> Value {
    let st = &ix.stats;
    json!({"entries": ix.len(), "files": st.files, "dirs": st.dirs, "bytes": st.bytes, "denied": st.denied, "errors": st.errors, "ms": st.elapsed_ms, "scannedAt": st.scanned_at, "roots": ix.root_paths(), "indexBytes": ix.heap_bytes()})
}

pub fn parse_query(a: &Value, ix: &Index) -> Result<Query, String> {
    let mut q = Query::new(s(a, "query").unwrap_or(""));
    q.min_size = u(a, "minSize");
    q.max_size = u(a, "maxSize");
    q.after = secs(u(a, "after"));
    q.before = secs(u(a, "before"));
    if let Some(k) = s(a, "kind").filter(|k| !k.is_empty()) {
        q.group = Some(group_id(k).ok_or_else(|| format!("unknown kind {k}"))?);
    }
    q.only = match s(a, "only") {
        Some("file") | Some("files") => Some(Only::Files),
        Some("dir") | Some("dirs") => Some(Only::Dirs),
        _ => None,
    };
    if let Some(p) = s(a, "under").filter(|p| !p.is_empty()) {
        q.under = Some(ix.find(p).ok_or_else(|| format!("not in index: {p}"))?);
    }
    q.sort = Sort::parse(s(a, "sort").unwrap_or("size"));
    q.limit = u(a, "limit").map(|l| l as usize).unwrap_or(100).min(LIMIT_MAX);
    Ok(q)
}

impl Engine {
    pub fn new(data_dir: Option<PathBuf>, emit: Emit) -> Arc<Self> {
        Self::with_lister(data_dir, emit, crate::walk::sys::list)
    }

    pub fn with_lister(data_dir: Option<PathBuf>, emit: Emit, lister: Lister) -> Arc<Self> {
        Self::make(data_dir, emit, lister, None)
    }

    fn make(data_dir: Option<PathBuf>, emit: Emit, lister: Lister, mft_fixture: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Engine { index: RwLock::new(None), scan: Mutex::new(None), data_dir, emit, lister, mft_fixture })
    }

    pub fn from_env(emit: Emit) -> Arc<Self> {
        let dir = std::env::var("CCM_DATA_DIR").ok().filter(|v| !v.is_empty()).map(|d| PathBuf::from(d).join("disk"));
        let fixture = std::env::var_os("CCM_MFT_FIXTURE").filter(|v| !v.is_empty()).map(PathBuf::from);
        Self::make(dir, emit, crate::walk::sys::list, fixture)
    }

    pub fn with_mft_fixture(data_dir: Option<PathBuf>, emit: Emit, fixture: PathBuf) -> Arc<Self> {
        Self::make(data_dir, emit, crate::walk::sys::list, Some(fixture))
    }

    pub fn index(&self) -> Option<Arc<Index>> {
        self.index.read().unwrap().clone()
    }

    pub fn set_index(&self, ix: Option<Index>) {
        *self.index.write().unwrap() = ix.map(Arc::new);
    }

    pub fn scanning(&self) -> bool {
        self.scan.lock().unwrap().is_some()
    }

    fn dir(&self, a: &Value) -> Result<PathBuf, String> {
        s(a, "dir").filter(|d| !d.is_empty()).map(PathBuf::from).or_else(|| self.data_dir.clone()).ok_or_else(|| "no data dir".to_string())
    }

    fn control(&self) -> Option<Arc<Control>> {
        self.scan.lock().unwrap().clone()
    }

    pub fn dispatch(self: &Arc<Self>, channel: &str, args: &Value) -> Result<Value, String> {
        let a = match args {
            Value::Array(v) => v.first().cloned().unwrap_or(Value::Null),
            other => other.clone(),
        };
        let a = if a.is_object() { a } else { Value::Object(Map::new()) };
        match channel {
            "disk.scan" => self.start_scan(&a),
            "disk.pause" => Ok(json!({"ok": self.control().map(|c| c.pause()).is_some()})),
            "disk.resume" => Ok(json!({"ok": self.control().map(|c| c.resume()).is_some()})),
            "disk.cancel" => Ok(json!({"ok": self.control().map(|c| c.cancel()).is_some()})),
            "disk.status" => {
                let c = self.control();
                let mut v = json!({"scanning": c.is_some(), "paused": c.is_some_and(|c| c.is_paused())});
                if let Some(ix) = self.index() {
                    v["index"] = summary(&ix);
                }
                Ok(v)
            }
            "disk.children" => self.children(&a),
            "disk.search" => {
                let ix = self.index().ok_or("no index")?;
                let t = Instant::now();
                let q = parse_query(&a, &ix)?;
                let found = search(&ix, &q);
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                Ok(json!({"total": found.total, "ms": (ms * 100.0).round() / 100.0, "items": found.items.iter().map(|i| item(&ix, *i)).collect::<Vec<_>>()}))
            }
            "disk.save" => {
                let ix = self.index().ok_or("no index")?;
                let dir = self.dir(&a)?;
                let t = Instant::now();
                let (file, bytes) = store::save(&ix, &dir).map_err(|e| e.to_string())?;
                Ok(json!({"path": file.to_string_lossy(), "bytes": bytes, "ms": t.elapsed().as_millis() as u64}))
            }
            "disk.load" => {
                let dir = self.dir(&a)?;
                let t = Instant::now();
                match store::load(&dir).map_err(|e| e.to_string())? {
                    None => Ok(json!({"loaded": false})),
                    Some(ix) => {
                        let mut v = summary(&ix);
                        v["loaded"] = json!(true);
                        v["loadMs"] = json!(t.elapsed().as_millis() as u64);
                        self.set_index(Some(ix));
                        Ok(v)
                    }
                }
            }
            "disk.clear" => {
                self.set_index(None);
                Ok(json!({"ok": true}))
            }
            "disk.forget" => self.forget(&a),
            "disk.sizes" => self.sizes(&a),
            "disk.mft" => self.start_mft(&a),
            "disk.dupes" => self.start_dupes(&a),
            "disk.volumes" => Ok(Value::Array(volumes::list().iter().map(|v| v.to_json()).collect())),
            _ => Err(format!("unknown channel {channel}")),
        }
    }

    fn forget(&self, a: &Value) -> Result<Value, String> {
        let paths = strings(a, "paths");
        let mut slot = self.index.write().unwrap();
        let ix = slot.clone().ok_or("no index")?;
        let Some((next, removed)) = ix.forget(&paths) else {
            return Ok(json!({"removed": 0, "freed": 0, "entries": ix.len()}));
        };
        let freed: u64 = removed.iter().map(|i| ix.nodes[*i as usize].size).sum();
        let entries = next.len();
        *slot = Some(Arc::new(next));
        Ok(json!({"removed": removed.len(), "freed": freed, "entries": entries}))
    }

    fn sizes(&self, a: &Value) -> Result<Value, String> {
        let ix = self.index().ok_or("no index")?;
        let mut sizes = serde_json::Map::new();
        for p in strings(a, "paths").into_iter().take(SIZES_MAX) {
            let v = ix.find(&p).map_or(Value::Null, |n| json!(ix.nodes[n as usize].size));
            sizes.insert(p, v);
        }
        Ok(json!({"version": format!("{}:{}", ix.stats.scanned_at, ix.len()), "sizes": sizes}))
    }

    fn children(&self, a: &Value) -> Result<Value, String> {
        let ix = self.index().ok_or("no index")?;
        let sort = Sort::parse(s(a, "sort").unwrap_or("size"));
        let limit = u(a, "limit").map(|l| l as usize).unwrap_or(500).min(LIMIT_MAX);
        let path = s(a, "path").unwrap_or("");
        let (node, mut list) = if path.is_empty() {
            (None, ix.roots.clone())
        } else {
            let n = ix.find(path).ok_or_else(|| format!("not in index: {path}"))?;
            (Some(n), ix.children(n).collect::<Vec<_>>())
        };
        let total = list.len();
        ix.sorted(&mut list, sort);
        list.truncate(limit);
        let size = match node {
            Some(n) => ix.nodes[n as usize].size,
            None => ix.stats.bytes,
        };
        Ok(json!({"path": node.map(|n| ix.path(n)), "size": size, "total": total, "items": list.iter().map(|i| item(&ix, *i)).collect::<Vec<_>>()}))
    }

    fn start_scan(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let roots = strings(a, "roots");
        if roots.is_empty() {
            return Err("roots required".into());
        }
        let mut opts = ScanOptions::new(roots);
        opts.level = Level::parse(s(a, "level").unwrap_or("balanced"));
        opts.excludes = strings(a, "excludes");
        opts.cross_mounts = b(a, "crossMounts").unwrap_or(false);
        if let Some(h) = b(a, "hardlinks") {
            opts.hardlinks = h;
        }
        let merge = b(a, "merge").unwrap_or(true);
        let save = b(a, "save").unwrap_or(false);
        let control = Arc::new(Control::default());
        {
            let mut slot = self.scan.lock().unwrap();
            if slot.is_some() {
                return Err("busy".into());
            }
            *slot = Some(control.clone());
        }
        let threads = opts.level.threads();
        let level = opts.level.name();
        let me = self.clone();
        std::thread::spawn(move || me.run_scan(opts, control, merge, save));
        Ok(json!({"started": true, "threads": threads, "level": level}))
    }

    fn run_scan(self: Arc<Self>, opts: ScanOptions, control: Arc<Control>, merge: bool, save: bool) {
        let payload = catch_unwind(AssertUnwindSafe(|| self.scan_payload(&opts, &control, merge, save))).unwrap_or_else(|_| json!({"ok": false, "error": "scan panicked"}));
        *self.scan.lock().unwrap_or_else(|e| e.into_inner()) = None;
        (self.emit)(EVENT_DONE, payload);
    }

    fn scan_payload(&self, opts: &ScanOptions, control: &Arc<Control>, merge: bool, save: bool) -> Value {
        let progress = Arc::new(Progress::default());
        let started = Instant::now();
        let snap = progress.clone();
        let reporter = Reporter::start(Box::new(move || snap.snapshot()), control.clone(), self.emit.clone(), started, EVENT_PROGRESS);
        let r = scan_with(opts, control, &progress, self.lister);
        drop(reporter);
        match r {
            Ok(ix) => self.install(ix, merge, save, started),
            Err(ScanError::Cancelled) => json!({"ok": false, "cancelled": true}),
            Err(e) => json!({"ok": false, "error": e.to_string()}),
        }
    }

    fn install(&self, ix: Index, merge: bool, save: bool, started: Instant) -> Value {
        let ix = match (merge, self.index()) {
            (true, Some(old)) if !Index::can_merge(&old, &ix) => return json!({"ok": false, "error": "index too large to merge"}),
            (true, Some(old)) => Index::merge(&old, ix),
            _ => ix,
        };
        let mut v = summary(&ix);
        v["ok"] = json!(true);
        v["scanMs"] = json!(started.elapsed().as_millis() as u64);
        if save {
            match self.dir(&Value::Null).and_then(|d| store::save(&ix, &d).map_err(|e| e.to_string())) {
                Ok((file, bytes)) => v["saved"] = json!({"path": file.to_string_lossy(), "bytes": bytes}),
                Err(e) => v["saveError"] = json!(e),
            }
        }
        self.set_index(Some(ix));
        v
    }

    fn start_dupes(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let ix = self.index().ok_or("no index")?;
        let mut opts = dupes::Options::new(strings(a, "under"));
        if opts.under.is_empty() {
            opts.under = ix.root_paths();
        }
        opts.min_size = u(a, "minSize").unwrap_or(dupes::DEFAULT_MIN).max(1);
        opts.excludes = strings(a, "excludes");
        opts.level = Level::parse(s(a, "level").unwrap_or("balanced"));
        if !opts.under.iter().any(|p| ix.find(p).is_some()) {
            return Err(format!("not in index: {}", opts.under.first().cloned().unwrap_or_default()));
        }
        let control = Arc::new(Control::default());
        {
            let mut slot = self.scan.lock().unwrap();
            if slot.is_some() {
                return Err("busy".into());
            }
            *slot = Some(control.clone());
        }
        let threads = opts.level.threads();
        let level = opts.level.name();
        let me = self.clone();
        std::thread::spawn(move || me.run_dupes(ix, opts, control));
        Ok(json!({"started": true, "threads": threads, "level": level}))
    }

    fn run_dupes(self: Arc<Self>, ix: Arc<Index>, opts: dupes::Options, control: Arc<Control>) {
        let progress = Arc::new(DupeProgress::default());
        let started = Instant::now();
        let snap = progress.clone();
        let reporter = Reporter::start(Box::new(move || snap.snapshot()), control.clone(), self.emit.clone(), started, EVENT_DUPES_PROGRESS);
        let r = catch_unwind(AssertUnwindSafe(|| dupes::find(&ix, &opts, &control, &progress)));
        drop(reporter);
        let payload = match r {
            Ok(Ok(out)) => {
                let mut v = out.to_json();
                v["ok"] = json!(true);
                v
            }
            Ok(Err(dupes::Error::Cancelled)) => json!({"ok": false, "cancelled": true}),
            Ok(Err(e)) => json!({"ok": false, "error": e.to_string()}),
            Err(_) => json!({"ok": false, "error": "dupes panicked"}),
        };
        *self.scan.lock().unwrap_or_else(|e| e.into_inner()) = None;
        (self.emit)(EVENT_DUPES_DONE, payload);
    }

    fn start_mft(self: &Arc<Self>, a: &Value) -> Result<Value, String> {
        let drive = s(a, "drive").and_then(mft::drive_letter).ok_or("drive required")?;
        let fixture = self.mft_fixture.clone();
        if fixture.is_none() && !mft::supported(drive) {
            return Err(MftError::Unsupported.to_string());
        }
        let merge = b(a, "merge").unwrap_or(true);
        let save = b(a, "save").unwrap_or(false);
        let control = Arc::new(Control::default());
        {
            let mut slot = self.scan.lock().unwrap();
            if slot.is_some() {
                return Err("busy".into());
            }
            *slot = Some(control.clone());
        }
        let job = mft::Job { drive, fixture };
        let me = self.clone();
        std::thread::spawn(move || {
            let payload = catch_unwind(AssertUnwindSafe(|| me.mft_payload(&job, &control, merge, save))).unwrap_or_else(|_| json!({"ok": false, "error": "scan panicked"}));
            *me.scan.lock().unwrap_or_else(|e| e.into_inner()) = None;
            (me.emit)(EVENT_DONE, payload);
        });
        Ok(json!({"started": true, "mft": true, "root": mft::root_name(drive)}))
    }

    fn mft_payload(&self, job: &mft::Job, control: &Arc<Control>, merge: bool, save: bool) -> Value {
        let started = Instant::now();
        let root = mft::root_name(job.drive);
        let emit = self.emit.clone();
        let tick = |records: u64, total: u64| {
            emit(EVENT_PROGRESS, json!({"files": records, "dirs": 0, "bytes": 0, "denied": 0, "errors": 0, "current": root, "paused": false, "elapsedMs": started.elapsed().as_millis() as u64, "phase": "mft", "records": records, "total": total}));
        };
        match mft::run(job, control, &tick) {
            Ok(mut ix) => {
                ix.stats.elapsed_ms = started.elapsed().as_millis() as u64;
                ix.stats.scanned_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
                let mut v = self.install(ix, merge, save, started);
                v["mft"] = json!(true);
                v
            }
            Err(MftError::Cancelled) => json!({"ok": false, "cancelled": true, "mft": true}),
            Err(e) => json!({"ok": false, "error": e.to_string(), "mft": true}),
        }
    }
}

struct Reporter {
    done: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Reporter {
    fn start(snapshot: Box<dyn Fn() -> Value + Send>, control: Arc<Control>, emit: Emit, started: Instant, event: &'static str) -> Reporter {
        let done = Arc::new(AtomicBool::new(false));
        let flag = done.clone();
        let handle = std::thread::spawn(move || loop {
            let tick = Instant::now();
            while tick.elapsed() < TICK && !flag.load(Ordering::SeqCst) {
                std::thread::park_timeout(TICK.saturating_sub(tick.elapsed()));
            }
            if flag.load(Ordering::SeqCst) {
                break;
            }
            let mut v = snapshot();
            v["paused"] = json!(control.is_paused());
            v["elapsedMs"] = json!(started.elapsed().as_millis() as u64);
            emit(event, v);
        });
        Reporter { done, handle: Some(handle) }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            let _ = h.join();
        }
    }
}

pub struct Out(Mutex<Box<dyn Write + Send>>);

impl Out {
    pub fn new(w: Box<dyn Write + Send>) -> Self {
        Out(Mutex::new(w))
    }

    pub fn send(&self, v: &Value) {
        let mut line = serde_json::to_string(v).unwrap_or_default();
        line.push('\n');
        let mut o = self.0.lock().unwrap();
        let _ = o.write_all(line.as_bytes()).and_then(|_| o.flush());
    }
}

struct Inflight(Arc<AtomicUsize>);

impl Drop for Inflight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn read_line(input: &mut impl BufRead, buf: &mut Vec<u8>) -> bool {
    buf.clear();
    matches!(input.read_until(b'\n', buf), Ok(n) if n > 0)
}

pub fn serve(engine: Arc<Engine>, mut input: impl BufRead, out: Arc<Out>) -> i32 {
    out.send(&json!({"ready": true, "pid": std::process::id(), "channels": CHANNELS}));
    let inflight = Arc::new(AtomicUsize::new(0));
    let mut buf = Vec::new();
    while read_line(&mut input, &mut buf) {
        let line = String::from_utf8_lossy(&buf);
        let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&line) else { continue };
        if m.get("type").and_then(Value::as_str) == Some("shutdown") {
            break;
        }
        let Some(raw_id) = m.get("id").filter(|v| !v.is_null()) else { continue };
        let (Some(id), Some(channel)) = (raw_id.as_u64(), m.get("channel").and_then(Value::as_str)) else {
            let why = if raw_id.as_u64().is_none() { "id must be an unsigned integer" } else { "channel required" };
            out.send(&json!({"id": raw_id, "ok": false, "error": why}));
            continue;
        };
        let args = m.get("args").cloned().unwrap_or(Value::Null);
        let (engine, out, channel) = (engine.clone(), out.clone(), channel.to_string());
        inflight.fetch_add(1, Ordering::SeqCst);
        let ticket = Inflight(inflight.clone());
        std::thread::spawn(move || {
            let _ticket = ticket;
            let msg = match catch_unwind(AssertUnwindSafe(|| engine.dispatch(&channel, &args))) {
                Ok(Ok(value)) => json!({"id": id, "ok": true, "value": value}),
                Ok(Err(e)) => json!({"id": id, "ok": false, "error": e}),
                Err(_) => json!({"id": id, "ok": false, "error": "internal error"}),
            };
            out.send(&msg);
        });
    }
    if let Some(c) = engine.control() {
        c.cancel();
    }
    let deadline = Instant::now() + DRAIN;
    while (inflight.load(Ordering::SeqCst) > 0 || engine.scanning()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    0
}

pub fn run() -> i32 {
    let out = Arc::new(Out::new(Box::new(std::io::stdout())));
    let sink = out.clone();
    let engine = Engine::from_env(Arc::new(move |event, payload| sink.send(&json!({"event": event, "payload": payload}))));
    serve(engine, std::io::stdin().lock(), out)
}
