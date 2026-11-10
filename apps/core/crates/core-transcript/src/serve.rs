use crate::codex::serve::Rollouts;
use crate::index::{Options, UsageIndex};
use crate::jobs::{drain, full_scan, sweep, Shared, Sink};
use crate::omap::OMap;
use crate::query::scan_json;
use crate::sigserve::Handles;
use crate::tracker::serve::Trackers;
use crate::time::{now_ms, today};
use core_procs::lines::Lines;
use serde_json::{json, Map, Value};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub const DAYS_MAX: u64 = 3660;

pub fn write(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}").and_then(|_| out.flush());
}

struct Events;

impl Sink for Events {
    fn scan(&self, scan: Value) {
        write(&json!({ "event": "usage.scan", "scan": scan }));
    }
    fn reset(&self, snapshot: Value) {
        write(&json!({ "event": "usage.reset", "snapshot": snapshot }));
    }
    fn update(&self, buckets: Vec<Value>) {
        write(&json!({ "event": "usage.update", "buckets": buckets }));
    }
}

pub enum Job {
    Scan,
    Notify(Vec<String>),
    Idle(Value),
}

fn failed(id: &Value, kind: &str) -> Value {
    json!({ "id": id, "ok": false, "error": kind })
}

fn ok(id: &Value, extra: Map<String, Value>) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), id.clone());
    m.insert("ok".into(), true.into());
    m.extend(extra);
    Value::Object(m)
}

fn ms(m: &Map<String, Value>, key: &str, default: u64) -> u64 {
    m.get(key).and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0).map_or(default, |v| v as u64)
}

pub fn options_of(m: &Map<String, Value>) -> Options {
    let d = Options::default();
    Options {
        sweep_ms: ms(m, "sweepMs", d.sweep_ms),
        file_debounce_ms: ms(m, "fileDebounceMs", d.file_debounce_ms),
        save_debounce_ms: ms(m, "saveDebounceMs", d.save_debounce_ms),
        id_retention_ms: m.get("idRetentionMs").and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0).unwrap_or(d.id_retention_ms),
    }
}

fn days_of(m: &Map<String, Value>) -> Option<u32> {
    let d = m.get("days")?.as_f64()?;
    (d.fract() == 0.0 && d >= 0.0 && d <= DAYS_MAX as f64).then_some(d as u32)
}

pub fn query(idx: &UsageIndex, m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
    let day = m.get("today").and_then(Value::as_str).map_or_else(|| today(now_ms()), str::to_owned);
    let mut out = Map::new();
    match m.get("q").and_then(Value::as_str) {
        Some("heatmap") => {
            out.insert("heatmap".into(), idx.heatmap(days_of(m).ok_or("days")?, &day));
        }
        Some("toolStats") => {
            let project = match m.get("project") {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(s.as_str()),
                Some(_) => return Err("project"),
            };
            out.insert("toolStats".into(), idx.tool_stats(days_of(m).ok_or("days")?, project, &day));
        }
        Some("scanInfo") => {
            out.insert("scan".into(), scan_json(&idx.scan));
        }
        Some("snapshot") => {
            out.insert("snapshot".into(), idx.snapshot_json());
        }
        _ => return Err("q"),
    }
    Ok(out)
}

struct Worker {
    sh: Arc<Shared>,
    rx: mpsc::Receiver<Job>,
    timers: OMap<Instant>,
    pending: OMap<()>,
    queue: VecDeque<Job>,
    next_sweep: Option<Instant>,
    save_at: Option<Instant>,
    opts: Options,
    has_cache: bool,
}

enum Work {
    Scan,
    Drain,
    Sweep,
    Idle(Value),
}

