use std::sync::Mutex;

use super::*;

fn raw(pid: u32, ppid: u32, name: &str, start: Option<&str>, ram: Option<u64>) -> RawProc {
    RawProc { pid, ppid, name: name.into(), start: start.map(str::to_owned), ram, cpu_ms: Some(100.0) }
}

struct Fake {
    rows: Vec<RawProc>,
    live: HashMap<u32, String>,
    answer: bool,
    killed: Mutex<Vec<u32>>,
    asked: Mutex<Vec<String>>,
    now: Mutex<f64>,
}

impl RamDeps for Fake {
    fn read(&self) -> Option<Vec<RawProc>> {
        Some(self.rows.clone())
    }
    fn self_pid(&self) -> u32 {
        100
    }
    fn plat(&self) -> Plat {
        Plat::Win
    }
    fn cpus(&self) -> usize {
        4
    }
    fn total_mem(&self) -> u64 {
        16
    }
    fn free_mem(&self) -> u64 {
        8
    }
    fn now(&self) -> f64 {
        *self.now.lock().unwrap()
    }
    fn live_starts(&self, pids: &[u32]) -> HashMap<u32, String> {
        self.live.iter().filter(|(k, _)| pids.contains(k)).map(|(k, v)| (*k, v.clone())).collect()
    }
    fn kill_tree(&self, pid: u32) -> bool {
        self.killed.lock().unwrap().push(pid);
        true
    }
    fn confirm(&self, lines: &[String], _n: usize, _first: &str) -> bool {
        self.asked.lock().unwrap().extend(lines.iter().cloned());
        self.answer
    }
}

fn fake(answer: bool) -> Fake {
    let rows = vec![
        raw(1, 0, "explorer.exe", Some("a"), Some(50)),
        raw(50, 1, "node.exe", Some("e2e"), Some(2048 * 1024 * 1024)),
        raw(100, 50, "orrery-shell.exe", Some("s"), Some(300)),
        raw(101, 100, "msedgewebview2.exe", Some("w"), None),
        raw(200, 1, "claude.exe", Some("c"), Some(400 * 1024 * 1024)),
        raw(300, 1, "svchost.exe", None, Some(10)),
    ];
    let live = rows.iter().filter_map(|r| r.start.clone().map(|s| (r.pid, s))).collect();
    Fake { rows, live, answer, killed: Mutex::new(Vec::new()), asked: Mutex::new(Vec::new()), now: Mutex::new(10_000.0) }
}

#[test]
fn tags_and_system_names() {
    assert_eq!(tag_of("Claude.exe"), Some("agent"));
    assert_eq!(tag_of("qemu-system-x86_64.exe"), Some("dev"));
    assert_eq!(tag_of("notepad.exe"), None);
    assert!(system_name("svchost.exe", Plat::Win) && system_name("kernel_task", Plat::Mac) && !system_name("svchost", Plat::Mac));
    assert!(same_start(Some(" a  b"), Some("a b")) && !same_start(Some(""), Some("")) && !same_start(None, Some("a")));
}

#[test]
fn targets_are_validated() {
    assert_eq!(parse_targets(&json!({ "targets": [{ "pid": 5, "start": "x" }, { "pid": 5, "start": "y" }] })), Some(vec![(5, "x".to_owned())]));
    assert_eq!(parse_targets(&json!({ "targets": [] })), None);
    assert_eq!(parse_targets(&json!({ "targets": [{ "pid": 0, "start": "x" }] })), None);
    assert_eq!(parse_targets(&json!({ "targets": [{ "pid": 5, "start": " " }] })), None);
    assert_eq!(parse_targets(&json!({ "targets": [{ "pid": 5.5, "start": "x" }] })), None);
}

#[test]
fn snapshot_marks_own_tree_parents_and_system() {
    let svc = Service::new(fake(true));
    let snap = svc.snapshot();
    assert_eq!(snap["ok"], json!(true));
    let lock = |pid: u32| snap["procs"].as_array().unwrap().iter().find(|p| p["pid"] == json!(pid)).unwrap()["lock"].clone();
    assert_eq!(lock(100), json!("self"));
    assert_eq!(lock(101), json!("self"));
    assert_eq!(lock(50), json!("parent"));
    assert_eq!(lock(1), json!("parent"));
    assert_eq!(lock(300), json!("system"));
    assert_eq!(lock(200), Value::Null);
    assert_eq!(snap["procs"][0]["pid"], json!(50));
    let again = svc.snapshot();
    assert_eq!(again["at"], snap["at"]);
}

#[test]
fn kill_refuses_locked_and_gone_and_asks_before_ending() {
    let svc = Service::new(fake(true));
    svc.snapshot();
    assert_eq!(svc.kill(&json!({ "targets": [{ "pid": 100, "start": "s" }] })), json!({ "ok": false, "killed": 0, "error": "locked", "lock": "self" }));
    assert_eq!(svc.kill(&json!({ "targets": [{ "pid": 999, "start": "1" }] }))["error"], json!("gone"));
    assert_eq!(svc.kill(&json!({ "targets": [{ "pid": 200, "start": "other" }] }))["error"], json!("gone"));
    assert_eq!(svc.kill(&json!({ "targets": [{ "pid": 200, "start": "c" }] })), json!({ "ok": true, "killed": 1 }));
    assert_eq!(*svc.deps.killed.lock().unwrap(), vec![200]);
    assert_eq!(svc.deps.asked.lock().unwrap()[0], "claude.exe, 400 MB, pid 200");
    let no = Service::new(fake(false));
    no.snapshot();
    assert_eq!(no.kill(&json!({ "targets": [{ "pid": 200, "start": "c" }] }))["error"], json!("cancelled"));
    assert!(no.deps.killed.lock().unwrap().is_empty());
}
