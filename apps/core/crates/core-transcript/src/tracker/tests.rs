use super::serve::Trackers;
use super::{Out, Plan, Step, SyncArgs, Tracker};
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const SID: &str = "11111111-2222-3333-4444-555555555555";
const NOW: i64 = 1_791_400_000_000;

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ccm-t3-{tag}-{}-{}", std::process::id(), NOW));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("-work-demo").join(SID).join("subagents")).unwrap();
    d
}

fn line(v: Value) -> String {
    format!("{v}\n")
}

fn tool_use(msg: &str, id: &str, name: &str, at: &str) -> String {
    line(json!({ "type": "assistant", "timestamp": at, "message": { "id": msg, "stop_reason": "tool_use", "usage": { "input_tokens": 10, "output_tokens": 2 }, "content": [{ "type": "tool_use", "id": id, "name": name, "input": { "command": "SECRET" } }] } }))
}

fn tool_result(id: &str, at: &str, err: bool) -> String {
    line(json!({ "type": "user", "timestamp": at, "message": { "content": [{ "type": "tool_result", "tool_use_id": id, "is_error": err, "content": "SECRET output" }] } }))
}

fn append(p: &Path, s: &str) {
    fs::OpenOptions::new().create(true).append(true).open(p).unwrap().write_all(s.as_bytes()).unwrap();
}

fn args(catch_up: bool, epoch: f64) -> SyncArgs {
    SyncArgs { session_id: Some(SID.into()), cwd: Some("/work/demo".into()), epoch, catch_up, plan: Plan { all: true, ..Plan::default() }, now: NOW }
}

fn run(t: &mut Tracker, a: SyncArgs, budget: u64) -> (Out, usize) {
    let mut out = Out::default();
    let mut pages = 1;
    let mut next = Some(a);
    while let Step::More = t.sync(next.take(), budget, &mut out) {
        pages += 1;
    }
    (out, pages)
}

fn fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = temp(tag);
    let main = dir.join("-work-demo").join(format!("{SID}.jsonl"));
    let subs = dir.join("-work-demo").join(SID).join("subagents");
    let mut body = tool_use("m0", "toolu_a1", "Agent", "2026-10-07T10:00:00.000Z");
    for i in 0..300 {
        body += &tool_use(&format!("m{i}"), &format!("t{i}"), "Read", "2026-10-07T10:00:01.000Z");
        body += &tool_result(&format!("t{i}"), "2026-10-07T10:00:02.000Z", i % 50 == 0);
    }
    append(&main, &body);
    fs::write(subs.join("agent-a1.meta.json"), r#"{"agentType":"Explore","toolUseId":"toolu_a1","description":"look"}"#).unwrap();
    append(&subs.join("agent-a1.jsonl"), &tool_use("s1", "g1", "Grep", "2026-10-07T10:00:03.000Z"));
    (dir, main, subs)
}