impl Worker {
    fn notify(&mut self, files: Vec<String>) {
        if self.sh.stopped() {
            return;
        }
        let due = Instant::now() + Duration::from_millis(self.opts.file_debounce_ms);
        for rel in files {
            if rel.ends_with(".jsonl") && !self.timers.contains(&rel) {
                self.timers.insert(&rel, due);
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.timers.iter().map(|(_, t)| *t).chain(self.next_sweep).chain(self.save_at).min()
    }

    fn fire(&mut self, work: &mut VecDeque<Work>) {
        let now = Instant::now();
        let due: Vec<String> = self.timers.iter().filter(|(_, t)| **t <= now).map(|(k, _)| k.to_owned()).collect();
        for rel in &due {
            self.timers.remove(rel);
            self.pending.insert(rel, ());
            work.push_back(Work::Drain);
        }
        if self.next_sweep.is_some_and(|t| t <= now) {
            work.push_back(Work::Sweep);
            self.next_sweep = Some(now + Duration::from_millis(self.opts.sweep_ms));
        }
        if self.save_at.is_some_and(|t| t <= now) {
            self.save_at = None;
            self.sh.lock().save();
        }
    }

    fn schedule_save(&mut self) {
        if self.save_at.is_none() && self.has_cache {
            self.save_at = Some(Instant::now() + Duration::from_millis(self.opts.save_debounce_ms));
        }
    }

    fn run_one(&mut self, w: Work) {
        let result = match w {
            Work::Scan => full_scan(&self.sh, &Events).map(|done| {
                if done {
                    self.save_at = None;
                }
                false
            }),
            Work::Drain => {
                let files: Vec<String> = self.pending.iter().map(|(k, _)| k.to_owned()).collect();
                self.pending.clear();
                drain(&self.sh, files, &Events)
            }
            Work::Sweep => sweep(&self.sh, &Events),
            Work::Idle(id) => {
                write(&ok(&id, Map::new()));
                Ok(false)
            }
        };
        match result {
            Ok(true) => self.schedule_save(),
            Ok(false) => {}
            Err(e) => eprintln!("[collector] usage index: {e}"),
        }
    }

    fn run(mut self) {
        let mut work: VecDeque<Work> = VecDeque::new();
        if self.opts.sweep_ms > 0 {
            self.next_sweep = Some(Instant::now() + Duration::from_millis(self.opts.sweep_ms));
        }
        loop {
            if self.sh.stopped() {
                return;
            }
            let got = if !work.is_empty() || !self.queue.is_empty() {
                self.rx.try_recv().map_err(|e| matches!(e, mpsc::TryRecvError::Disconnected))
            } else if let Some(t) = self.next_deadline() {
                self.rx.recv_timeout(t.saturating_duration_since(Instant::now())).map_err(|e| matches!(e, mpsc::RecvTimeoutError::Disconnected))
            } else {
                self.rx.recv().map_err(|_| true)
            };
            let mut received = Vec::new();
            match got {
                Ok(job) => received.push(job),
                Err(true) => return,
                Err(false) => {}
            }
            received.extend(self.rx.try_iter());
            for job in received {
                match job {
                    Job::Notify(files) => self.notify(files),
                    other => self.queue.push_back(other),
                }
            }
            self.fire(&mut work);
            while let Some(job) = self.queue.pop_front() {
                work.push_back(match job {
                    Job::Scan => Work::Scan,
                    Job::Idle(id) => Work::Idle(id),
                    Job::Notify(_) => continue,
                });
            }
            if let Some(w) = work.pop_front() {
                self.run_one(w);
            }
        }
    }
}

fn str_list(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_owned)).take(4096).collect(),
        _ => Vec::new(),
    }
}

struct Open {
    sh: Arc<Shared>,
    tx: mpsc::Sender<Job>,
    worker: std::thread::JoinHandle<()>,
}

fn open(m: &Map<String, Value>) -> Result<(Open, Value), &'static str> {
    let dir = m.get("projectsDir").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("projectsDir")?;
    let cache = match m.get("cacheFile") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() => Some(PathBuf::from(s)),
        Some(_) => return Err("cacheFile"),
    };
    let opts = options_of(m);
    let has_cache = cache.is_some();
    let mut idx = UsageIndex::new(PathBuf::from(dir), cache, opts.clone());
    idx.load_cache();
    let snapshot = idx.snapshot_json();
    let sh = Arc::new(Shared::new(idx));
    let (tx, rx) = mpsc::channel();
    let w = Worker { sh: sh.clone(), rx, timers: OMap::new(), pending: OMap::new(), queue: VecDeque::new(), next_sweep: None, save_at: None, opts, has_cache };
    let worker = std::thread::Builder::new().name("usage-index".into()).spawn(move || w.run()).map_err(|_| "spawn")?;
    Ok((Open { sh, tx, worker }, snapshot))
}

fn stop(o: &Open) {
    o.sh.stopped.store(true, Ordering::SeqCst);
    o.sh.lock().save();
}

