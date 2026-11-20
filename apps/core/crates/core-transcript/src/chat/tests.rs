use super::page::{last_reply, page, Poll};
use super::*;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

struct Fake;

impl Clean for Fake {
    fn line(&self, text: &str, max: usize) -> String {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ").replace("sk-SECRET", "••••");
        flat.chars().take(max).collect()
    }
    fn block(&self, text: &str, max: usize) -> (String, bool) {
        let t = text.replace("sk-SECRET", "••••");
        let n = t.chars().count();
        (t.chars().take(max).collect(), n > max)
    }
}

const ROOT: &str = "C:\\work\\api";
const TS: &str = "2026-11-20T01:00:00.000Z";

fn file(name: &str, body: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-chat-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("s.jsonl");
    std::fs::write(&p, body).unwrap();
    p
}

fn line(v: Value) -> String {
    format!("{v}\n")
}

fn prompt(uuid: &str, text: &str) -> String {
    line(json!({ "type": "user", "uuid": uuid, "timestamp": TS, "isSidechain": false, "message": { "role": "user", "content": text } }))
}

fn say(id: &str, text: &str, stop: Option<&str>) -> String {
    line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "claude-opus-5-5", "content": [{ "type": "text", "text": text }], "stop_reason": stop } }))
}

fn think(id: &str) -> String {
    line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "m", "content": [{ "type": "thinking", "thinking": "private thoughts" }] } }))
}

fn tool(id: &str, tid: &str, name: &str, input: Value) -> String {
    line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "m", "content": [{ "type": "tool_use", "id": tid, "name": name, "input": input }], "stop_reason": "tool_use" } }))
}

fn result(tid: &str, err: bool) -> String {
    line(json!({ "type": "user", "timestamp": TS, "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": tid, "is_error": err, "content": "RAW TOOL OUTPUT" }] } }))
}

fn sample() -> String {
    let mut s = String::new();
    s += &line(json!({ "type": "mode", "mode": "normal" }));
    s += &prompt("u1", "Login fails <system-reminder>hidden rules</system-reminder> with key sk-SECRET");
    s += &line(json!({ "type": "user", "isMeta": true, "timestamp": TS, "message": { "role": "user", "content": "meta caveat" } }));
    s += &think("m1");
    s += &tool("m1", "t1", "Read", json!({ "file_path": "C:\\work\\api\\auth\\google.ts" }));
    s += &result("t1", false);
    s += &tool("m2", "t2", "Bash", json!({ "command": "rg redirect_uri && export TOKEN=abc" }));
    s += &result("t2", true);
    s += &tool("m3", "t3", "Edit", json!({ "file_path": "C:\\work\\api\\auth\\google.ts", "old_string": "a", "new_string": "b\nc" }));
    s += &tool("m3", "t4", "Write", json!({ "file_path": "D:\\other\\x.txt", "content": "1\n2\n3\n" }));
    s += &line(json!({ "type": "assistant", "isSidechain": true, "timestamp": TS, "message": { "id": "side", "content": [{ "type": "text", "text": "subagent says" }] } }));
    s += &say("m4", "Fixed the redirect.", None);
    s += &say("m4", "Done.", Some("end_turn"));
    s += &line(json!({ "type": "system", "subtype": "compact_boundary", "timestamp": TS }));
    s += &prompt("u2", "<command-name>/review</command-name>\n<command-message>review</command-message>\n<command-args>auth</command-args>");
    s += &line(json!({ "type": "user", "timestamp": TS, "message": { "role": "user", "content": [{ "type": "text", "text": "[Request interrupted by user]" }] } }));
    s
}

