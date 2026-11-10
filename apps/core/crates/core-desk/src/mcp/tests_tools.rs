use super::host::{Args, ToolHost};
use super::parse_run::parse_mcp_run_result;
use super::tools::call_tool;
use serde_json::{json, Value};
use std::sync::Mutex;

pub type Responder = Box<dyn Fn(&Value) -> Value + Send + Sync>;

pub struct Fake {
    pub root: String,
    pub allow: bool,
    pub page: Responder,
    pub desk: Option<Responder>,
    pub clock: Mutex<f64>,
    pub calls: Mutex<Vec<Value>>,
    pub takes: Mutex<Vec<(String, usize, u64)>>,
}

impl Fake {
    pub fn new(page: Responder) -> Fake {
        Fake { root: std::env::temp_dir().to_string_lossy().to_string(), allow: true, page, desk: None, clock: Mutex::new(1_000.0), calls: Mutex::new(Vec::new()), takes: Mutex::new(Vec::new()) }
    }
    pub fn with_desk(desk: Responder) -> Fake {
        let mut f = Fake::new(Box::new(|_| json!({ "ok": false, "error": "failed" })));
        f.desk = Some(desk);
        f
    }
}

impl ToolHost for Fake {
    fn root(&self) -> String {
        self.root.clone()
    }
    fn preset(&self) -> Option<String> {
        None
    }
    fn take(&self, key: &str, max: usize, window_ms: u64) -> bool {
        self.takes.lock().unwrap().push((key.to_string(), max, window_ms));
        self.allow
    }
    fn open_session(&self, _args: &Args) -> Value {
        Value::Null
    }
    fn notify(&self, _text: &str, _level: &str) -> Value {
        Value::Null
    }
    fn checkout_roots(&self) -> Vec<String> {
        Vec::new()
    }
    fn page(&self, call: Value) -> Value {
        let out = (self.page)(&call);
        self.calls.lock().unwrap().push(call);
        out
    }
    fn desk(&self, call: Value) -> Option<Value> {
        let d = self.desk.as_ref()?;
        let out = d(&call);
        self.calls.lock().unwrap().push(call);
        Some(out)
    }
    fn now_ms(&self) -> f64 {
        *self.clock.lock().unwrap()
    }
    fn sleep_ms(&self, ms: u64) {
        *self.clock.lock().unwrap() += ms as f64;
    }
}

pub fn run_page(raw: Value) -> Responder {
    Box::new(move |c| {
        let tool = c["tool"].as_str().unwrap_or("");
        let mut raw = raw.clone();
        if raw.get("error").is_some() {
            raw["ok"] = json!(false);
            return raw;
        }
        raw["ok"] = json!(true);
        parse_mcp_run_result(raw.as_object().unwrap(), tool).unwrap_or_else(|| json!({ "ok": false, "error": "failed" }))
    })
}

pub fn call(host: &Fake, name: &str, args: Value) -> Value {
    call_tool(name, args.as_object().unwrap(), host)
}

pub fn text(r: &Value) -> String {
    r["content"][0]["text"].as_str().unwrap().to_string()
}

pub fn is_error(r: &Value) -> bool {
    r.get("isError") == Some(&json!(true))
}

fn session_raw(state: &str) -> Value {
    json!({
        "id": "0b6f8f4e-1111-4222-8333-944445555666", "configId": "auto-android-app", "configName": "app", "type": "android",
        "deviceId": "emulator-5554", "deviceName": "Pixel 7", "rawState": state, "startedAt": 1_700_000_000_000i64,
        "endedAt": null, "appId": "com.example", "lastError": null
    })
}

const LINE: &str = "\"app\" [auto-android-app, android] on \"Pixel 7\" (emulator-5554): running, started 2023-11-14T22:13:20.000Z";

#[test]
fn run_app_validates_before_asking() {
    let host = Fake::new(run_page(json!({})));
    let bad = [
        json!({}),
        json!({ "action": "deploy" }),
        json!({ "action": "start", "config": "-rf" }),
        json!({ "action": "start", "config": "a b" }),
        json!({ "action": "start", "device": "../x" }),
        json!({ "action": "start", "full": true }),
        json!({ "action": "reload", "full": "yes" }),
    ];
    for args in bad {
        assert!(is_error(&call(&host, "run_app", args)));
    }
    assert!(host.calls.lock().unwrap().is_empty());
    let r = call(&host, "run_app", json!({ "action": "x" }));
    assert_eq!(text(&r), "action must be one of start, reload, restart, stop.");
    assert_eq!(host.takes.lock().unwrap()[0], ("run".to_string(), 6, 60_000));
}

