use super::page::{lines, read_range, Window};
use super::{Agent, Body, Builder, Clean, SCAN_BYTES};
pub use crate::time::civil_from_days as civil;
use crate::tracker::cwd_slug;
use memchr::memmem;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const KEEP_MS: i64 = 30 * 24 * 3_600_000;
pub const STORE_MAX: usize = 5 * 1024 * 1024;
pub const BEFORE_MS: i64 = 6 * 3_600_000;
pub const AFTER_MS: i64 = 120_000;
pub const STRONG: f64 = 0.8;
pub const WEAK: f64 = 0.3;
pub const CHIPS_MAX: usize = 2;
pub const FIRST_SCAN: u64 = 32 * 1024 * 1024;
pub const PROMPT_MAX: usize = 80;
pub const PATHS_MAX: usize = 300;
const HEAD_BYTES: u64 = 1024 * 1024;
const DAY_MS: i64 = 86_400_000;
const EDIT_NAMES: [&[u8]; 4] = [b"\"name\":\"Edit\"", b"\"name\":\"Write\"", b"\"name\":\"MultiEdit\"", b"\"name\":\"NotebookEdit\""];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnEdit {
    pub session_id: String,
    pub agent: String,
    pub label: String,
    pub transcript_path: String,
    pub cwd: String,
    pub turn: u64,
    #[serde(default)]
    pub start: Option<i64>,
    pub end: i64,
    pub paths: Vec<String>,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Seen {
    size: u64,
    done: u64,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    skip: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    files: BTreeMap<String, Seen>,
}

pub struct Sources {
    pub projects: PathBuf,
    pub sessions: PathBuf,
}

pub struct Meta<'a> {
    pub session_id: &'a str,
    pub agent: Agent,
    pub cwd: &'a str,
    pub root: &'a str,
    pub transcript: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub session_id: String,
    pub agent: String,
    pub label: String,
    pub cwd: String,
    pub turn: u64,
    pub end: i64,
    pub score: f64,
    pub prompt: String,
}

pub fn fold(root: &str) -> bool {
    let r = root.replace('\\', "/");
    r.as_bytes().get(1) == Some(&b':') || r.starts_with("//")
}

fn norm_dir(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_owned()
}

fn folded(s: &str, on: bool) -> String {
    if on {
        s.to_ascii_lowercase()
    } else {
        s.to_owned()
    }
}

pub fn under(cwd: &str, root: &str) -> bool {
    let on = fold(root);
    let (c, r) = (folded(&norm_dir(cwd), on), folded(&norm_dir(root), on));
    if r.is_empty() || c.is_empty() {
        return false;
    }
    c == r || (c.starts_with(&r) && c.as_bytes().get(r.len()) == Some(&b'/'))
}

fn resolve(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for (i, seg) in p.split('/').enumerate() {
        match seg {
            "." => {}
            "" if i > 0 => {}
            ".." if parts.len() > 1 => {
                parts.pop();
            }
            _ => parts.push(seg),
        }
    }
    parts.join("/")
}

pub fn repo_path(raw: &str, cwd: &str, root: &str) -> Option<String> {
    let p = raw.replace('\\', "/");
    if p.starts_with('~') {
        return None;
    }
    let abs = if p.starts_with('/') || p.as_bytes().get(1) == Some(&b':') { p } else { format!("{}/{}", norm_dir(cwd), p) };
    let abs = resolve(&abs);
    let on = fold(root);
    let r = folded(&norm_dir(root), on);
    let a = folded(&abs, on);
    (!r.is_empty() && a.len() > r.len() + 1 && a.starts_with(&r) && a.as_bytes()[r.len()] == b'/').then(|| abs[r.len() + 1..].to_owned())
}

