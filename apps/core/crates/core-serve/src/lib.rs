pub mod alerts;
pub mod config;
pub mod dispatch;
pub mod host_ops;
pub mod procs;
pub mod runtime;
pub mod sampler;
pub mod sensitive;
pub mod stats;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use core_http::control::{spawn_stdin, write_line, ControlIn};
use core_http::{start, ClientContext, ClientMessage, Config, Hooks, StartError, StoreHandle, PORT_IN_USE_EXIT};
use core_monitor::host::{HostClient, HostDeps};
use core_monitor::{Deps, SystemDeps};
use core_store::persist::daystart::first_start_today;
use core_store::persist::localtime::Zone;
use core_store::time::now_ms;
use core_store::{Store, StoreOptions};
use serde_json::Value;

use crate::alerts::{lock, Alerts, Emit};
use crate::config::{ConfigError, ServeConfig};
use crate::host_ops::HostOps;
use crate::procs::ProcCache;
use crate::runtime::{listening, sensitive_listener, Cmd, Setup};
use crate::sampler::Sampling;

pub const CONFIG_EXIT: i32 = 2;

pub fn main(args: &[String]) -> i32 {
    let dev = args.iter().any(|a| a == "--dev");
    match ServeConfig::from_env(dev) {
        Ok(cfg) => run(cfg),
        Err(e) => {
            eprintln!("[collector] {e}");
            if e == ConfigError::NoAppKey {
                1
            } else {
                CONFIG_EXIT
            }
        }
    }
}

fn sender(tx: &std::sync::mpsc::Sender<Cmd>) -> Mutex<std::sync::mpsc::Sender<Cmd>> {
    Mutex::new(tx.clone())
}

pub fn run(cfg: ServeConfig) -> i32 {
    if cfg.app_key.is_none() && !cfg.dev {
        eprintln!("[collector] {}", ConfigError::NoAppKey);
        return 1;
    }
    let emit: Emit = Arc::new(write_line);
    let data = cfg.monitor.data_dir.clone();
    let (tx, rx) = channel::<Cmd>();
    let mut store = Store::new(StoreOptions { activity_limit: cfg.activity_limit, batch_ms: cfg.batch_ms });
    store.status_line_command = Some(cfg.status_line_command.clone());
    store.day_started_at = Some(first_start_today(&data.join("day-start.json"), now_ms(), Zone::Local));
    let (proj_tx, new_projects) = channel::<Vec<String>>();
    let proj_tx = Mutex::new(proj_tx);
    store.set_on_new_projects(Some(Box::new(move |dirs| {
        if let Ok(t) = proj_tx.lock() {
            let _ = t.send(dirs);
        }
    })));
    let alerts = Arc::new(Mutex::new(Alerts::open(&cfg, cfg.port, emit.clone())));
    {
        let a = lock(&alerts);
        store.set_snoozes(a.snoozes.all());
        store.set_notify(a.toaster.settings());
    }
    let roots = Arc::new(Mutex::new(HashMap::new()));
    let sensitive = sensitive::open_watch(&cfg.monitor.claude_root, data.join("sensitive.json"), roots.clone());
    if let Ok(w) = sensitive.lock() {
        store.set_sensitive(w.settings().clone());
    }
    let sensitive_events = Arc::new(Mutex::new(Vec::new()));
    store.subscribe(sensitive_listener(sensitive_events.clone(), sender(&tx)));
    store.set_loading(true);
    let handle = StoreHandle::new(store);
    let host_client = cfg.host_rpc.then(|| HostClient::new(Box::new(std::io::stdout())));
    let deps: Arc<dyn Deps> = match &host_client {
        Some(c) => Arc::new(HostDeps::new(c.clone())),
        None => Arc::new(SystemDeps),
    };
    let host = Arc::new(HostOps::new(deps.clone(), cfg.monitor.claude_root.join("projects"), tx.clone()));
    let sampling = Sampling::start(cfg.res_fixture.clone(), handle.clone(), alerts.clone(), emit.clone());
    sampling.sync(lock(&alerts).toaster.idle_watch());
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[collector] runtime: {e}");
            return 1;
        }
    };
    let mut config = Config::new(cfg.port, cfg.web_dist.clone());
    config.app_key = if cfg.dev { None } else { cfg.app_key.clone() };
    config.dev_origin_ports = vec![cfg.dev_origin_port];
    let hooks = hooks(&tx, &handle, &alerts, &sampling, &host);
    runtime.block_on(async move {
        let server = match start(config, handle.clone(), hooks).await {
            Ok(s) => s,
            Err(StartError::PortInUse(e)) => {
                eprintln!("[collector] port {} is already in use (another Orrery, or another app: 4317 is also the OpenTelemetry OTLP/gRPC default). Quit it or set CCM_PORT. ({e})", cfg.port);
                sampling.stop();
                return PORT_IN_USE_EXIT;
            }
            Err(e) => {
                eprintln!("[collector] fatal: {e}");
                sampling.stop();
                return 1;
            }
        };
        emit(&listening(server.port()));
        eprintln!("[collector] watching {}", cfg.monitor.claude_root.display());
        eprintln!("[collector] data dir {}", data.display());
        let setup = Setup {
            handle: handle.clone(),
            config: cfg.monitor.clone(),
            deps,
            cache: ProcCache::system(),
            alerts: alerts.clone(),
            sensitive,
            sampling: sampling.clone(),
            host,
            roots,
            new_projects,
            sensitive_events,
            emit: emit.clone(),
            host_rpc: cfg.host_rpc,
            stats: cfg.stats,
        };
        let wake_tx = sender(&tx);
        let worker = std::thread::Builder::new()
            .name("monitor".into())
            .spawn(move || {
                let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                    if let Ok(t) = wake_tx.lock() {
                        let _ = t.send(Cmd::Wake);
                    }
                });
                runtime::run(setup, rx, wake);
            })
            .ok();
        let (ctl_tx, mut ctl_rx) = tokio::sync::mpsc::unbounded_channel();
        spawn_stdin(ctl_tx);
        while let Some(m) = ctl_rx.recv().await {
            match m {
                ControlIn::Shutdown | ControlIn::Eof => break,
                ControlIn::Message(v) => control(&v, host_client.as_deref(), &sampling, &tx),
            }
        }
        if let Some(c) = &host_client {
            c.close();
        }
        let _ = tx.send(Cmd::Shutdown);
        sampling.stop();
        if let Some(w) = worker {
            let _ = tokio::task::spawn_blocking(move || w.join()).await;
        }
        server.shutdown().await;
        0
    })
}

