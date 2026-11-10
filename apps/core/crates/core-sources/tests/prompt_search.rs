use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_sources::prompt_search::scan::{scan_transcript, ScanOptions, MAX_LINE_BYTES};
use core_sources::prompt_search::searcher::{PromptSearchRequest, PromptSearcher, SearcherDeps};
use core_sources::prompt_search::text::project_dir_name;
use serde_json::{json, Value};

static N: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    let d = std::env::temp_dir().join(format!("ccm-psearch-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

fn user(content: Value, extra: Value) -> Value {
    let mut rec = json!({"parentUuid": null, "isSidechain": false, "userType": "external", "entrypoint": "cli", "cwd": "D:\\work\\app", "sessionId": "s", "version": "2.1.294", "gitBranch": "main", "uuid": "u", "timestamp": "2026-10-20T08:00:00.000Z", "type": "user", "message": {"role": "user", "content": content}});
    for (k, v) in extra.as_object().unwrap() {
        rec.as_object_mut().unwrap().insert(k.clone(), v.clone());
    }
    rec
}

fn write_session(root: &Path, cwd: &str, n: u64, lines: &[String]) -> PathBuf {
    let dir = root.join(project_dir_name(cwd));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{}.jsonl", uuid(n)));
    fs::write(&file, lines.join("\n") + "\n").unwrap();
    file
}

fn s(v: &Value) -> String {
    v.to_string()
}

fn scan(file: &Path, terms: &[&str]) -> core_sources::prompt_search::scan::SessionScan {
    let terms: Vec<String> = terms.iter().map(|t| (*t).to_owned()).collect();
    scan_transcript(file, &ScanOptions { terms: &terms, mask: &|t| t.to_owned(), cancelled: &|| false }).unwrap()
}

#[test]
fn scan_counts_matches_keeps_three_snippets_reads_titles_and_skips_huge_lines() {
    let r = root();
    let file = write_session(
        &r,
        "D:\\work\\app",
        1,
        &[
            s(&json!({"type": "ai-title", "aiTitle": "Paywall work", "sessionId": "s"})),
            s(&user(json!("paywall one"), json!({"promptSource": "typed"}))),
            s(&user(json!(format!("{} paywall", "x".repeat(MAX_LINE_BYTES + 10))), json!({}))),
            s(&user(json!([{"type": "tool_result", "content": "paywall"}]), json!({"toolUseResult": {}}))),
            s(&user(json!("paywall two"), json!({}))),
            s(&user(json!("paywall three"), json!({}))),
            s(&user(json!("paywall four"), json!({}))),
            s(&json!({"type": "custom-title", "customTitle": "Renamed", "sessionId": "s"})),
        ],
    );
    let out = scan(&file, &["paywall"]);
    assert_eq!(out.matches, 4);
    assert_eq!(out.snippets, vec!["paywall one", "paywall two", "paywall three"]);
    assert_eq!((out.custom_title.as_deref(), out.ai_title.as_deref()), (Some("Renamed"), Some("Paywall work")));
    assert_eq!((out.cwd.as_deref(), out.git_branch.as_deref(), out.started_at.as_deref()), (Some("D:\\work\\app"), Some("main"), Some("2026-10-20T08:00:00.000Z")));
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn scan_handles_chunk_boundaries_a_last_line_without_newline_and_decomposed_text() {
    let r = root();
    let dir = r.join("p");
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("a.jsonl");
    let mut text = String::new();
    for i in 0..3000 {
        text.push_str(&s(&user(json!(format!("filler line number {i} ").repeat(3)), json!({}))));
        text.push('\n');
    }
    text.push_str(&s(&user(json!("Vie\u{0302}\u{0323}t needle"), json!({}))));
    fs::write(&file, text).unwrap();
    assert!(fs::metadata(&file).unwrap().len() > 3 * 64 * 1024);
    let out = scan(&file, &["vi\u{1ec7}t", "needle"]);
    assert_eq!((out.matches, out.snippets.len()), (1, 1));
    assert_eq!(scan(&file, &["filler"]).matches, 3000);
    let _ = fs::remove_dir_all(&r);
}

struct Collected {
    events: Vec<Value>,
}

impl Collected {
    fn hits(&self) -> Vec<Value> {
        self.events.iter().filter(|e| e["type"] == "promptSearch.progress").flat_map(|e| e["payload"]["hits"].as_array().unwrap().clone()).collect()
    }

    fn done(&self) -> Value {
        self.events.iter().find(|e| e["type"] == "promptSearch.done").cloned().unwrap()
    }
}

fn run(searcher: &PromptSearcher, client: u64, id: &str, query: &str, days: u32, project: Option<&str>) -> (&'static str, Collected) {
    let mut events = Vec::new();
    let req = PromptSearchRequest { search_id: id.into(), query: query.into(), days, project: project.map(str::to_owned) };
    let state = searcher.start(client, &req, &mut |e| events.push(e));
    (state, Collected { events })
}

fn deps() -> SearcherDeps {
    SearcherDeps { yield_fn: Some(Box::new(|| {})), ..SearcherDeps::default() }
}

#[test]
fn one_hit_per_session_with_every_word_matched_skipping_subagent_files() {
    let r = root();
    let cwd = r.to_string_lossy().into_owned();
    write_session(&r, &cwd, 1, &[s(&user(json!("Build the PAYWALL with IAP"), json!({"cwd": cwd})))]);
    write_session(&r, &cwd, 2, &[s(&user(json!("paywall only"), json!({"cwd": cwd})))]);
    let sub = r.join(project_dir_name(&cwd)).join(uuid(1)).join("subagents");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("agent-a.jsonl"), s(&user(json!("paywall iap"), json!({"isSidechain": true}))) + "\n").unwrap();
    let searcher = PromptSearcher::new(&r, deps());
    let (state, c) = run(&searcher, 1, "a", "iap paywall", 0, None);
    assert_eq!(state, "done");
    let hits = c.hits();
    assert_eq!(hits.iter().map(|h| h["sessionId"].as_str().unwrap().to_owned()).collect::<Vec<_>>(), vec![uuid(1)]);
    assert_eq!(hits[0]["cwdExists"], true);
    assert_eq!(hits[0]["snippets"], json!(["Build the PAYWALL with IAP"]));
    let done = c.done();
    assert_eq!((&done["payload"]["state"], &done["payload"]["filesTotal"], &done["payload"]["filesDone"], &done["payload"]["sessions"]), (&json!("done"), &json!(2), &json!(2), &json!(1)));
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn stops_at_two_hundred_sessions_and_fails_on_an_empty_query() {
    let r = root();
    for i in 0..205 {
        write_session(&r, "D:\\work\\app", i, &[s(&user(json!("paywall"), json!({})))]);
    }
    let searcher = PromptSearcher::new(&r, deps());
    let (state, c) = run(&searcher, 1, "a", "paywall", 0, None);
    assert_eq!((state, c.hits().len()), ("capped", 200));
    let (state, c) = run(&searcher, 1, "b", "   ", 0, None);
    assert_eq!(state, "failed");
    assert_eq!(c.events.len(), 1);
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn cancels_on_request_and_when_a_new_search_starts() {
    let r = root();
    for i in 0..20 {
        write_session(&r, "D:\\work\\app", i, &[s(&user(json!("paywall"), json!({})))]);
    }
    let slow = || SearcherDeps { yield_fn: Some(Box::new(|| std::thread::sleep(Duration::from_millis(15)))), ..SearcherDeps::default() };
    let searcher = Arc::new(PromptSearcher::new(&r, slow()));
    let s1 = searcher.clone();
    let first = std::thread::spawn(move || run(&s1, 1, "a", "paywall", 0, None));
    std::thread::sleep(Duration::from_millis(60));
    searcher.cancel(1, Some("other"));
    searcher.cancel(1, Some("a"));
    let (state, c) = first.join().unwrap();
    assert_eq!(state, "cancelled");
    assert!(c.hits().len() < 20);

    let s2 = searcher.clone();
    let second = std::thread::spawn(move || run(&s2, 1, "b", "paywall", 0, None));
    std::thread::sleep(Duration::from_millis(60));
    let (state3, c3) = run(&searcher, 1, "c", "paywall", 0, None);
    let (state2, _) = second.join().unwrap();
    assert_eq!((state2, state3, c3.hits().len()), ("cancelled", "done", 20));
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn filters_by_project_and_date_and_opens_only_folders_it_found() {
    let r = root();
    let project = r.join("proj");
    fs::create_dir_all(&project).unwrap();
    let p = project.to_string_lossy().into_owned();
    let other = r.join("other").to_string_lossy().into_owned();
    write_session(&r, &p, 1, &[s(&user(json!("paywall here"), json!({"cwd": p})))]);
    write_session(&r, &other, 2, &[s(&user(json!("paywall there"), json!({"cwd": other})))]);
    let opened = Arc::new(Mutex::new(Vec::<String>::new()));
    let o = opened.clone();
    let d = SearcherDeps { yield_fn: Some(Box::new(|| {})), open_folder: Some(Box::new(move |dir, _| (o.lock().unwrap().push(dir.to_owned()), "ok").1)), ..SearcherDeps::default() };
    let searcher = PromptSearcher::new(&r, d);
    let (_, c) = run(&searcher, 7, "a", "paywall", 7, Some(&p));
    assert_eq!(c.hits().iter().map(|h| h["sessionId"].as_str().unwrap().to_owned()).collect::<Vec<_>>(), vec![uuid(1)]);
    let mut events = Vec::new();
    searcher.open(7, "a", &uuid(1), &mut |e| events.push(e));
    searcher.open(7, "a", &uuid(2), &mut |e| events.push(e));
    searcher.open(8, "a", &uuid(1), &mut |e| events.push(e));
    assert_eq!(*opened.lock().unwrap(), vec![p.clone()]);
    assert_eq!(events.iter().map(|e| e["payload"]["result"].as_str().unwrap().to_owned()).collect::<Vec<_>>(), vec!["ok", "notFound", "notFound"]);

    let later_now = Arc::new(AtomicUsize::new(0));
    let l = later_now.clone();
    let d = SearcherDeps { yield_fn: Some(Box::new(|| {})), now: Some(Box::new(move || core_store::time::now_ms() as f64 + 30.0 * 86_400_000.0 + l.load(Ordering::SeqCst) as f64)), ..SearcherDeps::default() };
    let (_, old) = run(&PromptSearcher::new(&r, d), 1, "b", "paywall", 7, None);
    assert!(old.hits().is_empty());
    let none = PromptSearcher::new(&r, deps());
    let mut ev = Vec::new();
    none.open(1, "x", &uuid(1), &mut |e| ev.push(e));
    assert_eq!(ev[0]["payload"]["result"], "notFound");
    let _ = fs::remove_dir_all(&r);
}
