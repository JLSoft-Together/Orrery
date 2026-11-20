use serde_json::{json, Value};

use crate::Desk;

pub const BANNER: &[u8] = b"$ orrery e2e ready\r\n";
pub const BIG_LEN: usize = 65536;
pub const VAPID_KEY: &str = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";

pub fn big_payload() -> Vec<u8> {
    let mut out = Vec::with_capacity(BIG_LEN);
    let mut n = 0u32;
    while out.len() < BIG_LEN {
        let line = format!("line {n:05} {}\r\n", "abcdefghij".repeat(6));
        let room = BIG_LEN - out.len();
        let bytes = line.as_bytes();
        out.extend_from_slice(&bytes[..bytes.len().min(room)]);
        n += 1;
    }
    out
}

pub const RESIZE_LINES: u32 = 4700;
pub const RESIZE_AT: u32 = 2350;
pub const RESIZE_ROWS: u32 = 12;

pub fn resize_lines(prefix: &str, from: u32, to: u32) -> Vec<u8> {
    (from..=to).map(|n| format!("{prefix}-{n:05}-xxx\r\n")).collect::<String>().into_bytes()
}

pub fn resize_repaint(prefix: &str) -> Vec<u8> {
    let filled = RESIZE_ROWS - 1;
    let mut out = String::from("\x1b[?25l\x1b[H");
    for r in 0..filled {
        out.push_str(&format!("{prefix}-{:05}-xxx\x1b[K\r\n", RESIZE_AT - filled + 1 + r));
    }
    out.push_str(&format!("\x1b[K\x1b[{};1H\x1b[?25h", RESIZE_ROWS));
    out.into_bytes()
}

pub fn resize_snapshot(prefix: &str) -> String {
    let lines: Vec<String> = (1..=RESIZE_AT).map(|n| format!("\x1b[m{prefix}-{n:05}-xxx")).collect();
    format!("{}\x1b[m\r\n\x1b[m\x1b[{};1H", lines.join("\x1b[m\r\n"), RESIZE_ROWS)
}

pub fn pane_key(key: Option<&str>) -> Option<&'static str> {
    match key? {
        "p1" => Some("p1"),
        "p2" => Some("p2"),
        "p3" => Some("p3"),
        _ => None,
    }
}

pub fn ws_snapshot(desk: &Desk) -> Value {
    let (rail, wait) = if desk.asked {
        ("asked", Some("Chạy lệnh: npm test? [y/n]"))
    } else {
        ("working", None)
    };
    let mut p1 = json!({
        "key": "p1", "kind": "term", "preset": "claude", "agent": "claude", "title": desk.title, "state": rail, "rail": rail,
        "subagents": 0, "dirty": false, "busy": true, "sessionId": "sess-p1", "terminalId": "t1",
        "cwd": "D:/Code/api", "branch": "feat/api"
    });
    if let Some(text) = wait {
        p1["waitText"] = json!(text);
    }
    let mut p2 = json!({"key": "p2", "kind": "term", "preset": "codex", "title": "web codex", "state": "working", "rail": "working", "subagents": 0, "dirty": false, "busy": true, "sessionId": "sess-p2", "terminalId": "t2", "cwd": "D:/Code/web", "branch": "main"});
    let mut p3 = json!({"key": "p3", "kind": "term", "preset": "shell", "title": "npm run dev", "state": "idle", "rail": "idle", "subagents": 0, "dirty": false, "busy": false, "sessionId": "sess-p3", "terminalId": "t4", "cwd": "D:/Code/infra", "branch": "infra"});
    let mut p4 = json!({"key": "p4", "kind": "term", "preset": "shell", "title": "db shell", "state": "idle", "rail": "idle", "subagents": 0, "dirty": false, "busy": false, "sessionId": "sess-p4", "terminalId": "t3", "cwd": "D:/Code/api", "branch": "feat/api"});
    let mut extra = Vec::new();
    if desk.inbox == 1 {
        p1["inbox"] = json!({"reason": "permission", "since": "2026-11-20T08:00:00Z"});
        p2["inbox"] = json!({"reason": "clash", "since": "2026-11-20T08:05:00Z", "file": "src/app.ts", "peer": "api claude"});
        p3["inbox"] = json!({"reason": "done", "since": "2026-11-20T07:00:00Z"});
        p4["inbox"] = json!({"reason": "menu", "since": "2026-11-20T08:10:00Z"});
        extra.push(p4);
    } else if desk.inbox == 2 {
        p1["attention"] = json!("waiting");
        p3["attention"] = json!("done");
    }
    let mut first = Vec::new();
    let mut second = Vec::new();
    for (key, pane) in [("p1", p1), ("p2", p2), ("p3", p3)] {
        if desk.hidden.contains(&key) {
            continue;
        }
        if key == "p3" && desk.moved {
            second.push(pane);
        } else {
            first.push(pane);
        }
    }
    first.extend(extra);
    let name = desk.tab_name.clone();
    let mut tab1 = json!({"key": "tab1", "project": "api", "tree": {"pane": "p1"}, "panes": first});
    if let Some(name) = name {
        tab1["name"] = json!(name);
    } else {
        tab1["name"] = json!("api");
    }
    if desk.groups {
        let mut groups = Vec::new();
        let mut keys: Vec<&str> = first.iter().filter_map(|p| p["key"].as_str()).collect();
        keys.sort();
        if keys.contains(&"p1") {
            groups.push(json!({"project": "D:/Code/api", "name": "api", "tone": 0, "panes": ["p1"]}));
        }
        if keys.contains(&"p2") {
            groups.push(json!({"project": "D:/Code/web", "name": "web", "tone": 2, "panes": ["p2"]}));
        }
        if keys.contains(&"p3") {
            groups.push(json!({"project": "D:/Code/infra", "name": "infra", "tone": 4, "folded": true, "panes": ["p3"]}));
        }
        tab1["groups"] = json!(groups);
    }
    let mut tabs = vec![tab1];
    if desk.tab2 {
        tabs.push(json!({"key": "tab2", "name": "web", "project": "web", "panes": second}));
    }
    let mut out = json!({
        "t": "ws.snapshot",
        "rev": desk.rev,
        "closedCount": desk.hidden.len(),
        "tabs": tabs
    });
    if desk.inbox == 1 {
        out["inbox"] = json!(true);
    }
    out
}