fn handle(m: &Map<String, Value>, state: &mut Option<Open>) -> Option<Value> {
    let id = m.get("id").filter(|v| v.is_number() || v.is_string()).cloned();
    let op = m.get("op").and_then(Value::as_str);
    if op == Some("usage.notify") {
        if let Some(o) = state.as_ref() {
            let _ = o.tx.send(Job::Notify(str_list(m.get("files"))));
        }
        return id.map(|id| ok(&id, Map::new()));
    }
    let id = id?;
    let reply = match (op, state.as_ref()) {
        (Some("ping"), _) => ok(&id, [("version".to_owned(), Value::from(crate::VERSION))].into_iter().collect()),
        (Some("usage.scan"), Some(_)) => failed(&id, "open"),
        (Some("usage.scan"), None) => match open(m) {
            Ok((o, snapshot)) => {
                write(&ok(&id, [("snapshot".to_owned(), snapshot)].into_iter().collect()));
                let _ = o.tx.send(Job::Scan);
                *state = Some(o);
                return None;
            }
            Err(kind) => failed(&id, kind),
        },
        (Some("usage.query" | "usage.idle" | "usage.stop"), None) => failed(&id, "closed"),
        (Some("usage.query"), Some(o)) => match query(&o.sh.lock(), m) {
            Ok(extra) => ok(&id, extra),
            Err(kind) => failed(&id, kind),
        },
        (Some("usage.idle"), Some(o)) => {
            if o.tx.send(Job::Idle(id.clone())).is_ok() && !o.sh.stopped() {
                return None;
            }
            ok(&id, Map::new())
        }
        (Some("usage.stop"), Some(o)) => {
            stop(o);
            ok(&id, Map::new())
        }
        _ => failed(&id, "op"),
    };
    Some(reply)
}

fn signal_op(m: &Map<String, Value>, signals: &mut Handles) -> Option<Option<Value>> {
    let op = m.get("op").and_then(Value::as_str).filter(|o| o.starts_with("signals."))?;
    let Some(id) = m.get("id").filter(|v| v.is_number() || v.is_string()) else { return Some(None) };
    Some(Some(match signals.handle(op, m) {
        Some(Ok(extra)) => ok(id, extra),
        Some(Err(kind)) => failed(id, kind),
        None => failed(id, "op"),
    }))
}

fn codex_op(m: &Map<String, Value>, rollouts: &mut Rollouts) -> Option<Option<Value>> {
    let op = m.get("op").and_then(Value::as_str).filter(|o| o.starts_with("codex."))?;
    let id = m.get("id").filter(|v| v.is_number() || v.is_string());
    if id.is_none() && op != "codex.close" {
        return Some(None);
    }
    let reply = rollouts.handle(op, m);
    let Some(id) = id else { return Some(None) };
    Some(Some(match reply {
        Some(Ok(extra)) => ok(id, extra),
        Some(Err(kind)) => failed(id, kind),
        None => failed(id, "op"),
    }))
}

fn tracker_op(m: &Map<String, Value>, trackers: &mut Trackers) -> Option<Option<Value>> {
    let op = m.get("op").and_then(Value::as_str).filter(|o| o.starts_with("tracker."))?;
    let id = m.get("id").filter(|v| v.is_number() || v.is_string());
    if id.is_none() && !matches!(op, "tracker.close" | "tracker.closeSpans") {
        return Some(None);
    }
    let reply = trackers.handle(op, m);
    let Some(id) = id else { return Some(None) };
    Some(Some(match reply {
        Some(Ok(extra)) => ok(id, extra),
        Some(Err(kind)) => failed(id, kind),
        None => failed(id, "op"),
    }))
}

pub fn run() -> i32 {
    let mut state: Option<Open> = None;
    let mut signals = Handles::default();
    let mut trackers = Trackers::default();
    let mut rollouts = Rollouts::default();
    let mut lines = Lines::new(std::io::stdin().lock());
    while let Some(line) = lines.next_line() {
        let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&line) else { continue };
        if let Some(reply) = codex_op(&m, &mut rollouts) {
            if let Some(reply) = reply {
                write(&reply);
            }
            continue;
        }
        if let Some(reply) = tracker_op(&m, &mut trackers) {
            if let Some(reply) = reply {
                write(&reply);
            }
            continue;
        }
        if let Some(reply) = signal_op(&m, &mut signals) {
            if let Some(reply) = reply {
                write(&reply);
            }
            continue;
        }
        if let Some(reply) = handle(&m, &mut state) {
            write(&reply);
        }
    }
    if let Some(o) = state {
        stop(&o);
        drop(o.tx);
        let _ = o.worker.join();
    }
    0
}
