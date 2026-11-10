use super::parse_desk::parse_mcp_desk_result;
use super::tests_tools::{call, is_error, run_page, text, Fake, Responder};
use serde_json::{json, Value};

fn desk_ok(v: Value) -> Responder {
    Box::new(move |c| {
        let tool = c["tool"].as_str().unwrap_or("");
        parse_mcp_desk_result(&v, tool)
    })
}

fn project_file(name: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("mcp-desk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("res/layout")).unwrap();
    std::fs::write(dir.join("res/layout/main.xml"), "<a/>").unwrap();
    std::fs::write(dir.join("res/layout/readme.txt"), "x").unwrap();
    std::fs::write(dir.join(".env"), "S=1").unwrap();
    std::fs::write(dir.join("report.log"), "l").unwrap();
    let root = super::paths::real_of(&crate::util::path_str(&dir)).unwrap();
    (root.clone(), root)
}

#[test]
fn preview_layout_branches() {
    let (root, _) = project_file("layout");
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "png": "/t/layout-main.png", "width": 1080, "height": 2340, "renderMs": 120, "frameworkOnly": true, "diagnostics": [{ "severity": "error", "message": "Missing class", "line": 3 }, { "severity": "weird", "message": "m" }] })));
    host.root = root.clone();
    let r = call(&host, "preview_layout", json!({ "path": "res/layout/main.xml", "device": "tablet", "theme": "dark" }));
    assert_eq!(text(&r), "Rendered 1080×2340 PNG: /t/layout-main.png (120 ms, framework views only: no project model loaded).\nDiagnostics:\n- error line 3: Missing class\n- info: m");
    let path = format!("{root}{}res{}layout{}main.xml", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR);
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "preview_layout", "path": path, "device": "tablet", "night": true }));
    assert_eq!(r["structuredContent"]["path"], json!(path));
    assert_eq!(r["structuredContent"]["tool"], "preview_layout");
    assert_eq!(host.takes.lock().unwrap()[0], ("layout".to_string(), 10, 60_000));

    for (args, want) in [
        (json!({ "path": "res/layout/main.xml", "device": "watch" }), "device must be phone or tablet."),
        (json!({ "path": "res/layout/main.xml", "theme": "blue" }), "theme must be light or dark."),
        (json!({ "path": "../x.xml" }), "That file is outside this session's project folder; only layouts of the project can be rendered."),
        (json!({ "path": "res/layout/none.xml" }), "That file does not exist."),
        (json!({ "path": "res/layout" }), "That is a folder; give a layout XML file."),
        (json!({ "path": 5 }), "path must be a file path (no UNC or device paths)."),
        (json!({ "path": "res/layout/readme.txt" }), "Only .xml layout files can be rendered."),
    ] {
        let r = call(&host, "preview_layout", args);
        assert!(is_error(&r));
        assert_eq!(text(&r), want);
    }

    let mut host = Fake::new(Box::new(|_| Value::Null));
    host.root = root.clone();
    let r = call(&host, "preview_layout", json!({ "path": "res/layout/main.xml" }));
    assert_eq!(text(&r), "Orrery cannot render layouts right now.");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "unavailable" }));

    let mut host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "starting", "retryInSec": 6 })));
    host.root = root.clone();
    let r = call(&host, "preview_layout", json!({ "path": "res/layout/main.xml" }));
    assert_eq!(text(&r), "The layout renderer is starting. Call again in about 6 seconds.");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "starting", "retryInSec": 6 }));
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "render-failed", "detail": "boom" })));
    host.root = root.clone();
    let r = call(&host, "preview_layout", json!({ "path": "res/layout/main.xml" }));
    assert_eq!(text(&r), "layoutlib could not render this layout. (boom)");
    let mut host = Fake::with_desk(Box::new(|_| json!({ "ok": true, "tool": "list_servers", "at": "", "servers": [], "outside": [] })));
    host.root = root;
    assert_eq!(text(&call(&host, "preview_layout", json!({ "path": "res/layout/main.xml" }))), "Orrery could not render the layout.");
}

