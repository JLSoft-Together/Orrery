use crate::index::{Options, UsageIndex};
use crate::jobs::{drain, full_scan, Shared, Sink};
use crate::record::{clean_name, mcp_server_of, parse_line, tool_calls_of, usage_record_of};
use crate::time::local_day;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const TS: &str = "2026-10-07T09:00:00.000Z";
const CWD: &str = "D:\\work\\demo";

fn assistant(id: &str, usage: Value, model: &str) -> String {
    format!("{}\n", json!({ "type": "assistant", "timestamp": TS, "cwd": CWD, "message": { "id": id, "model": model, "content": [{ "type": "text", "text": "secret" }], "usage": usage } }))
}

fn tool_use(mid: &str, id: &str, name: &str, input: Value) -> String {
    format!("{}\n", json!({ "type": "assistant", "timestamp": TS, "cwd": CWD, "message": { "id": mid, "model": "claude-opus-5-5", "content": [{ "type": "tool_use", "id": id, "name": name, "input": input }], "usage": { "output_tokens": 1 } } }))
}

fn tool_result(id: &str, err: bool) -> String {
    format!("{}\n", json!({ "type": "user", "timestamp": TS, "cwd": CWD, "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": id, "is_error": err, "content": "boom" }] } }))
}

#[derive(Default)]
struct Rec(Mutex<Vec<Value>>);

impl Sink for Rec {
    fn scan(&self, _: Value) {}
    fn reset(&self, s: Value) {
        self.0.lock().unwrap().push(s);
    }
    fn update(&self, b: Vec<Value>) {
        self.0.lock().unwrap().push(Value::Array(b));
    }
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-core-usage-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("projects/p")).unwrap();
    dir
}

fn open(root: &Path) -> Shared {
    let mut idx = UsageIndex::new(root.join("projects"), Some(root.join("usage-index.json")), Options::default());
    idx.load_cache();
    Shared::new(idx)
}

fn opus(sh: &Shared) -> Value {
    let snap = sh.lock().snapshot_json();
    snap["buckets"].as_array().unwrap().iter().find(|b| b["model"] == "claude-opus-5-5").cloned().unwrap_or(Value::Null)
}

#[test]
fn parses_usage_lines_and_skips_synthetic_or_user() {
    let usage = json!({ "input_tokens": 2, "output_tokens": 10, "cache_creation_input_tokens": 30, "cache_read_input_tokens": 400, "cache_creation": { "ephemeral_5m_input_tokens": 10, "ephemeral_1h_input_tokens": 20 } });
    let line = parse_line(assistant("m1", usage, "claude-opus-5-5").trim_end().as_bytes()).unwrap();
    let r = usage_record_of(&line, "slug").unwrap();
    assert_eq!(r.day, local_day(TS).unwrap());
    assert_eq!(r.project, CWD);
    assert_eq!((r.totals.input, r.totals.output, r.totals.cache_write_5m, r.totals.cache_write_1h, r.totals.cache_read), (2.0, 10.0, 10.0, 20.0, 400.0));
    let synthetic = parse_line(assistant("m2", json!({ "input_tokens": 1 }), "<synthetic>").as_bytes()).unwrap();
    assert!(usage_record_of(&synthetic, "x").is_none());
    let legacy = parse_line(assistant("m3", json!({ "cache_creation_input_tokens": 9, "speed": "fast" }), "claude-opus-5-5").as_bytes()).unwrap();
    let r = usage_record_of(&legacy, "x").unwrap();
    assert_eq!(r.totals.cache_write_5m, 9.0);
    assert!(r.key.ends_with("|f"));
    let odd = br#"{"type":"assistant","type":"assistant","timestamp":"2026-10-07T09:00:00Z","message":{"id":"d","model":"m","usage":{"input_tokens":-3,"output_tokens":"7","cache_read_input_tokens":2.5}}}"#;
    let r = usage_record_of(&parse_line(odd).unwrap(), "fb").unwrap();
    assert_eq!((r.totals.input, r.totals.output, r.totals.cache_read, r.project.as_str()), (0.0, 0.0, 2.5, "fb"));
    assert!(parse_line(b"{broken").is_none());
    assert!(parse_line(b"[1]").is_none());
    let mut bad = br#"{"type":"assistant","timestamp":"2026-10-07T09:00:00Z","cwd":"x"#.to_vec();
    bad.extend_from_slice(&[0xff, b'"', b'}']);
    assert_eq!(parse_line(&bad).unwrap().cwd.as_deref(), Some("x\u{fffd}"));
}

