use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use core_store::extra::{UsageBucket, UsageScan, UsageSnapshot};
use core_transcript::index::{Options, UsageIndex};
use core_transcript::jobs::{drain, full_scan, sweep, Shared, Sink};
use core_transcript::omap::OMap;
use core_transcript::query::scan_json;
use core_transcript::time::{now_ms, today};
use serde_json::Value;

pub enum UsageEvent {
    Scan(UsageScan),
    Reset(UsageSnapshot),
    Update(Vec<UsageBucket>),
}

struct ChannelSink {
    tx: Sender<UsageEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

fn snapshot_of(v: &Value) -> Option<UsageSnapshot> {
    let buckets = v.get("buckets")?.as_array()?.iter().filter_map(|b| serde_json::from_value(b.clone()).ok()).collect();
    let scan = serde_json::from_value(v.get("scan")?.clone()).ok()?;
    Some(UsageSnapshot { buckets, scan, roots: None })
}

impl Sink for ChannelSink {
    fn scan(&self, scan: Value) {
        if let Ok(s) = serde_json::from_value(scan) {
            let _ = self.tx.send(UsageEvent::Scan(s));
            (self.wake)();
        }
    }
    fn reset(&self, snapshot: Value) {
        if let Some(s) = snapshot_of(&snapshot) {
            let _ = self.tx.send(UsageEvent::Reset(s));
            (self.wake)();
        }
    }
    fn update(&self, buckets: Vec<Value>) {
        let list: Vec<UsageBucket> = buckets.into_iter().filter_map(|b| serde_json::from_value(b).ok()).collect();
        let _ = self.tx.send(UsageEvent::Update(list));
        (self.wake)();
    }
}

enum Job {
    Scan,
    Notify(Vec<String>),
}

pub struct Usage {
    sh: Arc<Shared>,
    tx: Sender<Job>,
    pub events: Receiver<UsageEvent>,
    worker: Option<std::thread::JoinHandle<()>>,
}

struct Worker {
    sh: Arc<Shared>,
    sink: ChannelSink,
    rx: Receiver<Job>,
    timers: OMap<Instant>,
    pending: OMap<()>,
    next_sweep: Option<Instant>,
    save_at: Option<Instant>,
    opts: Options,
    has_cache: bool,
}

impl Worker {
    fn next_deadline(&self) -> Option<Instant> {
        self.timers.iter().map(|(_, t)| *t).chain(self.next_sweep).chain(self.save_at).min()
    }

    fn note(&mut self, result: std::io::Result<bool>) {
        match result {
            Ok(true) if self.save_at.is_none() && self.has_cache => self.save_at = Some(Instant::now() + Duration::from_millis(self.opts.save_debounce_ms)),
            Ok(_) => {}
            Err(e) => eprintln!("[collector] usage index: {e}"),
        }
    }

    fn run(mut self) {
        if self.opts.sweep_ms > 0 {
            self.next_sweep = Some(Instant::now() + Duration::from_millis(self.opts.sweep_ms));
        }
        let mut queue: VecDeque<Job> = VecDeque::new();
        loop {
            if self.sh.stopped() {
                return;
            }
            let got = match self.next_deadline() {
                _ if !queue.is_empty() => self.rx.try_recv().map_err(|e| matches!(e, TryRecvError::Disconnected)),
                Some(t) => self.rx.recv_timeout(t.saturating_duration_since(Instant::now())).map_err(|e| matches!(e, RecvTimeoutError::Disconnected)),
                None => self.rx.recv().map_err(|_| true),
            };
            match got {
                Ok(job) => queue.push_back(job),
                Err(true) => return,
                Err(false) => {}
            }
            queue.extend(self.rx.try_iter());
            let mut work = Vec::new();
            for job in queue.drain(..) {
                match job {
                    Job::Notify(files) => {
                        let due = Instant::now() + Duration::from_millis(self.opts.file_debounce_ms);
                        for rel in files.into_iter().filter(|r| r.ends_with(".jsonl")) {
                            if !self.timers.contains(&rel) {
                                self.timers.insert(&rel, due);
                            }
                        }
                    }
                    Job::Scan => work.push(0),
                }
            }
            let now = Instant::now();
            let due: Vec<String> = self.timers.iter().filter(|(_, t)| **t <= now).map(|(k, _)| k.to_owned()).collect();
            for rel in &due {
                self.timers.remove(rel);
                self.pending.insert(rel, ());
            }
            if !due.is_empty() {
                work.push(1);
            }
            if self.next_sweep.is_some_and(|t| t <= now) {
                work.push(2);
                self.next_sweep = Some(now + Duration::from_millis(self.opts.sweep_ms));
            }
            if self.save_at.is_some_and(|t| t <= now) {
                self.save_at = None;
                self.sh.lock().save();
            }
            for w in work {
                let r = match w {
                    0 => full_scan(&self.sh, &self.sink).map(|done| {
                        if done {
                            self.save_at = None;
                        }
                        false
                    }),
                    1 => {
                        let files: Vec<String> = self.pending.iter().map(|(k, _)| k.to_owned()).collect();
                        self.pending.clear();
                        drain(&self.sh, files, &self.sink)
                    }
                    _ => sweep(&self.sh, &self.sink),
                };
                self.note(r);
            }
        }
    }
}

impl Usage {
    pub fn open(projects_dir: PathBuf, cache_file: Option<PathBuf>, wake: Arc<dyn Fn() + Send + Sync>) -> (Usage, Option<UsageSnapshot>) {
        let opts = Options::default();
        let has_cache = cache_file.is_some();
        let mut idx = UsageIndex::new(projects_dir, cache_file, opts.clone());
        idx.load_cache();
        let snapshot = snapshot_of(&idx.snapshot_json());
        let sh = Arc::new(Shared::new(idx));
        let (tx, rx) = channel();
        let (etx, events) = channel();
        let w = Worker { sh: sh.clone(), sink: ChannelSink { tx: etx, wake }, rx, timers: OMap::new(), pending: OMap::new(), next_sweep: None, save_at: None, opts, has_cache };
        let worker = std::thread::Builder::new().name("usage-index".into()).spawn(move || w.run()).ok();
        let _ = tx.send(Job::Scan);
        (Usage { sh, tx, events, worker }, snapshot)
    }

    pub fn notify(&self, rel: &str) {
        if rel.ends_with(".jsonl") {
            let _ = self.tx.send(Job::Notify(vec![rel.replace('\\', "/")]));
        }
    }

    pub fn scan_info(&self) -> Option<UsageScan> {
        serde_json::from_value(scan_json(&self.sh.lock().scan)).ok()
    }

    pub fn heatmap(&self, days: u32) -> Value {
        self.sh.lock().heatmap(days, &today(now_ms()))
    }

    pub fn tool_stats(&self, days: u32, project: Option<&str>) -> Value {
        self.sh.lock().tool_stats(days, project, &today(now_ms()))
    }

    pub fn stop(&mut self) {
        self.sh.stopped.store(true, Ordering::SeqCst);
        self.sh.lock().save();
        let (tx, _) = channel();
        drop(std::mem::replace(&mut self.tx, tx));
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
