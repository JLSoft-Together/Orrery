use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

use core_monitor::codex::process::{CodexProcess, ProcessMode};
use core_monitor::codex::{CodexConfig, CodexMonitor};
use core_monitor::job_stop::JobStopResult;
use core_monitor::monitor::ClaudeTarget;
use core_monitor::script::{FakeProcs, ScriptDeps};
use core_monitor::view::blank_terminal;
use core_store::model::{AgentRole, AgentStatus, Provider, TerminalStatus};
use core_store::persist::localtime::{civil_from_days, Zone, DAY_MS};
use core_store::time::iso_of_ms;
use core_store::{Store, StoreOptions};
use serde_json::{json, Value};

const TID: &str = "01a125e6-f212-7711-a885-025fd33ab05a";
const T0: i64 = 1_791_702_000_000;

fn rec(kind: &str, payload: Value, at: i64) -> Value {
    json!({ "timestamp": iso_of_ms(at), "ordinal": 0, "type": kind, "payload": payload })
}

fn meta(id: &str, over: Value) -> Value {
    let mut p = json!({ "id": id, "session_id": id, "timestamp": iso_of_ms(T0), "cwd": "C:\\Temp\\art-hero-x1", "originator": "codex_exec", "cli_version": "0.162.0", "source": "exec", "base_instructions": "secret ".repeat(50) });
    if let (Some(p), Some(o)) = (p.as_object_mut(), over.as_object()) {
        p.extend(o.clone());
    }
    rec("session_meta", p, T0)
}

fn started() -> Value {
    rec("event_msg", json!({ "type": "task_started", "turn_id": "t1", "model_context_window": 258_400 }), T0)
}

fn context() -> Value {
    rec("turn_context", json!({ "turn_id": "t1", "model": "gpt-6-luna", "effort": "medium" }), T0)
}

fn call(id: &str) -> Value {
    rec("response_item", json!({ "type": "custom_tool_call", "call_id": id, "name": "exec", "input": "secret command" }), T0)
}

fn tokens(used: u32) -> Value {
    let usage = |i: u32, c: u32, o: u32| json!({ "input_tokens": i, "cached_input_tokens": c, "output_tokens": o, "total_tokens": i + o });
    rec(
        "event_msg",
        json!({
            "type": "token_count",
            "info": { "total_token_usage": usage(18_000, 12_000, 2_000), "last_token_usage": usage(18_000, 12_000, 2_000), "model_context_window": 258_400 },
            "rate_limits": { "limit_id": "codex", "primary": { "used_percent": used, "window_minutes": 300, "resets_at": 1_791_650_000 }, "secondary": { "used_percent": 12, "window_minutes": 10_080, "resets_at": 1_792_000_000 }, "plan_type": "plus" }
        }),
        T0,
    )
}

fn complete() -> Value {
    rec("event_msg", json!({ "type": "task_complete", "turn_id": "t1", "last_agent_message": "secret answer" }), T0)
}

struct Rig {
    home: PathBuf,
    dir: PathBuf,
    store: Store,
    deps: Arc<ScriptDeps>,
    procs: Arc<FakeProcs>,
    targets: Arc<Mutex<Vec<ClaudeTarget>>>,
}

