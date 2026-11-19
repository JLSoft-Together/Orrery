use super::defs::tool_defs;
use super::parse_desk::parse_mcp_desk_result;
use super::tests_tools::{call, is_error, text, Fake, Responder};
use serde_json::{json, Value};

fn desk_ok(v: Value) -> Responder {
    Box::new(move |c| {
        let tool = c["tool"].as_str().unwrap_or("");
        parse_mcp_desk_result(&v, tool)
    })
}

fn code(r: &Value) -> String {
    r["structuredContent"]["error"].as_str().unwrap_or("").to_string()
}

#[test]
fn the_three_tools_are_declared_with_their_limits() {
    let defs = tool_defs(false);
    let find = |n: &str| defs.iter().find(|t| t["name"] == n).unwrap().clone();
    let s = find("disk_search");
    assert_eq!(s["inputSchema"]["properties"]["limit"]["maximum"], 200);
    assert_eq!(s["inputSchema"]["additionalProperties"], false);
    assert_eq!(s["annotations"]["readOnlyHint"], true);
    assert_eq!(find("disk_usage")["inputSchema"]["properties"]["limit"]["maximum"], 100);
    assert_eq!(find("disk_usage")["inputSchema"]["required"], json!(["path"]));
    let d = find("disk_propose_delete");
    assert_eq!(d["inputSchema"]["properties"]["paths"]["maxItems"], 200);
    assert_eq!(d["inputSchema"]["properties"]["reason"]["maxLength"], 500);
    assert_eq!(d["inputSchema"]["required"], json!(["paths", "reason"]));
    assert_eq!(d["annotations"]["destructiveHint"], true);
    assert!(d["inputSchema"]["properties"].get("confirm").is_none() && d["inputSchema"]["properties"].get("skipConfirm").is_none());
}

#[test]
fn search_checks_arguments_before_asking() {
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "total": 0, "ms": 1, "items": [] })));
    let bad = [
        (json!({ "query": 5 }), "query"),
        (json!({ "kind": "movie" }), "kind"),
        (json!({ "minSize": -1 }), "size"),
        (json!({ "maxSize": 1.5 }), "size"),
        (json!({ "olderThanDays": 36501 }), "days"),
        (json!({ "under": "relative/dir" }), "under"),
        (json!({ "limit": 201 }), "limit"),
        (json!({ "limit": 0 }), "limit"),
    ];
    for (args, field) in bad {
        let r = call(&host, "disk_search", args);
        assert!(is_error(&r));
        assert_eq!(code(&r), "invalid");
        assert_eq!(r["structuredContent"]["detail"], field);
    }
    assert!(host.calls.lock().unwrap().is_empty());
    assert!(host.takes.lock().unwrap().is_empty());
}

#[test]
fn search_formats_matches_and_defaults_the_limit() {
    let raw = json!({ "ok": true, "total": 3, "ms": 4, "items": [
        { "path": "C:\\Users\\a\\Tai lieu\\Bao cao.zip", "name": "Bao cao.zip", "size": 5368709120u64, "dir": false, "mtime": 1700000000000u64, "kind": "archive" },
        { "path": "C:\\Users\\a\\node_modules", "name": "node_modules", "size": 123456, "dir": true, "mtime": 0 },
        { "path": "bad\npath", "name": "x", "size": 1, "dir": false, "mtime": 1 }
    ] });
    let host = Fake::with_desk(desk_ok(raw));
    let r = call(&host, "disk_search", json!({ "query": "zip", "kind": "archive", "olderThanDays": 30, "under": "C:\\Users\\a" }));
    assert_eq!(text(&r), "3 matching entries (4 ms), showing 2, biggest first:\n- 5.0 GB C:\\Users\\a\\Tai lieu\\Bao cao.zip [archive, modified 2023-11-14]\n- 121 KB C:\\Users\\a\\node_modules [folder, modified unknown date]");
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "disk_search", "query": "zip", "kind": "archive", "olderThanDays": 30, "under": "C:\\Users\\a", "limit": 50 }));
    assert_eq!(host.takes.lock().unwrap()[0], ("disk-read".to_string(), 30, 60_000));
    assert_eq!(r["structuredContent"]["items"].as_array().unwrap().len(), 2);
}

#[test]
fn search_and_usage_say_what_is_wrong_with_the_index() {
    for (error, starts) in [("not-indexed", "The disk has not been scanned yet. Ask the user to open Settings > Data > Disk"), ("scanning", "The disk index is being built"), ("not-in-index", "That path is not in the disk index")] {
        let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": error })));
        let r = call(&host, "disk_search", json!({}));
        assert!(is_error(&r));
        assert!(text(&r).starts_with(starts), "{}", text(&r));
        assert_eq!(code(&r), error);
        let r = call(&host, "disk_usage", json!({ "path": "/home/a" }));
        assert_eq!(code(&r), error);
    }
    let host = Fake::new(Box::new(|_| Value::Null));
    assert_eq!(text(&call(&host, "disk_search", json!({}))), "Orrery cannot use the disk index right now.");
}

#[test]
fn usage_validates_and_lists_children() {
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "path": "/home/a", "size": 2048, "total": 1, "items": [{ "path": "/home/a/x", "name": "x", "size": 2048, "dir": true, "mtime": 1700000000000u64 }] })));
    for args in [json!({}), json!({ "path": "rel" }), json!({ "path": 5 }), json!({ "path": "/home/a", "limit": 101 }), json!({ "path": "/home/a", "limit": 0 })] {
        assert_eq!(code(&call(&host, "disk_usage", args)), "invalid");
    }
    assert!(host.calls.lock().unwrap().is_empty());
    let r = call(&host, "disk_usage", json!({ "path": "/home/a" }));
    assert_eq!(text(&r), "/home/a: 2.0 KB in total, 1 entry directly inside, showing 1, biggest first:\n- 2.0 KB /home/a/x [folder, modified 2023-11-14]");
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "disk_usage", "path": "/home/a", "limit": 20 }));
}

