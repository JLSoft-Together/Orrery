use crate::index::{meta_of, Meta, ScanState, UsageIndex};
use crate::time::now_ms;
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub trait Sink {
    fn scan(&self, scan: Value);
    fn reset(&self, snapshot: Value);
    fn update(&self, buckets: Vec<Value>);
}

pub struct Shared {
    pub idx: Mutex<UsageIndex>,
    pub stopped: AtomicBool,
}

impl Shared {
    pub fn new(idx: UsageIndex) -> Shared {
        Shared { idx: Mutex::new(idx), stopped: AtomicBool::new(false) }
    }

    pub fn lock(&self) -> MutexGuard<'_, UsageIndex> {
        self.idx.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

pub fn read_dir_sorted(dir: &Path) -> Option<Vec<std::fs::DirEntry>> {
    #[allow(unused_mut)]
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    #[cfg(unix)]
    entries.sort_by(|a, b| a.file_name().as_encoded_bytes().cmp(b.file_name().as_encoded_bytes()));
    Some(entries)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Some(entries) = read_dir_sorted(dir) else { return };
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
        if is_dir {
            walk(&e.path(), &rel, out);
        } else if name.ends_with(".jsonl") {
            out.push(rel);
        }
    }
}

pub fn list_jsonl(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out
}

fn stat_all(dir: &Path, files: &[String]) -> Vec<Option<Meta>> {
    files.iter().map(|rel| meta_of(&dir.join(rel))).collect()
}

pub fn full_scan(sh: &Shared, sink: &dyn Sink) -> std::io::Result<bool> {
    let dir = sh.lock().projects_dir.clone();
    let files = list_jsonl(&dir);
    let scan = {
        let mut idx = sh.lock();
        idx.mark_gone(&files);
        idx.scan = crate::index::Scan { state: ScanState::Scanning, files_done: 0, files_total: files.len() };
        crate::query::scan_json(&idx.scan)
    };
    sink.scan(scan);
    let stats = stat_all(&dir, &files);
    let mut last = Instant::now();
    for (rel, st) in files.iter().zip(stats) {
        if sh.stopped() {
            return Ok(false);
        }
        let report = {
            let mut idx = sh.lock();
            idx.ingest(rel, st)?;
            idx.scan.files_done += 1;
            (last.elapsed() > Duration::from_millis(400)).then(|| crate::query::scan_json(&idx.scan))
        };
        if let Some(s) = report {
            last = Instant::now();
            sink.scan(s);
        }
    }
    let snapshot = {
        let mut idx = sh.lock();
        idx.scan = crate::index::Scan { state: ScanState::Ready, files_done: files.len(), files_total: files.len() };
        idx.dirty.clear();
        idx.prune_old_ids(now_ms() as f64);
        idx.snapshot_json()
    };
    sink.reset(snapshot);
    let mut idx = sh.lock();
    idx.save_needed = true;
    idx.save();
    Ok(true)
}

fn flush_dirty(idx: &mut UsageIndex, sink: &dyn Sink) -> bool {
    if idx.dirty.is_empty() {
        return false;
    }
    let changed = idx.take_dirty();
    sink.update(changed);
    idx.save_needed = true;
    true
}

pub fn sweep(sh: &Shared, sink: &dyn Sink) -> std::io::Result<bool> {
    if sh.lock().scan.state != ScanState::Ready {
        return Ok(false);
    }
    let dir = sh.lock().projects_dir.clone();
    let files = list_jsonl(&dir);
    sh.lock().mark_gone(&files);
    let stats = stat_all(&dir, &files);
    for (rel, st) in files.iter().zip(stats) {
        let Some(st) = st else { continue };
        let mut idx = sh.lock();
        let stale = idx.files.get(rel).is_none_or(|e| e.gone || e.size != st.size || e.mtime_ms != Some(st.mtime_ms));
        if stale {
            idx.ingest(rel, Some(st))?;
        }
    }
    let mut idx = sh.lock();
    idx.prune_old_ids(now_ms() as f64);
    Ok(flush_dirty(&mut idx, sink))
}

pub fn drain(sh: &Shared, files: Vec<String>, sink: &dyn Sink) -> std::io::Result<bool> {
    for rel in &files {
        sh.lock().ingest(rel, None)?;
    }
    let mut idx = sh.lock();
    if idx.scan.state != ScanState::Ready {
        return Ok(false);
    }
    Ok(flush_dirty(&mut idx, sink))
}