impl Rig {
    fn new(name: &str) -> Rig {
        let home = std::env::temp_dir().join(format!("ccm-codex-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let now = T0 + 10_000;
        let local = now + Zone::Local.offset_ms(now);
        let (y, m, d) = civil_from_days(local.div_euclid(DAY_MS));
        let dir = home.join("sessions").join(y.to_string()).join(format!("{m:02}")).join(format!("{d:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        let deps = Arc::new(ScriptDeps { now: AtomicI64::new(now), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
        let procs = Arc::new(FakeProcs { codex: Mutex::new(Some(Vec::new())), deepseek: Mutex::new(Some(Vec::new())) });
        Rig { home, dir, store: Store::new(StoreOptions { activity_limit: 100, batch_ms: 1 }), deps, procs, targets: Arc::new(Mutex::new(Vec::new())) }
    }

    fn monitor(&self) -> CodexMonitor {
        let config = CodexConfig { codex_home: self.home.clone(), stale_ttl_ms: 120 * 60_000, reconcile_ms: 5_000, verify_ms: 60_000, verify_processes: true };
        CodexMonitor::new(config, self.deps.clone(), self.procs.clone(), None)
    }

    fn start(&mut self, codex: &mut CodexMonitor) {
        let targets = self.targets.clone();
        codex.start(&mut self.store, &move |_| targets.lock().map(|t| t.clone()).unwrap_or_default(), false);
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("rollout-2026-10-11T14-00-00-{id}.jsonl"))
    }

    fn write(&self, id: &str, lines: &[Value]) {
        std::fs::write(self.file(id), lines.iter().map(|l| format!("{l}\n")).collect::<String>()).unwrap();
    }

    fn append(&self, id: &str, lines: &[Value]) {
        let mut f = std::fs::OpenOptions::new().append(true).open(self.file(id)).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
    }

    fn set_procs(&self, list: Vec<CodexProcess>) {
        *self.procs.codex.lock().unwrap() = Some(list);
    }

    fn kill(&self, pid: u32) {
        self.deps.dead.lock().unwrap().push(pid);
    }

    fn snapshot(&mut self) -> Value {
        serde_json::to_value(self.store.snapshot_message()).unwrap()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn proc(pid: u32, start: i64, ancestors: Vec<u32>) -> CodexProcess {
    CodexProcess { pid, start, mode: ProcessMode::Exec, resume_id: None, ancestors, orphan: false }
}

fn payload(snapshot: &Value) -> &Value {
    &snapshot["events"][0]["payload"]
}

#[test]
fn puts_a_running_exec_launched_by_claude_into_that_session_under_the_calling_agent() {
    let mut rig = Rig::new("claude");
    rig.store.upsert_terminal(blank_terminal("claude-1", "demo", TerminalStatus::Working));
    rig.targets.lock().unwrap().push(ClaudeTarget {
        terminal_id: "claude-1".into(),
        pid: 100,
        cwd: None,
        main_agent_id: "claude-1:main".into(),
        ended: false,
        last_activity_at: None,
        codex_calls: vec![("sub-a".into(), iso_of_ms(T0 - 1_000))],
    });
    rig.set_procs(vec![proc(300, T0 - 200, vec![250, 100])]);
    rig.write(TID, &[meta(TID, json!({})), started(), context(), call("c1"), tokens(7)]);
    let mut codex = rig.monitor();
    rig.start(&mut codex);
    let agent = rig.store.get_agent(&format!("codex:{TID}")).expect("agent").clone();
    assert_eq!((agent.terminal_id.as_str(), agent.parent_id.as_deref(), agent.role, agent.provider, agent.r#type.as_deref()), ("claude-1", Some("sub-a"), AgentRole::Subagent, Some(Provider::Codex), Some("exec")));
    assert_eq!((agent.name.as_deref(), agent.status, agent.current_tool.as_deref(), agent.model.as_deref()), (Some("art-hero-x1"), AgentStatus::Working, Some("exec"), Some("gpt-6-luna")));
    assert_eq!((agent.input_tokens, agent.cache_read_tokens, agent.output_tokens, agent.total_tokens, agent.context_tokens, agent.context_window), (Some(6_000), Some(12_000), Some(2_000), Some(8_000), Some(20_000), Some(258_400)));
    let snap = rig.snapshot();
    assert_eq!(payload(&snap)["codexLimits"]["plan"], "plus");
    assert_eq!(payload(&snap)["codexLimits"]["fiveHour"]["usedPct"], 7);
    assert!(!snap.to_string().contains("secret"));
    rig.kill(300);
    codex.reconcile(&mut rig.store);
    assert_eq!(rig.store.get_agent(&format!("codex:{TID}")).map(|a| a.status), Some(AgentStatus::Cancelled));
    codex.stop();
}

#[test]
fn shows_other_runs_as_their_own_session_and_ends_them_with_the_process_skipping_desktop_threads() {
    let mut rig = Rig::new("alone");
    rig.set_procs(vec![proc(400, T0 - 100, vec![1])]);
    rig.write(TID, &[meta(TID, json!({})), started(), context()]);
    let desktop = "01a125e6-0000-7000-8000-000000000001";
    rig.write(desktop, &[meta(desktop, json!({ "id": desktop, "session_id": desktop, "source": "vscode", "originator": "Codex Desktop" })), started()]);
    let mut codex = rig.monitor();
    rig.start(&mut codex);
    let terminal_id = format!("codex:{TID}");
    let t = rig.store.get_terminal(&terminal_id).expect("terminal").clone();
    assert_eq!((t.provider, t.status, t.process_id, t.entrypoint.as_deref(), t.codex_version.as_deref()), (Some(Provider::Codex), TerminalStatus::Working, Some(400), Some("exec"), Some("0.162.0")));
    assert_eq!(rig.store.get_agent(&format!("{terminal_id}:main")).map(|a| (a.role, a.status)), Some((AgentRole::Main, AgentStatus::Working)));
    assert!(rig.store.get_terminal(&format!("codex:{desktop}")).is_none());
    assert_eq!(codex.pid_of(&terminal_id), Some(400));

    rig.append(TID, &[complete()]);
    let file = rig.file(TID);
    codex.sync(&mut rig.store, TID, &file);
    assert_eq!(rig.store.get_terminal(&terminal_id).map(|t| t.status), Some(TerminalStatus::Idle));
    rig.kill(400);
    codex.reconcile(&mut rig.store);
    assert_eq!(rig.store.get_terminal(&terminal_id).map(|t| t.status), Some(TerminalStatus::Stale));
    assert_eq!(rig.store.get_agent(&format!("{terminal_id}:main")).map(|a| a.status), Some(AgentStatus::Completed));
    assert_eq!(codex.dismiss_ended(&mut rig.store, Some(&terminal_id)), 1);
    assert!(rig.store.get_terminal(&terminal_id).is_none());
    codex.stop();
}

#[test]
fn ignores_finished_and_orphaned_runs_at_startup_but_still_reads_the_latest_limits() {
    let mut rig = Rig::new("startup");
    rig.write(TID, &[meta(TID, json!({})), started(), tokens(33), complete()]);
    let killed = "01a125e6-0000-7000-8000-000000000002";
    rig.write(killed, &[meta(killed, json!({ "id": killed, "session_id": killed })), started(), call("c1")]);
    let mut codex = rig.monitor();
    rig.start(&mut codex);
    assert_eq!(rig.store.terminal_list().count(), 0);
    assert_eq!(rig.store.agent_list().count(), 0);
    let snap = rig.snapshot();
    assert_eq!(payload(&snap)["codexLimits"]["fiveHour"]["usedPct"], 33);
    codex.stop();
}

#[test]
fn a_new_thread_waits_for_one_process_scan_before_it_is_shown() {
    let mut rig = Rig::new("scan");
    let mut codex = rig.monitor();
    rig.start(&mut codex);
    rig.write(TID, &[meta(TID, json!({})), started(), context()]);
    let file = rig.file(TID);
    codex.sync(&mut rig.store, TID, &file);
    assert!(rig.store.get_terminal(&format!("codex:{TID}")).is_none());
    rig.set_procs(vec![proc(500, T0 - 50, vec![1])]);
    let targets = rig.targets.clone();
    codex.scan_now(&mut rig.store, &move |_| targets.lock().map(|t| t.clone()).unwrap_or_default());
    let t = rig.store.get_terminal(&format!("codex:{TID}")).expect("shown after the scan").clone();
    assert_eq!(t.process_id, Some(500));
    let snap = rig.snapshot();
    let kinds: Vec<&str> = payload(&snap)["activity"].as_array().unwrap().iter().filter_map(|a| a["kind"].as_str()).collect();
    assert!(kinds.contains(&"terminal.started"));
    codex.stop();
}

#[test]
fn follows_rollout_changes_and_the_scan_timer_through_the_file_watcher() {
    let mut rig = Rig::new("watch");
    rig.set_procs(vec![proc(500, T0 - 50, vec![1])]);
    let mut codex = rig.monitor();
    let targets = rig.targets.clone();
    let tg = move |_: &Store| targets.lock().map(|t| t.clone()).unwrap_or_default();
    codex.start(&mut rig.store, &tg, true);
    rig.write(TID, &[meta(TID, json!({})), started(), context()]);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while rig.store.get_terminal(&format!("codex:{TID}")).is_none() && std::time::Instant::now() < until {
        codex.tick(&mut rig.store, &tg);
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    assert_eq!(rig.store.get_terminal(&format!("codex:{TID}")).and_then(|t| t.process_id), Some(500));
    codex.stop();
}