#[test]
fn tool_calls_read_only_names() {
    let block = |v: Value| parse_line(json!({ "type": "assistant", "message": { "content": [v] } }).to_string().as_bytes()).unwrap().message.unwrap().content.unwrap().remove(0).unwrap();
    assert_eq!(tool_calls_of(&block(json!({ "type": "tool_use", "name": "Skill", "input": { "skill": "pdf", "args": "secret" } }))), vec![('t', "Skill".into()), ('s', "pdf".into())]);
    assert_eq!(tool_calls_of(&block(json!({ "type": "tool_use", "name": "Agent", "input": { "prompt": "x" } })))[1], ('a', "general-purpose".into()));
    assert_eq!(tool_calls_of(&block(json!({ "type": "tool_use", "name": "mcp__github__get_me" }))), vec![('m', "github".into())]);
    assert_eq!(tool_calls_of(&block(json!({ "type": "tool_use", "name": "a|b" }))), vec![('t', "ab".into())]);
    assert!(tool_calls_of(&block(json!({ "type": "text", "name": "Bash" }))).is_empty());
    assert_eq!(mcp_server_of("mcp__claude_ai_Gmail__send"), Some("claude_ai_Gmail"));
    assert_eq!(mcp_server_of("mcp__x"), None);
    assert_eq!(clean_name(Some(" \u{feff}x\u{1}y\u{a0}")), Some("xy".into()));
    assert_eq!(clean_name(Some(&"é".repeat(150))).unwrap().chars().count(), 100);
    assert_eq!(clean_name(Some(" | ")), None);
}

