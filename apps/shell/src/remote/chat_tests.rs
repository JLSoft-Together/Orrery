use std::collections::HashMap;
use std::path::PathBuf;

use core_transcript::chat::{Agent, Clean, EVENT_BYTES, WATCH_MAX};
use serde_json::{json, Value};

use super::chat::{parse_ask, ChatState, Watch};
use super::chat_src::{block_text, find_rollout, gen_name, items_events, last_event, mask_menus, page_of, remember, target_of, Ask, Hub, Roots, ShellClean, Target};
use super::test_util::temp_dir;

fn summary() -> Value {
    json!({ "rev": 1, "tabs": [{ "key": "t1", "panes": [
        { "key": "p1", "kind": "term", "agent": "claude", "terminalId": "123:1", "cwd": "C:\\work\\api" },
        { "key": "p2", "kind": "term", "agent": "shell", "terminalId": "9:1" },
        { "key": "p3", "kind": "file", "agent": "claude" },
        { "key": "p4", "kind": "term", "agent": "codex", "terminalId": "codex:0199aaaa-bbbb-7ccc-8ddd-eeeeffff0000" }
    ] }] })
}

#[test]
fn only_agent_terminal_panes_have_a_chat() {
    let s = summary();
    assert_eq!(target_of(&s, "p1"), Some(Target { agent: Agent::Claude, terminal: Some("123:1".into()), cwd: "C:\\work\\api".into() }));
    assert_eq!(target_of(&s, "p2"), None);
    assert_eq!(target_of(&s, "p3"), None);
    assert_eq!(target_of(&s, "nope"), None);
    assert_eq!(target_of(&s, "p4").map(|t| t.agent), Some(Agent::Codex));
}

#[test]
fn shell_clean_masks_and_keeps_markdown_lines() {
    let (text, cut) = ShellClean.block("line one\n\tAPI_KEY=abc123 \u{202E}evil\r\n```\ncode\n```", 4000);
    assert!(!cut);
    assert!(text.contains("API_KEY=••••"), "{text}");
    assert!(text.contains("line one\n\t"));
    assert!(!text.contains('\u{202E}') && !text.contains('\r'));
    let (short, cut) = block_text(&"x".repeat(50), &|s: &str| s.to_owned(), 10);
    assert!(cut);
    assert_eq!(short.chars().count(), 10);
    assert_eq!(ShellClean.line("a\n  b\tsend Bearer abcdefghijkl", 120), "a b send Bearer ••••");
}

fn write_claude(projects: &PathBuf, cwd: &str, sid: &str, body: &str) -> PathBuf {
    let dir = projects.join(core_transcript::tracker::cwd_slug(cwd));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{sid}.jsonl"));
    std::fs::write(&p, body).unwrap();
    p
}

