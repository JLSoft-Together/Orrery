#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod win;

#[cfg(unix)]
pub use unix as sys;
#[cfg(windows)]
pub use win as sys;

use crate::fold::glob_ci;
use crate::index::{clean_path, flag, under, Builder, Index, Stats, NONE};
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Light,
    Balanced,
    Max,
}

impl Level {
    pub fn parse(s: &str) -> Level {
        match s {
            "light" => Level::Light,
            "max" => Level::Max,
            _ => Level::Balanced,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Light => "light",
            Level::Balanced => "balanced",
            Level::Max => "max",
        }
    }

    pub fn threads(self) -> usize {
        let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        match self {
            Level::Light => 2,
            Level::Balanced => (n / 2).max(2),
            Level::Max => n.max(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
    Link,
}

#[derive(Debug, Clone, Copy)]
pub struct Raw {
    pub name_start: u32,
    pub name_len: u32,
    pub kind: Kind,
    pub flags: u16,
    pub len: u64,
    pub alloc: u64,
    pub mtime: u32,
    pub dev: u64,
    pub ino: u64,
    pub nlink: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListError {
    Denied,
    Missing,
    Other,
}

pub struct DirCtx {
    pub hardlinks: bool,
    pub cluster: u64,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub roots: Vec<String>,
    pub level: Level,
    pub excludes: Vec<String>,
    pub cross_mounts: bool,
    pub hardlinks: bool,
    pub threads: Option<usize>,
}

impl ScanOptions {
    pub fn new(roots: Vec<String>) -> Self {
        ScanOptions { roots, level: Level::Balanced, excludes: Vec::new(), cross_mounts: false, hardlinks: cfg!(unix), threads: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    Cancelled,
    NoRoots,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ScanError::Cancelled => "cancelled",
            ScanError::NoRoots => "no readable root",
        })
    }
}

#[derive(Default)]
pub struct Control {
    cancel: AtomicBool,
    paused: Mutex<bool>,
    cv: Condvar,
}

impl Control {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        let _g = self.paused.lock().unwrap();
        self.cv.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn pause(&self) {
        *self.paused.lock().unwrap() = true;
    }

    pub fn resume(&self) {
        *self.paused.lock().unwrap() = false;
        self.cv.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    fn wait_if_paused(&self) {
        let mut p = self.paused.lock().unwrap();
        while *p && !self.is_cancelled() {
            p = self.cv.wait(p).unwrap();
        }
    }
}

#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub denied: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<String>,
}

impl Progress {
    pub fn snapshot(&self) -> Value {
        json!({
            "files": self.files.load(Ordering::Relaxed),
            "dirs": self.dirs.load(Ordering::Relaxed),
            "bytes": self.bytes.load(Ordering::Relaxed),
            "denied": self.denied.load(Ordering::Relaxed),
            "errors": self.errors.load(Ordering::Relaxed),
            "current": self.current.lock().unwrap().clone(),
        })
    }
}

struct Excludes {
    names: Vec<Vec<u8>>,
    paths: Vec<String>,
}

impl Excludes {
    fn new(list: &[String]) -> Self {
        let mut names = Vec::new();
        let mut paths = Vec::new();
        for e in list.iter().map(|e| e.trim()).filter(|e| !e.is_empty()) {
            if e.contains('/') || (cfg!(windows) && e.contains('\\')) {
                paths.push(clean_path(e));
            } else {
                names.push(e.as_bytes().to_vec());
            }
        }
        Excludes { names, paths }
    }

    fn name(&self, name: &[u8]) -> bool {
        self.names.iter().any(|p| if cfg!(any(windows, target_os = "macos")) { glob_ci(name, p) } else { glob_exact(name, p) })
    }

    fn path(&self, display: &str) -> bool {
        self.paths.iter().any(|p| under(display, p))
    }
}

fn glob_exact(name: &[u8], pat: &[u8]) -> bool {
    if !pat.contains(&b'*') && !pat.contains(&b'?') {
        return name == pat;
    }
    glob_ci(name, pat)
}

pub struct Item {
    pub name_start: u32,
    pub name_len: u32,
    pub flags: u16,
    pub len: u64,
    pub alloc: u64,
    pub mtime: u32,
    pub dir_id: u32,
}

pub struct Listing {
    pub id: u32,
    pub names: String,
    pub items: Vec<Item>,
    pub flags: u16,
}

struct Job {
    id: u32,
    path: PathBuf,
    dev: u64,
    cluster: u64,
}

struct Queue {
    jobs: Vec<Job>,
    busy: usize,
    done: bool,
}

struct Shared<'a> {
    q: Mutex<Queue>,
    cv: Condvar,
    next_id: AtomicU32,
    seen: Mutex<HashSet<(u64, u64)>>,
    excludes: Excludes,
    threads: usize,
    opts: &'a ScanOptions,
    control: &'a Control,
    progress: &'a Progress,
}

struct Root {
    display: String,
    path: PathBuf,
    dev: u64,
    mtime: u32,
}

fn pick_roots(list: &[String]) -> Vec<Root> {
    let mut out: Vec<Root> = Vec::new();
    let mut cleaned: Vec<(String, PathBuf)> = list.iter().filter_map(|r| sys::normalize_root(r)).collect();
    cleaned.sort_by_key(|(d, _)| d.len());
    for (display, path) in cleaned {
        if out.iter().any(|r| under(&display, &r.display)) {
            continue;
        }
        let Some((dev, mtime)) = sys::root_meta(&path) else { continue };
        out.push(Root { display, path, dev, mtime });
    }
    out
}

fn worker(sh: &Shared) -> Vec<Listing> {
    sys::lower_priority(sh.opts.level);
    let mut out = Vec::new();
    let mut raw: Vec<Raw> = Vec::new();
    let mut tick = 0u32;
    loop {
        sh.control.wait_if_paused();
        let job = {
            let mut q = sh.q.lock().unwrap();
            loop {
                if q.done || sh.control.is_cancelled() {
                    q.done = true;
                    sh.cv.notify_all();
                    return out;
                }
                if let Some(j) = q.jobs.pop() {
                    q.busy += 1;
                    break j;
                }
                if q.busy == 0 {
                    q.done = true;
                    sh.cv.notify_all();
                    return out;
                }
                q = sh.cv.wait(q).unwrap();
            }
        };
        let mut listing = Listing { id: job.id, names: String::new(), items: Vec::new(), flags: 0 };
        raw.clear();
        let ctx = DirCtx { hardlinks: sh.opts.hardlinks, cluster: job.cluster };
        if let Err(e) = sys::list(&job.path, &ctx, &mut listing.names, &mut raw) {
            match e {
                ListError::Denied => {
                    listing.flags = flag::DENIED;
                    sh.progress.denied.fetch_add(1, Ordering::Relaxed);
                }
                ListError::Missing | ListError::Other => {
                    listing.flags = flag::ERROR;
                    sh.progress.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        tick = tick.wrapping_add(1);
        if tick % 64 == 1 {
            if let Ok(mut c) = sh.progress.current.try_lock() {
                *c = sys::display(&job.path);
            }
        }
        let mut children = Vec::new();
        let (mut files, mut dirs, mut bytes) = (0u64, 0u64, 0u64);
        listing.items.reserve_exact(raw.len());
        for r in &raw {
            let name = &listing.names[r.name_start as usize..(r.name_start + r.name_len) as usize];
            if sh.excludes.name(name.as_bytes()) {
                continue;
            }
            let mut flags = r.flags;
            let mut alloc = r.alloc;
            let mut dir_id = NONE;
            match r.kind {
                Kind::Link => flags |= flag::LINK,
                Kind::File => {
                    if sh.opts.hardlinks && r.nlink > 1 && !sh.seen.lock().unwrap().insert((r.dev, r.ino)) {
                        alloc = 0;
                        flags |= flag::HARDLINK;
                    }
                }
                Kind::Dir => {
                    flags |= flag::DIR;
                    let path = job.path.join(name);
                    if !sh.excludes.paths.is_empty() && sh.excludes.path(&sys::display(&path)) {
                        continue;
                    }
                    if sh.opts.cross_mounts || r.dev == job.dev {
                        dir_id = sh.next_id.fetch_add(1, Ordering::Relaxed);
                        children.push(Job { id: dir_id, path, dev: if sh.opts.cross_mounts { r.dev } else { job.dev }, cluster: job.cluster });
                    } else {
                        flags |= flag::MOUNT;
                    }
                }
            }
            if flags & flag::DIR != 0 {
                dirs += 1;
            } else {
                files += 1;
                bytes += alloc;
            }
            listing.items.push(Item { name_start: r.name_start, name_len: r.name_len, flags, len: if flags & flag::DIR != 0 { 0 } else { r.len }, alloc: if flags & flag::DIR != 0 { 0 } else { alloc }, mtime: r.mtime, dir_id });
        }
        sh.progress.files.fetch_add(files, Ordering::Relaxed);
        sh.progress.dirs.fetch_add(dirs, Ordering::Relaxed);
        sh.progress.bytes.fetch_add(bytes, Ordering::Relaxed);
        if listing.names.capacity() > listing.names.len() * 2 {
            listing.names.shrink_to_fit();
        }
        out.push(listing);
        let mut q = sh.q.lock().unwrap();
        q.busy -= 1;
        let n = children.len();
        q.jobs.extend(children);
        if q.busy == 0 && q.jobs.is_empty() {
            sh.cv.notify_all();
        } else {
            for _ in 0..n.min(sh.threads) {
                sh.cv.notify_one();
            }
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn scan(opts: &ScanOptions, control: &Control, progress: &Progress) -> Result<Index, ScanError> {
    let started = Instant::now();
    let roots = pick_roots(&opts.roots);
    if roots.is_empty() {
        return Err(ScanError::NoRoots);
    }
    let jobs: Vec<Job> = roots.iter().enumerate().map(|(i, r)| Job { id: i as u32, path: r.path.clone(), dev: r.dev, cluster: sys::cluster_size(&r.path) }).collect();
    let threads = opts.threads.unwrap_or_else(|| opts.level.threads()).max(1);
    let sh = Shared {
        q: Mutex::new(Queue { jobs, busy: 0, done: false }),
        cv: Condvar::new(),
        next_id: AtomicU32::new(roots.len() as u32),
        seen: Mutex::new(HashSet::new()),
        excludes: Excludes::new(&opts.excludes),
        threads,
        opts,
        control,
        progress,
    };
    let listings: Vec<Listing> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads).map(|_| s.spawn(|| worker(&sh))).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    if control.is_cancelled() {
        return Err(ScanError::Cancelled);
    }
    let total = sh.next_id.load(Ordering::Relaxed) as usize;
    let mut ix = build(&roots, listings, total);
    ix.stats.elapsed_ms = started.elapsed().as_millis() as u64;
    ix.stats.scanned_at = now_ms();
    Ok(ix)
}

fn build(roots: &[Root], listings: Vec<Listing>, total: usize) -> Index {
    let entries: usize = listings.iter().map(|l| l.items.len()).sum();
    let names: usize = listings.iter().map(|l| l.names.len()).sum();
    let mut slots: Vec<Option<Listing>> = (0..total).map(|_| None).collect();
    for l in listings {
        let id = l.id as usize;
        slots[id] = Some(l);
    }
    let mut b = Builder::with_capacity(entries + roots.len(), names + names / 8);
    let mut queue = VecDeque::new();
    let mut root_ids = Vec::new();
    for (i, r) in roots.iter().enumerate() {
        let n = b.push(NONE, &r.display, flag::DIR, 0, 0, r.mtime);
        root_ids.push(n);
        queue.push_back((n, i));
    }
    while let Some((node, id)) = queue.pop_front() {
        let Some(l) = slots[id].take() else { continue };
        b.nodes[node as usize].flags |= l.flags;
        let first = b.nodes.len() as u32;
        for it in &l.items {
            let name = &l.names[it.name_start as usize..(it.name_start + it.name_len) as usize];
            let c = b.push(node, name, it.flags, it.alloc, it.len, it.mtime);
            if it.dir_id != NONE {
                queue.push_back((c, it.dir_id as usize));
            }
        }
        let n = &mut b.nodes[node as usize];
        n.first = first;
        n.count = l.items.len() as u32;
    }
    let mut ix = b.finish(root_ids, Stats::default());
    ix.sum_sizes();
    ix.recount();
    ix
}
