use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_store::diagnostics::CodexDiagnostics;
use core_store::model::AgentStatus;
use core_store::persist::forecast::LimitHistory;
use core_store::time::now_ms;
use core_store::Store;
use core_transcript::codex::extract::{Limits, Meta};
use core_transcript::codex::tail::RolloutTailer;
use indexmap::IndexMap;

use super::state::{CodexMode, CodexThreadState};
use super::threads::file_id;
use crate::agentprocs::AgentProcs;
use crate::fswatch::{FsEvent, FsWatch};
use crate::monitor::{ClaudeTarget, Deps};
use crate::watch::Wake;

pub type Targets<'a> = &'a dyn Fn(&Store) -> Vec<ClaudeTarget>;

#[derive(Debug, Clone)]
pub struct CodexConfig {
    pub codex_home: PathBuf,
    pub stale_ttl_ms: i64,
    pub reconcile_ms: i64,
    pub verify_ms: i64,
    pub verify_processes: bool,
}

#[derive(Debug, Clone)]
pub struct Published {
    pub terminal_id: String,
    pub agent_id: String,
    pub standalone: bool,
}

pub struct Thread {
    pub id: String,
    pub meta: Meta,
    pub mode: CodexMode,
    pub state: CodexThreadState,
    pub tailer: RolloutTailer,
    pub proc_: Option<(u32, i64)>,
    pub proc_gone: bool,
    pub superseded: bool,
    pub link: Option<(String, String)>,
    pub awaiting_scan: bool,
    pub initial: bool,
    pub last_write_at: i64,
    pub ended_at: Option<String>,
    pub finished_at: Option<i64>,
    pub published: Option<Published>,
    pub last_status: Option<AgentStatus>,
}

#[derive(Clone, Copy)]
struct Timers {
    tick: i64,
    verify: i64,
}

pub const IDLE_END_MS: i64 = 15 * 60_000;
const SCAN_DEBOUNCE_MS: i64 = 300;
const SCAN_WAIT_MS: i64 = 3_000;
const SYNC_DEBOUNCE_MS: i64 = 150;
const MAX_FINISHED_PER_TERMINAL: usize = 20;
pub(super) const NAMES_FILE: &str = "session_index.jsonl";

pub struct CodexMonitor {
    pub(super) config: CodexConfig,
    pub(super) deps: Arc<dyn Deps>,
    pub(super) procs: Arc<dyn AgentProcs>,
    pub(super) limit_history: Option<LimitHistory>,
    pub(super) threads: IndexMap<String, Thread>,
    pub(super) skipped: HashSet<String>,
    pub(super) resume_searched: HashSet<String>,
    pub(super) sessions_dir: PathBuf,
    pub(super) index_file: PathBuf,
    pub(super) names: HashMap<String, String>,
    watch: Option<FsWatch>,
    wake: Option<Wake>,
    watching: bool,
    sessions_watched: bool,
    home_watched: bool,
    sync_due: IndexMap<String, (i64, PathBuf)>,
    scan_due: Option<i64>,
    publish_due: IndexMap<String, i64>,
    timers: Option<Timers>,
    pub(super) processes: usize,
    pub(super) last_scan_ok: bool,
    pub(super) limits: Option<Limits>,
    pub(super) booting: bool,
    stopped: bool,
}

impl CodexMonitor {
    pub fn new(config: CodexConfig, deps: Arc<dyn Deps>, procs: Arc<dyn AgentProcs>, limit_history: Option<LimitHistory>) -> Self {
        CodexMonitor {
            sessions_dir: config.codex_home.join("sessions"),
            index_file: config.codex_home.join(NAMES_FILE),
            config,
            deps,
            procs,
            limit_history,
            threads: IndexMap::new(),
            skipped: HashSet::new(),
            resume_searched: HashSet::new(),
            names: HashMap::new(),
            watch: None,
            wake: None,
            watching: false,
            sessions_watched: false,
            home_watched: false,
            sync_due: IndexMap::new(),
            scan_due: None,
            publish_due: IndexMap::new(),
            timers: None,
            processes: 0,
            last_scan_ok: false,
            limits: None,
            booting: true,
            stopped: false,
        }
    }