fn transcript() -> String {
    let lines = [
        json!({ "type": "user", "timestamp": "2026-11-20T01:00:00.000Z", "message": { "role": "user", "content": "deploy with PASSWORD=hunter22 please" } }),
        json!({ "type": "assistant", "timestamp": "2026-11-20T01:00:01.000Z", "message": { "id": "m1", "content": [{ "type": "thinking", "thinking": "secret plan" }, { "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": "curl -H 'Authorization: Bearer abcdefghijklmnop' https://x.io" } }] } }),
        json!({ "type": "user", "timestamp": "2026-11-20T01:00:02.000Z", "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "RESULT BODY" }] } }),
        json!({ "type": "assistant", "timestamp": "2026-11-20T01:00:03.000Z", "message": { "id": "m2", "content": [{ "type": "text", "text": "Deployed.\n\nUse token=zzzzzzzz to log in." }], "stop_reason": "end_turn" } }),
    ];
    lines.iter().map(|l| format!("{l}\n")).collect()
}

#[test]
fn claude_page_is_masked_and_gen_follows_truncation() {
    let root = temp_dir("chat-claude");
    let roots = Roots { projects: root.join("projects"), sessions: root.join("sessions") };
    let sid = "11111111-2222-3333-4444-555555555555";
    let cwd = "C:\\work\\api";
    let path = write_claude(&roots.projects, cwd, sid, &transcript());
    let hub = Hub::default();
    let target = target_of(&summary(), "p1").unwrap();
    let term = json!({ "claudeSessionId": sid, "cwd": cwd });
    let src = hub.source_of(&target, Some(&term), &roots).expect("source");
    assert_eq!(src.path, path);
    let ask = Ask { pane: "p1", target: &target, gen: None, before: None, limit: 30, live: true };
    let paged = page_of(&hub, &ask, Some(src.clone())).unwrap();
    let text = paged.reply.to_string();
    for bad in ["hunter22", "abcdefghijklmnop", "secret plan", "RESULT BODY", "zzzzzzzz"] {
        assert!(!text.contains(bad), "{bad}: {text}");
    }
    assert_eq!(paged.reply["gen"], gen_name(sid, 0));
    assert_eq!(paged.reply["agent"], "claude");
    assert_eq!(paged.reply["older"], Value::Null);
    assert_eq!(paged.reply["items"].as_array().unwrap().len(), 3);
    assert!(paged.live.is_some());
    let last = hub.last(&src, Agent::Claude).unwrap();
    assert_eq!(last.0, "Use token=•••• to log in.");
    std::fs::write(&path, "").unwrap();
    assert_eq!(hub.gen(&src), gen_name(sid, 1));
    let stale = Ask { pane: "p1", target: &target, gen: Some("old:0"), before: Some(5), limit: 30, live: false };
    let fresh = page_of(&hub, &stale, Some(src)).unwrap();
    assert_eq!(fresh.reply["gen"], gen_name(sid, 1));
    assert_eq!(fresh.reply["items"], json!([]));
}

#[test]
fn a_pane_without_transcript_answers_empty() {
    let hub = Hub::default();
    let target = target_of(&summary(), "p1").unwrap();
    let ask = Ask { pane: "p1", target: &target, gen: None, before: None, limit: 30, live: true };
    let paged = page_of(&hub, &ask, None).unwrap();
    assert_eq!(paged.reply, json!({ "ok": true, "paneKey": "p1", "agent": "claude", "gen": "", "items": [], "older": null }));
    let roots = Roots { projects: temp_dir("chat-none"), sessions: temp_dir("chat-none-s") };
    assert!(hub.source_of(&target, Some(&json!({ "claudeSessionId": "../../etc", "cwd": "x" })), &roots).is_none());
    assert!(hub.source_of(&target, None, &roots).is_none());
}

#[test]
fn codex_rollout_is_found_by_thread_id() {
    let sessions = temp_dir("chat-codex");
    let id = "0199aaaa-bbbb-7ccc-8ddd-eeeeffff0000";
    let day = sessions.join("2026").join("11").join("19");
    std::fs::create_dir_all(&day).unwrap();
    let file = day.join(format!("rollout-2026-11-19T10-00-00-{id}.jsonl"));
    std::fs::write(&file, "").unwrap();
    assert_eq!(find_rollout(&sessions, id), Some(file.clone()));
    let roots = Roots { projects: temp_dir("chat-codex-p"), sessions };
    let target = target_of(&summary(), "p4").unwrap();
    let src = Hub::default().source_of(&target, None, &roots).unwrap();
    assert_eq!(src.path, file);
    assert_eq!(src.id, id);
}

fn watch(pane: &str) -> Watch {
    Watch { pane: pane.into(), target: Target { agent: Agent::Claude, terminal: None, cwd: String::new() }, gen: String::new(), source: None, live: None }
}

#[test]
fn a_third_watch_is_refused_and_rewatch_replaces() {
    let mut st = ChatState::default();
    st.put_watch(watch("a"));
    st.put_watch(watch("b"));
    assert_eq!(st.watches.len(), WATCH_MAX);
    assert!(!st.can_watch("c"));
    assert!(st.can_watch("a"));
    st.put_watch(watch("a"));
    assert_eq!(st.watches.len(), 2);
    st.unwatch("a");
    assert!(st.can_watch("c"));
}

#[tokio::test]
async fn no_chat_cap_means_no_chat_last_work() {
    let mut st = ChatState::default();
    st.full_last();
    st.nudge(true);
    assert!(st.due.is_none());
    st.cap = true;
    st.nudge(false);
    assert!(st.due.is_none());
    st.nudge(true);
    assert!(st.due.is_some());
}

#[test]
fn ask_fields_are_validated() {
    assert_eq!(parse_ask("chat.page", &json!({ "paneKey": "p", "before": 10, "limit": 5, "gen": "g:0" })), Ok(("p".into(), Some("g:0".into()), Some(10), 5)));
    assert_eq!(parse_ask("chat.watch", &json!({ "paneKey": "p" })), Ok(("p".into(), None, None, 30)));
    for bad in [json!({}), json!({ "paneKey": "p", "limit": 0 }), json!({ "paneKey": "p", "limit": 51 }), json!({ "paneKey": "p", "before": -1 }), json!({ "paneKey": "p", "before": 1.5 }), json!({ "paneKey": "" })] {
        assert_eq!(parse_ask("chat.page", &bad), Err("invalid"), "{bad}");
    }
}

#[test]
fn items_events_stay_under_the_event_cap() {
    let items: Vec<Value> = (0..20).map(|i| json!({ "id": format!("a:{i}"), "k": "ai", "seq": i, "rev": 0, "text": "é".repeat(7000) })).collect();
    let huge = json!({ "id": "a:big", "k": "ai", "seq": 99, "rev": 0, "text": "x".repeat(200_000) });
    let mut all = items;
    all.push(huge);
    let events = items_events("p1", "g:0", all);
    assert!(events.len() > 1);
    let mut n = 0;
    for e in &events {
        assert!(serde_json::to_vec(e).unwrap().len() <= EVENT_BYTES, "event too big");
        assert_eq!(e["t"], "chat.items");
        n += e["items"].as_array().unwrap().len();
    }
    assert_eq!(n, 21);
    assert_eq!(events.last().unwrap()["items"].as_array().unwrap().last().unwrap()["cut"], true);
}

#[test]
fn chat_last_sends_full_then_only_changes() {
    let rows = vec![("p1".to_owned(), Some("done".to_owned()), Some(5)), ("p4".to_owned(), None, None)];
    let full = last_event(true, &rows, &HashMap::new()).unwrap();
    assert_eq!(full, json!({ "t": "chat.last", "full": true, "items": [{ "paneKey": "p1", "text": "done", "at": 5 }] }));
    let sent = remember(&rows);
    assert!(last_event(false, &rows, &sent).is_none());
    let next = vec![("p1".to_owned(), None, None), ("p4".to_owned(), Some("new".to_owned()), Some(7))];
    let diff = last_event(false, &next, &sent).unwrap();
    assert_eq!(diff, json!({ "t": "chat.last", "items": [{ "paneKey": "p1", "text": null }, { "paneKey": "p4", "text": "new", "at": 7 }] }));
    let gone = last_event(false, &[], &sent).unwrap();
    assert_eq!(gone["items"], json!([{ "paneKey": "p1", "text": null }]));
}

#[test]
fn menus_are_masked_capped_or_dropped_before_publish() {
    let items: Vec<Value> = (1..=15).map(|i| json!({ "index": i, "label": format!("Run npm test --token=abc{i}\u{202E}") })).collect();
    let mut s = json!({ "rev": 2, "tabs": [{ "key": "t", "panes": [
        { "key": "a", "menu": { "sig": "1a2b3c4d", "kind": "permission", "agent": "claude", "question": "Allow  SECRET_KEY=xyz?", "items": items, "numbered": true, "confident": true, "since": 1 } },
        { "key": "b", "menu": { "sig": "bad sig!", "kind": "ask", "items": [{ "index": 1, "label": "x" }] } },
        { "key": "c" }
    ] }] });
    mask_menus(&mut s);
    let a = &s["tabs"][0]["panes"][0]["menu"];
    assert_eq!(a["items"].as_array().unwrap().len(), 12);
    assert_eq!(a["question"], "Allow SECRET_KEY=••••");
    let label = a["items"][0]["label"].as_str().unwrap();
    assert!(label.contains("--token=••••") && !label.contains('\u{202E}'), "{label}");
    assert!(s["tabs"][0]["panes"][1].get("menu").is_none());
    assert!(s["tabs"][0]["panes"][2].get("menu").is_none());
}
