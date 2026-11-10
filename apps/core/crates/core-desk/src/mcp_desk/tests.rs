use super::deps::Deps;
use super::desk::{run, Opts};
use super::scope::ProcRow;
use super::shot::screenshot;
use crate::resources::Target;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::PathBuf;

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Default)]
struct Fake {
    report: Option<Value>,
    rows: Vec<ProcRow>,
    render: Option<Value>,
    shots: RefCell<Vec<String>>,
    kill: RefCell<Vec<(Vec<i64>, String, i64)>>,
    kill_result: Option<Value>,
    receive: i64,
    tg: RefCell<Value>,
    tg_report: Option<Value>,
    snapshot: Option<Value>,
    runs: Vec<Value>,
    cap: Option<Value>,
    active: bool,
}

impl Deps for Fake {
    fn render(&self, _f: &str, _d: &Value, _n: bool, _s: &str) -> Value {
        self.render.clone().unwrap_or(json!({"ok": false, "code": "unavailable"}))
    }
    fn write_shot(&self, _png: &[u8], name: &str) -> Option<String> {
        self.shots.borrow_mut().push(name.to_string());
        None
    }
    fn report(&self) -> Option<Value> {
        self.report.clone()
    }
    fn proc_rows(&self) -> Option<Vec<ProcRow>> {
        Some(self.rows.clone())
    }
    fn kill(&self, t: &[Target], pane: &str, port: i64) -> Value {
        self.kill.borrow_mut().push((t.iter().map(|x| x.pid).collect(), pane.to_string(), port));
        self.kill_result.clone().unwrap_or(json!({"ok": true, "killed": t.len()}))
    }
    fn confirm_receive(&self, _p: &str, _o: &str) -> i64 {
        self.receive
    }
    fn tg_available(&self) -> bool {
        !self.tg.borrow().is_null()
    }
    fn tg_state(&self) -> Value {
        self.tg.borrow().clone()
    }
    fn tg_set(&self, patch: Value) -> Value {
        let mut s = self.tg.borrow_mut();
        for (k, v) in patch.as_object().unwrap() {
            s[k.as_str()] = v.clone();
        }
        json!({"ok": true})
    }
    fn tg_route_for(&self, cwd: &str) -> Value {
        if cwd.ends_with("routed") { json!({"label": "x"}) } else { Value::Null }
    }
    fn tg_report(&self, _b: Value) -> Value {
        self.tg_report.clone().unwrap_or(json!({"ok": false, "error": "network"}))
    }
    fn snapshot(&self) -> Option<Value> {
        self.snapshot.clone()
    }
    fn screencap(&self, _s: &str) -> Value {
        self.cap.clone().unwrap_or(json!({"ok": false, "error": "x"}))
    }
    fn runs(&self) -> Vec<Value> {
        self.runs.clone()
    }
    fn capture_active(&self) -> bool {
        self.active
    }
}

fn opts() -> Opts {
    Opts { win: false, home: "/Users/a".into(), self_pids: vec![], fallback_dir: std::env::temp_dir().join("ccm-mcpdesk-fb"), tmp_root: std::env::temp_dir().join("ccm-mcpdesk-tmp"), deadline_ms: 5000 }
}

fn call(tool: &str, extra: Value) -> Value {
    let mut c = json!({"id": "1", "from": "pane1", "cwd": "/Users/a/app", "pid": 10, "name": "app\u{1}x", "tool": tool});
    for (k, v) in extra.as_object().unwrap() {
        c[k.as_str()] = v.clone();
    }
    c
}

fn report() -> Value {
    json!({"at": "t", "sessions": [], "servers": [{"pid": 12, "start": "1", "name": "vite", "kind": "dev-server", "ports": [{"port": 5173}], "cwd": "/Users/a/app", "cmd": "node vite", "idleSinceMs": null, "orphan": false}]})
}

fn fake() -> Fake {
    Fake { report: Some(report()), rows: vec![ProcRow { pid: 10, ppid: 1, start_ms: None }], ..Default::default() }
}

#[test]
fn list_servers_needs_a_report() {
    let none = Fake::default();
    assert_eq!(run(&none, &opts(), &call("list_servers", json!({}))), json!({"ok": false, "error": "no-report"}));
    let r = run(&fake(), &opts(), &call("list_servers", json!({})));
    assert_eq!(r["servers"][0]["mine"], true);
    assert_eq!(r["at"], "t");
}

#[test]
fn free_port_confirms_once_and_maps_errors() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let f = fake();
    let r = run(&f, &opts(), &call("free_port", json!({"port": 5173})));
    assert_eq!(r, json!({"ok": true, "tool": "free_port", "port": 5173, "stopped": [{"pid": 12, "name": "vite"}]}));
    assert_eq!(f.kill.borrow()[0], (vec![12], "app x".to_string(), 5173));
    let cancelled = Fake { kill_result: Some(json!({"ok": false, "error": "cancelled"})), ..fake() };
    assert_eq!(run(&cancelled, &opts(), &call("free_port", json!({"port": 5173}))), json!({"ok": false, "error": "cancelled"}));
    let invalid = Fake { kill_result: Some(json!({"ok": false, "error": "invalid"})), ..fake() };
    assert_eq!(run(&invalid, &opts(), &call("free_port", json!({"port": 5173}))), json!({"ok": false, "error": "failed"}));
    assert_eq!(run(&fake(), &opts(), &call("free_port", json!({"port": 9}))), json!({"ok": false, "error": "not-found"}));
}

