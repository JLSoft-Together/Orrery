use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

use core_monitor::job_stop::JobStopResult;
use core_monitor::script::ScriptDeps;
use core_monitor::{Monitor, MonitorConfig, MonitorStores};
use core_store::{Store, StoreOptions};
use serde_json::{json, Value};

const SID: &str = "0e1d2c3b-4a59-4687-9a1b-2c3d4e5f6a7b";
const NOW: i64 = 1_791_367_000_000;

fn append(file: &Path, v: Value) {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap();
    writeln!(f, "{v}").unwrap();
}

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!("ccm-prepare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("projects").join("D--w");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(root.join("sessions")).unwrap();
    let main = project.join(format!("{SID}.jsonl"));
    append(&main, json!({"type":"user","timestamp":"2026-10-07T09:59:00.000Z","message":{"content":"hi"}}));
    append(&main, json!({"type":"assistant","timestamp":"2026-10-07T09:59:01.000Z","message":{"id":"m1","model":"claude-opus-5-5","stop_reason":"tool_use","usage":{"input_tokens":3,"output_tokens":2},"content":[{"type":"tool_use","id":"toolu_1","name":"Read","input":{}}]}}));
    std::fs::write(root.join("sessions").join("4242.json"), json!({"pid": 4242, "sessionId": SID, "cwd": "D:\\w", "startedAt": 1_791_366_000_000i64, "status": "busy"}).to_string()).unwrap();
    root
}

fn run(root: &Path, prepare: bool) -> (Vec<Value>, Vec<Value>) {
    let deps = Arc::new(ScriptDeps { now: AtomicI64::new(NOW), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
    let mut config = MonitorConfig::new(root.to_path_buf(), root.join("data"));
    config.verify_processes = false;
    let mut monitor = Monitor::new(config, deps.clone(), MonitorStores::memory(deps), None);
    let mut store = Store::new(StoreOptions { activity_limit: 100, batch_ms: 50 });
    if prepare {
        monitor.prepare(NOW);
    }
    monitor.start(&mut store, false);
    let terminals = store.terminal_list().map(|t| serde_json::to_value(t).unwrap()).collect();
    let agents = store.agent_list().map(|a| serde_json::to_value(a).unwrap()).collect();
    (terminals, agents)
}

#[test]
fn trackers_made_ahead_of_start_publish_the_same_state() {
    let root = fixture();
    let plain = run(&root, false);
    let ahead = run(&root, true);
    assert_eq!(plain.0.len(), 1);
    assert_eq!(plain, ahead);
    let _ = std::fs::remove_dir_all(&root);
}