#[test]
fn list_servers_and_free_port() {
    let raw = json!({
        "ok": true, "at": "2026-01-01T00:00:00.000Z",
        "servers": [
            { "pid": 10, "name": "node", "kind": "dev-server", "ports": [5173], "mine": true, "cmd": "node vite", "idleSec": 600, "orphan": false },
            { "pid": 11, "name": "java", "kind": "daemon", "ports": [], "idleSec": null, "orphan": true },
            { "pid": 12, "name": "a", "kind": "daemon", "ports": [1, 2], "idleSec": 45 },
            { "pid": 13, "name": "b", "kind": "daemon", "ports": [3], "idleSec": 4500 },
            { "pid": 14, "name": "c", "kind": "daemon", "ports": [3], "idleSec": 5400 },
            { "pid": 15, "name": "d", "kind": "daemon", "ports": [3], "idleSec": 90 }
        ],
        "outside": [{ "port": 8080, "name": "java" }]
    });
    let host = Fake::with_desk(desk_ok(raw));
    let r = call(&host, "list_servers", json!({}));
    let want = "Sample of 2026-01-01T00:00:00.000Z.\nAgent servers (6):\n- [mine] node (dev-server, pid 10) :5173 — idle 10 min\n  node vite\n- java (daemon, pid 11) no port — busy, session ended\n- a (daemon, pid 12) :1, :2 — idle 45 s\n- b (daemon, pid 13) :3 — idle 1.3 h\n- c (daemon, pid 14) :3 — idle 1.5 h\n- d (daemon, pid 15) :3 — idle 2 min\nPorts held outside every agent (cannot be freed from Orrery):\n- :8080 java";
    assert_eq!(text(&r), want);
    assert_eq!(host.takes.lock().unwrap()[0], ("servers".to_string(), 20, 60_000));
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "servers": [], "outside": [] })));
    assert_eq!(text(&call(&host, "list_servers", json!({}))), "Sample of unknown time.\nNo agent servers.");
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "no-report" })));
    assert!(text(&call(&host, "list_servers", json!({}))).starts_with("Orrery has no process sample yet"));
    assert_eq!(text(&call(&Fake::new(run_page(json!({}))), "list_servers", json!({}))), "Orrery cannot list servers right now.");

    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "port": 3000, "stopped": [{ "pid": 4, "name": "node" }, { "pid": 5, "name": "esbuild" }] })));
    for p in [json!(0), json!(65536), json!(1.5), json!("80"), Value::Null] {
        assert_eq!(call(&host, "free_port", json!({ "port": p }))["structuredContent"]["error"], "port");
    }
    assert_eq!(call(&host, "free_port", json!({}))["structuredContent"]["error"], "port");
    assert!(host.takes.lock().unwrap().is_empty());
    let r = call(&host, "free_port", json!({ "port": 3000 }));
    assert_eq!(text(&r), "The user confirmed: stopped node (pid 4), esbuild (pid 5); port 3000 is free once it has exited.");
    assert_eq!(host.takes.lock().unwrap()[0], ("free-port".to_string(), 3, 60_000));
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "free_port", "port": 3000 }));
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "not-mine", "detail": "pid 9" })));
    let r = call(&host, "free_port", json!({ "port": 3000 }));
    assert_eq!(text(&r), "That port is held by a process another session or project started; it can only be stopped from Orrery's Resources by the user. (pid 9)");
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "not-mine" }));
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "cancelled", "detail": "x" })));
    assert_eq!(text(&call(&host, "free_port", json!({ "port": 3000 }))), "The user declined: nothing was stopped.");
}

#[test]
fn telegram_notify_and_receive() {
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "enabled": true, "configured": true, "projects": ["a", "b"], "routed": true })));
    let r = call(&host, "telegram_notify", json!({ "state": " ON " }));
    assert_eq!(text(&r), "Telegram reports are ON for the whole app.\nProjects with a topic: a, b.\nThis session's project has a topic: when a task finishes, send a short result summary (and any build file) with telegram_send.");
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "telegram_notify", "state": "on" }));
    call(&host, "telegram_notify", json!({ "state": "" }));
    call(&host, "telegram_notify", json!({}));
    assert_eq!(host.calls.lock().unwrap()[1]["state"], "status");
    assert_eq!(host.calls.lock().unwrap()[2]["state"], "status");
    assert_eq!(call(&host, "telegram_notify", json!({ "state": "maybe" }))["structuredContent"]["error"], "state");
    assert_eq!(call(&host, "telegram_notify", json!({ "state": 1 }))["structuredContent"]["error"], "state");
    assert_eq!(host.takes.lock().unwrap()[0], ("telegram-notify".to_string(), 10, 60_000));

    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "enabled": false, "projects": [] })));
    assert_eq!(text(&call(&host, "telegram_notify", json!({}))), "Telegram reports are OFF for the whole app.\nNo project has a Telegram topic yet; the user assigns them in Orrery Settings › Notifications › Telegram.");
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "enabled": true, "projects": ["a"], "routed": false })));
    assert!(text(&call(&host, "telegram_notify", json!({}))).ends_with("This session's project has no topic, so it is not reported."));
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "telegram-config" })));
    assert!(text(&call(&host, "telegram_notify", json!({}))).starts_with("No Telegram bot is set up yet: ask"));
    assert_eq!(text(&call(&Fake::new(run_page(json!({}))), "telegram_notify", json!({}))), "Orrery cannot change Telegram reports right now.");

    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "receiving": true, "paired": true, "owner": "@me" })));
    assert_eq!(text(&call(&host, "telegram_receive", json!({ "state": "on" }))), "Telegram commands are ON. Only @me can send.");
    assert_eq!(host.takes.lock().unwrap()[0], ("telegram-receive".to_string(), 10, 60_000));
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "receiving": false, "paired": false })));
    assert_eq!(text(&call(&host, "telegram_receive", json!({}))), "Telegram commands are OFF. No account is paired yet.");
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "not-paired" })));
    assert!(text(&call(&host, "telegram_receive", json!({ "state": "on" }))).starts_with("No Telegram account is paired yet"));
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "cancelled" })));
    assert!(text(&call(&host, "telegram_receive", json!({ "state": "on" }))).starts_with("The user did not allow"));
}

