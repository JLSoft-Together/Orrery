use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use core_monitor::agentprocs::AgentProcs;
use core_monitor::codex::process::{list_codex_processes_in, CodexProcess};
use core_monitor::deepseek::process::{list_deepseek_processes_in, DeepseekProcess};
use core_monitor::job_stop::JobStopResult;
use core_monitor::monitor::{LaunchError, LaunchMode, TerminalApp};
use core_monitor::process::{query_processes_in, ProcessInfo};
use core_monitor::Deps;
use core_procs::Proc;
use core_store::event::{FocusWindowResult, FolderPickResult, OpenApp, OpenResult};

pub type TableFn = Box<dyn Fn() -> Option<Vec<Proc>> + Send + Sync>;

pub struct ProcCache {
    read: TableFn,
    armed: Mutex<Option<Arc<Vec<Proc>>>>,
    pub prefetched: AtomicU64,
    pub missed: AtomicU64,
}

impl ProcCache {
    pub fn new(read: TableFn) -> Arc<ProcCache> {
        Arc::new(ProcCache { read, armed: Mutex::new(None), prefetched: AtomicU64::new(0), missed: AtomicU64::new(0) })
    }

    pub fn system() -> Arc<ProcCache> {
        Self::new(Box::new(core_procs::table))
    }

    pub fn prefetch(&self) -> f64 {
        let started = Instant::now();
        let table = (self.read)().map(Arc::new);
        if let Ok(mut a) = self.armed.lock() {
            *a = table;
        }
        self.prefetched.fetch_add(1, Ordering::Relaxed);
        started.elapsed().as_secs_f64() * 1000.0
    }

    pub fn disarm(&self) {
        if let Ok(mut a) = self.armed.lock() {
            *a = None;
        }
    }

    pub fn table(&self) -> Option<Arc<Vec<Proc>>> {
        if let Some(t) = self.armed.lock().ok().and_then(|a| a.clone()) {
            return Some(t);
        }
        self.missed.fetch_add(1, Ordering::Relaxed);
        (self.read)().map(Arc::new)
    }
}

pub struct CachedDeps {
    inner: Arc<dyn Deps>,
    cache: Arc<ProcCache>,
}

impl CachedDeps {
    pub fn new(inner: Arc<dyn Deps>, cache: Arc<ProcCache>) -> CachedDeps {
        CachedDeps { inner, cache }
    }
}

impl Deps for CachedDeps {
    fn now(&self) -> i64 {
        self.inner.now()
    }
    fn is_pid_alive(&self, pid: u32) -> bool {
        self.inner.is_pid_alive(pid)
    }
    fn query_processes(&self, pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
        query_processes_in(pids, &self.cache.table()?)
    }
    fn stop_job(&self, job_id: &str) -> JobStopResult {
        self.inner.stop_job(job_id)
    }
    fn launch(&self, dir: &str, mode: LaunchMode, app: Option<TerminalApp>) -> Option<Result<(), LaunchError>> {
        self.inner.launch(dir, mode, app)
    }
    fn focus_window(&self, pid: u32) -> Option<FocusWindowResult> {
        self.inner.focus_window(pid)
    }
    fn open_folder(&self, dir: &str, app: OpenApp) -> Option<OpenResult> {
        self.inner.open_folder(dir, app)
    }
    fn pick_folder(&self) -> Option<(FolderPickResult, Option<String>)> {
        self.inner.pick_folder()
    }
}

pub struct CachedAgentProcs(pub Arc<ProcCache>);

impl AgentProcs for CachedAgentProcs {
    fn codex(&self) -> Option<Vec<CodexProcess>> {
        list_codex_processes_in(&self.0.table()?)
    }

    fn deepseek(&self) -> Option<Vec<DeepseekProcess>> {
        list_deepseek_processes_in(&self.0.table()?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn cache_counting() -> (Arc<ProcCache>, Arc<AtomicUsize>) {
        let reads = Arc::new(AtomicUsize::new(0));
        let r = reads.clone();
        let cache = ProcCache::new(Box::new(move || {
            r.fetch_add(1, Ordering::SeqCst);
            Some(vec![Proc { pid: 42, ppid: 7, name: "claude.exe".into(), start_ms: Some(1), start_raw: Some("134358400013081891".into()) }, Proc { pid: 7, ppid: 1, name: "pwsh.exe".into(), start_ms: Some(1), start_raw: None }])
        }));
        (cache, reads)
    }

    #[test]
    fn a_prefetched_table_serves_every_query_until_disarmed() {
        let (cache, reads) = cache_counting();
        cache.prefetch();
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert_eq!(cache.table().unwrap().len(), 2);
        assert_eq!(cache.table().unwrap().len(), 2);
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert_eq!(cache.missed.load(Ordering::SeqCst), 0);
        cache.disarm();
        assert!(cache.table().is_some());
        assert_eq!(reads.load(Ordering::SeqCst), 2);
        assert_eq!(cache.missed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_deps_answer_process_queries_from_the_table() {
        let (cache, _) = cache_counting();
        cache.prefetch();
        let deps = CachedDeps::new(Arc::new(core_monitor::SystemDeps), cache);
        let info = deps.query_processes(&[42, 0]);
        if cfg!(any(windows, target_os = "macos")) {
            let info = info.unwrap_or_default();
            if cfg!(windows) {
                assert_eq!(info[&42].parent_name.as_deref(), Some("pwsh.exe"));
            }
        } else {
            assert!(info.is_none());
        }
    }
}
