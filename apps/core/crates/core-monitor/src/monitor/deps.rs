use std::collections::HashMap;
use std::sync::Arc;

use core_store::event::{FocusWindowResult, FolderPickResult, OpenApp, OpenResult};
use core_store::persist::aliases::{AliasStore, MAX_ALIAS_LENGTH, MAX_NOTE_LENGTH};
use core_store::persist::datadir::Platform;
use core_store::persist::favorites::FavoriteStore;
use core_store::persist::forecast::LimitHistory;
use core_store::persist::history::SessionHistory;
use core_store::persist::json_file::JsonFile;
use core_store::persist::response::ResponseTracker;
use core_store::persist::timeline::StatusTimeline;
use core_store::time::now_ms;

use super::{LaunchError, LaunchMode, TerminalApp};
use crate::job_stop::{stop_with_claude, JobStopResult};
use crate::process::{is_pid_alive, query_processes, ProcessInfo};

pub trait Deps: Send + Sync {
    fn now(&self) -> i64;
    fn is_pid_alive(&self, pid: u32) -> bool;
    fn query_processes(&self, pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>>;
    fn stop_job(&self, job_id: &str) -> JobStopResult;
    fn launch(&self, _dir: &str, _mode: LaunchMode, _app: Option<TerminalApp>) -> Option<Result<(), LaunchError>> {
        None
    }
    fn focus_window(&self, _pid: u32) -> Option<FocusWindowResult> {
        None
    }
    fn open_folder(&self, _dir: &str, _app: OpenApp) -> Option<OpenResult> {
        None
    }
    fn pick_folder(&self) -> Option<(FolderPickResult, Option<String>)> {
        None
    }
}

pub struct SystemDeps;

impl Deps for SystemDeps {
    fn now(&self) -> i64 {
        now_ms()
    }
    fn is_pid_alive(&self, pid: u32) -> bool {
        is_pid_alive(pid)
    }
    fn query_processes(&self, pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
        query_processes(pids)
    }
    fn stop_job(&self, job_id: &str) -> JobStopResult {
        stop_with_claude(job_id)
    }
}

pub struct MonitorStores {
    pub aliases: AliasStore,
    pub notes: AliasStore,
    pub favorites: FavoriteStore,
    pub response: ResponseTracker,
    pub limit_history: LimitHistory,
    pub history: SessionHistory,
    pub timeline: StatusTimeline,
    pub dismissed_jobs: Option<JsonFile>,
}

impl MonitorStores {
    pub fn memory(deps: Arc<dyn Deps>) -> Self {
        MonitorStores {
            aliases: AliasStore::new(None, MAX_ALIAS_LENGTH),
            notes: AliasStore::new(None, MAX_NOTE_LENGTH),
            favorites: FavoriteStore::new(None, Platform::HOST),
            response: ResponseTracker::new(None),
            limit_history: LimitHistory::new(None),
            history: SessionHistory::new(None),
            timeline: StatusTimeline::with_clock(None, core_store::persist::localtime::Zone::Local, Box::new(move || deps.now())),
            dismissed_jobs: None,
        }
    }

    pub fn load_from(data_dir: &std::path::Path) -> Self {
        let mut s = MonitorStores {
            aliases: AliasStore::new(Some(data_dir.join("aliases.json")), MAX_ALIAS_LENGTH),
            notes: AliasStore::new(Some(data_dir.join("notes.json")), MAX_NOTE_LENGTH),
            favorites: FavoriteStore::new(Some(data_dir.join("favorites.json")), Platform::HOST),
            response: ResponseTracker::new(Some(data_dir.join("response-stats.json"))),
            limit_history: LimitHistory::new(Some(data_dir.join("limit-history.json"))),
            history: SessionHistory::new(Some(data_dir.join("history.json"))),
            timeline: StatusTimeline::new(Some(data_dir.join("timeline.json"))),
            dismissed_jobs: Some(JsonFile::new(data_dir.join("dismissed-jobs.json"), 500)),
        };
        s.aliases.load();
        s.notes.load();
        s.favorites.load();
        s.response.load();
        s.limit_history.load();
        s.history.load();
        s.timeline.load();
        s
    }
}
