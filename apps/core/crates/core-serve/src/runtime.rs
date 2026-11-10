use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core_http::{ClientContext, ClientMessage, StoreHandle};
use core_monitor::gitstat::DiffStatReader;
use core_monitor::monitor::{LaunchError, LaunchMode};
use core_monitor::repo::RepoRoots;
use core_monitor::usage::{Usage, UsageEvent};
use core_monitor::{Deps, Monitor, MonitorConfig, MonitorStores};
use core_sources::resources::ports::ConflictCall;
use core_sources::resources::runner::BusyTracker;
use core_sources::sensitive::watch::SensitiveWatch;
use core_store::event::{FocusWindowResult, MonitorEvent};
use core_store::extra::JobState;
use core_store::model::{ActivityEvent, ActivityKind, AgentRole, AgentStatus, TerminalStatus};
use core_store::time::now_ms;
use core_store::Store;
use serde_json::{json, Value};
use tokio::sync::oneshot;

use crate::alerts::{lock, Emit, SharedAlerts};
use crate::dispatch;
use crate::host_ops::HostOps;
use crate::procs::{CachedAgentProcs, CachedDeps, ProcCache};
use crate::sampler::Sampling;
use crate::sensitive::{Roots, SensitiveAdapter};
use crate::stats::Stats;

pub const TOAST_TIMER_MS: i64 = 15_000;

pub enum Cmd {
    Client(Box<(ClientMessage, ClientContext)>),
    FocusRun(String, String, oneshot::Sender<Option<FocusWindowResult>>),
    Control(Value),
    LaunchDone(String, LaunchMode, Result<(), LaunchError>),
    Wake,
    Shutdown,
}

pub struct Setup {
    pub handle: StoreHandle,
    pub config: MonitorConfig,
    pub deps: Arc<dyn Deps>,
    pub cache: Arc<ProcCache>,
    pub alerts: SharedAlerts,
    pub sensitive: Arc<Mutex<SensitiveWatch>>,
    pub sampling: Arc<Sampling>,
    pub host: Arc<HostOps>,
    pub roots: Roots,
    pub new_projects: Receiver<Vec<String>>,
    pub sensitive_events: Arc<Mutex<Vec<ActivityEvent>>>,
    pub emit: Emit,
    pub host_rpc: bool,
    pub stats: bool,
}

pub struct Runtime {
    pub monitor: Monitor,
    pub alerts: SharedAlerts,
    pub sensitive: Arc<Mutex<SensitiveWatch>>,
    pub sampling: Arc<Sampling>,
    pub host: Arc<HostOps>,
    pub usage: Option<Usage>,
    pub office_noted: bool,
    roots: Roots,
    repo: RepoRoots,
    new_projects: Receiver<Vec<String>>,
    sensitive_events: Arc<Mutex<Vec<ActivityEvent>>>,
    cache: Arc<ProcCache>,
    busy: Option<BusyTracker>,
    emit: Emit,
    toast_at: i64,
}

pub fn run(setup: Setup, rx: Receiver<Cmd>, wake: Arc<dyn Fn() + Send + Sync>) {
    let Setup { handle, config, deps, cache, alerts, sensitive, sampling, host, roots, new_projects, sensitive_events, emit, host_rpc, stats } = setup;
    let stores = MonitorStores::load_from(&config.data_dir);
    let projects = config.claude_root.join("projects");
    let usage_cache = config.data_dir.join("usage-index.json");
    let verify = config.verify_processes;
    let cached: Arc<dyn Deps> = Arc::new(CachedDeps::new(deps, cache.clone()));
    let mut monitor = Monitor::new(config, cached, stores, Some(DiffStatReader::system()));
    monitor.sensitive = Some(Box::new(SensitiveAdapter(sensitive.clone())));
    monitor.set_wake(wake.clone());
    monitor.enable_agents(Arc::new(CachedAgentProcs(cache.clone())), true, verify);
    let busy = host_rpc.then(BusyTracker::default);
    let mut rt = Runtime { monitor, alerts, sensitive, sampling, host, usage: None, office_noted: false, roots, repo: RepoRoots::system(), new_projects, sensitive_events, cache, busy, emit, toast_at: now_ms() + TOAST_TIMER_MS };
    rt.cache.prefetch();
    rt.monitor.prepare(now_ms());
    handle.with(|s| rt.monitor.start(s, true));
    rt.cache.disarm();
    handle.with(|s| {
        s.set_loading(false);
        post_busy(&mut rt, s);
    });
    handle.with(|s| rt.monitor.start_agents(s, true));
    let (usage, snapshot) = Usage::open(projects, Some(usage_cache), wake);
    if let Some(snap) = snapshot {
        handle.with(|s| s.reset_usage(snap));
    }
    rt.usage = Some(usage);
    let mut stats = Stats::new(stats);
    loop {
        let now = now_ms();
        let due = [rt.monitor.next_due(), lock(&rt.alerts).next_due(), rt.sensitive.lock().ok().and_then(|w| w.next_due()), Some(rt.toast_at)].into_iter().flatten().min();
        let wait = due.map_or(Duration::from_millis(1_000), |d| Duration::from_millis((d - now).clamp(1, 1_000) as u64));
        let first = match rx.recv_timeout(wait) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let cmds: Vec<Cmd> = first.into_iter().chain(rx.try_iter()).collect();
        if cmds.iter().any(|c| matches!(c, Cmd::Shutdown)) {
            break;
        }
        let pairs = resolve_roots(&mut rt);
        let now = now_ms();
        if rt.monitor.procs_wanted(now) {
            stats.prefetch(rt.cache.prefetch());
        }
        rt.monitor.prepare(now);
        let started = Instant::now();
        let phases = handle.with(|s| {
            for c in cmds {
                command(&mut rt, s, c);
            }
            let t0 = Instant::now();
            rt.monitor.tick(s);
            let t1 = Instant::now();
            after_tick(&mut rt, s, pairs);
            [t0 - started, t1 - t0, t1.elapsed()]
        });
        stats.tick(started.elapsed(), phases);
        rt.cache.disarm();
    }
    stats.report(&rt.cache);
    rt.monitor.stop();
    rt.monitor.flush_persist();
    lock(&rt.alerts).close();
    if let Ok(mut w) = rt.sensitive.lock() {
        w.flush();
    }
    if let Some(u) = rt.usage.as_mut() {
        u.stop();
    }
}