#[test]
fn telegram_send_branches() {
    let (root, _) = project_file("tg");
    let sent = json!({ "ok": true, "topic": "app" });
    let mut host = Fake::with_desk(desk_ok(sent.clone()));
    host.root = root.clone();
    for (args, err) in [
        (json!({}), "text"),
        (json!({ "text": "   " }), "text"),
        (json!({ "text": "x".repeat(3001) }), "text"),
        (json!({ "text": "a", "file": "../x" }), "outside"),
        (json!({ "text": "a", "file": "none.log" }), "missing"),
        (json!({ "text": "a", "file": ".env" }), "secret"),
        (json!({ "text": "a", "buttons": [] }), "buttons"),
        (json!({ "text": "a", "buttons": "x" }), "buttons"),
        (json!({ "text": "a", "buttons": [{ "label": "", "value": "v" }] }), "buttons"),
        (json!({ "text": "a", "file": "report.log", "buttons": [{ "label": "A", "value": "v" }] }), "buttons-file"),
    ] {
        let r = call(&host, "telegram_send", args.clone());
        assert!(is_error(&r));
        assert_eq!(r["structuredContent"], json!({ "ok": false, "error": err }), "{args}");
    }
    assert!(host.takes.lock().unwrap().is_empty());
    assert!(text(&call(&host, "telegram_send", json!({ "text": "x".repeat(3000) }))).starts_with("Sent the report to Telegram topic app."));

    let r = call(&host, "telegram_send", json!({ "text": "done", "buttons": [{ "label": " Yes\n", "value": " go " }, { "label": "No", "value": "stop" }] }));
    assert_eq!(text(&r), "Sent the report to Telegram topic app.");
    let calls = host.calls.lock().unwrap().clone();
    assert_eq!(calls.last().unwrap(), &json!({ "tool": "telegram_send", "text": "done", "buttons": [{ "label": "Yes", "value": "go" }, { "label": "No", "value": "stop" }] }));
    assert_eq!(host.takes.lock().unwrap().last().unwrap(), &("telegram-send".to_string(), 6, 60_000));

    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "topic": "app", "buttons": 2 })));
    host.root = root.clone();
    let r = call(&host, "telegram_send", json!({ "text": "t", "buttons": [{ "label": "A", "value": "a" }, { "label": "B", "value": "b" }] }));
    assert_eq!(text(&r), "Sent the report with 2 buttons to Telegram topic app. The user's tap arrives here as your next prompt.");
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "topic": "app", "file": "report.log", "uploading": true })));
    host.root = root.clone();
    let r = call(&host, "telegram_send", json!({ "text": "t", "file": "report.log" }));
    assert_eq!(text(&r), "Sent the report to Telegram topic app; report.log is uploading in the background and posts there when done.");
    let want_file = format!("{root}{}report.log", std::path::MAIN_SEPARATOR);
    assert_eq!(host.calls.lock().unwrap()[0]["file"], json!(want_file));
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "topic": "app", "file": "r.apk", "linkOnly": true })));
    host.root = root.clone();
    assert_eq!(text(&call(&host, "telegram_send", json!({ "text": "t", "file": "report.log" }))), "Sent the report (file r.apk too large, sent as path) to Telegram topic app.");
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "topic": "app", "file": "r.apk" })));
    host.root = root.clone();
    assert_eq!(text(&call(&host, "telegram_send", json!({ "text": "t", "file": "report.log" }))), "Sent the report with r.apk to Telegram topic app.");
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "no-topic" })));
    host.root = root.clone();
    let r = call(&host, "telegram_send", json!({ "text": "t" }));
    assert!(text(&r).starts_with("This session's project has no Telegram topic"));
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "status", "detail": "token" })));
    host.root = root;
    let r = call(&host, "telegram_send", json!({ "text": "t" }));
    assert_eq!(r["structuredContent"], json!({ "ok": false, "error": "token" }));
    assert_eq!(text(&r), "The Telegram bot token was refused; ask the user to check it in Orrery Settings.");
}