pub const FILE_TEXT: &str = "import 'package:flutter/material.dart';\n\nclass KeyBar extends StatelessWidget {\n  const KeyBar({super.key});\n}\n";

const DIFF_TEXT: &str = "diff --git a/src/app.ts b/src/app.ts\n--- a/src/app.ts\n+++ b/src/app.ts\n@@ -12,6 +12,9 @@ class KeyBar\n   return SizedBox(\n-    child: Row(\n+    child: ListView(\n+      scrollDirection: Axis.horizontal,\n     children: [\n";

pub fn viewer(kind: &str, value: &Value) -> Option<Value> {
    let root = value.get("root").and_then(Value::as_str).unwrap_or("D:/Code/api");
    match kind {
        "fs.list" => {
            let dir = value.get("dir").and_then(Value::as_str).unwrap_or(root).trim_end_matches('/').to_owned();
            if dir.ends_with("/src") {
                Some(json!([{"name": "app.ts", "path": format!("{dir}/app.ts"), "directory": false}]))
            } else {
                Some(json!([
                    {"name": "src", "path": format!("{dir}/src"), "directory": true},
                    {"name": "README.md", "path": format!("{dir}/README.md"), "directory": false},
                    {"name": "key_bar.dart", "path": format!("{dir}/key_bar.dart"), "directory": false}
                ]))
            }
        }
        "fs.read" => {
            let path = value.get("path").and_then(Value::as_str).unwrap_or("");
            Some(json!({"kind": "text", "path": path, "size": FILE_TEXT.len(), "mtimeMs": 1, "text": FILE_TEXT}))
        }
        "git.status" => Some(json!({
            "root": root, "branch": "feat/api",
            "entries": [
                {"path": "src/app.ts", "kind": "modified", "x": ".", "y": "M", "added": 3, "removed": 1},
                {"path": "notes.txt", "kind": "untracked", "x": "?", "y": "?"}
            ]
        })),
        "git.diff" => Some(json!({"text": DIFF_TEXT})),
        "git.watch" | "git.unwatch" | "fs.watch" | "fs.unwatch" => Some(json!(true)),
        "log.page" => Some(json!({
            "commits": [
                {"hash": "a1b2c3d4e5f6", "parents": [], "author": "Cham", "time": 1767225600, "refs": "HEAD, feat/api", "subject": "Thêm thanh phím phụ"},
                {"hash": "0f9e8d7c6b5a", "parents": [], "author": "Cham", "time": 1767139200, "refs": "", "subject": "Khởi tạo"}
            ],
            "more": false
        })),
        "run.sessions" => Some(json!([{"id": "r1", "configName": "flutter run", "state": "running", "cwd": "D:/Code/api", "progress": "Hot reload 1.2s"}])),
        _ => None,
    }
}

pub fn demo_payload() -> Vec<u8> {
    let text = "\x1b[2J\x1b[H\x1b[90m$ claude --resume 4f2a\x1b[0m\r\n\x1b[31m●\x1b[0m Đọc \x1b[36mapps/mobile/lib/ui/terminal/key_bar.dart\x1b[0m\r\n\x1b[31m●\x1b[0m Sửa \x1b[36mdocs/design/v97-mobile-ui/index.html\x1b[0m\r\n  \x1b[32m+ 214\x1b[0m \x1b[31m- 0\x1b[0m\r\n\x1b[31m●\x1b[0m Bash(npm run typecheck -w @ccm/web)\r\n  └ 0 lỗi, 2 cảnh báo\r\n\r\n\x1b[33m✻\x1b[0m Đang dựng khung… (2m 14s · esc)\r\n\r\n\x1b[35m> \x1b[0m";
    text.as_bytes().to_vec()
}