fn command(rt: &mut Runtime, s: &mut Store, c: Cmd) {
    match c {
        Cmd::Client(b) => {
            let (m, ctx) = *b;
            dispatch::client(rt, s, m, &ctx);
        }
        Cmd::FocusRun(id, key, done) => {
            let mut a = lock(&rt.alerts);
            if !a.toaster.accepts(Some(&key)) {
                let _ = done.send(None);
            } else if s.get_terminal(&id).is_none() {
                let _ = done.send(Some(FocusWindowResult::NotFound));
            } else {
                a.toaster.hide_for(&id);
                a.pump();
                rt.host.focus_run(rt.monitor.focus_pid(&id), done);
            }
        }
        Cmd::Control(v) => lock(&rt.alerts).control(s, &v),
        Cmd::LaunchDone(label, mode, result) => {
            Monitor::launch_done(s, label, mode, result);
        }
        Cmd::Wake | Cmd::Shutdown => {}
    }
}

fn resolve_roots(rt: &mut Runtime) -> Vec<(String, String)> {
    let dirs: Vec<String> = rt.new_projects.try_iter().flatten().collect();
    dirs.into_iter().map(|d| (d.clone(), rt.repo.resolve(&d))).collect()
}

fn after_tick(rt: &mut Runtime, s: &mut Store, pairs: Vec<(String, String)>) {
    if let Some(u) = rt.usage.as_ref() {
        for f in rt.monitor.take_project_files() {
            u.notify(&f);
        }
        let events: Vec<UsageEvent> = u.events.try_iter().collect();
        for e in events {
            match e {
                UsageEvent::Scan(scan) => s.set_usage_scan(scan),
                UsageEvent::Reset(snap) => s.reset_usage(snap),
                UsageEvent::Update(b) => s.update_usage(b),
            }
        }
    }
    if !pairs.is_empty() {
        if let Ok(mut r) = rt.roots.lock() {
            r.extend(pairs.iter().cloned());
        }
        s.set_project_roots(pairs.into_iter().collect());
    }
    let now = now_ms();
    if let Ok(mut w) = rt.sensitive.lock() {
        w.poll(now);
    }
    let timer = now >= rt.toast_at;
    let mut a = lock(&rt.alerts);
    if timer || s.next_flush_at().is_some() {
        a.observe(s, &rt.monitor);
    }
    if timer {
        rt.toast_at = now + TOAST_TIMER_MS;
        a.check_loops(s);
    }
    let events: Vec<ActivityEvent> = rt.sensitive_events.lock().map(|mut e| std::mem::take(&mut *e)).unwrap_or_default();
    if !events.is_empty() {
        a.sensitive(&events);
    }
    a.poll(now);
    let idle = a.toaster.idle_minutes();
    drop(a);
    if let Ok(mut x) = rt.sampling.extras.lock() {
        x.idle_minutes = idle;
        x.last_device_call_at = rt.monitor.last_device_call_at().map(|v| v as f64);
        x.conflicts = rt.monitor.port_conflicts().into_iter().map(|(session_id, port, at)| ConflictCall { session_id, port: port as i64, at: at as f64 }).collect();
    }
    post_busy(rt, s);
}

fn post_busy(rt: &mut Runtime, s: &Store) {
    let Some(tracker) = rt.busy.as_mut() else { return };
    let agents = s.terminal_list().filter(|t| t.status == TerminalStatus::Working).count() + s.agent_list().filter(|a| a.role == AgentRole::Subagent && a.status == AgentStatus::Working).count();
    let jobs = s.job_list().iter().filter(|j| j.state == JobState::Working).count();
    if let Some(m) = tracker.changed((agents as u64, jobs as u64)) {
        (rt.emit)(&m);
    }
}

pub fn sensitive_listener(events: Arc<Mutex<Vec<ActivityEvent>>>, back: Mutex<Sender<Cmd>>) -> core_store::store::Listener {
    Box::new(move |m| {
        let hits: Vec<ActivityEvent> = m.events.iter().filter_map(|e| if let MonitorEvent::Activity(a) = e { (a.kind == ActivityKind::Sensitive).then(|| (**a).clone()) } else { None }).collect();
        if !hits.is_empty() {
            if let Ok(mut e) = events.lock() {
                e.extend(hits);
            }
            if let Ok(tx) = back.lock() {
                let _ = tx.send(Cmd::Wake);
            }
        }
        Ok(())
    })
}

pub fn listening(port: u16) -> Value {
    json!({ "type": "listening", "port": port })
}