#[test]
fn run_app_rate_comes_first() {
    let mut host = Fake::new(run_page(json!({})));
    host.allow = false;
    let r = call(&host, "run_app", json!({ "action": "bogus" }));
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "rate" }));
    assert_eq!(text(&r), "At most 6 run_app calls a minute per session; wait a little.");
}

#[test]
fn run_app_reports_and_refuses() {
    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "start", "session": session_raw("running") })));
    let r = call(&host, "run_app", json!({ "action": "start", "config": "auto-android-app", "device": "emulator-5554" }));
    assert_eq!(text(&r), format!("Started: {LINE}. Call run_status / run_log to follow it."));
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "run_app", "action": "start", "config": "auto-android-app", "device": "emulator-5554" }));
    assert_eq!(r["structuredContent"]["session"]["state"], "running");
    assert!(!is_error(&r));

    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "reload", "session": session_raw("running") })));
    let r = call(&host, "run_app", json!({ "action": "reload", "full": true }));
    assert!(text(&r).starts_with("Hot restarted: "));
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "run_app", "action": "reload", "full": true }));

    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "stop", "session": session_raw("stopped") })));
    let r = call(&host, "run_app", json!({ "action": "stop" }));
    assert!(text(&r).ends_with("stopped, started 2023-11-14T22:13:20.000Z."));

    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "start", "refused": { "code": "running" }, "session": session_raw("running") })));
    let r = call(&host, "run_app", json!({ "action": "start" }));
    assert!(is_error(&r));
    assert_eq!(text(&r), format!("That configuration is already running on that device. Use action reload (hot reload) or restart.\nCurrent run: {LINE}"));
    assert_eq!(r["structuredContent"]["refused"], json!({ "code": "running" }));

    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "stop", "refused": { "code": "no-session" } })));
    let r = call(&host, "run_app", json!({ "action": "stop" }));
    assert_eq!(text(&r), "Nothing of this project is running.");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "action": "stop", "refused": { "code": "no-session" }, "session": null }));

    let host = Fake::new(run_page(json!({ "tool": "run_app", "action": "start", "refused": { "code": "waiting" } })));
    let r = call(&host, "run_app", json!({ "action": "start" }));
    assert!(!is_error(&r));
    assert_eq!(r["structuredContent"], json!({ "ok": true, "action": "start", "pending": true, "session": null }));

    let host = Fake::new(run_page(json!({ "error": "target" })));
    let r = call(&host, "run_app", json!({ "action": "start" }));
    assert!(is_error(&r));
    assert_eq!(text(&r), "No such session in this project. Call list_sessions for the current ones.");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "target" }));

    let host = Fake::new(Box::new(|_| json!({ "ok": true, "tool": "run_status", "configs": [], "devices": [], "sessions": [], "focused": null })));
    let r = call(&host, "run_app", json!({ "action": "start" }));
    assert_eq!(text(&r), "Orrery could not do that.");
}

#[test]
fn run_status_text() {
    let raw = json!({
        "tool": "run_status",
        "configs": [{ "id": "auto-android-app", "name": "app", "type": "android", "selected": true, "device": "emulator-5554" }, { "id": "b", "name": "B", "type": "flutter" }],
        "devices": [{ "serial": "emulator-5554", "name": "Pixel 7", "platform": "android", "state": "ready", "selected": true }],
        "sessions": [session_raw("running")],
        "focused": "0b6f8f4e-1111-4222-8333-944445555666"
    });
    let host = Fake::new(run_page(raw));
    let r = call(&host, "run_status", json!({}));
    let want = format!(
        "Run configurations:\n- auto-android-app: \"app\" (android, pinned to emulator-5554) [selected]\n- b: \"B\" (flutter)\n\nDevices:\n- emulator-5554: \"Pixel 7\" (android, ready) [selected]\n\nRuns (newest first):\n- [shown] {LINE}"
    );
    assert_eq!(text(&r), want);
    assert_eq!(host.takes.lock().unwrap()[0], ("runStatus".to_string(), 30, 60_000));
    assert_eq!(r["structuredContent"]["focused"], "0b6f8f4e-1111-4222-8333-944445555666");

    let host = Fake::new(run_page(json!({ "tool": "run_status" })));
    let r = call(&host, "run_status", json!({}));
    assert_eq!(text(&r), "This project has no Run configuration.\n\nNo device is connected.\n\nNothing has run in this project yet.");
    assert_eq!(r["structuredContent"]["focused"], Value::Null);
}

