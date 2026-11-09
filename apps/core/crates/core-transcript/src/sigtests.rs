use crate::extract::{extract_signals, mentions_codex, mentions_device};
use crate::rules::{command_hits, touch_of};
use crate::sigserve::Transcript;
use crate::tail::{lite_filter, signal_line_filter, tool_use_ids_of, Item, JsonlTailer, TailOptions, Verdict};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-core-signals-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("t.jsonl")
}

fn append(file: &PathBuf, text: &str) {
    std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap().write_all(text.as_bytes()).unwrap();
}

fn verdict(line: &str) -> String {
    match signal_line_filter(line.as_bytes()) {
        Verdict::Parse => "parse".into(),
        Verdict::Drop => "drop".into(),
        Verdict::Branch(b) => format!("branch:{b}"),
    }
}

fn kinds(rec: Value) -> Vec<Value> {
    extract_signals(&rec).iter().map(|s| s.to_json()).collect()
}

#[test]
fn line_filter_matches_raw_keys_like_node() {
    assert_eq!(verdict(r#"{"type":"user","message":{}}"#), "parse");
    assert_eq!(verdict(r#"{"x":1,"type":"user""#), "parse");
    assert_eq!(verdict(r#"{"type":"attachment","attachment":{"type":"ab"}}"#), "drop");
    assert_eq!(verdict(r#"{"type":"attachment","gitBranch":"main"}"#), "branch:main");
    assert_eq!(verdict(r#"{"type":"attachment","attachment":{"type":"queued_command","origin":{"kind":"peer"}},"gitBranch":"main"}"#), "parse");
    assert_eq!(verdict(r#"{"type":"attachment","attachment":{"type":"queued_command","origin":{"kind":"human"}},"gitBranch":"main"}"#), "branch:main");
    assert_eq!(verdict(r#"{"type":"ai-title","gitBranch":"we\"ird"}"#), "parse");
    assert_eq!(verdict(&format!(r#"{{"type":"ai-title","gitBranch":"{}"}}"#, "b".repeat(201))), "parse");
    assert_eq!(verdict(r#"{"content":"escaped {\"type\":\"user\"}"}"#), "drop");
}

#[test]
fn lite_filter_keeps_notifications_and_agent_results() {
    let ids: HashSet<String> = ["toolu_a".to_owned()].into();
    let user = br#"{"type":"user","message":{"content":[{"tool_use_id":"toolu_x","type":"tool_result"}]}}"#;
    let mine = br#"{"type":"user","message":{"content":[{"tool_use_id":"toolu_a","type":"tool_result"}]}}"#;
    let size = 10 * 1024 * 1024;
    assert!(matches!(lite_filter(user, 10, size, &ids), Verdict::Drop));
    assert!(matches!(lite_filter(mine, 10, size, &ids), Verdict::Parse));
    assert!(matches!(lite_filter(user, size - 5, size, &ids), Verdict::Parse));
    assert_eq!(tool_use_ids_of(mine), vec!["toolu_a".to_owned()]);
}

#[test]
fn signals_keep_only_whitelisted_fields() {
    let secret = "sk-ant-api03-SECRET";
    let rec = json!({ "type": "assistant", "timestamp": "2026-10-07T10:00:00.000Z", "effort": "BAD VALUE", "gitBranch": "x".repeat(300),
        "message": { "id": "m1", "model": "claude-opus-5-5", "stop_reason": "end_turn", "usage": { "input_tokens": 1, "cache_creation_input_tokens": 2, "output_tokens": 3, "cache_read_input_tokens": -4 },
        "content": [{ "type": "thinking", "thinking": secret }, { "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": format!("codex exec {secret} && git push -f && adb install x") } }, { "type": "text", "text": format!("  {}  ", "r".repeat(4000)) }] } });
    let out = kinds(rec);
    let text = serde_json::to_string(&out).unwrap();
    assert!(!text.contains("SECRET"));
    assert_eq!(out[0]["branch"].as_str().unwrap().len(), 120);
    assert_eq!(out[2], json!({ "k": "model", "model": "claude-opus-5-5" }));
    assert_eq!(out[3], json!({ "k": "usage", "messageId": "m1", "input": 3, "output": 3, "cacheRead": 0, "at": "2026-10-07T10:00:00.000Z" }));
    assert_eq!(out[4], json!({ "k": "tool_use", "id": "t1", "name": "Bash", "at": "2026-10-07T10:00:00.000Z", "codex": true, "device": true, "touch": { "cmds": [{ "rule": "git-force-push" }] } }));
    assert_eq!(out[5]["text"].as_str().unwrap().len(), 3000);
    assert_eq!(out[6], json!({ "k": "assistant_end", "messageId": "m1", "at": "2026-10-07T10:00:00.000Z" }));
}

#[test]
fn shell_rules_follow_the_node_rules() {
    let rules = |c: &str| command_hits(c, 0).iter().map(|h| (h.rule, h.targets.clone())).collect::<Vec<_>>();
    assert_eq!(rules("rm -rf ./build /tmp/x"), vec![("rm-rf", Some(vec!["./build".to_owned(), "/tmp/x".to_owned()]))]);
    assert_eq!(rules("Remove-Item -Recurse -Force -Path a,b"), vec![("remove-item-recurse", Some(vec!["a".to_owned(), "b".to_owned()]))]);
    assert_eq!(rules("bash -c 'git reset --hard'"), vec![("git-reset-hard", None)]);
    assert_eq!(rules("git -C repo clean -fd"), vec![("git-clean", None)]);
    assert_eq!(rules("C:\\tools\\RD.EXE /s /q out"), vec![("del-s", Some(vec!["out".to_owned()]))]);
    assert_eq!(rules("psql -c \"DROP  table users\""), vec![("drop-table", None)]);
    assert_eq!(rules("adb shell pm uninstall pkg"), vec![("adb-uninstall", None)]);
    assert!(rules("git push origin main; ls -R").is_empty());
    assert_eq!(touch_of("Edit", &json!({ "file_path": "a.ts" })).unwrap().to_json(), json!({ "file": "a.ts" }));
    assert!(touch_of("Edit", &json!({ "file_path": "" })).is_none());
    assert!(touch_of("toString", &json!({ "command": "rm -rf /" })).is_none());
    assert!(mentions_codex("run CODEX now") && !mentions_codex("codexx"));
    assert!(mentions_device("flutter\t run -d x") && mentions_device("gradlew installFreeDebug") && !mentions_device("flutter-run") && !mentions_device("installDebugX"));
}

#[test]
fn tailer_handles_partial_lines_truncation_and_appends() {
    let file = tmp("tail");
    append(&file, "{\"a\":1}\n{\"a\":2}\n{\"a\":");
    let mut t = JsonlTailer::new(file.clone(), 5, TailOptions::default());
    let mut got = Vec::new();
    let mut parse = |_: &[u8], _: u64, _: u64| Verdict::Parse;
    let take = |got: &mut Vec<Value>, t: &mut JsonlTailer, parse: &mut dyn FnMut(&[u8], u64, u64) -> Verdict| {
        t.read_each(parse, &mut |i| {
            if let Item::Record(v) = i {
                got.push(v);
            }
        })
    };
    take(&mut got, &mut t, &mut parse);
    assert_eq!(got, vec![json!({ "a": 1 }), json!({ "a": 2 })]);
    append(&file, "3}\n not json\n\n\u{feff}{\"a\":4}\r\n");
    got.clear();
    take(&mut got, &mut t, &mut parse);
    assert_eq!(got, vec![json!({ "a": 3 }), json!({ "a": 4 })]);
    std::fs::write(&file, "{\"b\":1}\n").unwrap();
    got.clear();
    take(&mut got, &mut t, &mut parse);
    assert_eq!(got, vec![json!({ "b": 1 })]);
    let mut ends = Vec::new();
    let mut t2 = JsonlTailer::new(file.clone(), 3, TailOptions { from_end: Some(4), max_line: Some(100) });
    append(&file, "{\"c\":22}\n{\"d\":1}\n");
    t2.read_each(&mut |_, end, _| {
        ends.push(end);
        Verdict::Parse
    }, &mut |_| {});
    assert_eq!(ends, Vec::<u64>::new());
    let size = std::fs::metadata(&file).unwrap().len();
    let mut t3 = JsonlTailer::new(file.clone(), 7, TailOptions { from_end: Some(10), max_line: None });
    t3.read_each(&mut |_, end, _| {
        ends.push(end);
        Verdict::Parse
    }, &mut |_| {});
    assert_eq!(ends, vec![size]);
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn transcript_reads_signals_state_and_spans() {
    let file = tmp("transcript");
    append(&file, &format!("{}\n", json!({ "type": "assistant", "timestamp": "2026-10-07T10:00:00.000Z", "message": { "id": "m1", "content": [{ "type": "tool_use", "id": "t1", "name": "Read", "input": {} }] } })));
    let mut t = Transcript::new(file.clone(), None, Some(("term".into(), "main".into())), 1024, TailOptions::default());
    let (signals, spans) = t.read(0);
    assert_eq!(signals.len(), 2);
    assert_eq!(spans.len(), 1);
    assert_eq!(t.state.to_json()["currentTool"], "Read");
    append(&file, &format!("{}\n", json!({ "type": "user", "timestamp": "2026-10-07T10:00:01.000Z", "message": { "content": [{ "type": "tool_result", "tool_use_id": "t1", "is_error": true, "content": "listen EADDRINUSE :::3000" }] } })));
    let (signals, spans) = t.read(0);
    assert_eq!(signals[1].to_json()["portConflict"], 3000);
    assert_eq!(spans[0]["outcome"], "error");
    assert_eq!(t.state.to_json()["portConflict"], json!({ "port": 3000, "at": "2026-10-07T10:00:01.000Z" }));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
#[ignore]
fn measure_signals_over_a_dir() {
    let Ok(root) = std::env::var("CCM_T2_BENCH") else { return };
    let mut files = Vec::new();
    let mut dirs = vec![PathBuf::from(root)];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "jsonl") {
                files.push(p);
            }
        }
    }
    let t0 = std::time::Instant::now();
    let mut all = Vec::new();
    for f in &files {
        let mut t = Transcript::new(f.clone(), None, None, crate::tail::CHUNK_BYTES, TailOptions::default());
        all.push(t.read(0).0);
    }
    let read = t0.elapsed();
    let t1 = std::time::Instant::now();
    let bytes: usize = all.iter().map(|s| serde_json::to_string(&s.iter().map(|x| x.to_json()).collect::<Vec<_>>()).unwrap().len()).sum();
    eprintln!("{} files, {} signals, read+parse {:?}, reply json {:?} ({} bytes)", files.len(), all.iter().map(Vec::len).sum::<usize>(), read, t1.elapsed(), bytes);
}

#[test]
fn budgeted_reads_add_up_to_one_full_read() {
    let file = tmp("budget");
    let mut body = String::new();
    for i in 0..200 {
        body.push_str(&format!("{}\n", json!({ "type": "assistant", "timestamp": "2026-10-07T10:00:00.000Z", "message": { "id": format!("m{i}"), "content": [{ "type": "tool_use", "id": format!("t{i}"), "name": "Read", "input": {} }], "usage": { "output_tokens": i } } })));
    }
    append(&file, &body);
    let mut full = Transcript::new(file.clone(), None, Some(("t".into(), "a".into())), 64, TailOptions::default());
    let (all, spans) = full.read(0);
    let mut part = Transcript::new(file.clone(), None, Some(("t".into(), "a".into())), 64, TailOptions::default());
    let (mut got, mut got_spans, mut rounds) = (Vec::new(), Vec::new(), 0);
    loop {
        let (s, sp, more) = part.read_some(0, 1000);
        got.extend(s);
        got_spans.extend(sp);
        rounds += 1;
        if !more {
            break;
        }
    }
    assert!(rounds > 10);
    assert_eq!(got, all);
    assert_eq!(got_spans, spans);
    assert_eq!(part.state.to_json(), full.state.to_json());
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
