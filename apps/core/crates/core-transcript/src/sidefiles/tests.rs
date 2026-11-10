use super::SideFiles;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ID: &str = "12345678-1234-1234-1234-123456789abc";
const ID2: &str = "87654321-4321-4321-4321-cba987654321";

fn temp() -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ccm-sidefiles-{}-{n}-{seq}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn call(op: &str, values: Value) -> Result<Map<String, Value>, &'static str> {
    SideFiles.handle(op, values.as_object().unwrap()).unwrap()
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64
}

fn age(path: &PathBuf, ms: u64) {
    let f = fs::File::options().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_millis(ms))
        .unwrap();
}

#[test]
fn hooks_are_validated_like_the_node_reader() {
    let dir = temp();
    let body = json!({"sessionId":ID,"at":"2026-11-10T10:00:00Z","deniedCount":3.0,"deniedTools":["Read","Write","Bash","Edit","Grep","Glob","bad tool","é"],"compactingSince":"2026-11-10T09:00:00Z","compactTrigger":"manual"});
    fs::write(dir.join(format!("{ID}.json")), body.to_string()).unwrap();
    fs::write(dir.join(format!("{ID2}.JSON")), json!({"sessionId":ID2,"at":"2026-11-10T10:00:00Z","deniedCount":-1}).to_string()).unwrap();
    fs::write(dir.join("wrong.json"), "{}").unwrap();
    let r = call(
        "sidefiles.hooks",
        json!({"dir": dir, "now": now_ms()}),
    )
    .unwrap();
    let records = r["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["sessionId"], json!(ID));
    assert_eq!(records[0]["deniedCount"], json!(3));
    assert_eq!(
        records[0]["deniedTools"],
        json!(["Write", "Bash", "Edit", "Grep", "Glob"])
    );
    assert_eq!(records[0]["compactTrigger"], json!("manual"));
    assert_eq!(records[1]["deniedCount"], json!(0));
    assert!(records[1].get("compactingSince").is_none());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn statusline_clamps_and_drops_out_of_range_numbers() {
    let dir = temp();
    fs::write(dir.join(format!("{ID}.json")), json!({"sessionId":"x","at":"2026-11-10T10:00:00Z","costUsd":1.5,"contextWindow":999,"contextPct":101,"bufferPct":50,"fiveHour":{"usedPct":120,"resetsAt":"2026-11-10T12:00:00Z"},"sevenDay":{"usedPct":"x"}}).to_string()).unwrap();
    let r = call(
        "sidefiles.statusline",
        json!({"dir": dir, "now": now_ms()}),
    )
    .unwrap();
    let rec = &r["records"][0];
    assert_eq!(rec["costUsd"], json!(1.5));
    assert!(rec.get("contextWindow").is_none());
    assert!(rec.get("contextPct").is_none());
    assert_eq!(rec["bufferPct"], json!(50));
    assert_eq!(rec["fiveHour"]["usedPct"], json!(100));
    assert!(rec.get("sevenDay").is_none());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn skips_oversized_broken_and_vanished_files_and_sweeps_only_old_ones() {
    let dir = temp();
    let other = temp();
    let ok = dir.join(format!("{ID}.json"));
    let old = dir.join(format!("{ID2}.json"));
    let big = dir.join("aaaaaaaa-1234-1234-1234-123456789abc.json");
    let torn = dir.join("bbbbbbbb-1234-1234-1234-123456789abc.json");
    let foreign = other.join(format!("{ID}.json"));
    let valid = json!({"sessionId":ID,"at":"2026-11-10T10:00:00Z"}).to_string();
    fs::write(&ok, &valid).unwrap();
    fs::write(&old, valid.replace(ID, ID2)).unwrap();
    fs::write(&foreign, &valid).unwrap();
    fs::write(&big, format!("{valid}{}", " ".repeat(5000))).unwrap();
    fs::write(&torn, "{\"sessionId\":").unwrap();
    fs::create_dir(dir.join("cccccccc-1234-1234-1234-123456789abc.json")).unwrap();
    age(&old, 25 * 3_600_000);
    age(&foreign, 25 * 3_600_000);
    let r = call("sidefiles.hooks", json!({"dir": dir, "now": now_ms()})).unwrap();
    assert_eq!(r["records"].as_array().unwrap().len(), 1);
    assert!(ok.exists());
    assert!(!old.exists());
    assert!(foreign.exists());
    let s = call(
        "sidefiles.statusline",
        json!({"dir": dir, "now": now_ms()}),
    )
    .unwrap();
    assert!(s["records"].as_array().is_some());
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(other).unwrap();
}

#[test]
fn statusline_keeps_files_for_seven_days() {
    let dir = temp();
    let f = dir.join(format!("{ID}.json"));
    fs::write(&f, json!({"sessionId":ID,"at":"2026-11-10T10:00:00Z"}).to_string()).unwrap();
    age(&f, 3 * 24 * 3_600_000);
    let r = call("sidefiles.statusline", json!({"dir": dir, "now": now_ms()})).unwrap();
    assert_eq!(r["records"].as_array().unwrap().len(), 1);
    age(&f, 8 * 24 * 3_600_000);
    let r = call("sidefiles.statusline", json!({"dir": dir, "now": now_ms()})).unwrap();
    assert!(r["records"].as_array().unwrap().is_empty());
    assert!(!f.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_relative_dirs_and_missing_params() {
    assert_eq!(call("sidefiles.hooks", json!({"dir":"rel","now":1})), Err("dir"));
    assert_eq!(call("sidefiles.hooks", json!({"now":1})), Err("dir"));
    assert_eq!(call("sidefiles.hooks", json!({"dir":temp()})), Err("now"));
    let missing = temp().join("nope");
    let r = call("sidefiles.hooks", json!({"dir": missing, "now": 1})).unwrap();
    assert!(r["records"].as_array().unwrap().is_empty());
    assert!(SideFiles.handle("sidefiles.x", &Map::new()).is_none());
}

#[test]
fn outbox_pages_whole_lines_and_resumes_from_the_returned_state() {
    let dir = temp();
    let file = dir.join("outbox.jsonl");
    let a = json!({"event":"turn_start","thread_id":"th-1","timestamp":1730000000}).to_string();
    let b = json!({"event":"subagent_spawn","thread_id":"th-1","timestamp":"2026-11-10T10:00:00Z","kind":"turn.started","payload":{"workspace":"C:\\w","subagent":{"id":"s1","name":"x y","status":"Done"}}}).to_string();
    fs::write(&file, format!("{a}\n{{\"event\":")).unwrap();
    let r = call("sidefiles.outbox", json!({"file": file})).unwrap();
    assert_eq!(r["records"].as_array().unwrap().len(), 1);
    assert_eq!(r["records"][0]["at"], json!("2024-10-27T03:33:20.000Z"));
    let offset = r["offset"].as_u64().unwrap();
    assert_eq!(offset, a.len() as u64 + 1);
    assert!(r.get("more").is_none());
    fs::write(&file, format!("{a}\n{b}\n")).unwrap();
    let r = call(
        "sidefiles.outbox",
        json!({"file": file, "started": true, "offset": offset, "skipping": false}),
    )
    .unwrap();
    let rec = &r["records"][0];
    assert_eq!(rec["workspace"], json!("C:\\w"));
    assert_eq!(rec["subagent"], json!({"id":"s1","outcome":"completed"}));
    assert_eq!(rec["at"], json!("2026-11-10T10:00:00.000Z"));
    assert_eq!(r["offset"].as_u64().unwrap(), (a.len() + b.len() + 2) as u64);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn outbox_starts_near_the_end_drops_long_lines_and_restarts_after_truncation() {
    let dir = temp();
    let file = dir.join("outbox.jsonl");
    let line = |n: u32| json!({"event":"turn_end","thread_id":format!("th-{n}")}).to_string();
    let long = format!("{{\"event\":\"turn_end\",\"thread_id\":\"{}\"}}", "a".repeat(600));
    fs::write(&file, format!("{}\n{long}\n{}\n{}\n", line(1), line(2), line(3))).unwrap();
    let r = call("sidefiles.outbox", json!({"file": file, "maxLine": 400})).unwrap();
    let ids: Vec<_> = r["records"].as_array().unwrap().iter().map(|v| v["threadId"].as_str().unwrap().to_owned()).collect();
    assert_eq!(ids, ["th-1", "th-2", "th-3"]);
    let r = call("sidefiles.outbox", json!({"file": file, "fromEnd": line(3).len() + 5})).unwrap();
    let ids: Vec<_> = r["records"].as_array().unwrap().iter().map(|v| v["threadId"].as_str().unwrap().to_owned()).collect();
    assert_eq!(ids, ["th-3"]);
    fs::write(&file, format!("{}\n", line(9))).unwrap();
    let r = call("sidefiles.outbox", json!({"file": file, "started": true, "offset": 99999})).unwrap();
    assert_eq!(r["records"][0]["threadId"], json!("th-9"));
    let gone = dir.join("none.jsonl");
    assert_eq!(call("sidefiles.outbox", json!({"file": gone})).unwrap()["missing"], json!(true));
    fs::remove_dir_all(dir).unwrap();
}