pub fn store_key(root: &str) -> String {
    let digest = Sha256::digest(folded(&norm_dir(root), fold(root)).as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub fn relevant(agent: Agent, line: &[u8]) -> bool {
    let has = |n: &[u8]| memmem::find(line, n).is_some();
    match agent {
        Agent::Claude => (has(b"\"type\":\"user\"") && !has(b"\"type\":\"tool_result\"")) || (has(b"\"type\":\"tool_use\"") && EDIT_NAMES.iter().any(|n| has(n))),
        Agent::Codex => has(b"\"role\":\"user\"") || has(b"apply_patch") || has(b"\"FileChange\""),
    }
}

fn label_of(cwd: &str) -> String {
    norm_dir(cwd).rsplit('/').next().unwrap_or("").chars().take(40).collect()
}

pub fn scan(path: &Path, start: u64, end: u64, meta: &Meta, clean: &dyn Clean) -> std::io::Result<(Vec<TurnEdit>, u64)> {
    let mut b = Builder::new(meta.agent, "", false);
    b.touched = Some(Vec::new());
    let mut prompts: HashMap<u64, (Option<i64>, String)> = HashMap::new();
    let mut off = start;
    if start > 0 && read_range(path, start - 1, start)? != b"\n" {
        let head = read_range(path, start, end.min(start + SCAN_BYTES))?;
        match memchr::memchr(b'\n', &head) {
            Some(i) => off = start + i as u64 + 1,
            None => return Ok((Vec::new(), end)),
        }
    }
    while off < end {
        let to = end.min(off + SCAN_BYTES);
        let mut bytes = read_range(path, off, to)?;
        let Some(cut) = memchr::memrchr(b'\n', &bytes).map(|i| i + 1) else {
            if to - off >= SCAN_BYTES {
                off = to;
                continue;
            }
            break;
        };
        bytes.truncate(cut);
        let w = Window { start: off, end: off + cut as u64, bytes, from_top: false };
        for (o, line) in lines(&w) {
            if relevant(meta.agent, line) && b.wants(line) {
                b.feed(line, o);
            }
        }
        for item in b.take_dirty() {
            if let Body::Me { raw } = &item.body {
                prompts.insert(item.seq, (item.at, clean.line(raw, PROMPT_MAX)));
            }
        }
        off = w.end;
    }
    let done = b.turns.last().copied().unwrap_or(off);
    let mut by: BTreeMap<u64, (Option<i64>, BTreeSet<String>)> = BTreeMap::new();
    for t in b.touched.take().unwrap_or_default() {
        let Some(turn) = t.turn else { continue };
        let Some(p) = repo_path(&t.path, meta.cwd, meta.root) else { continue };
        let e = by.entry(turn).or_default();
        if let Some(at) = t.at {
            e.0 = Some(e.0.map_or(at, |x| x.max(at)));
        }
        if e.1.len() < PATHS_MAX {
            e.1.insert(p);
        }
    }
    let label = label_of(meta.cwd);
    let out = by
        .into_iter()
        .filter_map(|(turn, (end, paths))| {
            let (start, prompt) = prompts.get(&turn).cloned().unwrap_or((None, String::new()));
            let end = end.or(start)?;
            Some(TurnEdit {
                session_id: meta.session_id.to_owned(),
                agent: meta.agent.as_str().to_owned(),
                label: label.clone(),
                transcript_path: meta.transcript.to_owned(),
                cwd: meta.cwd.to_owned(),
                turn,
                start,
                end,
                paths: paths.into_iter().collect(),
                prompt,
            })
        })
        .collect();
    Ok((out, done))
}

pub fn compact(recs: Vec<TurnEdit>, now: i64) -> Vec<TurnEdit> {
    let mut by: HashMap<(String, String, u64), TurnEdit> = HashMap::new();
    for r in recs {
        by.insert((r.agent.clone(), r.session_id.clone(), r.turn), r);
    }
    let mut out: Vec<TurnEdit> = by.into_values().filter(|r| r.end >= now - KEEP_MS).collect();
    out.sort_by(|a, b| a.end.cmp(&b.end).then_with(|| a.session_id.cmp(&b.session_id)).then_with(|| a.turn.cmp(&b.turn)));
    let sizes: Vec<usize> = out.iter().map(|r| serde_json::to_string(r).map_or(0, |s| s.len() + 1)).collect();
    let mut total: usize = sizes.iter().sum();
    let mut drop = 0;
    while total > STORE_MAX && drop < out.len() {
        total -= sizes[drop];
        drop += 1;
    }
    out.drain(..drop);
    out
}

pub fn match_commit(recs: &[TurnEdit], time_ms: i64, files: &[String], fold: bool) -> (Vec<Hit>, usize) {
    let want: HashSet<String> = files.iter().map(|f| folded(&f.replace('\\', "/"), fold)).filter(|f| !f.is_empty()).collect();
    if want.is_empty() {
        return (Vec::new(), 0);
    }
    let mut by: HashMap<(&str, &str), (HashSet<String>, &TurnEdit, usize)> = HashMap::new();
    for r in recs.iter().filter(|r| r.end >= time_ms - BEFORE_MS && r.end <= time_ms + AFTER_MS) {
        let common: Vec<String> = r.paths.iter().map(|p| folded(p, fold)).filter(|p| want.contains(p)).collect();
        if common.is_empty() {
            continue;
        }
        let n = common.len();
        let e = by.entry((r.agent.as_str(), r.session_id.as_str())).or_insert_with(|| (HashSet::new(), r, 0));
        if n > e.2 || (n == e.2 && r.end > e.1.end) {
            e.1 = r;
            e.2 = n;
        }
        e.0.extend(common);
    }
    let mut hits: Vec<Hit> = by
        .into_values()
        .map(|(u, r, _)| Hit { session_id: r.session_id.clone(), agent: r.agent.clone(), label: r.label.clone(), cwd: r.cwd.clone(), turn: r.turn, end: r.end, score: u.len() as f64 / want.len() as f64, prompt: r.prompt.clone() })
        .filter(|h| h.score >= WEAK)
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| b.end.cmp(&a.end)).then_with(|| a.session_id.cmp(&b.session_id)));
    let more = hits.len().saturating_sub(CHIPS_MAX);
    hits.truncate(CHIPS_MAX);
    (hits, more)
}

