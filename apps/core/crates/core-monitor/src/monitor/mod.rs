mod actions;
mod agents;
mod deps;
mod extras;
mod publish;
mod reconcile;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use core_store::extra::{BackgroundJob, GitDiffStat};
use core_store::persist::json_file::JsonFile;
use core_store::time::{iso_of_ms, now_ms};
use core_store::Store;
use indexmap::{IndexMap, IndexSet};

use crate::codex::CodexMonitor;
use crate::deepseek::DeepseekMonitor;
use crate::gitstat::DiffStatReader;
use crate::hints::ContextHints;
use crate::job_stop::JobStopResult;
use crate::jobs::JobReader;
use crate::peers::{PeerLinkStore, PEER_TTL_MS};
use crate::sidefiles::HookRecord;
use crate::tracker::{MonTracker, SensitiveCheck, SAFETY_NET_MS};
use crate::watch::{Wake, WatchEvent, Watcher};

pub use actions::{ClaudeTarget, LaunchError, LaunchMode, TerminalApp};
pub use deps::{Deps, MonitorStores, SystemDeps};

pub const DIFF_DEBOUNCE_MS: i64 = 2_000;
pub const RESOLVED_SWEEP_MS: i64 = 60_000;
pub const REGISTRY_DEBOUNCE_MS: i64 = 100;
pub const PROJECT_DEBOUNCE_MS: i64 = 150;

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub claude_root: PathBuf,
    pub data_dir: PathBuf,
    pub stale_ttl_ms: i64,
    pub reconcile_ms: i64,
    pub process_verify_ms: i64,
    pub verify_processes: bool,
    pub peer_ttl_ms: i64,
    pub peer_hold_ms: Option<i64>,
    pub diff_delay_ms: i64,
    pub safety_net_ms: Option<u64>,
    pub reread_finished: bool,
    pub codex_home: Option<PathBuf>,
    pub codewhale_home: Option<PathBuf>,
    pub home_dir: PathBuf,
}

impl MonitorConfig {
    pub fn new(claude_root: PathBuf, data_dir: PathBuf) -> Self {
        MonitorConfig {
            claude_root,
            data_dir,
            stale_ttl_ms: 120 * 60_000,
            reconcile_ms: 5_000,
            process_verify_ms: 60_000,
            verify_processes: true,
            peer_ttl_ms: PEER_TTL_MS,
            peer_hold_ms: None,
            diff_delay_ms: DIFF_DEBOUNCE_MS,
            safety_net_ms: Some(SAFETY_NET_MS),
            reread_finished: true,
            codex_home: None,
            codewhale_home: None,
            home_dir: PathBuf::new(),
        }
    }
}

enum Done {
    Stop(String, String, JobStopResult),
}

pub struct Monitor {
    config: MonitorConfig,
    deps: Arc<dyn Deps>,
    stores: MonitorStores,
    pub hints: ContextHints,
    hints_unset: bool,
    pub sensitive: Option<Box<dyn SensitiveCheck + Send>>,
    trackers: IndexMap<String, MonTracker>,
    rejected: HashSet<String>,
    sessions_dir: PathBuf,
    projects_dir: PathBuf,
    swept_at: HashMap<String, i64>,
    sync_due: IndexMap<String, (i64, IndexSet<String>)>,
    registry_due: Option<i64>,
    diff_due: IndexMap<String, i64>,
    diffs: HashMap<String, Arc<GitDiffStat>>,
    diff_reader: Option<DiffStatReader>,
    sessions_watched: bool,
    projects_watched: bool,
    watcher: Option<Watcher>,
    stopped: bool,
    job_reader: JobReader,
    costs: HashMap<String, f64>,
    hooks: HashMap<String, HookRecord>,
    jobs: IndexMap<String, BackgroundJob>,
    jobs_loaded: bool,
    dismissed_jobs: IndexMap<String, String>,
    stopping_jobs: HashSet<String>,
    done_tx: Sender<Done>,
    done_rx: Receiver<Done>,
    started_at: String,
    status_reports: (u64, Option<String>),
    peers: PeerLinkStore,
    timers: Option<Timers>,
    project_files: Vec<String>,
    wake: Option<Wake>,
    pub codex: Option<CodexMonitor>,
    pub deepseek: Option<DeepseekMonitor>,
}