fn kinds(items: &[Value]) -> Vec<String> {
    items.iter().map(|i| i["k"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn claude_items_skip_private_parts_and_mask() {
    let p = file("claude", &sample());
    let pg = page(&p, Agent::Claude, ROOT, None, PAGE, &Fake, false).unwrap();
    let text = serde_json::to_string(&pg.items).unwrap();
    assert_eq!(kinds(&pg.items), ["me", "files", "tools", "ai", "sys", "me", "sys"]);
    for bad in ["private thoughts", "hidden rules", "RAW TOOL OUTPUT", "meta caveat", "subagent says", "sk-SECRET"] {
        assert!(!text.contains(bad), "{bad} leaked: {text}");
    }
    assert_eq!(pg.items[0]["text"], "Login fails  with key ••••");
    let tools = &pg.items[2];
    assert_eq!(tools["sum"], json!({ "read": 1, "run": 1, "edit": 2 }));
    assert_eq!(tools["items"][0], json!({ "tool": "Read", "cat": "read", "target": "auth/google.ts" }));
    assert_eq!(tools["items"][1]["err"], true);
    assert_eq!(tools["rev"], 4);
    let files = &pg.items[1];
    assert_eq!(files["seq"].as_u64().unwrap(), pg.items[0]["seq"].as_u64().unwrap() + 1);
    assert_eq!(files["files"][0], json!({ "path": "auth/google.ts", "add": 2, "del": 1 }));
    assert_eq!(files["files"][1]["out"], true);
    assert_eq!(files["files"][1]["add"], 3);
    assert_eq!(pg.items[3]["text"], "Fixed the redirect.\n\nDone.");
    assert_eq!(pg.items[3]["rev"], 1);
    assert_eq!(pg.items[4]["kind"], "compact");
    assert_eq!(pg.items[5]["text"], "/review auth");
    assert_eq!(pg.items[6]["kind"], "stop");
    assert_eq!(pg.older, None);
    assert!(pg.items.iter().all(|i| i["at"].as_i64().is_some_and(|a| a > 0) || i["k"] == "files"));
}

#[test]
fn last_reply_is_the_latest_finished_turn() {
    let mut s = sample();
    s += &prompt("u3", "next");
    s += &say("m9", "still working", None);
    let p = file("last", &s);
    let (text, at) = last_reply(&p, Agent::Claude).unwrap().unwrap();
    assert_eq!(text, "Done.");
    assert!(at.is_some());
    assert_eq!(last_paragraph("Intro\n\n```\ncode\n\nmore\n```\n\nFinal line here.\n"), "Final line here.");
    assert_eq!(last_paragraph("Only\n\n```rust\nfn x() {}\n```"), "Only");
    assert_eq!(last_paragraph("Done.\n\n## Kết quả: **ok** cho `redirect_uri`, xem [docs](http://x) và _note_ a * b"), "Kết quả: ok cho redirect_uri, xem docs và note a * b");
    assert_eq!(last_paragraph("- **src/a.ts**: thêm\n- my_var giữ"), "src/a.ts: thêm\nmy_var giữ");
}

fn turns(n: usize) -> String {
    let mut s = String::new();
    for k in 0..n {
        s += &prompt(&format!("u{k}"), &format!("question {k}"));
        s += &tool(&format!("t{k}"), &format!("tool{k}"), "Bash", json!({ "command": format!("echo {k}") }));
        s += &say(&format!("a{k}"), &format!("answer {k}"), Some("end_turn"));
    }
    s
}

#[test]
fn paging_walks_the_whole_file_without_gaps_or_repeats() {
    let p = file("paging", &turns(40));
    let full = page(&p, Agent::Claude, ROOT, None, 10_000, &Fake, false).unwrap();
    assert_eq!(full.items.len(), 120);
    assert_eq!(full.older, None);
    let mut seen: Vec<Value> = Vec::new();
    let mut before = None;
    let mut rounds = 0;
    loop {
        let pg = page(&p, Agent::Claude, ROOT, before, 7, &Fake, false).unwrap();
        assert_eq!(pg.items.first().unwrap()["k"], "me");
        assert!(pg.items.len() % 3 == 0 && (pg.items.len() >= 7 || pg.older.is_none()));
        if let Some(b) = before {
            assert!(pg.items.iter().all(|i| i["seq"].as_u64().unwrap() < b));
        }
        let mut next = pg.items.clone();
        next.extend(seen);
        seen = next;
        rounds += 1;
        match pg.older {
            Some(o) => before = Some(o),
            None => break,
        }
        assert!(rounds < 50);
    }
    assert_eq!(seen, full.items);
}

#[test]
fn replies_stay_under_the_byte_cap() {
    let mut s = String::new();
    for k in 0..60 {
        s += &prompt(&format!("u{k}"), &"q".repeat(3000));
        s += &say(&format!("a{k}"), &"é".repeat(7000), Some("end_turn"));
    }
    let p = file("budget", &s);
    let pg = page(&p, Agent::Claude, ROOT, None, PAGE_MAX, &Fake, false).unwrap();
    let bytes = serde_json::to_vec(&pg.items).unwrap().len();
    assert!(bytes < REPLY_BYTES, "{bytes}");
    assert!(pg.older.is_some());
    assert_eq!(pg.items[0]["k"], "me");
}

#[test]
fn live_feed_sends_new_and_changed_items_then_resets_on_truncate() {
    let p = file("live", &turns(2));
    let mut pg = page(&p, Agent::Claude, ROOT, None, PAGE, &Fake, true).unwrap();
    let live = pg.live.as_mut().unwrap();
    assert!(matches!(live.poll().unwrap(), Poll::Items(v) if v.is_empty()));
    let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
    f.write_all(prompt("u9", "more").as_bytes()).unwrap();
    f.write_all(tool("x", "tx", "Grep", json!({ "pattern": "foo" })).as_bytes()).unwrap();
    f.write_all(b"{\"type\":\"assistant\"").unwrap();
    let Poll::Items(got) = live.poll().unwrap() else { panic!("reset") };
    assert_eq!(got.iter().map(Item::kind).collect::<Vec<_>>(), ["me", "tools"]);
    f.write_all(b",\"timestamp\":\"2026-11-20T01:00:00.000Z\",\"message\":{\"id\":\"y\",\"content\":[{\"type\":\"tool_use\",\"id\":\"ty\",\"name\":\"Glob\",\"input\":{\"pattern\":\"*.ts\"}}]}}\n").unwrap();
    let Poll::Items(got) = live.poll().unwrap() else { panic!("reset") };
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].rev, 1);
    assert_eq!(got[0].to_json(&Fake)["sum"], json!({ "search": 2 }));
    drop(f);
    std::fs::write(&p, prompt("z", "new")).unwrap();
    assert!(matches!(live.poll().unwrap(), Poll::Reset));
}

fn ev(ty: &str, payload: Value) -> String {
    line(json!({ "timestamp": TS, "type": ty, "payload": payload }))
}

#[test]
fn codex_rollout_items_and_last_reply() {
    let mut s = String::new();
    s += &ev("session_meta", json!({ "id": "x", "cwd": ROOT }));
    s += &ev("response_item", json!({ "type": "message", "role": "developer", "content": [{ "type": "input_text", "text": "dev rules" }] }));
    s += &ev("response_item", json!({ "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "# AGENTS.md instructions\n<INSTRUCTIONS>x</INSTRUCTIONS>" }] }));
    s += &ev("response_item", json!({ "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "Fix login" }] }));
    s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "UserMessage", "content": [{ "type": "text", "text": "Fix login" }] } }));
    s += &ev("response_item", json!({ "type": "reasoning", "summary": [], "encrypted_content": "zzz" }));
    s += &ev("response_item", json!({ "type": "message", "role": "assistant", "phase": "commentary", "id": "m1", "content": [{ "type": "output_text", "text": "Looking." }] }));
    s += &ev("response_item", json!({ "type": "custom_tool_call", "name": "exec", "call_id": "c1", "input": "await tools.exec_command({cmd:'ls'})" }));
    s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "CommandExecution", "id": "e1", "command": ["powershell.exe", "-Command", "Get-Content a.ts"], "parsed_cmd": [{ "type": "read" }], "status": "completed", "exit_code": 0, "aggregated_output": "SECRET OUTPUT" } }));
    s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "CommandExecution", "id": "e2", "command": ["powershell.exe", "-Command", "npm test"], "status": "failed", "exit_code": 1 } }));
    s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "FileChange", "id": "e3", "changes": { "C:\\work\\api\\a.ts": { "type": "update", "unified_diff": "@@ -1 +1,2 @@\n-x\n+y\n+z\n" }, "C:\\work\\api\\b.ts": { "type": "add", "content": "1\n2\n" } } } }));
    s += &ev("response_item", json!({ "type": "message", "role": "assistant", "phase": "final_answer", "id": "m2", "content": [{ "type": "output_text", "text": "Fixed it." }] }));
    s += &ev("event_msg", json!({ "type": "task_complete", "last_agent_message": "Fixed it." }));
    s += &ev("event_msg", json!({ "type": "turn_aborted", "reason": "interrupted" }));
    let p = file("codex", &s);
    let pg = page(&p, Agent::Codex, ROOT, None, PAGE, &Fake, false).unwrap();
    let text = serde_json::to_string(&pg.items).unwrap();
    for bad in ["dev rules", "AGENTS.md", "zzz", "SECRET OUTPUT", "exec_command"] {
        assert!(!text.contains(bad), "{bad} leaked: {text}");
    }
    assert_eq!(kinds(&pg.items), ["me", "files", "ai", "tools", "ai", "sys"]);
    let tools = &pg.items[3];
    assert_eq!(tools["sum"], json!({ "read": 1, "run": 1, "edit": 1 }));
    assert_eq!(tools["items"][0]["target"], "Get-Content a.ts");
    assert_eq!(tools["items"][1]["err"], true);
    assert_eq!(pg.items[1]["files"], json!([{ "path": "a.ts", "add": 2, "del": 1 }, { "path": "b.ts", "add": 2 }]));
    assert_eq!(last_reply(&p, Agent::Codex).unwrap().unwrap().0, "Fixed it.");
}

