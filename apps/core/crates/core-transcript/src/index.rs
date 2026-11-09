use crate::omap::OMap;
use crate::record::{parse_line, tool_calls_of, tool_key, usage_record_of, Line, Record, Totals};
use memchr::memmem::Finder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const DAY_MS: f64 = 86_400_000.0;
const OPEN_TOOLS_MAX: usize = 500;
const CHUNK: usize = 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct FileEntry {
    pub size: u64,
    pub offset: u64,
    pub mtime_ms: Option<f64>,
    pub ids: Vec<String>,
    pub buckets: OMap<Totals>,
    pub hours: Option<OMap<f64>>,
    pub tools: Option<OMap<f64>>,
    pub gone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meta {
    pub size: u64,
    pub mtime_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scan {
    pub state: ScanState,
    pub files_done: usize,
    pub files_total: usize,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub sweep_ms: u64,
    pub file_debounce_ms: u64,
    pub save_debounce_ms: u64,
    pub id_retention_ms: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options { sweep_ms: 600_000, file_debounce_ms: 1_000, save_debounce_ms: 300_000, id_retention_ms: 30.0 * DAY_MS }
    }
}

pub struct UsageIndex {
    pub projects_dir: PathBuf,
    pub cache_file: Option<PathBuf>,
    pub opts: Options,
    pub files: OMap<FileEntry>,
    pub owners: HashMap<String, usize>,
    pub totals: OMap<Totals>,
    pub hour_totals: OMap<f64>,
    pub tool_totals: OMap<f64>,
    open_tools: HashMap<usize, OMap<String>>,
    pub dirty: OMap<()>,
    pub scan: Scan,
    pub save_needed: bool,
}

fn finders() -> &'static (Finder<'static>, Finder<'static>) {
    static F: OnceLock<(Finder<'static>, Finder<'static>)> = OnceLock::new();
    F.get_or_init(|| (Finder::new(b"\"usage\""), Finder::new(b"\"is_error\":true")))
}

pub fn meta_of(path: &Path) -> Option<Meta> {
    let m = std::fs::metadata(path).ok()?;
    let mtime_ms = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs() as f64 * 1000.0 + f64::from(d.subsec_nanos()) / 1e6);
    Some(Meta { size: m.len(), mtime_ms })
}

fn add_count(map: &mut OMap<f64>, key: &str, n: f64) {
    let v = map.get(key).copied().unwrap_or(0.0) + n;
    if v > 0.0 {
        map.insert(key, v);
    } else {
        map.remove(key);
    }
}

impl UsageIndex {
    pub fn new(projects_dir: PathBuf, cache_file: Option<PathBuf>, opts: Options) -> UsageIndex {
        UsageIndex {
            projects_dir,
            cache_file,
            opts,
            files: OMap::new(),
            owners: HashMap::new(),
            totals: OMap::new(),
            hour_totals: OMap::new(),
            tool_totals: OMap::new(),
            open_tools: HashMap::new(),
            dirty: OMap::new(),
            scan: Scan { state: ScanState::Idle, files_done: 0, files_total: 0 },
            save_needed: false,
        }
    }

    pub fn add_total(&mut self, key: &str, totals: &Totals, sign: f64) {
        self.totals.entry_or(key, Totals::default).add(totals, sign);
        self.dirty.insert(key, ());
    }

    pub fn add_hour(&mut self, key: &str, n: f64) {
        add_count(&mut self.hour_totals, key, n);
    }

    pub fn add_tool(&mut self, key: &str, n: f64) {
        add_count(&mut self.tool_totals, key, n);
    }

    pub fn release_ids(&mut self, slot: usize, entry: &mut FileEntry) {
        for id in entry.ids.drain(..) {
            if self.owners.get(&id) == Some(&slot) {
                self.owners.remove(&id);
            }
        }
    }

    fn with_entry<T>(&mut self, slot: usize, f: impl FnOnce(&mut Self, &mut FileEntry) -> T) -> T {
        let mut entry = std::mem::take(self.files.at_mut(slot).expect("file slot"));
        let out = f(self, &mut entry);
        *self.files.at_mut(slot).expect("file slot") = entry;
        out
    }

