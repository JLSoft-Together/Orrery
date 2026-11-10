use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use core_store::event::{FocusWindowResult, MonitorEvent, OpenApp, OpenResult, ServerMessage};
use core_store::store::{Store, StoreOptions};
use serde_json::{json, Value};

use crate::agentprocs::AgentProcs;
use crate::codex::process::{CodexProcess, ProcessMode};
use crate::deepseek::process::DeepseekProcess;
use crate::gitstat::DiffStatReader;
use crate::job_stop::JobStopResult;
use crate::monitor::{Deps, LaunchError, LaunchMode, Monitor, MonitorConfig, MonitorStores, TerminalApp};
use crate::process::ProcessInfo;

pub struct ScriptDeps {
    pub now: AtomicI64,
    pub dead: Mutex<Vec<u32>>,
    pub stop_result: Mutex<JobStopResult>,
    pub stops: Mutex<Vec<String>>,
}

fn stop_of(s: &str) -> JobStopResult {
    match s {
        "ok" => JobStopResult::Ok,
        "notFound" => JobStopResult::NotFound,
        "notRunning" => JobStopResult::NotRunning,
        "unconfirmed" => JobStopResult::Unconfirmed,
        "noCli" => JobStopResult::NoCli,
        _ => JobStopResult::Failed,
    }
}

impl Deps for ScriptDeps {
    fn now(&self) -> i64 {
        self.now.load(Ordering::SeqCst)
    }
    fn is_pid_alive(&self, pid: u32) -> bool {
        !self.dead.lock().is_ok_and(|d| d.contains(&pid))
    }
    fn query_processes(&self, _: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
        None
    }
    fn stop_job(&self, job_id: &str) -> JobStopResult {
        if let Ok(mut s) = self.stops.lock() {
            s.push(job_id.to_owned());
        }
        self.stop_result.lock().map_or(JobStopResult::Failed, |r| *r)
    }
    fn launch(&self, _: &str, _: LaunchMode, _: Option<TerminalApp>) -> Option<Result<(), LaunchError>> {
        None
    }
    fn focus_window(&self, _: u32) -> Option<FocusWindowResult> {
        None
    }
    fn open_folder(&self, _: &str, _: OpenApp) -> Option<OpenResult> {
        None
    }
}

pub struct FakeProcs {
    pub codex: Mutex<Option<Vec<CodexProcess>>>,
    pub deepseek: Mutex<Option<Vec<DeepseekProcess>>>,
}

impl AgentProcs for FakeProcs {
    fn codex(&self) -> Option<Vec<CodexProcess>> {
        self.codex.lock().ok().and_then(|l| l.clone())
    }
    fn deepseek(&self) -> Option<Vec<DeepseekProcess>> {
        self.deepseek.lock().ok().and_then(|l| l.clone())
    }
}

fn u32s(v: Option<&Value>) -> Vec<u32> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as u32)).collect()).unwrap_or_default()
}

fn codex_list(v: &Value) -> Option<Vec<CodexProcess>> {
    let list = v.get("list")?.as_array()?;
    Some(
        list.iter()
            .map(|p| CodexProcess {
                pid: n(p, "pid").unwrap_or(0) as u32,
                start: n(p, "start").unwrap_or(0),
                mode: match s(p, "mode").as_deref() {
                    Some("exec") => ProcessMode::Exec,
                    Some("app-server") => ProcessMode::AppServer,
                    Some("other") => ProcessMode::Other,
                    _ => ProcessMode::Tui,
                },
                resume_id: s(p, "resumeId"),
                ancestors: u32s(p.get("ancestors")),
                orphan: p.get("orphan").and_then(Value::as_bool).unwrap_or(false),
            })
            .collect(),
    )
}

fn deepseek_list(v: &Value) -> Option<Vec<DeepseekProcess>> {
    let list = v.get("list")?.as_array()?;
    Some(list.iter().map(|p| DeepseekProcess { pid: n(p, "pid").unwrap_or(0) as u32, start: n(p, "start").unwrap_or(0), ancestors: u32s(p.get("ancestors")), workspace: s(p, "workspace"), resume_id: s(p, "resumeId") }).collect())
}

