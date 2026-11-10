use crate::extract::{extract_signals, MAX_ENUM};
use crate::jsonline::deep_signals;
use crate::jsstr::js_slice;
use crate::omap::OMap;
use crate::signal::Signal;
use crate::spans::ToolSpanLog;
use crate::state::TranscriptState;
use crate::tail::{Item, JsonlTailer, TailOptions, CHUNK_BYTES};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

mod read;
pub mod serve;
mod subs;
mod view;

#[cfg(test)]
mod tests;

pub const MAX_LINE: usize = 64 * 1024 * 1024;

#[derive(Default, Clone)]
pub struct Plan {
    pub all: bool,
    pub unresolved: bool,
    pub files: HashSet<String>,
}

#[derive(Default)]
pub struct Out {
    pub activity: Vec<Value>,
    pub spans: Vec<Value>,
    pub inbound: Vec<Value>,
    pub sent: Vec<Value>,
}

#[derive(Default, Clone)]
pub struct Meta {
    pub agent_type: Option<String>,
    pub description: Option<String>,
    pub tool_use_id: Option<String>,
    pub spawn_depth: Option<f64>,
    pub parent_agent_id: Option<String>,
    pub model: Option<String>,
}

pub struct Sub {
    pub agent_id: String,
    pub meta: Meta,
    pub tailer: JsonlTailer,
    pub state: TranscriptState,
    pub started_at: Option<String>,
    pub status: &'static str,
    pub resolved: bool,
    pub completed_at: Option<String>,
}

#[derive(Default)]
pub struct Links {
    pub notifications: HashMap<String, (String, Option<String>)>,
    pub foreground: HashMap<String, (bool, Option<String>)>,
    pub agent_tool_uses: HashSet<String>,
    pub msg_in: HashMap<String, HashSet<String>>,
    pub msg_out: HashMap<String, HashSet<String>>,
}

pub struct SyncArgs {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub epoch: f64,
    pub catch_up: bool,
    pub plan: Plan,
    pub now: i64,
}

enum Stage {
    Main,
    Subs { order: Vec<String>, at: usize, lite: Option<bool> },
}

struct Run {
    sig: String,
    catch_up: bool,
    plan: Plan,
    now: i64,
    read_main: bool,
    fresh: HashSet<String>,
    stage: Stage,
}

pub struct Tracker {
    pub terminal_id: String,
    main_agent_id: String,
    projects_dir: PathBuf,
    session_id: Option<String>,
    cwd: Option<String>,
    epoch: Option<f64>,
    transcript: Option<PathBuf>,
    main_tailer: Option<JsonlTailer>,
    main: TranscriptState,
    subs: OMap<Sub>,
    links: Links,
    spans: ToolSpanLog,
    run: Option<Run>,
    full_view: bool,
    dirty_main: bool,
    dirty: HashSet<String>,
    sent_main: Map<String, Value>,
    sent_subs: HashMap<String, Map<String, Value>>,
    pub reread_finished: bool,
}

pub fn tailer(file: PathBuf) -> JsonlTailer {
    JsonlTailer::new(file, CHUNK_BYTES, TailOptions { from_end: None, max_line: Some(MAX_LINE) })
}

pub fn signals_of(item: Item) -> Vec<Signal> {
    match item {
        Item::Record(v) => extract_signals(&v),
        Item::Deep(text) => deep_signals(text),
        Item::Branch(b) => vec![Signal::Branch { branch: js_slice(&b, MAX_ENUM).to_owned() }],
    }
}

fn sig_of(a: &SyncArgs) -> String {
    let mut files: Vec<&String> = a.plan.files.iter().collect();
    files.sort();
    format!("{:?}|{:?}|{}|{}|{}|{}|{:?}", a.session_id, a.cwd, a.epoch, a.catch_up, a.plan.all, a.plan.unresolved, files)
}

pub enum Step {
    Done,
    More,
}