    pub fn prune_old_ids(&mut self, now_ms: f64) {
        let cutoff = now_ms - self.opts.id_retention_ms;
        let old: Vec<usize> = self.files.iter_slots().filter(|(_, _, e)| !e.ids.is_empty() && e.mtime_ms.is_some_and(|m| m < cutoff)).map(|(i, _, _)| i).collect();
        for slot in old {
            self.with_entry(slot, |me, e| me.release_ids(slot, e));
            self.save_needed = true;
        }
    }

    pub fn mark_gone(&mut self, present: &[String]) {
        let live: std::collections::HashSet<&str> = present.iter().map(String::as_str).collect();
        let gone: Vec<usize> = self.files.iter_slots().filter(|(_, rel, e)| !e.gone && !live.contains(rel)).map(|(i, _, _)| i).collect();
        for slot in gone {
            self.with_entry(slot, |me, e| {
                e.gone = true;
                me.release_ids(slot, e);
            });
            self.save_needed = true;
        }
    }

    fn reset_file(&mut self, slot: usize, entry: &mut FileEntry) {
        for (key, t) in entry.buckets.iter() {
            self.add_total(key, t, -1.0);
        }
        for (key, n) in entry.hours.iter().flat_map(|h| h.iter()) {
            add_count(&mut self.hour_totals, key, -n);
        }
        for (key, n) in entry.tools.iter().flat_map(|h| h.iter()) {
            add_count(&mut self.tool_totals, key, -n);
        }
        self.release_ids(slot, entry);
        self.open_tools.remove(&slot);
        entry.buckets = OMap::new();
        entry.hours = None;
        entry.tools = None;
        entry.offset = 0;
    }

    pub fn ingest(&mut self, rel: &str, known: Option<Meta>) -> std::io::Result<()> {
        let full = self.projects_dir.join(rel);
        let Some(st) = known.or_else(|| meta_of(&full)) else { return Ok(()) };
        let slot = match self.files.slot_of(rel) {
            Some(s) => s,
            None => self.files.insert(rel, FileEntry::default()),
        };
        self.with_entry(slot, |me, entry| me.ingest_entry(&full, rel, slot, st, entry))
    }

    fn ingest_entry(&mut self, full: &Path, rel: &str, slot: usize, st: Meta, entry: &mut FileEntry) -> std::io::Result<()> {
        if entry.gone {
            self.reset_file(slot, entry);
            entry.gone = false;
        }
        if st.size < entry.offset {
            self.reset_file(slot, entry);
        }
        entry.size = st.size;
        if entry.mtime_ms != Some(st.mtime_ms) {
            entry.mtime_ms = Some(st.mtime_ms);
            self.save_needed = true;
        }
        if st.size == entry.offset {
            return Ok(());
        }
        let fallback = rel.split('/').next().unwrap_or(rel).to_owned();
        let opened = File::open(full).and_then(|mut f| f.seek(SeekFrom::Start(entry.offset)).map(|_| f));
        let mut f = match opened {
            Ok(f) => f,
            Err(e) => {
                self.skipped(entry, &e);
                return Ok(());
            }
        };
        let mut remaining = st.size - entry.offset;
        let mut buf = vec![0u8; CHUNK];
        let mut carry: Vec<u8> = Vec::new();
        let mut consumed = 0u64;
        let result = loop {
            if remaining == 0 {
                break Ok(());
            }
            let want = CHUNK.min(usize::try_from(remaining).unwrap_or(CHUNK));
            let n = match f.read(&mut buf[..want]) {
                Ok(0) => break Ok(()),
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(e),
            };
            remaining -= n as u64;
            let chunk = &buf[..n];
            let mut start = 0;
            for idx in memchr::memchr_iter(b'\n', chunk) {
                if carry.is_empty() {
                    self.ingest_line(&chunk[start..idx], slot, &fallback, entry);
                    consumed += (idx - start) as u64 + 1;
                } else {
                    carry.extend_from_slice(&chunk[start..idx]);
                    let line = std::mem::take(&mut carry);
                    self.ingest_line(&line, slot, &fallback, entry);
                    consumed += line.len() as u64 + 1;
                }
                start = idx + 1;
            }
            carry.extend_from_slice(&chunk[start..]);
        };
        match result {
            Ok(()) => entry.offset += consumed,
            Err(e) => self.skipped(entry, &e),
        }
        Ok(())
    }

