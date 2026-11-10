use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

use core_monitor::job_stop::JobStopResult;
use core_monitor::script::ScriptDeps;
use core_monitor::{Monitor, MonitorConfig, MonitorStores};
use core_store::time::now_ms;
use core_store::{Store, StoreOptions};
use serde_json::{json, Value};

const SID: &str = "1f2e3d4c-5b6a-4798-8a1b-2c3d4e5f6a7b";
const NOW: i64 = 1_791_367_000_000;
const REL: &str = "D--w\\1f2e3d4c-5b6a-4798-8a1b-2c3d4e5f6a7b.jsonl";

fn append(file: &Path, v: Value) {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap();
    writeln!(f, "{v}").unwrap();
}

fn tool_use(id: &str, at: &str) -> Value {
    json!({"type":"assistant","timestamp":at,"message":{"id":format!("m-{id}"),"model":"claude-opus-5-5","stop_reason":"tool_use","usage":{"input_tokens":3,"output_tokens":2},"content":[{"type":"tool_use","id":id,"name":"Read","input":{}}]}})
}

fn tool_result(id: &str, at: &str) -> Value {
    json!({"type":"user","timestamp":at,"message":{"content":[{"type":"tool_result","tool_use_id":id,"content":"ok"}]}})
}

struct Rig {
    root: PathBuf,
    main: PathBuf,
    monitor: Monitor,
    store: Store,
}

fn rig(tag: &str, reconcile_ms: Option<i64>) -> Rig {
    let root = std::env::temp_dir().join(format!("ccm-presync-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("projects").join("D--w");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(root.join("sessions")).unwrap();
    let main = project.join(format!("{SID}.jsonl"));
    append(&main, json!({"type":"user","timestamp":"2026-10-07T09:59:00.000Z","message":{"content":"hi"}}));
    append(&main, tool_use("toolu_1", "2026-10-07T09:59:01.000Z"));
    std::fs::write(root.join("sessions").join("4242.json"), json!({"pid": 4242, "sessionId": SID, "cwd": "D:\\w", "startedAt": 1_791_366_000_000i64, "status": "busy"}).to_string()).unwrap();
    let deps = Arc::new(ScriptDeps { now: AtomicI64::new(NOW), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
    let mut config = MonitorConfig::new(root.clone(), root.join("data"));
    config.verify_processes = false;
    if let Some(ms) = reconcile_ms {
        config.reconcile_ms = ms;
    }
    let mut monitor = Monitor::new(config, deps.clone(), MonitorStores::memory(deps), None);
    let mut store = Store::new(StoreOptions { activity_limit: 100, batch_ms: 50 });
    monitor.start(&mut store, reconcile_ms.is_some());
    Rig { root, main, monitor, store }
}

fn state(store: &Store) -> Value {
    let mut v = serde_json::to_value(store.snapshot()).unwrap();
    for a in v["activity"].as_array_mut().unwrap() {
        a.as_object_mut().unwrap().remove("id");
    }
    json!({ "terminals": v["terminals"], "agents": v["agents"], "activity": v["activity"] })
}

fn changed(r: &mut Rig) {
    r.monitor.project_changed(REL.to_owned(), now_ms() - 1_000);
}

#[test]
fn a_presynced_tick_reads_nothing_under_the_store_lock() {
    let mut plain = rig("plain", None);
    let mut ahead = rig("ahead", None);
    for r in [&mut plain, &mut ahead] {
        append(&r.main, tool_result("toolu_1", "2026-10-07T09:59:02.000Z"));
        append(&r.main, tool_use("toolu_2", "2026-10-07T09:59:03.000Z"));
        changed(r);
    }
    plain.monitor.tick(&mut plain.store);
    ahead.monitor.presync(now_ms());
    append(&ahead.main, tool_result("toolu_2", "2026-10-07T09:59:04.000Z"));
    append(&ahead.main, tool_use("toolu_3", "2026-10-07T09:59:05.000Z"));
    ahead.monitor.tick(&mut ahead.store);
    let after = state(&ahead.store);
    assert_eq!(state(&plain.store), after);
    assert!(!serde_json::to_string(&after).unwrap().contains("09:59:05.000Z"));
    changed(&mut ahead);
    ahead.monitor.presync(now_ms());
    ahead.monitor.tick(&mut ahead.store);
    assert!(serde_json::to_string(&state(&ahead.store)).unwrap().contains("09:59:05.000Z"));
    for r in [plain, ahead] {
        let _ = std::fs::remove_dir_all(&r.root);
    }
}

#[test]
fn a_session_gone_from_the_registry_is_read_before_the_lock_and_ended_under_it() {
    let mut r = rig("gone", Some(1));
    append(&r.main, tool_result("toolu_1", "2026-10-07T09:59:02.000Z"));
    std::fs::remove_file(r.root.join("sessions").join("4242.json")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let now = now_ms();
    r.monitor.prepare(now);
    r.monitor.presync(now);
    append(&r.main, tool_use("toolu_9", "2026-10-07T09:59:06.000Z"));
    r.monitor.tick(&mut r.store);
    let s = state(&r.store);
    let terminal = &s["terminals"][0];
    assert!(terminal["endedAt"].is_string(), "{terminal}");
    let text = serde_json::to_string(&s).unwrap();
    assert!(!text.contains("09:59:06.000Z"));
    assert!(s["activity"].as_array().unwrap().iter().any(|a| a["kind"] == "terminal.ended"));
    r.monitor.stop();
    let _ = std::fs::remove_dir_all(&r.root);
}