#[test]
fn propose_delete_checks_arguments_and_cannot_skip_the_dialog() {
    let host = Fake::with_desk(desk_ok(json!({ "ok": true, "done": [], "failed": [], "freed": 0, "permanent": false, "cancelled": true })));
    let many: Vec<String> = (0..201).map(|i| format!("/tmp/f{i}")).collect();
    let bad = [
        (json!({ "reason": "r" }), "paths"),
        (json!({ "paths": [], "reason": "r" }), "paths"),
        (json!({ "paths": many, "reason": "r" }), "paths"),
        (json!({ "paths": ["rel"], "reason": "r" }), "paths"),
        (json!({ "paths": [5], "reason": "r" }), "paths"),
        (json!({ "paths": ["/tmp/a"] }), "reason"),
        (json!({ "paths": ["/tmp/a"], "reason": "   " }), "reason"),
        (json!({ "paths": ["/tmp/a"], "reason": "x".repeat(501) }), "reason"),
        (json!({ "paths": ["/tmp/a"], "reason": "r", "permanent": "yes" }), "permanent"),
    ];
    for (args, field) in bad {
        let r = call(&host, "disk_propose_delete", args);
        assert_eq!(code(&r), "invalid");
        assert_eq!(r["structuredContent"]["detail"], field);
    }
    assert!(host.calls.lock().unwrap().is_empty());
    assert!(host.takes.lock().unwrap().is_empty());

    let r = call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/a"], "reason": "old cache", "force": true, "confirm": false, "skipConfirm": true }));
    assert_eq!(text(&r), "The user declined: nothing was deleted.");
    assert_eq!(r["structuredContent"]["cancelled"], true);
    assert_eq!(host.calls.lock().unwrap()[0], json!({ "tool": "disk_propose_delete", "paths": ["/tmp/a"], "reason": "old cache", "permanent": false }));
    assert_eq!(host.takes.lock().unwrap()[0], ("disk-delete".to_string(), 3, 60_000));
}

#[test]
fn propose_delete_is_limited_to_three_a_minute() {
    let mut host = Fake::with_desk(desk_ok(json!({ "ok": true, "done": [], "failed": [], "freed": 0, "permanent": false, "cancelled": true })));
    host.allow = false;
    let r = call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/a"], "reason": "r" }));
    assert!(is_error(&r));
    assert_eq!(code(&r), "rate");
    assert!(text(&r).contains("max 3"));
    assert!(host.calls.lock().unwrap().is_empty());
}

#[test]
fn propose_delete_reports_the_outcome() {
    let ok = json!({ "ok": true, "done": [{ "path": "/tmp/a", "size": 1048576 }, { "path": "/tmp/b", "size": 1048576 }], "failed": [{ "path": "/tmp/c", "reason": "protected" }], "freed": 2097152, "permanent": false });
    let host = Fake::with_desk(desk_ok(ok));
    let r = call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/a", "/tmp/b", "/tmp/c"], "reason": "r" }));
    assert!(!is_error(&r));
    assert_eq!(text(&r), "The user confirmed: 2 items moved to the Recycle Bin / Trash, 2.0 MB freed.\nNot removed:\n- /tmp/c: protected");
    let perm = json!({ "ok": true, "done": [{ "path": "/tmp/a", "size": 10 }], "failed": [], "freed": 10, "permanent": true });
    let host = Fake::with_desk(desk_ok(perm));
    assert_eq!(text(&call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/a"], "reason": "r", "permanent": true }))), "The user confirmed: 1 item deleted permanently, 10 B freed.");
    assert_eq!(host.calls.lock().unwrap()[0]["permanent"], true);
    let none = json!({ "ok": true, "done": [], "failed": [{ "path": "/tmp/c", "reason": "outside" }], "freed": 0, "permanent": false });
    let host = Fake::with_desk(desk_ok(none));
    let r = call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/c"], "reason": "r" }));
    assert!(is_error(&r));
    assert_eq!(text(&r), "Nothing was deleted.\nNot removed:\n- /tmp/c: outside");
    let host = Fake::with_desk(desk_ok(json!({ "ok": false, "error": "busy" })));
    assert_eq!(code(&call(&host, "disk_propose_delete", json!({ "paths": ["/tmp/a"], "reason": "r" }))), "busy");
}

#[test]
fn the_desk_answer_is_sanitized() {
    let r = parse_mcp_desk_result(&json!({ "ok": true, "total": -5, "ms": 1.5, "items": [{ "path": "/a\u{202e}b", "size": 1 }, { "path": "/ok", "size": 3, "kind": "BAD", "mtime": -1 }] }), "disk_search");
    assert_eq!(r, json!({ "ok": true, "tool": "disk_search", "total": 0, "ms": 0, "items": [{ "path": "/ok", "name": "/ok", "size": 3, "dir": false, "mtime": 0 }] }));
    assert_eq!(parse_mcp_desk_result(&json!({ "ok": true }), "disk_usage"), json!({ "ok": false, "error": "failed" }));
    assert_eq!(parse_mcp_desk_result(&json!({ "ok": false, "error": "not-indexed" }), "disk_search"), json!({ "ok": false, "error": "not-indexed" }));
    assert_eq!(parse_mcp_desk_result(&json!({ "ok": false, "error": "other" }), "disk_search"), json!({ "ok": false, "error": "failed" }));
}