#[test]
fn catch_up_is_quiet_then_live_lines_report_activity() {
    let (dir, main, subs) = fixture("live");
    let mut t = Tracker::new("4242:1", dir.clone());
    let (out, _) = run(&mut t, args(true, 0.0), u64::MAX);
    assert!(out.activity.is_empty());
    assert_eq!(out.spans.iter().filter(|s| s.get("drop").is_none()).count(), 300 + 300 + 2);
    assert_eq!(out.spans.iter().filter(|s| s.get("drop").is_some()).count(), 300);
    let view = t.take_view().unwrap();
    assert_eq!(view["subs"][0]["agentId"], "a1");
    assert_eq!(view["subs"][0]["state"]["currentTool"], "Grep");
    assert!(t.take_view().is_none());
    append(&main, &tool_use("m9", "t9x", "Bash", "2026-10-07T10:01:00.000Z"));
    append(&main, &tool_result("t9x", "2026-10-07T10:01:01.000Z", true));
    append(&subs.join("agent-a1.jsonl"), &tool_result("g1", "2026-10-07T10:01:02.000Z", false));
    append(&main, &tool_result("toolu_a1", "2026-10-07T10:01:03.000Z", false));
    let (out, _) = run(&mut t, args(false, 0.0), u64::MAX);
    let kinds: Vec<&str> = out.activity.iter().map(|a| a["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["tool.started", "tool.failed", "agent.completed"]);
    assert_eq!(out.activity[1]["data"]["tool"], "Bash");
    assert!(!Value::Array(out.activity).to_string().contains("SECRET"));
    let view = t.take_view().unwrap();
    assert_eq!(view["subs"][0]["status"], "completed");
    assert!(view.to_string().find("SECRET").is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn pages_add_up_to_one_sync() {
    let (dir, ..) = fixture("pages");
    let mut whole = Tracker::new("4242:1", dir.clone());
    let (a, one) = run(&mut whole, args(true, 0.0), u64::MAX);
    let mut paged = Tracker::new("4242:1", dir.clone());
    let (b, many) = run(&mut paged, args(true, 0.0), 1000);
    assert_eq!(one, 1);
    assert!(many > 10, "{many} pages");
    assert_eq!(a.spans, b.spans);
    assert_eq!(whole.view(), paged.view());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_new_epoch_forgets_the_session_but_keeps_spans() {
    let (dir, ..) = fixture("epoch");
    let mut t = Tracker::new("4242:1", dir.clone());
    run(&mut t, args(true, 0.0), u64::MAX);
    assert!(t.take_view().is_some());
    let mut a = args(true, 1.0);
    a.session_id = Some("66666666-7777-8888-9999-000000000000".into());
    run(&mut t, a, u64::MAX);
    assert_eq!(t.take_view().unwrap()["subs"], json!([]));
    assert_eq!(t.close_spans("2026-10-07T10:05:00.000Z").len(), 2);
    let _ = fs::remove_dir_all(dir);
}

fn op(ts: &mut Trackers, op: &str, m: Value) -> Result<Map<String, Value>, &'static str> {
    let Value::Object(m) = m else { unreachable!() };
    ts.handle(op, &m).unwrap()
}

#[test]
fn ops_open_sync_close() {
    let (dir, ..) = fixture("ops");
    let mut ts = Trackers::default();
    let projects = dir.to_string_lossy().to_string();
    assert_eq!(op(&mut ts, "tracker.sync", json!({ "key": "k" })), Err("tracker"));
    assert_eq!(op(&mut ts, "tracker.open", json!({ "key": "k", "terminalId": "1:2" })), Err("projectsDir"));
    op(&mut ts, "tracker.open", json!({ "key": "k", "terminalId": "1:2", "projectsDir": projects })).unwrap();
    assert_eq!(op(&mut ts, "tracker.open", json!({ "key": "k", "terminalId": "1:2", "projectsDir": projects })), Err("key"));
    assert_eq!(op(&mut ts, "tracker.sync", json!({ "key": "k", "sessionId": 5 })), Err("args"));
    let sync = json!({ "key": "k", "sessionId": SID, "cwd": "/work/demo", "catchUp": true, "plan": { "all": true }, "nowMs": NOW, "maxBytes": 4096 });
    let mut pages = 0;
    loop {
        let r = op(&mut ts, "tracker.sync", sync.clone()).unwrap();
        pages += 1;
        if r.get("more").is_none() {
            assert!(r.contains_key("view"));
            break;
        }
        assert!(!r.contains_key("view"));
    }
    assert!(pages > 2);
    let r = op(&mut ts, "tracker.sync", sync).unwrap();
    assert!(!r.contains_key("view"));
    assert_eq!(op(&mut ts, "tracker.closeSpans", json!({ "key": "k", "at": "2026-10-07T11:00:00.000Z" })).unwrap()["spans"].as_array().unwrap().len(), 2);
    op(&mut ts, "tracker.close", json!({ "key": "k" })).unwrap();
    assert_eq!(op(&mut ts, "tracker.close", json!({ "key": "k" })), Err("tracker"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn new_args_drop_a_pending_run() {
    let (dir, ..) = fixture("restart");
    let mut t = Tracker::new("4242:1", dir.clone());
    let mut out = Out::default();
    assert!(matches!(t.sync(Some(args(true, 0.0)), 1000, &mut out), Step::More));
    assert!(matches!(t.sync(Some(args(true, 0.0)), 1000, &mut out), Step::More));
    let mut other = args(false, 1.0);
    other.session_id = Some("66666666-7777-8888-9999-000000000000".into());
    let mut out = Out::default();
    assert!(matches!(t.sync(Some(other), 1000, &mut out), Step::Done));
    assert!(!t.running());
    let view = t.take_view().unwrap();
    assert_eq!(view["full"], true);
    assert_eq!(view["subs"], json!([]));
    let (_, pages) = run(&mut t, args(true, 2.0), u64::MAX);
    assert_eq!(pages, 1);
    assert_eq!(t.take_view().unwrap()["subs"][0]["agentId"], "a1");
    let _ = fs::remove_dir_all(dir);
}