    pub fn is_codex_terminal(terminal_id: &str) -> bool {
        terminal_id.starts_with("codex:")
    }

    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    pub fn start(&mut self, store: &mut Store, targets: Targets, watch: bool) {
        self.load_names();
        self.discover_initial(store, targets);
        if !watch {
            return;
        }
        self.watching = true;
        self.setup_watchers();
        let wall = now_ms();
        self.timers = Some(Timers { tick: wall + self.config.reconcile_ms, verify: wall + self.config.verify_ms });
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        self.watch = None;
        self.timers = None;
        self.sync_due.clear();
        self.publish_due.clear();
        self.scan_due = None;
    }

    pub fn diagnostics(&self) -> CodexDiagnostics {
        CodexDiagnostics { home: self.config.codex_home.to_string_lossy().into_owned(), watching: self.sessions_watched, threads: self.threads.len() as u64, processes: self.processes as u64 }
    }

    fn standalone(&self, terminal_id: &str) -> Option<&Thread> {
        let id = terminal_id.strip_prefix("codex:")?;
        self.threads.get(id).filter(|t| t.published.as_ref().is_some_and(|p| p.standalone))
    }

    pub fn pid_of(&self, terminal_id: &str) -> Option<u32> {
        self.standalone(terminal_id).filter(|t| t.ended_at.is_none() && !t.proc_gone).and_then(|t| t.proc_.map(|p| p.0))
    }

    pub fn cwd_of(&self, terminal_id: &str) -> Option<String> {
        self.standalone(terminal_id).and_then(|t| t.meta.cwd.clone())
    }

    pub fn is_live(&self, terminal_id: &str) -> bool {
        self.standalone(terminal_id).is_some_and(|t| t.ended_at.is_none())
    }

    pub fn dismiss_ended(&mut self, store: &mut Store, terminal_id: Option<&str>) -> usize {
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        let mut removed = 0;
        for id in ids {
            let Some(t) = self.threads.get(&id) else { continue };
            let Some(p) = t.published.as_ref().filter(|p| p.standalone) else { continue };
            if t.ended_at.is_none() || terminal_id.is_some_and(|x| x != p.terminal_id) {
                continue;
            }
            self.forget(store, &id);
            removed += 1;
        }
        removed
    }

    pub(super) fn live_count(&self) -> usize {
        self.threads.values().filter(|t| t.ended_at.is_none() && t.finished_at.is_none()).count()
    }

    fn setup_watchers(&mut self) {
        if self.stopped || !self.watching {
            return;
        }
        let wake = self.wake.clone();
        let w = self.watch.get_or_insert_with(|| FsWatch::new(wake));
        if !self.sessions_watched {
            self.sessions_watched = w.watch("sessions", &self.sessions_dir, true).is_ok();
        }
        if !self.home_watched {
            self.home_watched = w.watch("home", &self.config.codex_home, false).is_ok();
        }
    }

    fn on_rollout_change(&mut self, path: &Path, wall: i64) {
        let Some(id) = file_id(&path.to_string_lossy()) else { return };
        if self.skipped.contains(&id) || self.sync_due.contains_key(&id) || self.stopped {
            return;
        }
        self.sync_due.insert(id, (wall + SYNC_DEBOUNCE_MS, path.to_path_buf()));
    }

    pub fn sync(&mut self, store: &mut Store, id: &str, file: &Path) {
        if !self.threads.contains_key(id) {
            let Some(tid) = self.add_thread(store, file, false, None) else { return };
            let now = self.deps.now();
            let Some(t) = self.threads.get_mut(&tid) else { return };
            if !t.awaiting_scan {
                return;
            }
            t.last_write_at = now;
            self.schedule_scan(SCAN_DEBOUNCE_MS);
            self.publish_due.insert(tid, now_ms() + SCAN_WAIT_MS);
            return;
        }
        let (drafts, turn_started) = self.read(store, id, false);
        if turn_started && self.threads.get(id).is_some_and(|t| t.ended_at.is_some() || t.proc_gone || t.superseded) {
            self.reopen(id);
        }
        self.publish(store, id, drafts);
    }

    fn reopen(&mut self, id: &str) {
        if let Some(t) = self.threads.get_mut(id) {
            t.ended_at = None;
            t.finished_at = None;
            t.proc_gone = false;
            t.superseded = false;
            t.proc_ = None;
        }
        self.schedule_scan(SCAN_DEBOUNCE_MS);
    }

    pub(super) fn schedule_scan(&mut self, delay: i64) {
        if self.scan_due.is_some() || self.stopped {
            return;
        }
        self.scan_due = Some(now_ms() + delay);
    }