#[test]
fn run_log_validation_and_text() {
    let host = Fake::new(run_page(json!({ "tool": "run_log" })));
    for args in [json!({ "lines": 0 }), json!({ "lines": 201 }), json!({ "lines": 1.5 }), json!({ "lines": "5" }), json!({ "errorsOnly": "y" }), json!({ "ignoreCase": 1 }), json!({ "match": "(" }), json!({ "match": 5 }), json!({ "since": "bad cursor" })] {
        assert!(is_error(&call(&host, "run_log", args.clone())), "{args}");
    }
    assert!(host.calls.lock().unwrap().is_empty());

    let raw = json!({
        "tool": "run_log", "session": session_raw("running"),
        "lines": [{ "stream": "build", "level": "E", "text": "FAILURE token=abcd1234secret" }, { "stream": "app", "text": "plain" }, { "stream": "app", "text": "three" }],
        "errors": [{ "file": "A.kt", "line": 3, "col": 7, "message": "Unresolved reference", "severity": "error" }],
        "partial": true, "cursor": "run-1.0.7", "reset": true
    });
    let host = Fake::new(run_page(raw.clone()));
    let r = call(&host, "run_log", json!({ "lines": 2, "match": "x", "ignoreCase": true }));
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "run_log", "lines": 2, "errorsOnly": false, "match": "x", "ignoreCase": true }));
    let want = format!(
        "Run: {LINE}\n(Output from before the Orrery window reloaded is not available.)\n(The cursor belonged to another run or a cleared log: this is the log from its start.)\n\n[app] plain\n[app] three\n\nBuild errors:\n- A.kt:3:7 error: Unresolved reference\n\ncursor: run-1.0.7"
    );
    assert_eq!(text(&r), want);
    assert_eq!(r["structuredContent"]["partial"], true);
    assert_eq!(r["structuredContent"]["reset"], true);
    assert!(r["structuredContent"].get("truncated").is_none());

    let r = call(&host, "run_log", json!({ "since": "run-1.0.2", "lines": 1 }));
    assert!(text(&r).contains("\n\n[build E] FAILURE token=[masked]\n\nBuild errors"));
    assert_eq!(host.calls.lock().unwrap()[1], json!({ "tool": "run_log", "lines": 1, "errorsOnly": false, "since": "run-1.0.2" }));

    let host = Fake::new(run_page(json!({ "tool": "run_log", "session": session_raw("running"), "lines": [] })));
    assert!(text(&call(&host, "run_log", json!({}))).ends_with("\n\n(no output yet)"));
    assert!(text(&call(&host, "run_log", json!({ "errorsOnly": true }))).ends_with("\n\n(no matching lines)"));
    assert!(text(&call(&host, "run_log", json!({ "since": "r.0.1" }))).ends_with("\n\n(no new lines)"));

    let host = Fake::new(run_page(json!({ "tool": "run_log" })));
    let r = call(&host, "run_log", json!({}));
    assert_eq!(text(&r), "Nothing has run in this project yet, so there is no Run log.");
    assert_eq!(r["structuredContent"], json!({ "ok": true, "session": null, "lines": [], "errors": [] }));
}

