mod dispatch;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_http::control::{spawn_stdin, write_line, ControlIn};
use core_http::{start, ClientContext, ClientMessage, Config, Hooks, StartError, StoreHandle, PORT_IN_USE_EXIT};
use core_store::extra::{NotifySettings, SensitiveSettings, ToastState};
use core_store::persist::daystart::first_start_today;
use core_store::persist::localtime::Zone;
use core_store::persist::notify_file::NotifyFile;
use core_store::persist::snooze::SnoozeStore;
use core_store::time::now_ms;
use core_store::{JsNum, Store, StoreOptions};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::gitstat::DiffStatReader;
use crate::host::{HostClient, HostDeps};
use crate::monitor::{Deps, Monitor, MonitorConfig, MonitorStores, SystemDeps};
use crate::repo::RepoRoots;
use crate::usage::{Usage, UsageEvent};

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub port: u16,
    pub web_dist: PathBuf,
    pub app_key: Option<String>,
    pub dev: bool,
    pub monitor: MonitorConfig,
    pub status_line_command: String,
    pub activity_limit: usize,
    pub batch_ms: i64,
    pub host_rpc: bool,
}

fn env_int(k: &str, fallback: i64) -> i64 {
    std::env::var(k).ok().and_then(|v| v.trim().parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(fallback)
}

fn env_str(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

impl ServeConfig {
    pub fn from_env(dev: bool) -> ServeConfig {
        let home = env_str("USERPROFILE").or_else(|| env_str("HOME")).unwrap_or_default();
        let claude_root = env_str("CLAUDE_CONFIG_DIR").map_or_else(|| PathBuf::from(&home).join(".claude"), PathBuf::from);
        let data_dir = env_str("CCM_DATA_DIR").map_or_else(
            || PathBuf::from(core_store::persist::datadir::default_data_dir(env_str("LOCALAPPDATA").as_deref(), core_store::persist::datadir::Platform::HOST, &home)),
            PathBuf::from,
        );
        let mut monitor = MonitorConfig::new(claude_root, data_dir);
        monitor.stale_ttl_ms = env_int("CCM_STALE_TTL_MIN", 120) * 60_000;
        monitor.peer_ttl_ms = env_int("CCM_PEER_TTL_MIN", 60) * 60_000;
        monitor.verify_processes = cfg!(any(windows, target_os = "macos")) && std::env::var("CCM_VERIFY_PROCESSES").as_deref() != Ok("0");
        let home_dir = PathBuf::from(&home);
        if env_str("CCM_CODEX").as_deref() != Some("0") {
            monitor.codex_home = Some(env_str("CODEX_HOME").map_or_else(|| home_dir.join(".codex"), PathBuf::from));
        }
        if env_str("CCM_DEEPSEEK").as_deref() != Some("0") {
            monitor.codewhale_home = Some(crate::deepseek::config::codewhale_home(env_str("CODEWHALE_HOME"), |p| p.exists(), &home_dir));
            monitor.home_dir = home_dir;
        }
        let bridge = env_str("CCM_STATUSLINE_BRIDGE").unwrap_or_default();
        ServeConfig {
            port: env_int("CCM_PORT", 4317).clamp(0, 65_535) as u16,
            web_dist: PathBuf::from(env_str("CCM_WEB_DIST").unwrap_or_else(|| "apps/web/dist".into())),
            app_key: env_str("CCM_APP_KEY"),
            dev,
            monitor,
            status_line_command: env_str("CCM_STATUSLINE_COMMAND").unwrap_or_else(|| format!("node \"{}\"", bridge.replace('\\', "/"))),
            activity_limit: 1000,
            batch_ms: 50,
            host_rpc: std::env::var("CCM_HOST_RPC").as_deref() == Ok("1"),
        }
    }
}

pub enum Cmd {
    Client(Box<(ClientMessage, ClientContext)>),
    Wake,
    Shutdown,
}

fn random_key() -> String {
    let seed = format!("{}:{}:{:?}", now_ms(), std::process::id(), std::thread::current().id());
    Sha256::digest(seed.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect()
}

fn notify_settings(file: &NotifyFile) -> NotifySettings {
    let p = file.prefs();
    NotifySettings { toast: ToastState::Unsupported, kinds: p.kinds.clone(), quiet: p.quiet.clone(), stuck_minutes: JsNum(p.stuck_minutes), idle_minutes: JsNum(p.idle_minutes), health: None }
}

fn default_sensitive() -> SensitiveSettings {
    let groups = ["secrets", "outside", "git", "delete", "data", "custom"].iter().map(|g| ((*g).to_owned(), true)).collect();
    SensitiveSettings { groups, globs: Vec::new(), ignore: Default::default() }
}

pub struct Runtime {
    pub monitor: Monitor,
    pub snoozes: SnoozeStore,
    pub usage: Option<Usage>,
    pub roots: RepoRoots,
    pub new_projects: Receiver<Vec<String>>,
    pub clients: usize,
}

pub fn run(cfg: ServeConfig) -> i32 {
    if cfg.app_key.is_none() && !cfg.dev {
        eprintln!("[collector] Orrery runs inside its desktop app: npm run desktop (UI work: npm run dev).");
        return 1;
    }
    let data = cfg.monitor.data_dir.clone();
    let mut store = Store::new(StoreOptions { activity_limit: cfg.activity_limit, batch_ms: cfg.batch_ms });
    store.status_line_command = Some(cfg.status_line_command.clone());
    store.day_started_at = Some(first_start_today(&data.join("day-start.json"), now_ms(), Zone::Local));
    let (proj_tx, new_projects) = channel::<Vec<String>>();
    let proj_tx = Mutex::new(proj_tx);
    store.set_on_new_projects(Some(Box::new(move |dirs| {
        if let Ok(tx) = proj_tx.lock() {
            let _ = tx.send(dirs);
        }
    })));
    let mut snoozes = SnoozeStore::new(Some(data.join("snooze.json")));
    snoozes.load();
    store.set_snoozes(snoozes.all());
    let mut notify = NotifyFile::new(Some(data.join("notify.json")), random_key);
    notify.load();
    store.set_notify(notify_settings(&notify));
    store.set_sensitive(default_sensitive());
    store.set_loading(true);
    let handle = StoreHandle::new(store);
    let (tx, rx) = channel::<Cmd>();
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[collector] runtime: {e}");
            return 1;
        }
    };
    let mut config = Config::new(cfg.port, cfg.web_dist.clone());
    config.app_key = if cfg.dev { None } else { cfg.app_key.clone() };
    config.dev_origin_ports = vec![env_int("CCM_DEV_ORIGIN_PORT", 5173) as u16];
    let msg_tx = Mutex::new(tx.clone());
    let hooks = Hooks {
        on_client_message: Some(Arc::new(move |m: ClientMessage, ctx: &ClientContext| {
            if let Ok(t) = msg_tx.lock() {
                let _ = t.send(Cmd::Client(Box::new((m, ctx.clone()))));
            }
        })),
        ..Hooks::default()
    };
    runtime.block_on(async move {
        let server = match start(config, handle.clone(), hooks).await {
            Ok(s) => s,
            Err(StartError::PortInUse(e)) => {
                eprintln!("[collector] port {} is already in use: {e}", cfg.port);
                return PORT_IN_USE_EXIT;
            }
            Err(e) => {
                eprintln!("[collector] fatal: {e}");
                return 1;
            }
        };
        write_line(&json!({ "type": "listening", "port": server.port() }));
        let host = cfg.host_rpc.then(|| HostClient::new(Box::new(std::io::stdout())));
        let deps: Arc<dyn Deps> = match &host {
            Some(c) => Arc::new(HostDeps::new(c.clone())),
            None => Arc::new(SystemDeps),
        };
        let worker = {
            let (h, mcfg, wake_tx) = (handle.clone(), cfg.monitor.clone(), Mutex::new(tx.clone()));
            std::thread::Builder::new()
                .name("monitor".into())
                .spawn(move || {
                    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                        if let Ok(t) = wake_tx.lock() {
                            let _ = t.send(Cmd::Wake);
                        }
                    });
                    monitor_loop(h, mcfg, deps, snoozes, new_projects, rx, wake);
                })
                .ok()
        };
        let (ctl_tx, mut ctl_rx) = tokio::sync::mpsc::unbounded_channel();
        spawn_stdin(ctl_tx);
        while let Some(m) = ctl_rx.recv().await {
            match m {
                ControlIn::Shutdown | ControlIn::Eof => break,
                ControlIn::Message(v) => {
                    if let Some(c) = &host {
                        c.on_message(&v);
                    }
                }
            }
        }
        if let Some(c) = &host {
            c.close();
        }
        let _ = tx.send(Cmd::Shutdown);
        if let Some(w) = worker {
            let _ = tokio::task::spawn_blocking(move || w.join()).await;
        }
        server.shutdown().await;
        0
    })
}

