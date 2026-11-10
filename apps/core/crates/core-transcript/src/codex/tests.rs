use super::classify::{classify_line, timestamp_in, LineClass};
use super::extract::{extract_codex_signals, CodexSignal};
use super::meta::{raw_field, read_session_meta};
use super::serve::{records_json, Rollouts};
use super::tail::{Records, RolloutTailer, MAX_LINE};
use super::value::parse_record;
use serde_json::{json, Map, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

const TID: &str = "01a125e6-f212-7711-a885-025fd33ab05a";
const AT: &str = "2026-10-11T07:00:00.000Z";

fn rec(kind: &str, payload: Value) -> String {
    let mut p = payload.as_object().cloned().unwrap_or_default();
    let parts: Vec<String> = p.remove("type").map(|t| format!("\"type\":{t}")).into_iter().chain(p.iter().map(|(k, v)| format!("{}:{v}", Value::from(k.as_str())))).collect();
    format!(r#"{{"timestamp":"{AT}","type":"{kind}","payload":{{{}}}}}"#, parts.join(","))
}

fn meta_line() -> String {
    rec("session_meta", json!({ "id": TID, "session_id": TID, "timestamp": AT, "cwd": "C:\\Temp\\art-hero-x1", "originator": "codex_exec", "cli_version": "0.162.0", "source": "exec", "base_instructions": "secret ".repeat(50) }))
}

fn tokens_line() -> String {
    let usage = json!({ "input_tokens": 18000, "cached_input_tokens": 12000, "output_tokens": 2000, "total_tokens": 20000 });
    rec(
        "event_msg",
        json!({ "type": "token_count", "info": { "total_token_usage": usage, "last_token_usage": usage, "model_context_window": 258400 }, "rate_limits": { "limit_id": "codex", "primary": { "used_percent": 7, "window_minutes": 300, "resets_at": 1_791_650_000 }, "secondary": { "used_percent": 12, "window_minutes": 10080 }, "plan_type": "plus" } }),
    )
}

fn message_line() -> String {
    rec("response_item", json!({ "type": "message", "role": "user", "content": [{ "type": "input_text", "text": "secret prompt ".repeat(40) }] }))
}

fn signals(line: &str) -> Vec<Value> {
    extract_codex_signals(&parse_record(line).unwrap()).iter().map(CodexSignal::to_json).collect()
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-codex-rs-{}-{name}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("rollout-x-{TID}.jsonl"))
}

fn append(file: &PathBuf, text: &str) {
    OpenOptions::new().create(true).append(true).open(file).unwrap().write_all(text.as_bytes()).unwrap();
}

fn kinds(records: &Records) -> Vec<&'static str> {
    records.iter().flatten().map(CodexSignal::kind).collect()
}