#[test]
fn device_screenshot_branches() {
    let host = Fake::new(run_page(json!({ "tool": "device_screenshot" })));
    assert!(is_error(&call(&host, "device_screenshot", json!({ "serial": "-s" }))));
    assert!(is_error(&call(&host, "device_screenshot", json!({ "serial": "a;rm" }))));
    assert!(host.calls.lock().unwrap().is_empty());
    let r = call(&host, "device_screenshot", json!({}));
    assert_eq!(text(&r), "Orrery could not do that.");

    let host = Fake::new(run_page(json!({ "tool": "device_screenshot", "refused": { "code": "several" }, "devices": [{ "serial": "A1", "name": "Pixel" }, { "serial": "B2", "name": "Tab" }] })));
    let r = call(&host, "device_screenshot", json!({}));
    assert!(is_error(&r));
    assert_eq!(text(&r), "More than one device is connected; pass serial.\nConnected: A1 (\"Pixel\"), B2 (\"Tab\")");
    assert_eq!(r["structuredContent"]["refused"], json!({ "code": "several" }));

    let host = Fake::new(run_page(json!({ "tool": "device_screenshot", "shot": { "path": "/tmp/a.png", "width": 1080, "height": 2340, "serial": "A1", "name": "Pixel", "saved": "capture" } })));
    let r = call(&host, "device_screenshot", json!({ "serial": "A1" }));
    assert_eq!(text(&r), "Saved a 1080×2340 screenshot of \"Pixel\" (A1) to the user's capture folder: /tmp/a.png");
    assert_eq!(r["structuredContent"], json!({ "ok": true, "path": "/tmp/a.png", "width": 1080, "height": 2340, "serial": "A1", "name": "Pixel", "saved": "capture" }));
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "device_screenshot", "serial": "A1" }));

    let host = Fake::new(Box::new(|_| json!({ "ok": true, "tool": "device_screenshot" })));
    let r = call(&host, "device_screenshot", json!({}));
    assert_eq!(text(&r), "The device returned no screenshot.");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "refused": { "code": "capture-failed" } }));
}

fn sessions_page(rows: Value) -> Responder {
    Box::new(move |c| match c["tool"].as_str().unwrap_or("") {
        "list_sessions" => json!({ "ok": true, "tool": "list_sessions", "sessions": rows.clone() }),
        "send_to_session" => json!({ "ok": true, "tool": "send_to_session", "name": "ds" }),
        _ => json!({ "ok": true, "tool": "close_session", "name": "ds", "confirm": false }),
    })
}

fn rows() -> Value {
    json!([
        { "sessionId": "pty-1", "name": "me", "agent": "claude", "status": "working", "self": true, "openedByYou": false },
        { "sessionId": "pty-2", "name": "ds", "agent": "codex", "status": "idle", "self": false, "openedByYou": true, "idleSince": "2026-01-01T00:00:00.000Z", "limit": { "window": "five_hour", "resetsAt": "2026-01-01T05:00:00.000Z" }, "agentSessionId": "0b6f8f4e-1111-4222-8333-944445555666" },
        { "sessionId": "pty-3", "name": "x", "agent": "shell", "status": "running", "self": false, "openedByYou": false, "limit": {} }
    ])
}

#[test]
fn list_sessions_text() {
    let host = Fake::new(sessions_page(rows()));
    let r = call(&host, "list_sessions", json!({}));
    let want = "Sessions in this project:\n- \"me\" (Claude Code, working, this is you) id pty-1\n- \"ds\" (Codex, idle, opened by you, idle since 2026-01-01T00:00:00.000Z, at usage limit (window five_hour, resets 2026-01-01T05:00:00.000Z)) id pty-2, agent session 0b6f8f4e-1111-4222-8333-944445555666\n- \"x\" (shell, running, at usage limit) id pty-3";
    assert_eq!(text(&r), want);
    let host = Fake::new(sessions_page(json!([])));
    assert_eq!(text(&call(&host, "list_sessions", json!({}))), "No session of this project is open in the workspace.");
    let host = Fake::new(Box::new(|_| json!({ "ok": false, "error": "unavailable" })));
    let r = call(&host, "list_sessions", json!({}));
    assert!(is_error(&r));
    assert_eq!(text(&r), "The Orrery window is not ready right now.");
}

