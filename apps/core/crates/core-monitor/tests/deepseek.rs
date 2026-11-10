use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

use core_monitor::deepseek::process::DeepseekProcess;
use core_monitor::deepseek::{DeepseekConfig, DeepseekMonitor};
use core_monitor::job_stop::JobStopResult;
use core_monitor::script::{FakeProcs, ScriptDeps};
use core_store::model::{AgentRole, AgentStatus, Provider, TerminalStatus};
use core_store::time::iso_of_ms;
use core_store::{Store, StoreOptions};
use serde_json::{json, Value};

const T0: i64 = 1_793_700_000_000;

fn at(s: i64) -> String {
    iso_of_ms(T0 + s * 1000)
}

fn line(event: &str, kind: &str, s: i64, thread: &str, payload: Value) -> String {
    let mut p = json!({ "workspace": "/home/cw/wt-lane" });
    if let (Some(p), Some(o)) = (p.as_object_mut(), payload.as_object()) {
        p.extend(o.clone());
    }
    format!("{}\n", json!({ "schema_version": 1, "seq": 1, "event": event, "kind": kind, "thread_id": thread, "turn_id": null, "item_id": null, "timestamp": at(s), "payload": p }))
}

fn proc(pid: u32, start: i64, ancestors: Vec<u32>) -> DeepseekProcess {
    DeepseekProcess { pid, start, ancestors, workspace: None, resume_id: None }
}

struct Rig {
    home: PathBuf,
    outbox: PathBuf,
    store: Store,
    deps: Arc<ScriptDeps>,
    procs: Arc<FakeProcs>,
}