#[test]
fn classifies_lines_from_their_head() {
    let head = |s: String| s.as_bytes()[..s.len().min(256)].to_vec();
    assert_eq!(classify_line(&head(meta_line())), LineClass::Parse);
    assert_eq!(classify_line(&head(message_line())), LineClass::Skip);
    assert_eq!(classify_line(&head(rec("world_state", json!({ "full": true })))), LineClass::Skip);
    assert_eq!(classify_line(&head(rec("event_msg", json!({ "type": "item_completed" })))), LineClass::Skip);
    assert_eq!(classify_line(&head(rec("event_msg", json!({ "type": "token_count" })))), LineClass::Parse);
    assert_eq!(classify_line(&head(rec("compacted", json!({ "message": "x" })))), LineClass::Compacted);
    assert_eq!(classify_line(br#"{"something":"else"}"#), LineClass::Parse);
    assert_eq!(classify_line(br#"{"type":"-x","type":"event_msg","payload":{"type":"item"}}"#), LineClass::Skip);
    assert_eq!(timestamp_in(br#"{"timestamp":"short","timestamp":"2026-10-11T07:00:00Z"}"#).as_deref(), Some("2026-10-11T07:00:00Z"));
}

#[test]
fn extracts_only_counts_ids_and_enums() {
    let out = signals(&tokens_line());
    assert_eq!(out[1], json!({ "k": "tokens", "total": { "input": 18000, "cached": 12000, "output": 2000, "total": 20000 }, "last": { "input": 18000, "cached": 12000, "output": 2000, "total": 20000 }, "contextWindow": 258400 }));
    assert_eq!(out[2]["limits"], json!({ "fiveHour": { "usedPct": 7, "resetsAt": "2026-10-10T16:33:20.000Z" }, "sevenDay": { "usedPct": 12 }, "plan": "plus", "at": AT }));
    let meta = signals(&meta_line());
    assert_eq!(meta[1]["meta"], json!({ "id": TID, "createdAt": 1_791_702_000_000i64, "cwd": "C:\\Temp\\art-hero-x1", "originator": "codex_exec", "source": "exec", "version": "0.162.0" }));
    assert!(!format!("{meta:?}").contains("secret"));
    let call = rec("response_item", json!({ "type": "function_call", "call_id": "c1", "namespace": "mcp__x", "name": "run", "arguments": "secret" }));
    assert_eq!(signals(&call)[1], json!({ "k": "tool_call", "id": "c1", "name": "mcp__x.run", "at": AT }));
    let bad = rec("response_item", json!({ "type": "function_call", "call_id": "c1", "name": "bad name" }));
    assert_eq!(signals(&bad).len(), 1);
    let other = rec("event_msg", json!({ "type": "token_count", "rate_limits": { "limit_id": "other", "primary": { "used_percent": 90, "window_minutes": 300 } } }));
    assert_eq!(signals(&other).len(), 1);
}

#[test]
fn finds_the_parent_thread_in_js_key_order() {
    let source = r#"{"b":{"parent_thread_id":"second"},"1":{"parent_thread_id":"first"},"a":{"x":{"y":{"z":{"w":{"parent_thread_id":"deep"}}}}}}"#;
    let line = format!(r#"{{"timestamp":"{AT}","type":"session_meta","payload":{{"id":"{TID}","timestamp":"{AT}","source":{source}}}}}"#);
    let meta = &signals(&line)[1]["meta"];
    assert_eq!(meta["source"], "subagent");
    assert_eq!(meta["parentThreadId"], "first");
}

#[test]
fn tails_partial_lines_skips_content_and_counts_compactions_once() {
    let file = temp("tail");
    let _ = std::fs::remove_file(&file);
    append(&file, &format!("{}\n{}\n{}\n", meta_line(), rec("event_msg", json!({ "type": "task_started", "model_context_window": 258400 })), message_line()));
    let mut t = RolloutTailer::new(file.clone(), None, 4096, MAX_LINE);
    let mut out = Records::new();
    t.read_all(&mut out);
    assert_eq!(kinds(&out), ["seen", "meta", "seen", "turn_start"]);
    let big = rec("compacted", json!({ "message": "z".repeat(300_000) }));
    let tokens = tokens_line();
    append(&file, &big[..1000]);
    out.clear();
    t.read_all(&mut out);
    assert_eq!(kinds(&out), ["seen", "compact"]);
    append(&file, &format!("{}\n{}", &big[1000..], &tokens[..40]));
    out.clear();
    t.read_all(&mut out);
    assert!(out.is_empty());
    assert_eq!(t.resume_point().unwrap().0, t.offset() - 40);
    append(&file, &format!("{}\n", &tokens[40..]));
    out.clear();
    t.read_all(&mut out);
    assert_eq!(kinds(&out), ["seen", "tokens", "limits"]);
    assert!(!records_json(&out).to_string().contains("secret"));
    std::fs::write(&file, format!("{}\n", tokens_line())).unwrap();
    out.clear();
    t.read_all(&mut out);
    assert_eq!(kinds(&out), ["seen", "tokens", "limits"]);
}

#[test]
fn starts_near_the_end_and_pages_add_up() {
    let file = temp("end");
    let body = format!("{}\n{}{}\n", meta_line(), format!("{}\n", message_line()).repeat(200), tokens_line());
    std::fs::write(&file, &body).unwrap();
    let mut t = RolloutTailer::new(file.clone(), Some(tokens_line().len() as u64 + 50), 4096, MAX_LINE);
    let mut out = Records::new();
    t.read_all(&mut out);
    assert_eq!(kinds(&out), ["seen", "tokens", "limits"]);
    let mut whole = Records::new();
    RolloutTailer::new(file.clone(), None, 4096, MAX_LINE).read_all(&mut whole);
    let mut paged = Records::new();
    let mut p = RolloutTailer::new(file.clone(), None, 4096, MAX_LINE);
    let mut pages = 1;
    while p.read_some(3000, &mut paged) {
        pages += 1;
    }
    assert!(pages > 10);
    assert_eq!(records_json(&whole), records_json(&paged));
}

#[test]
fn reads_the_meta_line_also_without_a_newline() {
    let file = temp("meta");
    std::fs::write(&file, format!("{}\n", meta_line())).unwrap();
    let full = read_session_meta(&file).unwrap();
    assert_eq!(full.cwd.as_deref(), Some("C:\\Temp\\art-hero-x1"));
    std::fs::write(&file, meta_line()).unwrap();
    assert_eq!(read_session_meta(&file), Some(full));
    assert_eq!(raw_field(r#""cwd":"a\"b","cwd":"c""#, "cwd"), Some(r#"a\"b"#));
}

#[test]
fn serves_open_read_close() {
    let file = temp("serve");
    std::fs::write(&file, format!("{}\n{}\n", meta_line(), tokens_line())).unwrap();
    let mut r = Rollouts::default();
    let m = |v: Value| -> Map<String, Value> { v.as_object().unwrap().clone() };
    let h = r.handle("codex.open", &m(json!({ "file": file }))).unwrap().unwrap()["handle"].clone();
    let read = r.handle("codex.read", &m(json!({ "handle": h, "maxBytes": 100 }))).unwrap().unwrap();
    assert_eq!(read["more"], true);
    assert!(read["records"].as_array().unwrap().is_empty());
    let read = r.handle("codex.read", &m(json!({ "handle": h }))).unwrap().unwrap();
    assert_eq!(read["records"].as_array().unwrap().len(), 2);
    assert_eq!(read["resumeAt"], read["offset"]);
    assert_eq!(r.handle("codex.meta", &m(json!({ "file": file }))).unwrap().unwrap()["meta"]["id"], TID);
    assert!(r.handle("codex.close", &m(json!({ "handle": h }))).unwrap().is_ok());
    assert!(r.is_empty());
    assert_eq!(r.handle("codex.read", &m(json!({ "handle": h }))).unwrap().unwrap_err(), "handle");
    assert_eq!(r.handle("codex.open", &m(json!({}))).unwrap().unwrap_err(), "file");
}