#[test]
fn rel_path_handles_case_and_outside_root() {
    assert_eq!(rel_path("c:\\Work\\API\\src\\x.ts", ROOT).as_deref(), Some("src/x.ts"));
    assert_eq!(rel_path("C:\\work\\apiary\\x.ts", ROOT), None);
    assert_eq!(rel_path("/home/u/p/x.ts", "/home/u/p").as_deref(), Some("x.ts"));
    assert_eq!(rel_path("src/x.ts", ROOT).as_deref(), Some("src/x.ts"));
}

#[test]
fn tool_groups_cap_items_and_count_the_rest() {
    let mut s = prompt("u", "go");
    for k in 0..35 {
        s += &tool(&format!("m{k}"), &format!("t{k}"), "mcp__orrery__open_session", json!({}));
    }
    let p = file("cap", &s);
    let pg = page(&p, Agent::Claude, ROOT, None, PAGE, &Fake, false).unwrap();
    let tools = &pg.items[1];
    assert_eq!(tools["items"].as_array().unwrap().len(), TOOLS_MAX);
    assert_eq!(tools["more"], 5);
    assert_eq!(tools["sum"]["other"], 35);
}

#[test]
fn file_tool_targets_stay_relative_when_root_is_under_home() {
    let home = crate::toolsum::host_home().to_owned();
    if home.is_empty() {
        return;
    }
    let sep = if cfg!(windows) { "\\" } else { "/" };
    let root = format!("{home}{sep}proj{sep}api");
    let inside = format!("{root}{sep}src{sep}auth.ts");
    let mut s = prompt("u1", "fix");
    s += &tool("m1", "t1", "Read", json!({ "file_path": inside }));
    s += &tool("m1", "t2", "Edit", json!({ "file_path": inside, "old_string": "a", "new_string": "b" }));
    s += &tool("m1", "t3", "Read", json!({ "file_path": format!("{home}{sep}other{sep}x.ts") }));
    let p = file("home-root", &s);
    let pg = page(&p, Agent::Claude, &root, None, PAGE, &Fake, false).unwrap();
    let tools = pg.items.iter().find(|i| i["k"] == "tools").unwrap();
    assert_eq!(tools["items"][0]["target"], "src/auth.ts");
    assert_eq!(tools["items"][1]["target"], "src/auth.ts");
    let outside = tools["items"][2]["target"].as_str().unwrap();
    assert!(outside.starts_with('~'), "{outside}");
}
