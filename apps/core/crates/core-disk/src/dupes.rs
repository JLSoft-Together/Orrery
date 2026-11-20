use crate::index::{eq_name, flag, Index};
use crate::walk::{Control, Excludes, Level};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Instant, UNIX_EPOCH};

pub const QUICK: usize = 64 * 1024;
pub const BUF: usize = 1 << 20;
pub const DEFAULT_MIN: u64 = 1 << 20;
pub const MAX_GROUPS: usize = 5_000;
pub const MAX_FILES: usize = 500;
pub const MAX_TOTAL_FILES: usize = 50_000;

pub const SKIP_DIRS: [&str; 15] = [
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "AppData",
    "Library",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "Windows",
    "$Recycle.Bin",
    "System Volume Information",
    ".Trash",
    ".Trashes",
    "$WinREAgent",
];

#[derive(Debug, Clone)]
pub struct Options {
    pub under: Vec<String>,
    pub min_size: u64,
    pub excludes: Vec<String>,
    pub level: Level,
    pub max_groups: usize,
    pub max_files: usize,
    pub max_total_files: usize,
    pub threads: Option<usize>,
}

impl Options {
    pub fn new(under: Vec<String>) -> Self {
        Options { under, min_size: DEFAULT_MIN, excludes: Vec::new(), level: Level::Balanced, max_groups: MAX_GROUPS, max_files: MAX_FILES, max_total_files: MAX_TOTAL_FILES, threads: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Size,
    Quick,
    Full,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Idle => "idle",
            Stage::Size => "size",
            Stage::Quick => "quick",
            Stage::Full => "full",
        }
    }

    fn from(v: u8) -> Stage {
        match v {
            1 => Stage::Size,
            2 => Stage::Quick,
            3 => Stage::Full,
            _ => Stage::Idle,
        }
    }
}

#[derive(Default)]
pub struct DupeProgress {
    stage: AtomicU8,
    candidates: AtomicU64,
    files: AtomicU64,
    done: AtomicU64,
    bytes: AtomicU64,
    read: AtomicU64,
    groups: AtomicU64,
    skipped: AtomicU64,
    current: Mutex<String>,
}

impl DupeProgress {
    pub fn stage(&self) -> Stage {
        Stage::from(self.stage.load(Ordering::Relaxed))
    }

    fn begin(&self, stage: u8, files: u64, bytes: u64) {
        self.files.store(files, Ordering::Relaxed);
        self.done.store(0, Ordering::Relaxed);
        self.bytes.store(bytes, Ordering::Relaxed);
        self.read.store(0, Ordering::Relaxed);
        self.stage.store(stage, Ordering::Relaxed);
    }

    fn note(&self, path: &str) {
        if let Ok(mut c) = self.current.try_lock() {
            c.clear();
            c.push_str(path);
        }
    }