pub struct Script {
    pub store: Store,
    pub monitor: Monitor,
    deps: Arc<ScriptDeps>,
    procs: Arc<FakeProcs>,
    frames: Arc<Mutex<Vec<ServerMessage>>>,
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

fn n(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_f64).map(|x| x as i64)
}

impl Script {
    pub fn init(v: &Value) -> Result<Script, String> {
        let root = PathBuf::from(s(v, "claudeRoot").ok_or("claudeRoot")?);
        let data = PathBuf::from(s(v, "dataDir").ok_or("dataDir")?);
        let deps = Arc::new(ScriptDeps { now: AtomicI64::new(n(v, "now").unwrap_or(0)), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
        let mut config = MonitorConfig::new(root, data);
        config.verify_processes = false;
        config.safety_net_ms = None;
        config.diff_delay_ms = n(v, "diffDelayMs").unwrap_or(0);
        config.reread_finished = v.get("rereadFinished").and_then(Value::as_bool).unwrap_or(true);
        if let Some(t) = n(v, "staleTtlMs") {
            config.stale_ttl_ms = t;
        }
        let diff = v.get("diff").and_then(Value::as_bool).unwrap_or(false).then(DiffStatReader::system);
        if v.get("codex").and_then(Value::as_bool).unwrap_or(false) {
            config.codex_home = Some(PathBuf::from(s(v, "codexHome").ok_or("codexHome")?));
        }
        if v.get("deepseek").and_then(Value::as_bool).unwrap_or(false) {
            config.codewhale_home = Some(PathBuf::from(s(v, "codewhaleHome").ok_or("codewhaleHome")?));
            config.home_dir = PathBuf::from(s(v, "homeDir").unwrap_or_default());
        }
        let stores = MonitorStores::memory(deps.clone());
        let mut monitor = Monitor::new(config, deps.clone(), stores, diff);
        let procs = Arc::new(FakeProcs { codex: Mutex::new(Some(Vec::new())), deepseek: Mutex::new(Some(Vec::new())) });
        monitor.enable_agents(procs.clone(), false, true);
        let mut store = Store::new(StoreOptions { activity_limit: n(v, "activityLimit").unwrap_or(300) as usize, batch_ms: n(v, "batchMs").unwrap_or(1) });
        let frames: Arc<Mutex<Vec<ServerMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = frames.clone();
        store.subscribe(Box::new(move |m| {
            if let Ok(mut f) = sink.lock() {
                f.push(m.clone());
            }
            Ok(())
        }));
        Ok(Script { store, monitor, deps, procs, frames })
    }

    pub fn take_frames(&mut self) -> Vec<ServerMessage> {
        self.frames.lock().map(|mut f| std::mem::take(&mut *f)).unwrap_or_default()
    }

    pub fn run(&mut self, v: &Value) -> Result<Value, String> {
        let cmd = s(v, "cmd").ok_or("cmd")?;
        let mut result = Value::Null;
        let (store, m) = (&mut self.store, &mut self.monitor);
        let sink = self.frames.clone();
        let mut reply = |e: MonitorEvent| {
            if let Ok(mut f) = sink.lock() {
                f.push(ServerMessage::direct(e));
            }
        };
        match cmd.as_str() {
            "now" => self.deps.now.store(n(v, "value").ok_or("value")?, Ordering::SeqCst),
            "dead" => {
                if let (Ok(mut d), Some(p)) = (self.deps.dead.lock(), n(v, "pid")) {
                    d.push(p as u32);
                }
            }
            "stopResult" => {
                if let Ok(mut r) = self.deps.stop_result.lock() {
                    *r = stop_of(&s(v, "value").unwrap_or_default());
                }
            }
            "start" => m.start(store, false),
            "reconcile" => m.reconcile(store, false, false),
            "pollExtras" => m.poll_extras(store),
            "refreshHints" => m.refresh_hints(store),
            "flush" => store.flush(),
            "snapshot" => {
                let snap = store.snapshot_message();
                if let Ok(mut f) = self.frames.lock() {
                    f.push(snap);
                }
            }
            "toolSpans" => m.send_tool_spans(&mut reply, s(v, "terminalId").as_deref(), s(v, "since").as_deref()),
            "alias" => result = Value::Bool(m.set_alias(store, &s(v, "terminalId").unwrap_or_default(), v.get("alias").unwrap_or(&Value::Null))),
            "note" => result = Value::Bool(m.set_note(store, &s(v, "terminalId").unwrap_or_default(), v.get("note").unwrap_or(&Value::Null))),
            "history" => m.send_history(store, Some(&mut reply)),
            "timeline" => m.send_timeline(store, Some(&mut reply)),
            "dismiss" => result = json!(m.dismiss_ended(store, s(v, "terminalId").as_deref())),
            "dismissJobs" => result = json!(m.dismiss_jobs(store, s(v, "jobId").as_deref())),
            "stopJob" => result = Value::Bool(m.stop_job(&s(v, "jobId").unwrap_or_default())),
            "toggleFavorite" => result = Value::Bool(m.toggle_favorite(store, &s(v, "terminalId").unwrap_or_default())),
            "addFavorite" => result = Value::Bool(m.add_favorite(store, &s(v, "dir").unwrap_or_default(), v.get("label").unwrap_or(&Value::Null))),
            "codexProcs" => *self.procs.codex.lock().map_err(|_| "lock")? = codex_list(v),
            "deepseekProcs" => *self.procs.deepseek.lock().map_err(|_| "lock")? = deepseek_list(v),
            "startAgents" => m.start_agents(store, false),
            "codexSync" => {
                let (id, file) = (s(v, "id").unwrap_or_default(), s(v, "file").unwrap_or_default());
                m.with_codex(store, |c, st, _| c.sync(st, &id, std::path::Path::new(&file)));
            }
            "codexScan" => {
                m.with_codex(store, |c, st, tg| c.scan_now(st, tg));
            }
            "codexNames" => {
                m.with_codex(store, |c, st, _| c.reload_names(st));
            }
            "codexTick" => {
                m.with_codex(store, |c, st, _| c.reconcile(st));
            }
            "deepseekRead" => {
                if let Some(d) = m.deepseek.as_mut() {
                    d.read_outbox(store, false);
                }
            }
            "deepseekResolve" => {
                if let Some(d) = m.deepseek.as_mut() {
                    d.resolve_outbox();
                }
            }
            "deepseekScan" => {
                if let Some(d) = m.deepseek.as_mut() {
                    d.scan_now(store);
                }
            }
            "deepseekTick" => {
                if let Some(d) = m.deepseek.as_mut() {
                    d.reconcile(store);
                }
            }
            "agentInfo" => {
                let id = s(v, "terminalId").unwrap_or_default();
                let codex = m.codex.as_ref();
                let deep = m.deepseek.as_ref();
                let (pid, cwd, live) = if id.starts_with("codex:") {
                    (codex.and_then(|c| c.pid_of(&id)), codex.and_then(|c| c.cwd_of(&id)), codex.is_some_and(|c| c.is_live(&id)))
                } else {
                    (deep.and_then(|d| d.pid_of(&id)), deep.and_then(|d| d.cwd_of(&id)), false)
                };
                result = json!({ "pid": pid, "cwd": cwd, "live": live });
            }
            "diagnostics" => {
                result = serde_json::to_value(m.diagnostics(None, 0)).map_err(|e| e.to_string())?;
            }
            "wait" => m.wait_background(store, std::time::Duration::from_millis(n(v, "ms").unwrap_or(10_000) as u64)),
            "stops" => result = json!(self.deps.stops.lock().map(|s| s.clone()).unwrap_or_default()),
            other => return Err(format!("unknown cmd {other}")),
        }
        let frames: Vec<Value> = self.take_frames().into_iter().filter_map(|f| serde_json::to_value(f).ok()).collect();
        Ok(json!({ "ok": true, "result": result, "frames": frames }))
    }
}
