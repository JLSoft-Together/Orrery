use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_store::diagnostics::DeepseekDiagnostics;
use core_store::store::NewActivity;
use core_store::time::now_ms;
use core_store::Store;
use core_transcript::time::parse_iso;
use indexmap::IndexMap;

use super::config::outbox_path_from;
use super::process::{match_deepseek_processes, DeepseekThreadKey};
use super::state::DeepseekThreadState;
use super::tail::OutboxTail;
use crate::agentprocs::AgentProcs;
use crate::codex::monitor::IDLE_END_MS;
use crate::fswatch::{FsEvent, FsWatch};
use crate::monitor::Deps;
use crate::watch::Wake;

pub(super) const PREFIX: &str = "deepseek:";
pub(super) const TAIL_BYTES: u64 = 256 * 1024;
pub(super) const MAX_LINE: u64 = 256 * 1024;
pub(super) const SCAN_DEBOUNCE_MS: i64 = 300;
pub(super) const SCAN_WAIT_MS: i64 = 3_000;
pub(super) const READ_DEBOUNCE_MS: i64 = 150;

#[derive(Debug, Clone)]
pub struct DeepseekConfig {
    pub codewhale_home: PathBuf,
    pub home_dir: PathBuf,
    pub stale_ttl_ms: i64,
    pub reconcile_ms: i64,
    pub verify_ms: i64,
    pub verify_processes: bool,
}

pub(super) struct Thread {
    pub(super) id: String,
    pub(super) state: DeepseekThreadState,
    pub(super) first_seen_at: i64,
    pub(super) proc_: Option<(u32, i64)>,
    pub(super) awaiting_scan: bool,
    pub(super) initial: bool,
    pub(super) published: bool,
    pub(super) pending: Vec<NewActivity>,
}

#[derive(Clone, Copy)]
struct Timers {
    tick: i64,
    verify: i64,
}

pub struct DeepseekMonitor {
    pub(super) config: DeepseekConfig,
    pub(super) deps: Arc<dyn Deps>,
    pub(super) procs: Arc<dyn AgentProcs>,
    pub(super) threads: IndexMap<String, Thread>,
    pub(super) outbox: Option<PathBuf>,
    pub(super) outbox_watched: bool,
    pub(super) home_watched: bool,
    pub(super) tailer: Option<OutboxTail>,
    pub(super) file_key: Option<String>,
    pub(super) ever_opened: bool,
    pub(super) watch: Option<FsWatch>,
    pub(super) wake: Option<Wake>,
    pub(super) watching: bool,
    pub(super) read_due: Option<i64>,
    pub(super) scan_due: Option<i64>,
    pub(super) publish_due: Option<i64>,
    timers: Option<Timers>,
    pub(super) processes: usize,
    pub(super) last_scan_ok: bool,
    pub(super) booting: bool,
    pub(super) stopped: bool,
}

fn ms_of(s: Option<&str>) -> Option<i64> {
    s.and_then(parse_iso)
}

impl DeepseekMonitor {
    pub fn new(config: DeepseekConfig, deps: Arc<dyn Deps>, procs: Arc<dyn AgentProcs>) -> Self {
        DeepseekMonitor {
            config,
            deps,
            procs,
            threads: IndexMap::new(),
            outbox: None,
            outbox_watched: false,
            home_watched: false,
            tailer: None,
            file_key: None,
            ever_opened: false,
            watch: None,
            wake: None,
            watching: false,
            read_due: None,
            scan_due: None,
            publish_due: None,
            timers: None,
            processes: 0,
            last_scan_ok: false,
            booting: true,
            stopped: false,
        }
    }

    pub fn is_deepseek_terminal(terminal_id: &str) -> bool {
        terminal_id.starts_with(PREFIX)
    }

    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    pub fn scan_wanted(&self, now: i64) -> bool {
        self.config.verify_processes && !self.threads.is_empty() && self.scan_due.is_some_and(|d| d <= now)
    }