fn monitor_loop(handle: StoreHandle, mcfg: MonitorConfig, deps: Arc<dyn Deps>, snoozes: SnoozeStore, new_projects: Receiver<Vec<String>>, rx: Receiver<Cmd>, wake: Arc<dyn Fn() + Send + Sync>) {
    let stores = MonitorStores::load_from(&mcfg.data_dir);
    let projects = mcfg.claude_root.join("projects");
    let cache = mcfg.data_dir.join("usage-index.json");
    let verify = mcfg.verify_processes;
    let mut monitor = Monitor::new(mcfg, deps, stores, Some(DiffStatReader::system()));
    monitor.set_wake(wake.clone());
    monitor.enable_agents(Arc::new(crate::agentprocs::SystemProcs), true, verify);
    let mut rt = Runtime { monitor, snoozes, usage: None, roots: RepoRoots::system(), new_projects, clients: 0 };
    handle.with(|s| rt.monitor.start(s, true));
    handle.with(|s| s.set_loading(false));
    handle.with(|s| rt.monitor.start_agents(s, true));
    let (usage, snapshot) = Usage::open(projects, Some(cache), wake);
    if let Some(snap) = snapshot {
        handle.with(|s| s.reset_usage(snap));
    }
    rt.usage = Some(usage);
    loop {
        let wait = rt.monitor.next_due().map_or(Duration::from_millis(1_000), |d| Duration::from_millis((d - now_ms()).clamp(1, 1_000) as u64));
        let first = match rx.recv_timeout(wait) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let cmds: Vec<Cmd> = first.into_iter().chain(rx.try_iter()).collect();
        if cmds.iter().any(|c| matches!(c, Cmd::Shutdown)) {
            break;
        }
        handle.with(|s| {
            for c in cmds {
                if let Cmd::Client(b) = c {
                    let (m, ctx) = *b;
                    dispatch::client(&mut rt, s, m, &ctx);
                }
            }
            rt.monitor.tick(s);
            after_tick(&mut rt, s);
        });
    }
    rt.monitor.stop();
    rt.monitor.flush_persist();
    rt.snoozes.flush();
    if let Some(u) = rt.usage.as_mut() {
        u.stop();
    }
}

fn after_tick(rt: &mut Runtime, s: &mut Store) {
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
    let dirs: Vec<String> = rt.new_projects.try_iter().flatten().collect();
    if !dirs.is_empty() {
        let roots = dirs.iter().map(|d| (d.clone(), rt.roots.resolve(d))).collect();
        s.set_project_roots(roots);
    }
    if rt.snoozes.prune(s.terminal_list()) {
        s.set_snoozes(rt.snoozes.all());
    }
    rt.snoozes.poll(now_ms());
}

pub type CmdSender = Sender<Cmd>;