#[test]
fn dedupes_globally_and_keeps_resume_copies_after_delete() {
    let root = tmp("dedupe");
    let p = root.join("projects/p");
    let u = |o: i32| json!({ "input_tokens": 5, "output_tokens": o });
    std::fs::write(p.join("a.jsonl"), assistant("m1", u(7), "claude-opus-5-5") + &assistant("m1", u(7), "claude-opus-5-5") + &assistant("m2", u(1), "claude-opus-5-5")).unwrap();
    std::fs::write(p.join("b.jsonl"), assistant("m1", u(7), "claude-opus-5-5") + &assistant("r1", u(2), "claude-opus-5-5")).unwrap();
    let sh = open(&root);
    assert!(full_scan(&sh, &Rec::default()).unwrap());
    assert_eq!((opus(&sh)["messages"].clone(), opus(&sh)["output"].clone()), (json!(3), json!(10)));
    drop(sh);
    std::fs::remove_file(p.join("a.jsonl")).unwrap();
    let again = open(&root);
    assert_eq!(opus(&again)["messages"], 3);
    full_scan(&again, &Rec::default()).unwrap();
    assert_eq!((opus(&again)["messages"].clone(), opus(&again)["output"].clone()), (json!(3), json!(10)));
    let cache: Value = serde_json::from_slice(&std::fs::read(root.join("usage-index.json")).unwrap()).unwrap();
    assert_eq!(cache["version"], 4);
    assert_eq!(cache["files"]["p/a.jsonl"]["gone"], true);
    assert_eq!(cache["files"]["p/a.jsonl"]["ids"], json!([]));
    assert_eq!(cache["files"]["p/b.jsonl"]["ids"], json!(["r1"]));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tails_partial_lines_counts_errors_and_recounts_truncation() {
    let root = tmp("tail");
    let f = root.join("projects/p/s.jsonl");
    let next = tool_use("m3", "tu3", "Bash", json!({ "command": "secret" }));
    std::fs::write(&f, tool_use("m1", "tu1", "mcp__github__x", json!({})) + &tool_result("tu1", true) + &next[..20]).unwrap();
    let sh = open(&root);
    let sink = Rec::default();
    full_scan(&sh, &sink).unwrap();
    assert_eq!(opus(&sh)["messages"], 1);
    let mut all = std::fs::read(&f).unwrap();
    all.extend_from_slice(&next.as_bytes()[20..]);
    all.extend_from_slice(tool_result("tu3", true).as_bytes());
    std::fs::write(&f, all).unwrap();
    assert!(drain(&sh, vec!["p/s.jsonl".into()], &sink).unwrap());
    assert_eq!(opus(&sh)["messages"], 2);
    let day = local_day(TS).unwrap();
    let stats = sh.lock().tool_stats(30, None, &day);
    assert_eq!(stats["mcp"], json!([{ "name": "github", "n": 1, "errors": 1 }]));
    assert_eq!(stats["tools"], json!([{ "name": "Bash", "n": 1, "errors": 1 }]));
    std::fs::write(&f, assistant("n1", json!({ "output_tokens": 4 }), "claude-opus-5-5")).unwrap();
    drain(&sh, vec!["p/s.jsonl".into()], &sink).unwrap();
    assert_eq!((opus(&sh)["messages"].clone(), opus(&sh)["output"].clone()), (json!(1), json!(4)));
    assert!(!serde_json::to_string(&*sink.0.lock().unwrap()).unwrap().contains("secret"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore]
fn measure_phases_over_a_corpus() {
    let dir = PathBuf::from(std::env::var("CCM_USAGE_BENCH_DIR").expect("CCM_USAGE_BENCH_DIR"));
    let files = crate::jobs::list_jsonl(&dir);
    let (usage, error) = (memchr::memmem::Finder::new(b"\"usage\""), memchr::memmem::Finder::new(b"\"is_error\":true"));
    let (mut bytes, mut lines, mut hits, mut parsed) = (0usize, 0usize, 0usize, 0usize);
    let t = std::time::Instant::now();
    let mut split = std::time::Duration::ZERO;
    let mut parse = std::time::Duration::ZERO;
    for rel in &files {
        let data = std::fs::read(dir.join(rel)).unwrap();
        bytes += data.len();
        let s = std::time::Instant::now();
        let mut found = Vec::new();
        for l in data.split(|b| *b == b'\n') {
            lines += 1;
            if usage.find(l).is_some() || error.find(l).is_some() {
                found.push(l);
            }
        }
        split += s.elapsed();
        hits += found.len();
        let s = std::time::Instant::now();
        parsed += found.iter().filter(|l| parse_line(l).is_some()).count();
        parse += s.elapsed();
    }
    eprintln!("bytes={bytes} lines={lines} hits={hits} parsed={parsed} total={:?} split+find={split:?} parse={parse:?}", t.elapsed());
}

#[cfg(windows)]
fn lock(path: &Path) -> impl Drop {
    use std::os::windows::fs::OpenOptionsExt;
    struct Held(#[allow(dead_code)] std::fs::File);
    impl Drop for Held {
        fn drop(&mut self) {}
    }
    Held(std::fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap())
}

#[cfg(unix)]
fn lock(path: &Path) -> impl Drop {
    use std::os::unix::fs::PermissionsExt;
    struct Held(PathBuf);
    impl Drop for Held {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o644));
        }
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    Held(path.to_owned())
}

#[test]
fn skips_an_unreadable_transcript_and_reads_it_later() {
    let root = tmp("locked");
    let p = root.join("projects/p");
    std::fs::write(p.join("a.jsonl"), assistant("a1", json!({ "output_tokens": 1 }), "claude-opus-5-5")).unwrap();
    std::fs::write(p.join("b.jsonl"), assistant("b1", json!({ "output_tokens": 2 }), "claude-opus-5-5")).unwrap();
    std::fs::write(p.join("c.jsonl"), assistant("c1", json!({ "output_tokens": 4 }), "claude-opus-5-5")).unwrap();
    let sh = open(&root);
    {
        let _held = lock(&p.join("b.jsonl"));
        assert!(full_scan(&sh, &Rec::default()).unwrap());
    }
    assert_eq!(opus(&sh)["output"], 5);
    assert_eq!(sh.lock().files.get("p/b.jsonl").unwrap().mtime_ms, None);
    drain(&sh, vec!["p/b.jsonl".into()], &Rec::default()).unwrap();
    assert_eq!(opus(&sh)["output"], 7);
    let _ = std::fs::remove_dir_all(&root);
}