fn dirs(dir: &Path) -> Vec<(i64, PathBuf)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<i64>().ok()).map(|n| (n, e.path())))
        .collect()
}

fn jsonl_in(dir: &Path, keep: &dyn Fn(&str) -> bool) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".jsonl") && keep(n)))
        .map(|e| e.path())
        .collect()
}

pub fn claude_files(projects: &Path, root: &str) -> Vec<PathBuf> {
    let slug = cwd_slug(&norm_dir(root)).to_ascii_lowercase();
    if slug.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for e in std::fs::read_dir(projects).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if name == slug || name.strip_prefix(&slug).is_some_and(|r| r.starts_with('-')) {
            out.extend(jsonl_in(&e.path(), &|_| true));
        }
    }
    out
}

pub fn codex_files(sessions: &Path, cutoff_ms: i64) -> Vec<PathBuf> {
    let floor = civil(cutoff_ms.div_euclid(DAY_MS) - 1);
    let mut out = Vec::new();
    for (y, ydir) in dirs(sessions).into_iter().filter(|(y, _)| *y >= floor.0) {
        for (m, mdir) in dirs(&ydir).into_iter().filter(|(m, _)| (y, *m) >= (floor.0, floor.1)) {
            for (d, ddir) in dirs(&mdir) {
                if (y, m, d) >= floor {
                    out.extend(jsonl_in(&ddir, &|n| n.starts_with("rollout-")));
                }
            }
        }
    }
    out
}