impl Rig {
    fn new(name: &str) -> Rig {
        let home = std::env::temp_dir().join(format!("ccm-ds-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let outbox = home.join("notifications").join("outbox.jsonl");
        std::fs::create_dir_all(outbox.parent().unwrap()).unwrap();
        std::fs::write(home.join("config.toml"), "[lifecycle_outbox]\npath = \"~/notifications/outbox.jsonl\"\n").unwrap();
        let deps = Arc::new(ScriptDeps { now: AtomicI64::new(T0 + 10_000), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
        let procs = Arc::new(FakeProcs { codex: Mutex::new(Some(Vec::new())), deepseek: Mutex::new(Some(vec![proc(500, T0 - 500, vec![400])])) });
        Rig { home, outbox, store: Store::new(StoreOptions { activity_limit: 100, batch_ms: 1 }), deps, procs }
    }

    fn monitor(&self) -> DeepseekMonitor {
        let config = DeepseekConfig { codewhale_home: self.home.clone(), home_dir: self.home.clone(), stale_ttl_ms: 120 * 60_000, reconcile_ms: 5_000, verify_ms: 60_000, verify_processes: true };
        DeepseekMonitor::new(config, self.deps.clone(), self.procs.clone())
    }

    fn append(&self, text: &str) {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.outbox).unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    fn kinds(&mut self) -> Vec<String> {
        let snap = serde_json::to_value(self.store.snapshot_message()).unwrap();
        snap["events"][0]["payload"]["activity"].as_array().unwrap().iter().filter_map(|a| a["kind"].as_str().map(str::to_owned)).collect()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

#[test]
fn rebuilds_a_live_session_at_start_links_its_process_and_follows_turns_and_subagents() {
    let mut rig = Rig::new("live");
    rig.append(&(line("session_start", "session.started", 0, "th-1", json!({})) + &line("turn_start", "turn.started", 1, "th-1", json!({}))));
    let mut ds = rig.monitor();
    ds.start(&mut rig.store, false);
    let t = rig.store.get_terminal("deepseek:th-1").expect("terminal").clone();
    assert_eq!((t.provider, t.status, t.process_id, t.deepseek_session_id.as_deref(), t.cwd.as_deref(), t.title.as_str()), (Some(Provider::Deepseek), TerminalStatus::Working, Some(500), Some("th-1"), Some("/home/cw/wt-lane"), "wt-lane"));
    assert_eq!(rig.store.get_agent("deepseek:th-1:main").map(|a| (a.role, a.provider, a.status)), Some((AgentRole::Main, Some(Provider::Deepseek), AgentStatus::Working)));
    assert_eq!(ds.pid_of("deepseek:th-1"), Some(500));

    rig.append(&(line("subagent_spawn", "subagent.spawned", 2, "th-1", json!({ "subagent": { "id": "sa", "name": "explorer" } })) + &line("turn_end", "turn.failed", 3, "th-1", json!({}))));
    ds.read_outbox(&mut rig.store, false);
    assert_eq!(rig.store.get_terminal("deepseek:th-1").map(|t| t.status), Some(TerminalStatus::Idle));
    let sub = rig.store.get_agent("deepseek:th-1:sub:sa").expect("sub").clone();
    assert_eq!((sub.parent_id.as_deref(), sub.role, sub.status, sub.r#type.as_deref()), (Some("deepseek:th-1:main"), AgentRole::Subagent, AgentStatus::Working, Some("explorer")));
    rig.append(&line("subagent_complete", "subagent.completed", 4, "th-1", json!({ "subagent": { "id": "sa" } })));
    ds.read_outbox(&mut rig.store, false);
    assert_eq!(rig.store.get_agent("deepseek:th-1:sub:sa").map(|a| a.status), Some(AgentStatus::Completed));
    let kinds = rig.kinds();
    for k in ["agent.started", "turn.failed", "agent.completed"] {
        assert!(kinds.iter().any(|x| x == k), "{k} in {kinds:?}");
    }
    assert!(!kinds.iter().any(|x| x == "turn.started"));

    rig.append(&line("session_end", "session.ended", 5, "th-1", json!({})));
    ds.read_outbox(&mut rig.store, false);
    let ended = rig.store.get_terminal("deepseek:th-1").expect("ended").clone();
    assert_eq!((ended.status, ended.ended_at.as_deref(), ended.process_id), (TerminalStatus::Stale, Some(at(5).as_str()), None));
    assert_eq!(ds.dismiss_ended(&mut rig.store, Some("deepseek:th-1")), 1);
    assert!(rig.store.get_terminal("deepseek:th-1").is_none());
    ds.stop();
}

#[test]
fn skips_ended_or_process_less_sessions_at_start_and_ends_a_session_whose_process_exits() {
    let mut rig = Rig::new("ended");
    rig.append(&(line("session_start", "session.started", 0, "done", json!({})) + &line("session_end", "session.ended", 1, "done", json!({})) + &line("session_start", "session.started", 0, "th-1", json!({}))));
    let mut ds = rig.monitor();
    ds.start(&mut rig.store, false);
    assert!(rig.store.get_terminal("deepseek:done").is_none());
    assert_eq!(rig.store.get_terminal("deepseek:th-1").map(|t| t.status), Some(TerminalStatus::Idle));
    rig.deps.dead.lock().unwrap().push(500);
    rig.deps.now.fetch_add(5_000, std::sync::atomic::Ordering::SeqCst);
    ds.reconcile(&mut rig.store);
    assert_eq!(rig.store.get_terminal("deepseek:th-1").map(|t| t.status), Some(TerminalStatus::Stale));
    ds.stop();

    *rig.procs.deepseek.lock().unwrap() = Some(Vec::new());
    rig.store = Store::new(StoreOptions { activity_limit: 100, batch_ms: 1 });
    let mut later = rig.monitor();
    later.start(&mut rig.store, false);
    assert!(rig.store.get_terminal("deepseek:th-1").is_none());
    later.stop();
}

#[test]
fn shows_a_new_session_only_after_a_scan_then_reports_its_first_turn() {
    let mut rig = Rig::new("scan");
    std::fs::write(&rig.outbox, "").unwrap();
    let mut ds = rig.monitor();
    ds.start(&mut rig.store, false);
    *rig.procs.deepseek.lock().unwrap() = Some(vec![proc(500, T0 - 500, vec![400]), proc(600, T0 + 59_000, vec![])]);
    rig.append(&(line("session_start", "session.started", 60, "th-2", json!({ "workspace": "/w/two" })) + &line("turn_start", "turn.started", 61, "th-2", json!({ "workspace": "/w/two" }))));
    ds.read_outbox(&mut rig.store, false);
    assert!(rig.store.get_terminal("deepseek:th-2").is_none());
    ds.scan_now(&mut rig.store);
    let t = rig.store.get_terminal("deepseek:th-2").expect("shown").clone();
    assert_eq!((t.process_id, t.status), (Some(600), TerminalStatus::Working));
    let kinds = rig.kinds();
    for k in ["terminal.started", "turn.started"] {
        assert!(kinds.iter().any(|x| x == k), "{k} in {kinds:?}");
    }
    ds.stop();
}

#[test]
fn stays_off_without_an_outbox_path() {
    let mut rig = Rig::new("off");
    std::fs::write(rig.home.join("config.toml"), "telemetry = false\n").unwrap();
    let mut ds = rig.monitor();
    ds.start(&mut rig.store, false);
    let d = ds.diagnostics();
    assert_eq!((d.outbox, d.threads, d.processes), (None, 0, 0));
    ds.stop();
}

#[test]
fn follows_outbox_appends_through_the_file_watcher() {
    let mut rig = Rig::new("watch");
    std::fs::write(&rig.outbox, "").unwrap();
    let mut ds = rig.monitor();
    ds.start(&mut rig.store, true);
    rig.append(&(line("session_start", "session.started", 0, "th-w", json!({})) + &line("turn_start", "turn.started", 1, "th-w", json!({}))));
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while rig.store.get_terminal("deepseek:th-w").is_none() && std::time::Instant::now() < until {
        ds.tick(&mut rig.store);
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    assert_eq!(rig.store.get_terminal("deepseek:th-w").map(|t| t.status), Some(TerminalStatus::Working));
    ds.stop();
}