#[test]
fn send_and_close_session() {
    let host = Fake::new(sessions_page(rows()));
    for args in [json!({ "text": "hi" }), json!({ "session": "", "text": "hi" }), json!({ "session": "a\u{0}b", "text": "hi" }), json!({ "session": "s", "text": 5 }), json!({ "session": "s", "text": "   " }), json!({ "session": "s", "text": "a".repeat(8001) })] {
        assert!(is_error(&call(&host, "send_to_session", args.clone())), "{args}");
    }
    assert!(host.takes.lock().unwrap().is_empty());
    let r = call(&host, "send_to_session", json!({ "session": "pty-2", "text": " do it\r\n" }));
    assert_eq!(text(&r), "Typed your message into \"ds\". It is waiting in its input box; the user presses Enter to send it.");
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "send_to_session", "session": "pty-2", "text": "do it" }));
    assert_eq!(host.takes.lock().unwrap()[0], ("send_to_session".to_string(), 20, 60_000));
    assert_eq!(r["structuredContent"], json!({ "ok": true, "session": "pty-2", "name": "ds" }));

    let r = call(&host, "close_session", json!({ "session": "pty-2" }));
    assert_eq!(text(&r), "Closed \"ds\"; the owner can undo it.");
    assert_eq!(r["structuredContent"], json!({ "ok": true, "session": "pty-2", "name": "ds", "confirm": false }));
    let host = Fake::new(Box::new(|_| json!({ "ok": true, "tool": "close_session", "name": "ds", "confirm": true })));
    assert_eq!(text(&call(&host, "close_session", json!({ "session": "p" }))), "\"ds\" still runs something, so Orrery asked the owner to confirm the close.");
    assert_eq!(text(&call(&host, "close_session", json!({}))), "session must be a session id from list_sessions.");

    let mut host = Fake::new(sessions_page(rows()));
    host.allow = false;
    assert_eq!(text(&call(&host, "send_to_session", json!({ "session": "s", "text": "x" }))), "Too many calls of this tool from this session in the last minute; wait a little.");
}

#[test]
fn wait_session_flow() {
    let host = Fake::new(sessions_page(rows()));
    for args in [json!({ "session": "pty-2", "until": "done" }), json!({ "session": "pty-2", "until": null }), json!({ "session": "pty-2", "timeoutMs": 999 }), json!({ "session": "pty-2", "timeoutMs": 600001 }), json!({ "session": "pty-2", "timeoutMs": 1500.5 }), json!({ "session": "pty-2", "timeoutMs": null })] {
        assert!(is_error(&call(&host, "wait_session", args.clone())), "{args}");
    }
    let r = call(&host, "wait_session", json!({ "session": "pty-2" }));
    assert_eq!(text(&r), "The session is idle (waited 0 s).");
    assert_eq!(r["structuredContent"], json!({ "ok": true, "tool": "wait_session", "session": "pty-2", "status": "idle", "reached": true, "waitedMs": 0 }));
    assert_eq!(host.takes.lock().unwrap().last().unwrap(), &("wait_session".to_string(), 30, 60_000));

    assert_eq!(call(&host, "wait_session", json!({ "session": "pty-1" }))["structuredContent"]["error"], "target");
    assert_eq!(call(&host, "wait_session", json!({ "session": "nope" }))["structuredContent"]["error"], "target");

    let host = Fake::new(sessions_page(rows()));
    let r = call(&host, "wait_session", json!({ "session": "pty-3", "timeoutMs": 5000 }));
    assert_eq!(text(&r), "The session is running, not idle, after 5 s.");
    assert_eq!(r["structuredContent"]["waitedMs"], 5000);
    assert_eq!(r["structuredContent"]["reached"], false);

    let step = std::sync::atomic::AtomicUsize::new(0);
    let host = Fake::new(Box::new(move |_| {
        let n = step.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let status = if n == 0 { "working" } else if n == 1 { return json!({ "ok": false, "error": "failed" }) } else { "exited-not-listed" };
        if status == "exited-not-listed" {
            return json!({ "ok": true, "tool": "list_sessions", "sessions": [{ "sessionId": "pty-9", "name": "o", "agent": "claude", "status": "idle" }] });
        }
        json!({ "ok": true, "tool": "list_sessions", "sessions": [{ "sessionId": "pty-2", "name": "ds", "agent": "claude", "status": status }] })
    }));
    let r = call(&host, "wait_session", json!({ "session": "pty-2", "until": "exited" }));
    assert_eq!(text(&r), "The session is exited (waited 4 s).");
    assert_eq!(r["structuredContent"]["reached"], true);

    let host = Fake::new(Box::new(|_| json!({ "ok": false, "error": "caller" })));
    assert_eq!(call(&host, "wait_session", json!({ "session": "pty-2" }))["structuredContent"]["error"], "caller");
}
