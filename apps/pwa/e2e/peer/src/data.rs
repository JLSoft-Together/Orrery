use serde_json::{json, Value};

use crate::Desk;

pub const BANNER: &[u8] = b"$ orrery e2e ready\r\n";
pub const BIG_LEN: usize = 65536;

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
    json!({
        "t": "ws.snapshot",
        "rev": desk.rev,
        "closedCount": 0,
        "tabs": [{
            "key": "tab1", "name": "api", "project": "api", "tree": {"pane": "p1"},
            "panes": [
                p1,
                {"key": "p2", "kind": "term", "preset": "codex", "title": "web codex", "state": "working", "rail": "working", "subagents": 0, "dirty": false, "busy": true, "sessionId": "sess-p2", "terminalId": "t2", "cwd": "D:/Code/web", "branch": "main"},
                {"key": "p3", "kind": "term", "preset": "shell", "title": "npm run dev", "state": "idle", "rail": "idle", "subagents": 0, "dirty": false, "busy": false, "sessionId": "sess-p3", "terminalId": "t4", "cwd": "D:/Code/infra", "branch": "infra"}
            ]
        }]
    })
}

const FILE_TEXT: &str = "import 'package:flutter/material.dart';\n\nclass KeyBar extends StatelessWidget {\n  const KeyBar({super.key});\n}\n";

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
