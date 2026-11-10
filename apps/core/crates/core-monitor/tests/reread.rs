use std::io::Write;
use std::path::{Path, PathBuf};

use core_monitor::hints::ContextHints;
use core_monitor::registry::RegistryRecord;
use core_monitor::tracker::{MonTracker, SyncScope};
use core_store::extra::ToolOutcome;
use core_store::model::AgentStatus;
use serde_json::json;

const SID: &str = "11111111-2222-3333-4444-555555555555";
const NOW: i64 = 1_791_367_000_000;

fn append(file: &Path, v: serde_json::Value) {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap();
    writeln!(f, "{v}").unwrap();
}

fn fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("ccm-reread-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("projects").join("D--w");
    let subs = project.join(SID).join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    let main = project.join(format!("{SID}.jsonl"));
    append(&main, json!({"type":"assistant","timestamp":"2026-10-07T09:59:00.000Z","message":{"id":"m1","model":"claude-opus-5-5","stop_reason":"tool_use","usage":{"input_tokens":1,"output_tokens":1},"content":[{"type":"tool_use","id":"toolu_agent","name":"Agent","input":{"prompt":"p"}}]}}));
    append(&main, json!({"type":"user","timestamp":"2026-10-07T09:59:01.000Z","toolUseResult":{"status":"async_launched"},"message":{"content":[{"type":"tool_result","tool_use_id":"toolu_agent"}]}}));
    std::fs::write(subs.join("agent-a1.meta.json"), r#"{"agentType":"Explore","toolUseId":"toolu_agent"}"#).unwrap();
    let sub = subs.join("agent-a1.jsonl");
    append(&sub, json!({"type":"assistant","isSidechain":true,"timestamp":"2026-10-07T09:59:03.000Z","message":{"id":"s1","stop_reason":"tool_use","usage":{"input_tokens":7,"output_tokens":2},"content":[{"type":"tool_use","id":"toolu_grep","name":"Grep"}]}}));
    (root, main, sub)
}

fn run(reread: bool) -> (Option<ToolOutcome>, AgentStatus, Option<u64>) {
    let (root, main, sub) = fixture(if reread { "on" } else { "off" });
    let record = RegistryRecord { pid: 4242, session_id: Some(SID.into()), cwd: Some("D:\\w".into()), status: Some("busy".into()), ..Default::default() };
    let hints = ContextHints::new(None);
    let mut t = MonTracker::new("4242:1", record, root.join("projects"), None, reread);
    t.sync(true, &SyncScope::All, NOW, None);
    append(&sub, json!({"type":"user","isSidechain":true,"timestamp":"2026-10-07T10:00:01.000Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_grep","content":"x"}]}}));
    append(&sub, json!({"type":"assistant","isSidechain":true,"timestamp":"2026-10-07T10:00:02.000Z","message":{"id":"s2","stop_reason":"end_turn","usage":{"input_tokens":30,"output_tokens":4},"content":[{"type":"text","text":"done"}]}}));
    append(&main, json!({"type":"queue-operation","operation":"enqueue","timestamp":"2026-10-07T10:00:05.000Z","content":"<task-notification><task-id>a1</task-id><tool-use-id>toolu_agent</tool-use-id><status>completed</status><summary>s</summary></task-notification>"}));
    t.sync(false, &SyncScope::Files(vec![main.to_string_lossy().into_owned()]), NOW, None);
    let span = t.tool_spans(NOW, None).into_iter().find(|s| s.tool == "Grep").and_then(|s| s.outcome);
    let terminal = t.build_terminal(&hints);
    let agent = t.build_agents(&hints, &terminal).into_iter().find(|a| a.id == "a1").unwrap();
    let _ = std::fs::remove_dir_all(&root);
    (span, agent.status, agent.input_tokens)
}

#[test]
fn rereads_a_finished_subagent_before_resolving_it() {
    assert_eq!(run(true), (Some(ToolOutcome::Ok), AgentStatus::Completed, Some(37)));
}

#[test]
fn without_the_reread_the_last_lines_of_a_finished_subagent_are_missed() {
    let (span, status, tokens) = run(false);
    assert_eq!(span, Some(ToolOutcome::Interrupted));
    assert_eq!(status, AgentStatus::Completed);
    assert_eq!(tokens, Some(7));
}
