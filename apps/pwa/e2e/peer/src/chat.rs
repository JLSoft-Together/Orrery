use serde_json::{json, Value};

pub const GEN: &str = "sess-p1:1";

pub fn last_full() -> Value {
    json!({"t": "chat.last", "full": true, "items": [
        {"paneKey": "p1", "text": "Lỗi do redirect_uri thiếu dấu / cuối nên Google từ chối callback.", "at": 1763600020000u64},
        {"paneKey": "p2", "text": null}
    ]})
}

fn latest() -> Vec<Value> {
    vec![
        json!({"id": "u:2", "k": "me", "seq": 200, "rev": 0, "at": 1763600000000u64, "text": "Đăng nhập Google bị lỗi 500, xem giúp."}),
        json!({"id": "f:2", "k": "files", "seq": 201, "rev": 1, "root": "D:/Code/api", "files": [
            {"path": "src/app.ts", "add": 2, "del": 1},
            {"path": "test/auth.test.ts", "add": 14},
            {"path": "~/notes.md", "out": true}
        ]}),
        json!({"id": "t:2", "k": "tools", "seq": 210, "rev": 2, "sum": {"read": 4, "run": 1}, "items": [
            {"tool": "Read", "cat": "read", "target": "auth/google.ts"},
            {"tool": "Bash", "cat": "run", "target": "rg redirect_uri", "err": true}
        ]}),
        json!({"id": "a:2", "k": "ai", "seq": 300, "rev": 1, "at": 1763600020000u64, "text": "Lỗi do `redirect_uri` thiếu dấu `/` cuối.\n\n- Sửa **auth/google.ts**\n- Thêm test\n\n```ts\nconst uri = `${base}/auth/callback/`\n```\n\n<b>raw</b>"}),
    ]
}

fn older() -> Vec<Value> {
    vec![
        json!({"id": "u:1", "k": "me", "seq": 10, "rev": 0, "text": "Câu hỏi cũ nhất"}),
        json!({"id": "a:1", "k": "ai", "seq": 20, "rev": 0, "text": "Trả lời cũ"}),
        json!({"id": "s:1", "k": "sys", "seq": 30, "rev": 0, "kind": "compact"}),
    ]
}

pub fn page(pane: &str, before: Option<u64>) -> Value {
    let agent = if pane == "p2" { "codex" } else { "claude" };
    if pane == "p2" {
        return json!({"paneKey": pane, "agent": agent, "gen": "", "items": [], "older": null});
    }
    match before {
        Some(_) => json!({"paneKey": pane, "agent": agent, "gen": GEN, "items": older(), "older": null}),
        None => json!({"paneKey": pane, "agent": agent, "gen": GEN, "items": latest(), "older": 200}),
    }
}

pub fn live_items() -> Value {
    json!({"t": "chat.items", "paneKey": "p1", "gen": GEN, "items": [
        {"id": "a:2", "k": "ai", "seq": 300, "rev": 2, "text": "Đã sửa xong, test pass."},
        {"id": "u:3", "k": "me", "seq": 400, "rev": 0, "text": "Cảm ơn"}
    ]})
}

pub fn reset() -> Value {
    json!({"t": "chat.reset", "paneKey": "p1", "gen": "sess-p1:2"})
}

pub fn menu() -> Value {
    json!({
        "sig": "ab12cd34", "kind": "permission", "agent": "claude", "question": "Do you want to proceed?",
        "items": [
            {"index": 1, "label": "Yes", "selected": true},
            {"index": 2, "label": "Yes, and don't ask again"},
            {"index": 3, "label": "No, and tell Claude what to do differently"}
        ],
        "numbered": true, "confident": true, "since": 1763600000000u64
    })
}