    fn skipped(&mut self, entry: &mut FileEntry, e: &std::io::Error) {
        entry.mtime_ms = None;
        self.save_needed = true;
        eprintln!("[collector] usage index: skipped a transcript: {e}");
    }

    pub fn ingest_line(&mut self, line: &[u8], slot: usize, fallback: &str, entry: &mut FileEntry) {
        let (usage, error) = finders();
        let has_usage = usage.find(line).is_some();
        let has_error = error.find(line).is_some();
        if !has_usage && !has_error {
            return;
        }
        let Some(rec) = parse_line(line) else { return };
        if has_error && rec.kind.as_deref() == Some("user") {
            return self.ingest_errors(&rec, slot, entry);
        }
        let Some(r) = usage_record_of(&rec, fallback) else { return };
        match self.owners.get(&r.message_id) {
            None => {
                self.owners.insert(r.message_id.clone(), slot);
                entry.ids.push(r.message_id.clone());
                entry.buckets.entry_or(&r.key, Totals::default).add(&r.totals, 1.0);
                self.add_total(&r.key, &r.totals, 1.0);
                let hk = format!("{}|{}", r.day, r.hour);
                add_count(entry.hours.get_or_insert_with(OMap::new), &hk, 1.0);
                self.add_hour(&hk, 1.0);
            }
            Some(&owner) if owner != slot => return,
            Some(_) => {}
        }
        self.ingest_tools(&rec, &r, slot, entry);
    }

    fn count_tool(&mut self, entry: &mut FileEntry, key: &str, n: f64) {
        let tools = entry.tools.get_or_insert_with(OMap::new);
        let v = tools.get(key).copied().unwrap_or(0.0) + n;
        tools.insert(key, v);
        self.add_tool(key, n);
    }

    fn ingest_tools(&mut self, rec: &Line, r: &Record, slot: usize, entry: &mut FileEntry) {
        let Some(content) = rec.message.as_ref().and_then(|m| m.content.as_ref()) else { return };
        for block in content.iter().flatten() {
            let calls = tool_calls_of(block);
            let Some((head_kind, head_name)) = calls.first() else { continue };
            for (kind, name) in &calls {
                self.count_tool(entry, &tool_key(&r.day, *kind, name, &r.project), 1.0);
            }
            let Some(id) = block.id.as_deref() else { continue };
            if id.len() > 200 && id.encode_utf16().count() > 200 {
                continue;
            }
            let err_kind = if *head_kind == 'm' { 'f' } else { 'e' };
            let open = self.open_tools.entry(slot).or_default();
            open.insert(id, tool_key(&r.day, err_kind, head_name, &r.project));
            if open.len() > OPEN_TOOLS_MAX {
                if let Some(first) = open.first_key().map(str::to_owned) {
                    open.remove(&first);
                }
            }
        }
    }

    fn ingest_errors(&mut self, rec: &Line, slot: usize, entry: &mut FileEntry) {
        let Some(content) = rec.message.as_ref().and_then(|m| m.content.as_ref()) else { return };
        if !self.open_tools.contains_key(&slot) {
            return;
        }
        for b in content.iter().flatten() {
            if b.kind.as_deref() != Some("tool_result") || !b.is_error {
                continue;
            }
            let Some(tid) = b.tool_use_id.as_deref() else { continue };
            let Some(key) = self.open_tools.get_mut(&slot).and_then(|o| o.remove(tid)) else { continue };
            self.count_tool(entry, &key, 1.0);
        }
    }
}