#[test]
fn preview_layout_refuses_outside_and_maps_render_codes() {
    let f = fake();
    assert_eq!(run(&f, &opts(), &call("preview_layout", json!({"path": "/Users/a/other/x.xml", "device": "phone", "night": false}))), json!({"ok": false, "error": "outside"}));
    let busy = Fake { render: Some(json!({"ok": false, "code": "starting", "retryInSec": 8})), ..fake() };
    assert_eq!(run(&busy, &opts(), &call("preview_layout", json!({"path": "/Users/a/app/x.xml", "device": "phone", "night": false}))), json!({"ok": false, "error": "starting", "retryInSec": 8}));
    let odd = Fake { render: Some(json!({"ok": false, "code": "weird", "detail": "d"})), ..fake() };
    assert_eq!(run(&odd, &opts(), &call("preview_layout", json!({"path": "/Users/a/app/x.xml", "device": "tablet", "night": true}))), json!({"ok": false, "error": "failed"}));
}

#[test]
fn telegram_flows() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let off = Fake::default();
    assert_eq!(run(&off, &opts(), &call("telegram_notify", json!({"state": "status"}))), json!({"ok": false, "error": "unavailable"}));
    let f = Fake { tg: RefCell::new(json!({"configured": true, "enabled": false, "receiving": false, "owner": {"name": "Jin"}, "routes": [{"label": "A"}]})), receive: 0, ..fake() };
    let r = run(&f, &opts(), &call("telegram_notify", json!({"state": "on"})));
    assert_eq!(r, json!({"ok": true, "tool": "telegram_notify", "enabled": true, "configured": true, "projects": ["A"], "routed": false}));
    let r = run(&f, &opts(), &call("telegram_receive", json!({"state": "on"})));
    assert_eq!(r, json!({"ok": true, "tool": "telegram_receive", "receiving": true, "paired": true, "owner": "Jin"}));
    let no = Fake { tg: RefCell::new(json!({"configured": true, "owner": {"name": "J"}, "receiving": false})), receive: 1, ..fake() };
    assert_eq!(run(&no, &opts(), &call("telegram_receive", json!({"state": "on"}))), json!({"ok": false, "error": "cancelled"}));
    let unpaired = Fake { tg: RefCell::new(json!({"configured": true, "owner": null})), ..fake() };
    assert_eq!(run(&unpaired, &opts(), &call("telegram_receive", json!({"state": "on"}))), json!({"ok": false, "error": "not-paired"}));
    let both = run(&f, &opts(), &call("telegram_send", json!({"text": "t", "file": "/a", "buttons": [{"label": "x"}]})));
    assert_eq!(both, json!({"ok": false, "error": "buttons-file"}));
    let sent = Fake { tg_report: Some(json!({"ok": true, "topic": "T", "buttons": 2})), ..Fake { tg: RefCell::new(json!({"configured": true})), ..fake() } };
    assert_eq!(run(&sent, &opts(), &call("telegram_send", json!({"text": "t"}))), json!({"ok": true, "tool": "telegram_send", "topic": "T", "buttons": 2}));
    let bad = Fake { tg_report: Some(json!({"ok": false, "error": "rate"})), ..Fake { tg: RefCell::new(json!({"configured": true})), ..fake() } };
    assert_eq!(run(&bad, &opts(), &call("telegram_send", json!({"text": "t"}))), json!({"ok": false, "error": "rate", "detail": "rate"}));
}

fn png_data() -> String {
    "data:image/png;base64,iVBORw0KGgo=".to_string()
}

fn snap() -> Value {
    json!({"adb": "/adb", "devices": [{"id": "A1", "name": "Pixel", "online": true, "state": "ready"}, {"id": "B2", "name": "Tab", "online": true, "state": "ready"}, {"id": "C3", "name": "Off", "online": false, "state": "ready"}]})
}

#[test]
fn screenshot_picks_the_device_and_reports_refusals() {
    let tmp = std::env::temp_dir().join(format!("ccm-mcpshot-{}", std::process::id()));
    let call = json!({"cwd": "/Users/a/app"});
    let go = |f: &Fake, c: &Value| screenshot(f, &PathBuf::from(&tmp), 5000, c);
    assert_eq!(go(&Fake::default(), &call), json!({"ok": false, "error": "unavailable"}));
    let noadb = Fake { snapshot: Some(json!({"adb": null, "devices": []})), ..Default::default() };
    assert_eq!(go(&noadb, &call)["refused"]["code"], "adb-missing");
    let two = Fake { snapshot: Some(snap()), ..Default::default() };
    let r = go(&two, &call);
    assert_eq!(r["refused"]["code"], "several");
    assert_eq!(r["devices"], json!([{"serial": "A1", "name": "Pixel"}, {"serial": "B2", "name": "Tab"}]));
    assert_eq!(go(&two, &json!({"cwd": "/x", "serial": "C3"}))["refused"]["code"], "not-connected");
    let run = Fake { snapshot: Some(snap()), runs: vec![json!({"state": "running", "deviceId": "B2", "cwd": "/Users/a/app", "startedAt": 5})], cap: Some(json!({"ok": true, "png": png_data(), "width": 1, "height": 2})), ..Default::default() };
    let r = go(&run, &call);
    assert_eq!(r["shot"]["serial"], "B2");
    assert_eq!(r["shot"]["saved"], "temp");
    assert!(std::fs::read(r["shot"]["path"].as_str().unwrap()).unwrap().starts_with(&[0x89, 0x50]));
    let secure = Fake { snapshot: Some(snap()), cap: Some(json!({"ok": false, "error": "secure"})), ..Default::default() };
    assert_eq!(go(&secure, &json!({"cwd": "/x", "serial": "A1"}))["refused"]["code"], "secure");
    let _ = std::fs::remove_dir_all(&tmp);
}