#[derive(Clone, Copy)]
struct Timers {
    reconcile: i64,
    extras: i64,
    verify: i64,
}

impl Monitor {
    pub fn new(config: MonitorConfig, deps: Arc<dyn Deps>, stores: MonitorStores, diff_reader: Option<DiffStatReader>) -> Self {
        let (done_tx, done_rx) = channel();
        let peers = PeerLinkStore::new(config.peer_ttl_ms, config.peer_hold_ms);
        Monitor {
            sessions_dir: config.claude_root.join("sessions"),
            projects_dir: config.claude_root.join("projects"),
            job_reader: JobReader::new(config.claude_root.join("jobs")),
            config,
            deps,
            stores,
            hints: ContextHints::from_env(),
            hints_unset: true,
            sensitive: None,
            trackers: IndexMap::new(),
            rejected: HashSet::new(),
            swept_at: HashMap::new(),
            sync_due: IndexMap::new(),
            registry_due: None,
            diff_due: IndexMap::new(),
            diffs: HashMap::new(),
            diff_reader,
            sessions_watched: false,
            projects_watched: false,
            watcher: None,
            stopped: false,
            costs: HashMap::new(),
            hooks: HashMap::new(),
            jobs: IndexMap::new(),
            jobs_loaded: false,
            dismissed_jobs: IndexMap::new(),
            stopping_jobs: HashSet::new(),
            done_tx,
            done_rx,
            started_at: iso_of_ms(now_ms()),
            status_reports: (0, None),
            peers,
            timers: None,
            project_files: Vec::new(),
            wake: None,
            codex: None,
            deepseek: None,
        }
    }

    pub fn stores(&mut self) -> &mut MonitorStores {
        &mut self.stores
    }

    pub fn start(&mut self, store: &mut Store, watch: bool) {
        store.set_favorites(self.stores.favorites.list());
        store.set_response(self.stores.response.stats());
        self.refresh_hints(store);
        self.poll_extras(store);
        self.reconcile(store, true, watch);
        if !watch {
            return;
        }
        self.setup_watchers(false);
        let wall = now_ms();
        self.timers = Some(Timers { reconcile: wall + self.config.reconcile_ms, extras: wall + self.config.reconcile_ms, verify: wall + self.config.process_verify_ms });
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        self.peers.stop();
        self.watcher = None;
        self.timers = None;
        self.sync_due.clear();
        self.diff_due.clear();
        self.registry_due = None;
        self.stop_agents();
    }

    pub fn set_wake(&mut self, wake: Wake) {
        self.set_agents_wake(&wake);
        self.wake = Some(wake);
    }

    pub fn take_project_files(&mut self) -> Vec<String> {
        std::mem::take(&mut self.project_files)
    }

    fn setup_watchers(&mut self, quiet: bool) {
        if self.stopped {
            return;
        }
        let wake = self.wake.clone();
        let w = self.watcher.get_or_insert_with(|| wake.map_or_else(Watcher::new, Watcher::with_wake));
        if !self.sessions_watched {
            match w.watch_sessions(&self.sessions_dir) {
                Ok(()) => self.sessions_watched = true,
                Err(e) if !quiet => eprintln!("[collector] cannot watch sessions dir: {e}"),
                Err(_) => {}
            }
        }
        if !self.projects_watched {
            match w.watch_projects(&self.projects_dir) {
                Ok(()) => self.projects_watched = true,
                Err(e) if !quiet => eprintln!("[collector] cannot watch projects dir: {e}"),
                Err(_) => {}
            }
        }
    }

