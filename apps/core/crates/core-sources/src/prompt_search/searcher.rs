use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use core_store::persist::paths::{cwd_string, resolve, Style};
use core_store::time::now_ms;
use core_transcript::jsstr::{iso_of_ms, js_space, js_trim};
use serde_json::{json, Map, Value};

use super::scan::{scan_transcript, ScanOptions};
use super::text::{project_dir_name, prompt_search_terms, PROMPT_SEARCH_MAX_SESSIONS};
use crate::mask::mask_secrets;
use crate::resources::sampler::{home_dir, host_platform};

const DAY_MS: f64 = 86_400_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub file: PathBuf,
    pub session_id: String,
    pub mtime_ms: f64,
}

pub struct PromptSearchRequest {
    pub search_id: String,
    pub query: String,
    pub days: u32,
    pub project: Option<String>,
}

pub type OpenFolder = Box<dyn Fn(&str, &str) -> &'static str + Send + Sync>;

pub struct SearcherDeps {
    pub now: Option<Box<dyn Fn() -> f64 + Send + Sync>>,
    pub home: String,
    pub platform: &'static str,
    pub open_folder: Option<OpenFolder>,
    pub progress_ms: f64,
    pub yield_fn: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Default for SearcherDeps {
    fn default() -> Self {
        Self { now: None, home: home_dir(), platform: host_platform(), open_folder: None, progress_ms: 250.0, yield_fn: None }
    }
}

struct Run {
    search_id: String,
    cancelled: AtomicBool,
}

struct Found {
    search_id: String,
    cwds: HashMap<String, String>,
}

pub struct PromptSearcher {
    projects_dir: PathBuf,
    deps: SearcherDeps,
    running: Mutex<HashMap<u64, Arc<Run>>>,
    found: Mutex<HashMap<u64, Found>>,
}

fn is_session_id(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5 && [8, 4, 4, 4, 12].iter().zip(&groups).all(|(n, g)| g.len() == *n && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn mtime_ms(meta: &fs::Metadata) -> Option<f64> {
    let d = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(d.as_secs() as f64 * 1000.0 + f64::from(d.subsec_nanos()) / 1e6)
}

pub fn list_transcripts(projects_dir: &Path, since_ms: f64, project_dir: Option<&str>) -> Vec<Candidate> {
    let Ok(dirs) = fs::read_dir(projects_dir) else { return Vec::new() };
    let mut out: Vec<Candidate> = Vec::new();
    for d in dirs.flatten() {
        if !d.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let name = d.file_name().to_string_lossy().into_owned();
        if let Some(p) = project_dir {
            if name != p && !name.starts_with(&format!("{p}-")) {
                continue;
            }
        }
        let dir = projects_dir.join(&name);
        let Ok(files) = fs::read_dir(&dir) else { continue };
        for f in files.flatten() {
            if !f.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let fname = f.file_name().to_string_lossy().into_owned();
            let Some(session_id) = fname.strip_suffix(".jsonl") else { continue };
            if !is_session_id(session_id) {
                continue;
            }
            let file = dir.join(&fname);
            let Some(m) = fs::metadata(&file).ok().and_then(|m| mtime_ms(&m)) else { continue };
            if m < since_ms {
                continue;
            }
            out.push(Candidate { file, session_id: session_id.to_owned(), mtime_ms: m });
        }
    }
    out.sort_by(|a, b| b.mtime_ms.total_cmp(&a.mtime_ms));
    out
}

fn same_or_under(dir: &str, root: &str, win: bool) -> bool {
    let cwd = cwd_string();
    let a = resolve(dir, Style::HOST, &cwd);
    let b = resolve(root, Style::HOST, &cwd);
    let (x, y) = if win { (a.to_lowercase(), b.to_lowercase()) } else { (a, b) };
    let sep = std::path::MAIN_SEPARATOR;
    let prefix = if y.ends_with(sep) { y.clone() } else { format!("{y}{sep}") };
    x == y || x.starts_with(&prefix)
}

fn hit_json(c: &Candidate, s: &super::scan::SessionScan, cwd: &str, exists: bool, title: Option<String>) -> Value {
    let mut m = Map::new();
    m.insert("sessionId".into(), json!(c.session_id));
    m.insert("cwd".into(), json!(cwd));
    m.insert("cwdExists".into(), json!(exists));
    m.insert("lastAt".into(), json!(iso_of_ms(c.mtime_ms).unwrap_or_default()));
    m.insert("matches".into(), json!(s.matches));
    m.insert("snippets".into(), json!(s.snippets));
    if let Some(v) = s.started_at.as_ref().filter(|v| !v.is_empty()) {
        m.insert("startedAt".into(), json!(v));
    }
    if let Some(v) = s.git_branch.as_ref().filter(|v| !v.is_empty()) {
        m.insert("gitBranch".into(), json!(v));
    }
    if let Some(v) = title.filter(|v| !v.is_empty()) {
        m.insert("title".into(), json!(v));
    }
    Value::Object(m)
}

fn clean_title(title: &str) -> String {
    let mut out = String::new();
    let mut in_space = false;
    for c in title.chars() {
        if js_space(c) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    js_trim(&out).to_owned()
}

impl PromptSearcher {
    pub fn new(projects_dir: impl Into<PathBuf>, deps: SearcherDeps) -> Self {
        Self { projects_dir: projects_dir.into(), deps, running: Mutex::new(HashMap::new()), found: Mutex::new(HashMap::new()) }
    }

    fn now(&self) -> f64 {
        self.deps.now.as_ref().map_or_else(|| now_ms() as f64, |f| f())
    }

    pub fn start(&self, client: u64, req: &PromptSearchRequest, reply: &mut dyn FnMut(Value)) -> &'static str {
        self.cancel(client, None);
        let run = Arc::new(Run { search_id: req.search_id.clone(), cancelled: AtomicBool::new(false) });
        self.running.lock().unwrap().insert(client, run.clone());
        self.found.lock().unwrap().insert(client, Found { search_id: req.search_id.clone(), cwds: HashMap::new() });
        let terms = prompt_search_terms(&req.query);
        let started = self.now();
        let win = self.deps.platform == "win32";
        let home = self.deps.home.clone();
        let platform = self.deps.platform;
        let mask = move |t: &str| mask_secrets(t, &home, platform);
        let mut files_done = 0u64;
        let mut files_total = 0u64;
        let mut sessions = 0u64;
        let mut state = "done";
        let mut pending: Vec<Value> = Vec::new();
        let mut sent_at = 0.0;
        let search_id = req.search_id.clone();
        let mut flush = |force: bool, done: u64, total: u64, pending: &mut Vec<Value>, now: f64| {
            if !force && now - sent_at < self.deps.progress_ms {
                return;
            }
            sent_at = now;
            reply(json!({"type": "promptSearch.progress", "payload": {"searchId": search_id, "filesDone": done, "filesTotal": total, "hits": std::mem::take(pending)}}));
        };
        'work: {
            if terms.is_empty() {
                state = "failed";
                break 'work;
            }
            let since = if req.days == 0 { 0.0 } else { started - f64::from(req.days) * DAY_MS };
            let files = list_transcripts(&self.projects_dir, since, req.project.as_deref().map(project_dir_name).as_deref());
            files_total = files.len() as u64;
            flush(true, files_done, files_total, &mut pending, self.now());
            for c in &files {
                if run.cancelled.load(Ordering::SeqCst) {
                    break;
                }
                let cancelled = || run.cancelled.load(Ordering::SeqCst);
                let scan = scan_transcript(&c.file, &ScanOptions { terms: &terms, mask: &mask, cancelled: &cancelled }).ok();
                files_done += 1;
                if run.cancelled.load(Ordering::SeqCst) {
                    break;
                }
                if let Some(s) = scan.filter(|s| s.matches > 0 && s.cwd.is_some()) {
                    let cwd = s.cwd.clone().unwrap_or_default();
                    let under = req.project.as_ref().is_none_or(|p| same_or_under(&cwd, p, win));
                    if under {
                        let title = s.custom_title.clone().or_else(|| s.ai_title.clone()).map(|t| {
                            let m = mask(&clean_title(&t));
                            core_transcript::jsstr::js_slice(&m, 120).to_owned()
                        });
                        pending.push(hit_json(c, &s, &cwd, Path::new(&cwd).is_dir(), title));
                        if let Some(f) = self.found.lock().unwrap().get_mut(&client) {
                            f.cwds.insert(c.session_id.clone(), cwd);
                        }
                        sessions += 1;
                        if sessions as usize >= PROMPT_SEARCH_MAX_SESSIONS {
                            state = "capped";
                            break;
                        }
                    }
                }
                flush(false, files_done, files_total, &mut pending, self.now());
                if let Some(y) = &self.deps.yield_fn {
                    y();
                }
            }
            if run.cancelled.load(Ordering::SeqCst) {
                state = "cancelled";
            }
        }
        if state != "cancelled" && (!pending.is_empty() || files_total > 0) {
            flush(true, files_done, files_total, &mut pending, self.now());
        }
        {
            let mut running = self.running.lock().unwrap();
            if running.get(&client).is_some_and(|r| Arc::ptr_eq(r, &run)) {
                running.remove(&client);
            }
        }
        reply(json!({"type": "promptSearch.done", "payload": {"searchId": req.search_id, "state": state, "filesDone": files_done, "filesTotal": files_total, "sessions": sessions}}));
        state
    }

    pub fn cancel(&self, client: u64, search_id: Option<&str>) {
        let mut running = self.running.lock().unwrap();
        let Some(run) = running.get(&client) else { return };
        if search_id.is_some_and(|s| run.search_id != s) {
            return;
        }
        run.cancelled.store(true, Ordering::SeqCst);
        running.remove(&client);
    }

    pub fn drop_client(&self, client: u64) {
        self.cancel(client, None);
        self.found.lock().unwrap().remove(&client);
    }

    pub fn open(&self, client: u64, search_id: &str, session_id: &str, reply: &mut dyn FnMut(Value)) {
        let cwd = self.found.lock().unwrap().get(&client).filter(|f| f.search_id == search_id).and_then(|f| f.cwds.get(session_id).cloned());
        let result = match (&cwd, &self.deps.open_folder) {
            (None, _) => "notFound",
            (Some(_), None) => "unsupported",
            (Some(dir), Some(open)) => open(dir, "explorer"),
        };
        reply(json!({"type": "promptSearch.openResult", "payload": {"searchId": search_id, "sessionId": session_id, "result": result}}));
    }
}