impl Tracker {
    pub fn new(terminal_id: &str, projects_dir: PathBuf) -> Self {
        Tracker {
            terminal_id: terminal_id.to_owned(),
            main_agent_id: format!("{terminal_id}:main"),
            projects_dir,
            session_id: None,
            cwd: None,
            epoch: None,
            transcript: None,
            main_tailer: None,
            main: TranscriptState::default(),
            subs: OMap::new(),
            links: Links::default(),
            spans: ToolSpanLog::with_drops(terminal_id),
            run: None,
            full_view: true,
            dirty_main: false,
            dirty: HashSet::new(),
            sent_main: Map::new(),
            sent_subs: HashMap::new(),
            reread_finished: false,
        }
    }

    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    fn reset_session(&mut self) {
        self.transcript = None;
        self.main_tailer = None;
        self.main = TranscriptState::default();
        self.subs.clear();
        self.links = Links::default();
        self.full_view = true;
        self.dirty_main = false;
        self.dirty.clear();
        self.sent_main.clear();
        self.sent_subs.clear();
    }

    fn begin(&mut self, a: SyncArgs) -> bool {
        if self.epoch.is_some_and(|e| e != a.epoch) {
            self.reset_session();
        }
        self.epoch = Some(a.epoch);
        let sig = sig_of(&a);
        self.session_id = a.session_id;
        self.cwd = a.cwd;
        let Some(sid) = self.session_id.clone().filter(|s| !s.is_empty()) else { return false };
        let mut read_main = a.plan.all || a.plan.unresolved || a.plan.files.contains(&format!("{sid}.jsonl"));
        if self.transcript.is_none() {
            let Some(found) = subs::find_transcript(&self.projects_dir, &sid, self.cwd.as_deref()) else { return false };
            self.main_tailer = Some(tailer(found.clone()));
            self.transcript = Some(found);
            read_main = true;
        }
        self.run = Some(Run { sig, catch_up: a.catch_up, plan: a.plan, now: a.now, read_main, fresh: HashSet::new(), stage: Stage::Main });
        true
    }

    pub fn sync(&mut self, args: Option<SyncArgs>, budget: u64, out: &mut Out) -> Step {
        if let (Some(run), Some(a)) = (&self.run, &args) {
            if run.sig != sig_of(a) {
                self.run = None;
            }
        }
        if self.run.is_none() {
            let Some(a) = args else { return Step::Done };
            if !self.begin(a) {
                return Step::Done;
            }
        }
        let Some(mut run) = self.run.take() else { return Step::Done };
        let mut left = budget.max(1);
        if let Stage::Main = run.stage {
            if run.read_main && self.read_main(&run, out, &mut left) {
                self.run = Some(run);
                return Step::More;
            }
            run.fresh = self.discover(run.catch_up, out);
            run.stage = Stage::Subs { order: self.subs.iter().map(|(k, _)| k.to_owned()).collect(), at: 0, lite: None };
        }
        if let Stage::Subs { order, at, lite } = &mut run.stage {
            while *at < order.len() {
                let id = order[*at].clone();
                let wanted = self.subs.get(&id).is_some_and(|e| run.plan.all || run.fresh.contains(&id) || run.plan.files.contains(&format!("agent-{id}.jsonl")) || (run.plan.unresolved && !e.resolved) || (self.reread_finished && self.about_to_resolve(e)));
                if wanted {
                    let l = match *lite {
                        Some(l) => l,
                        None => run.catch_up && self.subs.get(&id).is_some_and(|e| !e.resolved) && self.is_lite_catch_up(&id, run.now),
                    };
                    if self.read_sub(&id, l, run.catch_up, run.now, out, &mut left) {
                        *lite = Some(l);
                        self.run = Some(run);
                        return Step::More;
                    }
                }
                *lite = None;
                *at += 1;
            }
        }
        let ids: Vec<String> = self.subs.iter().map(|(k, _)| k.to_owned()).collect();
        for id in ids {
            self.resolve(&id, run.catch_up, out);
        }
        self.spans.prune(run.now);
        self.push_drops(out);
        Step::Done
    }

    fn push_drops(&mut self, out: &mut Out) {
        for id in self.spans.take_dropped() {
            out.spans.push(serde_json::json!({ "drop": id }));
        }
    }

    pub fn close_spans(&mut self, at: &str) -> Vec<Value> {
        self.spans.close_open(at, None).iter().map(|s| s.to_json()).collect()
    }

    pub fn take_view(&mut self) -> Option<Value> {
        let full = std::mem::take(&mut self.full_view);
        let main = std::mem::take(&mut self.dirty_main);
        let dirty = std::mem::take(&mut self.dirty);
        if !full && !main && dirty.is_empty() {
            return None;
        }
        self.view_of(full, main, &dirty)
    }
}