    pub fn start(&mut self, store: &mut Store, watch: bool) {
        self.resolve_outbox();
        self.read_outbox(store, true);
        self.scan_now(store);
        let now = self.deps.now();
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            let Some(t) = self.threads.get(&id) else { continue };
            let last = ms_of(t.state.last_at.as_deref()).unwrap_or(t.first_seen_at);
            let live = t.state.ended_at.is_none() && (t.proc_.is_some() || (!self.last_scan_ok && now - last < IDLE_END_MS));
            if !live {
                self.threads.shift_remove(&id);
            }
        }
        self.booting = false;
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            self.publish(store, &id, Vec::new());
        }
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
        self.read_due = None;
        self.scan_due = None;
        self.publish_due = None;
    }

    pub fn diagnostics(&self) -> DeepseekDiagnostics {
        DeepseekDiagnostics { outbox: self.outbox.as_ref().map(|p| p.to_string_lossy().into_owned()), threads: self.threads.len() as u64, processes: self.processes as u64 }
    }

    fn thread_of(&self, terminal_id: &str) -> Option<&Thread> {
        self.threads.get(terminal_id.strip_prefix(PREFIX)?)
    }

    pub fn pid_of(&self, terminal_id: &str) -> Option<u32> {
        self.thread_of(terminal_id).filter(|t| t.state.ended_at.is_none()).and_then(|t| t.proc_.map(|p| p.0))
    }

    pub fn cwd_of(&self, terminal_id: &str) -> Option<String> {
        self.thread_of(terminal_id).and_then(|t| t.state.workspace.clone())
    }

    pub fn dismiss_ended(&mut self, store: &mut Store, terminal_id: Option<&str>) -> usize {
        let ids: Vec<String> = self.threads.iter().filter(|(id, t)| t.state.ended_at.is_some() && terminal_id.is_none_or(|x| x == format!("{PREFIX}{id}"))).map(|(id, _)| id.clone()).collect();
        for id in &ids {
            self.forget(store, id);
        }
        ids.len()
    }

    pub fn resolve_outbox(&mut self) {
        let text = std::fs::read(self.config.codewhale_home.join("config.toml")).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
        let next = outbox_path_from(text.as_deref(), &self.config.codewhale_home, &self.config.home_dir);
        if next == self.outbox {
            return;
        }
        self.outbox = next;
        self.tailer = None;
        self.file_key = None;
        self.ever_opened = false;
        if let Some(w) = self.watch.as_mut() {
            w.unwatch("outbox");
        }
        self.outbox_watched = false;
        if !self.booting {
            self.setup_watchers();
            self.schedule_read();
        }
    }

    fn live_count(&self) -> usize {
        self.threads.values().filter(|t| t.state.ended_at.is_none()).count()
    }

    fn schedule_read(&mut self) {
        if self.read_due.is_none() && !self.stopped {
            self.read_due = Some(now_ms() + READ_DEBOUNCE_MS);
        }
    }

    pub(super) fn schedule_scan(&mut self, delay: i64) {
        if self.scan_due.is_none() && !self.stopped {
            self.scan_due = Some(now_ms() + delay);
        }
    }

    fn setup_watchers(&mut self) {
        if self.stopped || !self.watching {
            return;
        }
        let wake = self.wake.clone();
        let w = self.watch.get_or_insert_with(|| FsWatch::new(wake));
        if !self.home_watched {
            self.home_watched = w.watch("home", &self.config.codewhale_home, false).is_ok();
        }
        let Some(file) = self.outbox.clone() else { return };
        if self.outbox_watched {
            return;
        }
        let dir = file.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        if w.watch("outbox", &dir, false).is_ok() {
            self.outbox_watched = true;
            self.schedule_read();
        }
    }

    pub fn next_due(&self) -> Option<i64> {
        let mut due: Vec<i64> = Vec::new();
        if let Some(t) = &self.timers {
            due.extend([t.tick, t.verify]);
        }
        due.extend(self.read_due);
        due.extend(self.scan_due);
        due.extend(self.publish_due);
        due.into_iter().min()
    }

    pub fn tick(&mut self, store: &mut Store) {
        if self.stopped {
            return;
        }
        let wall = now_ms();
        let events = self.watch.as_mut().map(FsWatch::drain).unwrap_or_default();
        let mut config_changed = false;
        let base = self.outbox.as_ref().and_then(|f| f.file_name().map(|n| n.to_owned()));
        for e in events {
            match e {
                FsEvent::Changed("home", p) => config_changed |= p.file_name().is_some_and(|n| n == "config.toml"),
                FsEvent::Changed("outbox", p) => {
                    if p.file_name().is_none() || p.file_name() == base.as_deref() {
                        self.schedule_read();
                    }
                }
                FsEvent::Lost("home") => self.home_watched = false,
                FsEvent::Lost("outbox") => self.outbox_watched = false,
                _ => {}
            }
        }
        if config_changed {
            self.resolve_outbox();
        }
        if self.read_due.is_some_and(|d| d <= wall) {
            self.read_due = None;
            self.read_outbox(store, false);
        }
        if self.publish_due.is_some_and(|d| d <= wall) {
            self.publish_due = None;
            let ids: Vec<String> = self.threads.keys().cloned().collect();
            for id in ids {
                if self.threads.get_mut(&id).is_some_and(|t| std::mem::replace(&mut t.awaiting_scan, false)) {
                    self.publish(store, &id, Vec::new());
                }
            }
        }
        if self.scan_due.is_some_and(|d| d <= wall) {
            self.scan_due = None;
            self.scan_now(store);
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
    }

    pub fn scan_now(&mut self, store: &mut Store) {
        if self.threads.is_empty() {
            return;
        }
        let procs = if self.config.verify_processes { self.procs.deepseek() } else { None };
        self.last_scan_ok = procs.is_some();
        if let Some(procs) = procs {
            self.processes = procs.len();
            let live: Vec<String> = self.threads.values().filter(|t| t.state.ended_at.is_none()).map(|t| t.id.clone()).collect();
            let keys: Vec<DeepseekThreadKey> = live
                .iter()
                .filter_map(|id| self.threads.get(id))
                .map(|t| DeepseekThreadKey {
                    id: t.id.clone(),
                    workspace: t.state.workspace.clone(),
                    started_at: ms_of(t.state.session_start_at.as_deref()).or_else(|| ms_of(t.state.started_at.as_deref())).unwrap_or(t.first_seen_at),
                    pid: t.proc_.map(|p| p.0),
                    start: t.proc_.map(|p| p.1),
                })
                .collect();
            let matches: HashMap<String, _> = match_deepseek_processes(&keys, &procs, cfg!(windows));
            let now = self.deps.now();
            let booting = self.booting;
            for id in &live {
                let Some(t) = self.threads.get_mut(id) else { continue };
                if let Some(p) = matches.get(id) {
                    t.proc_ = Some((p.pid, p.start));
                } else if t.proc_.is_some() || (!booting && (procs.is_empty() || now - ms_of(t.state.last_at.as_deref()).unwrap_or(t.first_seen_at) > IDLE_END_MS)) {
                    Self::end(t, now);
                }
            }
        }
        for t in self.threads.values_mut() {
            t.awaiting_scan = false;
        }
        if !self.booting {
            let ids: Vec<String> = self.threads.keys().cloned().collect();
            for id in ids {
                self.publish(store, &id, Vec::new());
            }
        }
    }

    pub fn reconcile(&mut self, store: &mut Store) {
        if self.stopped {
            return;
        }
        if self.watching && (!self.home_watched || (self.outbox.is_some() && !self.outbox_watched)) {
            self.setup_watchers();
        }
        let now = self.deps.now();
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            let Some(t) = self.threads.get_mut(&id) else { continue };
            if t.state.ended_at.is_none() {
                if let Some((pid, _)) = t.proc_ {
                    if !self.deps.is_pid_alive(pid) {
                        Self::end(t, now);
                    }
                }
            }
            if t.state.ended_at.is_none() && t.proc_.is_none() && now - ms_of(t.state.last_at.as_deref()).unwrap_or(t.first_seen_at) >= self.config.stale_ttl_ms {
                Self::end(t, now);
            }
            self.publish(store, &id, Vec::new());
            let ended = self.threads.get(&id).and_then(|t| ms_of(t.state.ended_at.as_deref()));
            if ended.is_some_and(|e| now - e >= self.config.stale_ttl_ms) {
                self.forget(store, &id);
            }
        }
    }

}
