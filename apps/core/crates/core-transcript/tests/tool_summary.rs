use core_transcript::codex::extract::extract_codex_signals;
use core_transcript::codex::value::parse_record;
use core_transcript::extract::extract_signals;
use core_transcript::state::TranscriptState;
use core_transcript::toolsum::{codex_tool_summary_of, sanitize_tool_summary, tool_summary_of};
use serde_json::{json, Value};
use std::path::PathBuf;

fn cases() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../packages/shared/test/toolSummary.cases.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("cases")
}

fn opt(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

#[test]
fn sanitize_matches_the_shared_case_table() {
    let c = cases();
    let (home, platform) = (c["home"].as_str().unwrap(), c["platform"].as_str().unwrap());
    let rows = c["sanitize"].as_array().unwrap();
    assert!(rows.len() >= 20);
    for (i, row) in rows.iter().enumerate() {
        let h = row["home"].as_str().unwrap_or(home);
        let p = row["platform"].as_str().unwrap_or(platform);
        assert_eq!(sanitize_tool_summary(row["in"].as_str().unwrap(), h, p), opt(&row["out"]), "case {i}");
    }
}

#[test]
fn claude_inputs_match_the_shared_case_table() {
    let c = cases();
    let (home, platform) = (c["home"].as_str().unwrap(), c["platform"].as_str().unwrap());
    for row in c["claude"].as_array().unwrap() {
        let name = row[0].as_str().unwrap();
        assert_eq!(tool_summary_of(name, &row[1], home, platform), opt(&row[2]), "{name}");
    }
}

#[test]
fn codex_calls_match_the_shared_case_table() {
    let c = cases();
    let (home, platform) = (c["home"].as_str().unwrap(), c["platform"].as_str().unwrap());
    for row in c["codex"].as_array().unwrap() {
        let (kind, name, ns) = (row[0].as_str().unwrap(), row[1].as_str().unwrap(), row[2].as_bool().unwrap());
        let rec = parse_record(&json!({ "type": "response_item", "payload": row[3] }).to_string()).expect("record");
        let payload = rec.get("payload").expect("payload");
        assert_eq!(codex_tool_summary_of(kind, name, ns, payload, home, platform), opt(&row[4]), "{kind} {name}");
    }
}

fn at(s: u32) -> String {
    format!("2026-11-10T10:00:0{s}.000Z")
}

fn tool_use(id: &str, name: &str, input: Value, t: &str) -> Value {
    json!({ "type": "assistant", "timestamp": t, "message": { "id": format!("m-{id}"), "content": [{ "type": "tool_use", "id": id, "name": name, "input": input }] } })
}

fn tool_result(id: &str, t: &str) -> Value {
    json!({ "type": "user", "timestamp": t, "message": { "content": [{ "type": "tool_result", "tool_use_id": id, "content": "ok" }] } })
}

fn current(s: &TranscriptState) -> (Option<&str>, Option<&str>, Option<&str>) {
    (s.current_tool(), s.current_tool_summary(), s.current_tool_started_at())
}

#[test]
fn state_follows_the_newest_open_call_and_clears_with_it() {
    let mut s = TranscriptState::default();
    let (t1, t2) = (at(1), at(2));
    s.apply(&extract_signals(&tool_use("t1", "Bash", json!({ "command": "npm test -- --password hunter22" }), &t1)));
    assert_eq!(current(&s), (Some("Bash"), Some("npm test -- --password [masked]"), Some(t1.as_str())));
    s.apply(&extract_signals(&tool_use("t2", "mcp__x__y", json!({ "q": "secret" }), &t2)));
    assert_eq!(current(&s), (Some("mcp__x__y"), None, Some(t2.as_str())));
    let j = s.to_json();
    assert_eq!((j["currentTool"].clone(), j.get("currentToolSummary"), j["currentToolStartedAt"].clone()), (json!("mcp__x__y"), None, json!(t2)));
    s.apply(&extract_signals(&tool_result("t2", &at(3))));
    assert_eq!(current(&s), (Some("Bash"), Some("npm test -- --password [masked]"), Some(t1.as_str())));
    s.apply(&extract_signals(&tool_result("t1", &at(4))));
    assert_eq!(current(&s), (None, None, None));
    assert!(s.to_json().get("currentToolSummary").is_none());
}

#[test]
fn signal_carries_only_the_summary() {
    let sig = extract_signals(&tool_use("t1", "Edit", json!({ "file_path": "/tmp/a.ts", "old_string": "token=abcd1234", "new_string": "x" }), &at(1)));
    let text = Value::from(sig.iter().map(|s| s.to_json()).collect::<Vec<_>>()).to_string();
    assert!(text.contains("\"summary\":\"/tmp/a.ts\""));
    assert!(!text.contains("abcd1234"));
}

#[test]
fn codex_tool_call_signal_carries_the_command() {
    let line = json!({ "timestamp": at(1), "type": "response_item", "payload": { "type": "function_call", "call_id": "c1", "name": "exec_command", "arguments": json!({ "cmd": "cargo test -j 4" }).to_string() } });
    let sig = extract_codex_signals(&parse_record(&line.to_string()).expect("record"));
    let call = sig.iter().map(|s| s.to_json()).find(|j| j["k"] == "tool_call").expect("call");
    assert_eq!(call, json!({ "k": "tool_call", "id": "c1", "name": "exec_command", "summary": "cargo test -j 4", "at": at(1) }));
}