fn control(v: &Value, host: Option<&HostClient>, sampling: &Sampling, tx: &std::sync::mpsc::Sender<Cmd>) {
    if host.is_some_and(|c| c.on_message(v)) {
        return;
    }
    match v.get("type").and_then(Value::as_str) {
        Some("device.used") => sampling.device_used(v),
        Some("disk.alert" | "agent.note" | "toast.health") => {
            let _ = tx.send(Cmd::Control(v.clone()));
        }
        _ => {}
    }
}

fn hooks(tx: &std::sync::mpsc::Sender<Cmd>, handle: &StoreHandle, alerts: &alerts::SharedAlerts, sampling: &Arc<Sampling>, host: &Arc<HostOps>) -> Hooks {
    let msg_tx = sender(tx);
    let focus_tx = sender(tx);
    let (count_alerts, count_sampling) = (alerts.clone(), sampling.clone());
    let (close_alerts, close_host) = (alerts.clone(), host.clone());
    let (snooze_store, snooze_alerts) = (handle.clone(), alerts.clone());
    Hooks {
        on_client_message: Some(Arc::new(move |m: ClientMessage, ctx: &ClientContext| {
            if let Ok(t) = msg_tx.lock() {
                let _ = t.send(Cmd::Client(Box::new((m, ctx.clone()))));
            }
        })),
        on_client_count: Some(Arc::new(move |n| {
            count_sampling.set_windows(n);
            count_sampling.sync(lock(&count_alerts).toaster.idle_watch());
        })),
        on_client_close: Some(Arc::new(move |id| {
            lock(&close_alerts).toaster.drop_client(id);
            close_host.drop_client(id);
        })),
        on_focus_run: Some(Arc::new(move |id, key| {
            let (done, wait) = tokio::sync::oneshot::channel();
            if let Ok(t) = focus_tx.lock() {
                let _ = t.send(Cmd::FocusRun(id, key, done));
            }
            Box::pin(async move { wait.await.ok().flatten() })
        })),
        on_snooze_link: Some(Arc::new(move |id: &str, key: Option<&str>, minutes: f64| {
            snooze_store.with(|s| {
                let Some(t) = s.get_terminal(id).cloned() else { return false };
                let mut a = lock(&snooze_alerts);
                if !a.toaster.accepts(key) || !a.snoozes.set(&t, Some(minutes)) {
                    return false;
                }
                s.set_snoozes(a.snoozes.all());
                a.toaster.hide_for(id);
                a.pump();
                true
            })
        })),
    }
}
