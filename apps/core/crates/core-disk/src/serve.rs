use crate::fold::{group_id, GROUP_NAMES};
use crate::index::{flag, Index, Sort};
use crate::search::{search, Only, Query};
use crate::walk::{scan, Control, Level, Progress, ScanError, ScanOptions};
use crate::{store, volumes};
use serde_json::{json, Map, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

pub const CHANNELS: [&str; 12] = ["disk.scan", "disk.pause", "disk.resume", "disk.cancel", "disk.status", "disk.children", "disk.search", "disk.save", "disk.load", "disk.volumes", "disk.clear", "disk.forget"];
pub const EVENT_PROGRESS: &str = "disk:progress";
pub const EVENT_DONE: &str = "disk:done";
const TICK: Duration = Duration::from_millis(100);
const LIMIT_MAX: usize = 10_000;

pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

pub struct Engine {
    index: RwLock<Option<Arc<Index>>>,
    scan: Mutex<Option<Arc<Control>>>,
    data_dir: Option<PathBuf>,
    emit: Emit,
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
        Arc::new(Engine { index: RwLock::new(None), scan: Mutex::new(None), data_dir, emit })
    }

    pub fn from_env(emit: Emit) -> Arc<Self> {
        let dir = std::env::var("CCM_DATA_DIR").ok().filter(|v| !v.is_empty()).map(|d| PathBuf::from(d).join("disk"));
        Self::new(dir, emit)
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
        let progress = Arc::new(Progress::default());
        let done = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let reporter = {
            let (progress, done, control, emit) = (progress.clone(), done.clone(), control.clone(), self.emit.clone());
            std::thread::spawn(move || loop {
                let tick = Instant::now();
                while tick.elapsed() < TICK && !done.load(Ordering::SeqCst) {
                    std::thread::park_timeout(TICK.saturating_sub(tick.elapsed()));
                }
                if done.load(Ordering::SeqCst) {
                    break;
                }
                let mut v = progress.snapshot();
                v["paused"] = json!(control.is_paused());
                v["elapsedMs"] = json!(started.elapsed().as_millis() as u64);
                emit(EVENT_PROGRESS, v);
            })
        };
        let r = scan(&opts, &control, &progress);
        done.store(true, Ordering::SeqCst);
        reporter.thread().unpark();
        let _ = reporter.join();
        let payload = match r {
            Ok(ix) => {
                let ix = match (merge, self.index()) {
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
            Err(ScanError::Cancelled) => json!({"ok": false, "cancelled": true}),
            Err(e) => json!({"ok": false, "error": e.to_string()}),
        };
        *self.scan.lock().unwrap() = None;
        (self.emit)(EVENT_DONE, payload);
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

pub fn serve(engine: Arc<Engine>, input: impl BufRead, out: Arc<Out>) -> i32 {
    out.send(&json!({"ready": true, "pid": std::process::id(), "channels": CHANNELS}));
    for line in input.lines() {
        let Ok(line) = line else { break };
        let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&line) else { continue };
        if m.get("type").and_then(Value::as_str) == Some("shutdown") {
            break;
        }
        let (Some(id), Some(channel)) = (m.get("id").and_then(Value::as_u64), m.get("channel").and_then(Value::as_str)) else { continue };
        let args = m.get("args").cloned().unwrap_or(Value::Null);
        let (engine, out, channel) = (engine.clone(), out.clone(), channel.to_string());
        std::thread::spawn(move || {
            let msg = match engine.dispatch(&channel, &args) {
                Ok(value) => json!({"id": id, "ok": true, "value": value}),
                Err(e) => json!({"id": id, "ok": false, "error": e}),
            };
            out.send(&msg);
        });
    }
    if let Some(c) = engine.control() {
        c.cancel();
    }
    0
}

pub fn run() -> i32 {
    let out = Arc::new(Out::new(Box::new(std::io::stdout())));
    let sink = out.clone();
    let engine = Engine::from_env(Arc::new(move |event, payload| sink.send(&json!({"event": event, "payload": payload}))));
    serve(engine, std::io::stdin().lock(), out)
}