    pub fn scan_now(&mut self, store: &mut Store, targets: Targets) {
        self.scan_once(store, targets);
    }

    pub fn next_due(&self) -> Option<i64> {
        let mut due: Vec<i64> = Vec::new();
        if let Some(t) = &self.timers {
            due.extend([t.tick, t.verify]);
        }
        due.extend(self.sync_due.values().map(|(d, _)| *d));
        due.extend(self.scan_due);
        due.extend(self.publish_due.values().copied());
        due.extend(self.limit_history.as_ref().and_then(LimitHistory::next_flush_at));
        due.into_iter().min()
    }

    pub fn tick(&mut self, store: &mut Store, targets: Targets) {
        if self.stopped {
            return;
        }
        let wall = now_ms();
        let events = self.watch.as_mut().map(FsWatch::drain).unwrap_or_default();
        let mut names_changed = false;
        for e in events {
            match e {
                FsEvent::Changed("sessions", p) => self.on_rollout_change(&p, wall),
                FsEvent::Changed("home", p) => names_changed |= p.file_name().is_some_and(|n| n == NAMES_FILE),
                FsEvent::Lost("sessions") => self.sessions_watched = false,
                FsEvent::Lost("home") => self.home_watched = false,
                _ => {}
            }
        }
        if names_changed {
            self.reload_names(store);
        }
        let due: Vec<String> = self.sync_due.iter().filter(|(_, (d, _))| *d <= wall).map(|(k, _)| k.clone()).collect();
        for id in due {
            let Some((_, file)) = self.sync_due.shift_remove(&id) else { continue };
            self.sync(store, &id, &file);
        }
        let waits: Vec<String> = self.publish_due.iter().filter(|(_, d)| **d <= wall).map(|(k, _)| k.clone()).collect();
        for id in waits {
            self.publish_due.shift_remove(&id);
            if let Some(t) = self.threads.get_mut(&id).filter(|t| t.awaiting_scan) {
                t.awaiting_scan = false;
                self.publish(store, &id, Vec::new());
            }
        }
        if self.scan_due.is_some_and(|d| d <= wall) {
            self.scan_due = None;
            self.scan_now(store, targets);
        }
        if let Some(mut t) = self.timers {
            if t.tick <= wall {
                t.tick = wall + self.config.reconcile_ms;
                self.timers = Some(t);
                self.reconcile(store);
            }
            if t.verify <= wall {
                t.verify = wall + self.config.verify_ms;
                self.timers = Some(t);
                if self.live_count() > 0 {
                    self.schedule_scan(0);
                }
            }
        }
        if let Some(h) = self.limit_history.as_mut() {
            h.poll(wall);
        }
    }

    pub fn flush_persist(&mut self) {
        if let Some(h) = self.limit_history.as_mut() {
            h.save();
        }
    }

    pub fn reconcile(&mut self, store: &mut Store) {
        if self.stopped {
            return;
        }
        if self.watching && (!self.sessions_watched || !self.home_watched) {
            self.setup_watchers();
        }
        let now = self.deps.now();
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            let Some(t) = self.threads.get_mut(&id) else { continue };
            if let Some((pid, _)) = t.proc_ {
                if !t.proc_gone && !self.deps.is_pid_alive(pid) {
                    t.proc_gone = true;
                }
            }
            self.publish(store, &id, Vec::new());
            let Some(t) = self.threads.get(&id) else { continue };
            let ended = t.ended_at.as_deref().and_then(core_transcript::time::parse_iso);
            let ttl = self.config.stale_ttl_ms;
            if ended.is_some_and(|e| now - e >= ttl) || t.finished_at.is_some_and(|f| now - f >= ttl) {
                self.forget(store, &id);
            }
        }
        self.prune_finished(store);
    }

    fn prune_finished(&mut self, store: &mut Store) {
        let mut by_terminal: IndexMap<String, Vec<(String, i64)>> = IndexMap::new();
        for t in self.threads.values() {
            let (Some(link), Some(f)) = (&t.link, t.finished_at) else { continue };
            by_terminal.entry(link.0.clone()).or_default().push((t.id.clone(), f));
        }
        for mut list in by_terminal.into_values() {
            if list.len() <= MAX_FINISHED_PER_TERMINAL {
                continue;
            }
            list.sort_by_key(|x| std::cmp::Reverse(x.1));
            for (id, _) in list.into_iter().skip(MAX_FINISHED_PER_TERMINAL) {
                self.forget(store, &id);
            }
        }
    }
}