    pub fn next_due(&self) -> Option<i64> {
        let mut due: Vec<i64> = Vec::new();
        if let Some(t) = &self.timers {
            due.extend([t.reconcile, t.extras, t.verify]);
        }
        due.extend(self.registry_due);
        due.extend(self.sync_due.values().map(|(d, _)| *d));
        due.extend(self.diff_due.values().copied());
        due.extend(self.peers.wake_at());
        due.extend(self.stores.history.next_flush_at());
        due.extend(self.stores.timeline.next_flush_at());
        due.extend(self.stores.response.next_flush_at());
        due.extend(self.stores.limit_history.next_flush_at());
        due.extend(self.agents_due());
        due.extend(self.stores.dismissed_jobs.as_ref().and_then(JsonFile::next_due));
        if self.diff_reader.as_ref().is_some_and(DiffStatReader::busy) || !self.stopping_jobs.is_empty() {
            due.push(now_ms() + 50);
        }
        due.into_iter().min()
    }

    pub fn tick(&mut self, store: &mut Store) {
        if self.stopped {
            return;
        }
        let wall = now_ms();
        let events = self.watcher.as_mut().map(Watcher::drain).unwrap_or_default();
        for e in events {
            match e {
                WatchEvent::Registry => self.schedule_registry(wall),
                WatchEvent::Project(rel) => {
                    self.route_project_change(&rel, wall);
                    self.project_files.push(rel);
                }
                WatchEvent::SessionsLost => self.sessions_watched = false,
                WatchEvent::ProjectsLost => self.projects_watched = false,
            }
        }
        if self.registry_due.is_some_and(|d| d <= wall) {
            self.registry_due = None;
            self.reconcile(store, false, true);
        }
        let due: Vec<String> = self.sync_due.iter().filter(|(_, (d, _))| *d <= wall).map(|(k, _)| k.clone()).collect();
        for id in due {
            let Some((_, files)) = self.sync_due.shift_remove(&id) else { continue };
            self.sync_tracker(store, &id, &crate::tracker::SyncScope::Files(files.into_iter().collect()));
        }
        if let Some(mut t) = self.timers {
            if t.reconcile <= wall {
                t.reconcile = wall + self.config.reconcile_ms;
                self.timers = Some(t);
                self.reconcile(store, false, true);
            }
            if t.extras <= wall {
                t.extras = wall + self.config.reconcile_ms;
                self.timers = Some(t);
                self.poll_extras(store);
            }
            if t.verify <= wall {
                t.verify = wall + self.config.process_verify_ms;
                self.timers = Some(t);
                self.verify_live(store);
                self.refresh_hints(store);
            }
        }
        self.run_diffs(store, wall);
        if self.peers.wake_at().is_some_and(|w| w <= wall) {
            self.flush_peers(store, false);
        }
        self.tick_agents(store);
        self.drain_done(store);
        self.poll_persist(wall);
    }

    fn poll_persist(&mut self, wall: i64) {
        self.stores.history.poll(wall);
        self.stores.timeline.poll(wall);
        self.stores.response.poll(wall);
        self.stores.limit_history.poll(wall);
        if let Some(f) = self.stores.dismissed_jobs.as_mut() {
            let snapshot = dismissed_value(&self.dismissed_jobs);
            f.poll(wall, || snapshot);
        }
    }

    pub fn flush_persist(&mut self) {
        self.stores.history.save();
        self.stores.timeline.save();
        self.stores.response.save();
        self.stores.limit_history.save();
        self.flush_agents();
        if let Some(f) = self.stores.dismissed_jobs.as_mut() {
            let snapshot = dismissed_value(&self.dismissed_jobs);
            f.flush_pending(|| snapshot);
        }
    }

    pub fn wait_background(&mut self, store: &mut Store, timeout: std::time::Duration) {
        let until = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < until {
            let wall = now_ms();
            for id in self.diff_due.keys().cloned().collect::<Vec<_>>() {
                self.diff_due.insert(id, wall);
            }
            self.run_diffs(store, wall);
            self.drain_done(store);
            let busy = self.diff_reader.as_ref().is_some_and(DiffStatReader::busy) || !self.stopping_jobs.is_empty() || !self.diff_due.is_empty();
            if !busy {
                return;
            }
            if let Some(r) = self.diff_reader.as_mut() {
                let ready = r.wait(std::time::Duration::from_millis(20));
                self.apply_diffs(store, ready);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

fn dismissed_value(map: &IndexMap<String, String>) -> serde_json::Value {
    serde_json::Value::Object(map.iter().map(|(k, v)| (k.clone(), serde_json::Value::from(v.as_str()))).collect())
}
