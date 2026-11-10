use crate::tail::{Item, JsonlTailer, TailOptions, Verdict};
use crate::tracker::Tracker;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ccm-held-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn append(p: &Path, s: &str) {
    fs::OpenOptions::new().create(true).append(true).open(p).unwrap().write_all(s.as_bytes()).unwrap();
}

fn take(t: &mut JsonlTailer) -> Vec<Value> {
    let mut got = Vec::new();
    t.read_each(&mut |_, _, _| Verdict::Parse, &mut |i| {
        if let Item::Record(v) = i {
            got.push(v);
        }
    });
    got
}

fn pair(file: &Path) -> (JsonlTailer, JsonlTailer) {
    let held = {
        let mut t = JsonlTailer::new(file.to_path_buf(), 4, TailOptions::default());
        t.keep = true;
        t
    };
    (held, JsonlTailer::new(file.to_path_buf(), 4, TailOptions::default()))
}

fn same(held: &mut JsonlTailer, fresh: &mut JsonlTailer) -> Vec<Value> {
    let a = take(held);
    assert_eq!(a, take(fresh));
    assert_eq!(held.offset(), fresh.offset());
    a
}

#[test]
fn a_held_handle_reads_the_same_lines_as_reopening() {
    let dir = tmp("same");
    let file = dir.join("a.jsonl");
    append(&file, "{\"uuid\":1}\n{\"uuid\":");
    let (mut held, mut fresh) = pair(&file);
    assert_eq!(same(&mut held, &mut fresh).len(), 1);
    assert!(held.holds());
    assert!(!fresh.holds());
    append(&file, "2}\n{\"uuid\":3}\n");
    assert_eq!(same(&mut held, &mut fresh).len(), 2);
    assert!(same(&mut held, &mut fresh).is_empty());
    fs::write(&file, "{\"subtype\":1}\n").unwrap();
    assert_eq!(same(&mut held, &mut fresh), vec![serde_json::json!({ "subtype": 1 })]);
    held.release();
    assert!(!held.holds());
    append(&file, "{\"subtype\":2}\n");
    assert_eq!(same(&mut held, &mut fresh).len(), 1);
    assert!(held.holds());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_replaced_or_deleted_file_is_reopened_by_path() {
    let dir = tmp("replace");
    let file = dir.join("a.jsonl");
    append(&file, "{\"uuid\":1}\n");
    let (mut held, mut fresh) = pair(&file);
    same(&mut held, &mut fresh);
    fs::remove_file(&file).unwrap();
    assert!(same(&mut held, &mut fresh).is_empty());
    assert!(!held.holds());
    append(&file, "{\"uuid\":1}\n{\"subtype\":\"new file\"}\n");
    assert_eq!(same(&mut held, &mut fresh), vec![serde_json::json!({ "subtype": "new file" })]);
    assert!(held.holds());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn keep_off_never_holds_and_a_released_tracker_stops_holding() {
    let dir = tmp("off");
    let file = dir.join("a.jsonl");
    append(&file, "{\"uuid\":1}\n");
    let mut t = JsonlTailer::new(file, 4, TailOptions::default());
    take(&mut t);
    assert!(!t.holds());
    let mut tr = Tracker::new("1:1", dir.clone());
    tr.release();
    assert!(!tr.hold);
    assert_eq!(tr.held_files(), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_tracker_holds_live_files_and_lets_go_of_resolved_subagents() {
    let sid = "11111111-2222-3333-4444-555555555555";
    let dir = tmp("tracker");
    let subs = dir.join("-work-demo").join(sid).join("subagents");
    fs::create_dir_all(&subs).unwrap();
    let main = dir.join("-work-demo").join(format!("{sid}.jsonl"));
    append(&main, "{\"type\":\"assistant\",\"timestamp\":\"2026-10-07T10:00:00.000Z\",\"message\":{\"id\":\"m0\",\"stop_reason\":\"tool_use\",\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_a1\",\"name\":\"Agent\",\"input\":{}}]}}\n");
    fs::write(subs.join("agent-a1.meta.json"), r#"{"agentType":"Explore","toolUseId":"toolu_a1"}"#).unwrap();
    append(&subs.join("agent-a1.jsonl"), "{\"type\":\"assistant\",\"timestamp\":\"2026-10-07T10:00:01.000Z\",\"message\":{\"id\":\"s1\",\"stop_reason\":\"tool_use\",\"content\":[{\"type\":\"tool_use\",\"id\":\"g1\",\"name\":\"Grep\",\"input\":{}}]}}\n");
    let mut t = Tracker::new("1:1", dir.clone());
    t.hold = true;
    let sync = |t: &mut Tracker| {
        let args = crate::tracker::SyncArgs { session_id: Some(sid.into()), cwd: Some("/work/demo".into()), epoch: 0.0, catch_up: false, plan: crate::tracker::Plan { all: true, ..Default::default() }, now: 1_791_400_000_000 };
        let mut out = crate::tracker::Out::default();
        let mut next = Some(args);
        while let crate::tracker::Step::More = t.sync(next.take(), u64::MAX, &mut out) {}
    };
    sync(&mut t);
    assert_eq!(t.held_files(), 2);
    append(&main, "{\"type\":\"user\",\"timestamp\":\"2026-10-07T10:00:05.000Z\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"toolu_a1\",\"is_error\":false}]}}\n");
    sync(&mut t);
    assert_eq!(t.held_files(), 1);
    sync(&mut t);
    assert_eq!(t.held_files(), 1);
    t.release();
    assert_eq!(t.held_files(), 0);
    drop(t);
    let _ = fs::remove_dir_all(&dir);
}