pub fn rollout_id(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let id = stem.get(stem.len().checked_sub(36)?..)?;
    (id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')).then(|| id.to_owned())
}

fn head_cwd(path: &Path, agent: Agent) -> String {
    let size = std::fs::metadata(path).map_or(0, |m| m.len());
    let bytes = read_range(path, 0, size.min(HEAD_BYTES)).unwrap_or_default();
    for line in bytes.split(|b| *b == b'\n') {
        if memmem::find(line, b"\"cwd\"").is_none() {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(line) else { continue };
        let cwd = match agent {
            Agent::Claude => v["cwd"].as_str(),
            Agent::Codex => v["payload"]["cwd"].as_str(),
        };
        if let Some(c) = cwd.filter(|c| !c.is_empty()) {
            return c.to_owned();
        }
    }
    String::new()
}

fn mtime_ms(md: &std::fs::Metadata) -> i64 {
    md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64)
}

pub struct Store {
    file: PathBuf,
    index: PathBuf,
    root: String,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

impl Store {
    pub fn new(dir: &Path, root: &str) -> Store {
        let k = store_key(root);
        Store { file: dir.join(format!("{k}.jsonl")), index: dir.join(format!("{k}.idx.json")), root: root.to_owned() }
    }

    pub fn path(&self) -> &Path {
        &self.file
    }

    pub fn load(&self) -> Vec<TurnEdit> {
        let text = std::fs::read_to_string(&self.file).unwrap_or_default();
        text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
    }

    pub fn save(&self, recs: &[TurnEdit]) -> std::io::Result<()> {
        let mut out = String::new();
        for r in recs {
            if let Ok(line) = serde_json::to_string(r) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        write_atomic(&self.file, out.as_bytes())
    }

    fn read_index(&self) -> Index {
        std::fs::read(&self.index).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn ingest(&self, src: &Sources, now: i64, clean: &dyn Clean) -> std::io::Result<Vec<TurnEdit>> {
        self.ingest_since(src, now, 0, clean)
    }

    pub fn ingest_since(&self, src: &Sources, now: i64, since: i64, clean: &dyn Clean) -> std::io::Result<Vec<TurnEdit>> {
        let cutoff = now - KEEP_MS;
        let mut index = self.read_index();
        let found = claude_files(&src.projects, &self.root).into_iter().map(|p| (p, Agent::Claude)).chain(codex_files(&src.sessions, cutoff).into_iter().map(|p| (p, Agent::Codex)));
        let mut fresh: Vec<TurnEdit> = Vec::new();
        let mut live: HashSet<String> = HashSet::new();
        let mut touched = false;
        for (path, agent) in found {
            let Ok(md) = std::fs::metadata(&path) else { continue };
            let mtime = mtime_ms(&md);
            if mtime < cutoff {
                continue;
            }
            let key = path.to_string_lossy().into_owned();
            live.insert(key.clone());
            if mtime < since {
                continue;
            }
            let size = md.len();
            let prev = index.files.get(&key).cloned();
            if prev.as_ref().is_some_and(|p| p.skip || p.size == size) {
                continue;
            }
            let mut seen = match prev {
                Some(p) => p,
                None => {
                    let cwd = head_cwd(&path, agent);
                    if cwd.is_empty() {
                        continue;
                    }
                    let id = match agent {
                        Agent::Claude => path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned(),
                        Agent::Codex => rollout_id(&path).unwrap_or_default(),
                    };
                    let skip = id.is_empty() || !under(&cwd, &self.root);
                    Seen { size: 0, done: 0, cwd, id, skip }
                }
            };
            touched = true;
            if seen.skip {
                seen.size = size;
                index.files.insert(key, seen);
                continue;
            }
            let start = if seen.size == 0 || size < seen.size { size.saturating_sub(FIRST_SCAN) } else { seen.done };
            let meta = Meta { session_id: &seen.id, agent, cwd: &seen.cwd, root: &self.root, transcript: &key };
            let Ok((recs, done)) = scan(&path, start, size, &meta, clean) else { continue };
            fresh.extend(recs);
            seen.size = size;
            seen.done = done;
            index.files.insert(key, seen);
        }
        let known = index.files.len();
        index.files.retain(|k, _| live.contains(k));
        touched |= known != index.files.len();
        let old = self.load();
        let had = old.len();
        let grew = !fresh.is_empty();
        let mut all = old;
        all.extend(fresh);
        let all = compact(all, now);
        if grew || all.len() != had {
            self.save(&all)?;
        }
        if touched {
            if let Ok(bytes) = serde_json::to_vec(&index) {
                write_atomic(&self.index, &bytes)?;
            }
        }
        Ok(all)
    }
}