    pub fn bytes_read(&self) -> u64 {
        self.read.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "stage": self.stage().name(),
            "candidates": self.candidates.load(Ordering::Relaxed),
            "files": self.files.load(Ordering::Relaxed),
            "filesDone": self.done.load(Ordering::Relaxed),
            "bytes": self.bytes.load(Ordering::Relaxed),
            "bytesRead": self.read.load(Ordering::Relaxed),
            "groups": self.groups.load(Ordering::Relaxed),
            "skipped": self.skipped.load(Ordering::Relaxed),
            "current": self.current.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    NotInIndex(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Cancelled => f.write_str("cancelled"),
            Error::NotInIndex(p) => write!(f, "not in index: {p}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fail {
    Gone,
    Stale,
    Cloud,
    Locked,
    Changed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skipped {
    pub stale: u64,
    pub cloud: u64,
    pub locked: u64,
    pub changed: u64,
    pub gone: u64,
}

impl Skipped {
    pub fn total(&self) -> u64 {
        self.stale + self.cloud + self.locked + self.changed + self.gone
    }

    fn add(&mut self, f: Fail) {
        match f {
            Fail::Stale => self.stale += 1,
            Fail::Cloud => self.cloud += 1,
            Fail::Locked => self.locked += 1,
            Fail::Changed => self.changed += 1,
            Fail::Gone | Fail::Cancelled => self.gone += 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DupeFile {
    pub path: String,
    pub mtime: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub size: u64,
    pub hash: String,
    pub count: usize,
    pub files: Vec<DupeFile>,
}

impl Group {
    pub fn wasted(&self) -> u64 {
        self.size * (self.count as u64 - 1)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub groups: Vec<Group>,
    pub total_groups: usize,
    pub wasted: u64,
    pub candidates: u64,
    pub hashed_files: u64,
    pub hashed_bytes: u64,
    pub skipped: Skipped,
    pub ms: u64,
}

impl Outcome {
    pub fn to_json(&self) -> Value {
        let groups: Vec<Value> = self
            .groups
            .iter()
            .map(|g| {
                json!({
                    "size": g.size,
                    "hash": g.hash,
                    "count": g.count,
                    "files": g.files.iter().map(|f| json!({"path": f.path, "mtime": f.mtime as u64 * 1000})).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "groups": groups,
            "totalGroups": self.total_groups,
            "wasted": self.wasted,
            "candidates": self.candidates,
            "hashedFiles": self.hashed_files,
            "hashedBytes": self.hashed_bytes,
            "skipped": {"stale": self.skipped.stale, "cloud": self.skipped.cloud, "locked": self.skipped.locked, "changed": self.skipped.changed, "gone": self.skipped.gone},
            "ms": self.ms,
        })
    }
}

struct Job {
    path: String,
    len: u64,
    mtime: u32,
}

struct Quick {
    id: Option<(u64, u64)>,
    hash: [u8; 32],
    full: bool,
}

fn skip_dir(name: &str) -> bool {
    SKIP_DIRS.iter().any(|d| eq_name(name, d)) || (cfg!(target_os = "macos") && name.len() > 4 && name.as_bytes()[name.len() - 4..].eq_ignore_ascii_case(b".app"))
}

fn collect(ix: &Index, opts: &Options, control: &Control, progress: &DupeProgress) -> Result<Vec<(u64, u32)>, Error> {
    let ex = Excludes::new(&opts.excludes);
    let mut starts = Vec::new();
    let mut missing = None;
    for p in &opts.under {
        match ix.find(p) {
            Some(n) if ix.nodes[n as usize].is_dir() => starts.push(n),
            Some(_) => {}
            None => missing = missing.or(Some(p.clone())),
        }
    }
    if starts.is_empty() {
        return Err(Error::NotInIndex(missing.unwrap_or_default()));
    }
    let mut out = Vec::new();
    let mut stack = starts;
    while let Some(d) = stack.pop() {
        if control.is_cancelled() {
            return Err(Error::Cancelled);
        }
        control.wait_if_paused();
        for c in ix.children(d) {
            let n = &ix.nodes[c as usize];
            let name = ix.name(c);
            if ex.name(name.as_bytes()) || (ex.has_paths() && ex.path(&ix.path(c))) {
                continue;
            }
            if n.is_dir() {
                if n.flags & (flag::LINK | flag::CLOUD | flag::MOUNT | flag::SYSTEM | flag::DENIED) == 0 && !skip_dir(name) {
                    stack.push(c);
                }
            } else if n.flags & (flag::LINK | flag::CLOUD | flag::HARDLINK | flag::ERROR) == 0 && n.len >= opts.min_size.max(1) {
                out.push((n.len, c));
            }
        }
        progress.candidates.store(out.len() as u64, Ordering::Relaxed);
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

#[cfg(windows)]
fn open_path(p: &str) -> PathBuf {
    if p.starts_with("\\\\?\\") {
        PathBuf::from(p)
    } else if let Some(rest) = p.strip_prefix("\\\\") {
        PathBuf::from(format!("\\\\?\\UNC\\{rest}"))
    } else {
        PathBuf::from(format!("\\\\?\\{p}"))
    }
}

#[cfg(not(windows))]
fn open_path(p: &str) -> PathBuf {
    PathBuf::from(p)
}

fn stat(path: &Path) -> Result<(u64, u32), Fail> {
    let m = fs::symlink_metadata(path).map_err(|_| Fail::Gone)?;
    if !m.is_file() {
        return Err(Fail::Stale);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let attrs = m.file_attributes();
        if attrs & (0x1000 | 0x40000 | 0x400000) != 0 {
            return Err(Fail::Cloud);
        }
        if attrs & 0x400 != 0 {
            return Err(Fail::Stale);
        }
    }
    let mtime = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs().min(u32::MAX as u64) as u32).unwrap_or(0);
    Ok((m.len(), mtime))
}

fn open(path: &Path) -> Result<File, Fail> {
    let mut o = fs::OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.custom_flags(0x0800_0000);
    }
    o.open(path).map_err(|_| Fail::Locked)
}

#[cfg(unix)]
fn file_id(f: &File) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    f.metadata().ok().map(|m| (m.dev(), m.ino()))
}

#[cfg(windows)]
fn file_id(f: &File) -> Option<(u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(f.as_raw_handle() as _, &mut info) } == 0 {
        return None;
    }
    Some((info.dwVolumeSerialNumber as u64, ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64))
}

fn hash_stream(f: &mut File, h: &mut blake3::Hasher, len: u64, buf: &mut [u8], control: &Control, progress: &DupeProgress) -> Result<(), Fail> {
    let mut left = len;
    while left > 0 {
        if control.is_cancelled() {
            return Err(Fail::Cancelled);
        }
        control.wait_if_paused();
        let want = left.min(buf.len() as u64) as usize;
        let n = f.read(&mut buf[..want]).map_err(|_| Fail::Locked)?;
        if n == 0 {
            return Err(Fail::Changed);
        }
        h.update(&buf[..n]);
        left -= n as u64;
        progress.read.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(())
}

fn prepare(job: &Job) -> Result<(PathBuf, File, (u64, u32)), Fail> {
    let path = open_path(&job.path);
    let before = stat(&path)?;
    if before != (job.len, job.mtime) {
        return Err(Fail::Stale);
    }
    let f = open(&path)?;
    Ok((path, f, before))
}

fn quick(job: &Job, buf: &mut [u8], control: &Control, progress: &DupeProgress) -> Result<Quick, Fail> {
    let (path, mut f, before) = prepare(job)?;
    let id = file_id(&f);
    let mut h = blake3::Hasher::new();
    let whole = job.len <= 2 * QUICK as u64;
    if whole {
        hash_stream(&mut f, &mut h, job.len, buf, control, progress)?;
    } else {
        f.read_exact(&mut buf[..QUICK]).map_err(|_| Fail::Locked)?;
        h.update(&buf[..QUICK]);
        f.seek(SeekFrom::End(-(QUICK as i64))).map_err(|_| Fail::Locked)?;
        f.read_exact(&mut buf[..QUICK]).map_err(|_| Fail::Locked)?;
        h.update(&buf[..QUICK]);
        progress.read.fetch_add(2 * QUICK as u64, Ordering::Relaxed);
    }
    if stat(&path)? != before {
        return Err(Fail::Changed);
    }
    Ok(Quick { id, hash: *h.finalize().as_bytes(), full: whole })
}

fn full(job: &Job, buf: &mut [u8], control: &Control, progress: &DupeProgress) -> Result<[u8; 32], Fail> {
    let (path, mut f, before) = prepare(job)?;
    let mut h = blake3::Hasher::new();
    hash_stream(&mut f, &mut h, job.len, buf, control, progress)?;
    let mut probe = [0u8; 1];
    if f.read(&mut probe).map(|n| n != 0).unwrap_or(true) || stat(&path)? != before {
        return Err(Fail::Changed);
    }
    Ok(*h.finalize().as_bytes())
}

fn run<T: Send>(total: usize, threads: usize, level: Level, control: &Control, progress: &DupeProgress, work: impl Fn(usize, &mut [u8]) -> Result<T, Fail> + Sync) -> Result<Vec<Result<T, Fail>>, Error> {
    let next = AtomicUsize::new(0);
    let parts: Vec<Vec<(usize, Result<T, Fail>)>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads.min(total).max(1))
            .map(|_| {
                s.spawn(|| {
                    crate::walk::sys::lower_priority(level);
                    let mut buf = vec![0u8; BUF];
                    let mut out = Vec::new();
                    loop {
                        if control.is_cancelled() {
                            break;
                        }
                        control.wait_if_paused();
                        if control.is_cancelled() {
                            break;
                        }
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= total {
                            break;
                        }
                        let r = work(i, &mut buf);
                        progress.done.fetch_add(1, Ordering::Relaxed);
                        if r.is_err() {
                            progress.skipped.fetch_add(1, Ordering::Relaxed);
                        }
                        out.push((i, r));
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    });
    if control.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let mut slots: Vec<Option<Result<T, Fail>>> = (0..total).map(|_| None).collect();
    for (i, r) in parts.into_iter().flatten() {
        slots[i] = Some(r);
    }
    Ok(slots.into_iter().map(|r| r.unwrap_or(Err(Fail::Gone))).collect())
}

fn hex(h: &[u8; 32]) -> String {
    h[..16].iter().map(|b| format!("{b:02x}")).collect()
}

pub fn find(ix: &Index, opts: &Options, control: &Control, progress: &DupeProgress) -> Result<Outcome, Error> {
    let started = Instant::now();
    let threads = opts.threads.unwrap_or_else(|| opts.level.threads());
    progress.begin(1, 0, 0);
    let cands = collect(ix, opts, control, progress)?;
    let mut jobs: Vec<Job> = Vec::new();
    let mut at = 0;
    while at < cands.len() {
        let mut end = at + 1;
        while end < cands.len() && cands[end].0 == cands[at].0 {
            end += 1;
        }
        if end - at >= 2 {
            for &(len, node) in &cands[at..end] {
                jobs.push(Job { path: ix.path(node), len, mtime: ix.nodes[node as usize].mtime });
            }
        }
        at = end;
    }
    let mut out = Outcome { candidates: cands.len() as u64, ..Outcome::default() };
    drop(cands);

    let quick_bytes: u64 = jobs.iter().map(|j| j.len.min(2 * QUICK as u64)).sum();
    progress.begin(2, jobs.len() as u64, quick_bytes);
    let quicks = run(jobs.len(), threads, opts.level, control, progress, |i, buf| {
        progress.note(&jobs[i].path);
        quick(&jobs[i], buf, control, progress)
    })?;
    out.hashed_files += jobs.len() as u64;
    out.hashed_bytes += progress.bytes_read();

    let mut by_quick: HashMap<(u64, [u8; 32]), Vec<usize>> = HashMap::new();
    for (i, q) in quicks.iter().enumerate() {
        match q {
            Ok(q) => by_quick.entry((jobs[i].len, q.hash)).or_default().push(i),
            Err(f) => out.skipped.add(*f),
        }
    }
    let mut finished: Vec<(u64, [u8; 32], Vec<usize>)> = Vec::new();
    let mut pending: Vec<Vec<usize>> = Vec::new();
    let mut keys: Vec<_> = by_quick.keys().copied().collect();
    keys.sort_unstable();
    for k in keys {
        let mut members = by_quick.remove(&k).unwrap_or_default();
        let mut seen = HashSet::new();
        members.retain(|i| match quicks[*i].as_ref().ok().and_then(|q| q.id) {
            Some(id) => seen.insert(id),
            None => true,
        });
        if members.len() < 2 {
            continue;
        }
        if quicks[members[0]].as_ref().map(|q| q.full).unwrap_or(false) {
            finished.push((k.0, k.1, members));
        } else {
            pending.push(members);
        }
    }

    let flat: Vec<usize> = pending.iter().flatten().copied().collect();
    if !flat.is_empty() {
        let bytes: u64 = flat.iter().map(|i| jobs[*i].len).sum();
        progress.begin(3, flat.len() as u64, bytes);
        let fulls = run(flat.len(), threads, opts.level, control, progress, |n, buf| {
            progress.note(&jobs[flat[n]].path);
            full(&jobs[flat[n]], buf, control, progress)
        })?;
        out.hashed_files += flat.len() as u64;
        out.hashed_bytes += progress.bytes_read();
        let mut pos = 0;
        for members in &pending {
            let mut by_full: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
            for &m in members {
                match &fulls[pos] {
                    Ok(h) => by_full.entry(*h).or_default().push(m),
                    Err(f) => out.skipped.add(*f),
                }
                pos += 1;
            }
            for (h, v) in by_full {
                if v.len() >= 2 {
                    finished.push((jobs[v[0]].len, h, v));
                }
            }
        }
    }

    finished.sort_by(|a, b| {
        let wa = a.0 * (a.2.len() as u64 - 1);
        let wb = b.0 * (b.2.len() as u64 - 1);
        wb.cmp(&wa).then(b.0.cmp(&a.0)).then(a.1.cmp(&b.1))
    });
    out.total_groups = finished.len();
    out.wasted = finished.iter().map(|g| g.0 * (g.2.len() as u64 - 1)).sum();
    progress.groups.store(finished.len() as u64, Ordering::Relaxed);
    finished.truncate(opts.max_groups);
    let mut budget = opts.max_total_files;
    for (size, hash, members) in finished {
        let mut files: Vec<DupeFile> = members.iter().map(|i| DupeFile { path: jobs[*i].path.clone(), mtime: jobs[*i].mtime }).collect();
        files.sort_by(|a, b| a.mtime.cmp(&b.mtime).then(a.path.len().cmp(&b.path.len())).then(a.path.cmp(&b.path)));
        let count = files.len();
        let keep = opts.max_files.min(budget).max(2).min(count);
        budget = budget.saturating_sub(keep);
        files.truncate(keep);
        out.groups.push(Group { size, hash: hex(&hash), count, files });
    }
    out.ms = started.elapsed().as_millis() as u64;
    Ok(out)
}
