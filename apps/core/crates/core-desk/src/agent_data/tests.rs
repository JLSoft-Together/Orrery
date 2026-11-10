use super::procs::AgentProc;
use super::rules::{parse_clean_request, Agent};
use super::service::{AgentDataService, AgentHost, Roots};
use crate::util::{path_str, uuid};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

struct Fake {
    roots: Roots,
    procs: Mutex<Vec<AgentProc>>,
    answer: Mutex<i64>,
    pick: PathBuf,
    specs: Mutex<Vec<Value>>,
    collector: Mutex<Vec<&'static str>>,
}

impl AgentHost for Fake {
    fn roots(&self) -> Roots {
        self.roots.clone()
    }
    fn procs(&self, _agent: Agent) -> Vec<AgentProc> {
        self.procs.lock().unwrap().clone()
    }
    fn confirm(&self, spec: Value) -> Option<i64> {
        self.specs.lock().unwrap().push(spec);
        Some(*self.answer.lock().unwrap())
    }
    fn pick_folder(&self, _purpose: &str) -> Option<String> {
        Some(path_str(&self.pick))
    }
    fn stop_collector(&self) {
        self.collector.lock().unwrap().push("stop");
    }
    fn start_collector(&self) {
        self.collector.lock().unwrap().push("start");
    }
    fn emit(&self, _progress: Value) {}
    fn is_local(&self, _dir: &str) -> bool {
        true
    }
    fn free_bytes(&self, _dir: &str) -> Option<f64> {
        None
    }
    fn alive(&self, _pid: f64) -> bool {
        false
    }
    fn now(&self) -> f64 {
        crate::util::now_ms()
    }
}

fn old(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
    let t = SystemTime::now() - Duration::from_secs(200 * 86_400);
    super::tree::set_times(p, Some(t), Some(t));
}

fn setup() -> (PathBuf, Fake, AgentDataService) {
    let dir = std::env::temp_dir().join(format!("core-desk-ad-svc-{}", uuid()));
    let claude = dir.join("claude");
    let codex = dir.join("codex");
    old(&claude.join("debug/a/x.txt"), "xxxx");
    old(&claude.join("shell-snapshots/s.sh"), "s");
    old(&claude.join("settings.json"), "{}");
    old(&codex.join("sessions/r.jsonl"), "{}\n");
    fs::create_dir_all(dir.join("target")).unwrap();
    let fake = Fake {
        roots: Roots { claude: path_str(&claude), codex: path_str(&codex), claude_app: None },
        procs: Mutex::new(Vec::new()),
        answer: Mutex::new(0),
        pick: dir.join("target"),
        specs: Mutex::new(Vec::new()),
        collector: Mutex::new(Vec::new()),
    };
    let svc = AgentDataService::new(dir.join("data").join("agent-data-move.json"));
    (dir, fake, svc)
}

#[test]
fn clean_checks_running_agents_and_keeps_top_folders() {
    let (dir, fake, svc) = setup();
    let req = parse_clean_request(&json!({"agent": "claude", "groups": ["debugLogs", "shellSnapshots"], "olderDays": 90})).unwrap();
    *fake.procs.lock().unwrap() = vec![AgentProc { pid: 7.0, name: "claude".into() }];
    assert_eq!(svc.clean(&req, &fake).unwrap(), json!({"ok": false, "error": "running", "procs": [{"pid": 7.0, "name": "claude"}]}));
    fake.procs.lock().unwrap().clear();
    *fake.answer.lock().unwrap() = 1;
    assert_eq!(svc.clean(&req, &fake).unwrap()["error"], json!("cancelled"));
    *fake.answer.lock().unwrap() = 0;
    assert_eq!(svc.clean(&req, &fake).unwrap(), json!({"ok": true, "files": 2.0, "bytes": 5.0, "skipped": 0.0}));
    let claude = Path::new(&fake.roots.claude);
    assert!(claude.join("debug").is_dir() && !claude.join("debug/a").exists() && claude.join("settings.json").exists());
    assert_eq!(fake.specs.lock().unwrap()[1]["groups"], json!(["debugLogs", "shellSnapshots"]));
    assert_eq!(svc.clean(&req, &fake).unwrap()["error"], json!("nothing"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn move_then_undo_round_trips_and_writes_the_store() {
    let (dir, fake, svc) = setup();
    let codex = PathBuf::from(&fake.roots.codex);
    let out = svc.move_out(Agent::Codex, &fake).unwrap();
    assert_eq!(out["ok"], json!(true), "{out}");
    assert!(super::tree::link_target(&codex).is_some());
    let state = svc.state(&fake);
    assert_eq!(state["places"][1]["move"]["phase"], json!("linked"));
    let text = fs::read_to_string(dir.join("data").join("agent-data-move.json")).unwrap();
    assert!(text.contains("\"phase\": \"linked\""));
    assert_eq!(svc.move_out(Agent::Codex, &fake).unwrap()["error"], json!("already-moved"));
    assert_eq!(svc.discard(Agent::Codex, &fake).unwrap()["error"], json!("not-moved"));
    let undone = svc.undo(Agent::Codex, &fake).unwrap();
    assert_eq!(undone["ok"], json!(true), "{undone}");
    assert!(codex.join("sessions/r.jsonl").is_file() && super::tree::link_target(&codex).is_none());
    assert!(dir.join("target").join("codex").join("sessions/r.jsonl").is_file());
    assert_eq!(*fake.collector.lock().unwrap(), vec!["stop", "start", "stop", "start"]);
    assert!(svc.state(&fake)["places"][1]["move"].is_null());
    assert_eq!(svc.undo(Agent::Codex, &fake).unwrap()["error"], json!("not-moved"));
    assert_eq!(svc.delete_original(Agent::Codex, &fake).unwrap()["error"], json!("not-moved"));
    assert!(!svc.cancel());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn move_refuses_a_target_inside_the_source() {
    let (dir, mut fake, svc) = setup();
    fake.pick = PathBuf::from(&fake.roots.claude).join("debug");
    assert_eq!(svc.move_out(Agent::Claude, &fake).unwrap()["error"], json!("target-inside"));
    fake.pick = dir.join("target");
    fs::create_dir_all(dir.join("target").join("claude")).unwrap();
    assert_eq!(svc.move_out(Agent::Claude, &fake).unwrap()["error"], json!("target-exists"));
    assert!(fake.collector.lock().unwrap().is_empty());
    fs::remove_dir_all(dir).unwrap();
}
