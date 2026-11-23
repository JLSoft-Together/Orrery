use super::edits::*;
use super::page::{page, turn_bounds};
use super::{Agent, Clean};
use crate::time::parse_iso;
use crate::tracker::cwd_slug;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Plain;

impl Clean for Plain {
    fn line(&self, text: &str, max: usize) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ").replace("sk-SECRET", "••••").chars().take(max).collect()
    }
    fn block(&self, text: &str, max: usize) -> (String, bool) {
        (text.chars().take(max).collect(), text.chars().count() > max)
    }
}

const HOUR: i64 = 3_600_000;
const T: i64 = 1_795_000_000_000;

fn rec(session: &str, turn: u64, end: i64, paths: &[&str]) -> TurnEdit {
    TurnEdit { session_id: session.into(), agent: "claude".into(), label: "api".into(), transcript_path: String::new(), cwd: "C:/w/api".into(), turn, start: Some(end - 60_000), end, paths: paths.iter().map(|p| (*p).to_owned()).collect(), prompt: String::new() }
}

fn files(f: &[&str]) -> Vec<String> {
    f.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn a_turn_that_edited_every_file_is_a_strong_match() {
    let recs = vec![rec("s1", 10, T - HOUR, &["src/a.ts", "src/b.ts"])];
    let (hits, more) = match_commit(&recs, T, &files(&["src/a.ts", "src/b.ts"]), false);
    assert_eq!((hits.len(), more), (1, 0));
    assert_eq!((hits[0].session_id.as_str(), hits[0].turn), ("s1", 10));
    assert!(hits[0].score >= STRONG);
}

#[test]
fn part_of_the_files_is_weak_and_too_few_is_nothing() {
    let recs = vec![rec("s1", 10, T - HOUR, &["src/a.ts", "x.md"])];
    let (weak, _) = match_commit(&recs, T, &files(&["src/a.ts", "src/b.ts"]), false);
    assert_eq!(weak.len(), 1);
    assert!(weak[0].score >= WEAK && weak[0].score < STRONG);
    let (none, _) = match_commit(&recs, T, &files(&["src/a.ts", "b", "c", "d"]), false);
    assert!(none.is_empty());
}

#[test]
fn turns_outside_the_time_window_do_not_count() {
    let paths = ["src/a.ts"];
    let early = vec![rec("s1", 1, T - BEFORE_MS - 1, &paths)];
    assert!(match_commit(&early, T, &files(&paths), false).0.is_empty());
    let late = vec![rec("s1", 1, T + AFTER_MS + 1, &paths)];
    assert!(match_commit(&late, T, &files(&paths), false).0.is_empty());
    let edge = vec![rec("s1", 1, T + AFTER_MS - 1, &paths), rec("s2", 2, T - BEFORE_MS + 1, &paths)];
    assert_eq!(match_commit(&edge, T, &files(&paths), false).0.len(), 2);
}

#[test]
fn many_sessions_keep_two_chips_and_count_the_rest() {
    let recs = vec![
        rec("s1", 1, T - 3 * HOUR, &["a", "b"]),
        rec("s2", 2, T - 2 * HOUR, &["a", "b", "c", "d"]),
        rec("s3", 3, T - HOUR, &["c", "d"]),
        rec("s4", 4, T - HOUR, &["d"]),
    ];
    let (hits, more) = match_commit(&recs, T, &files(&["a", "b", "c", "d"]), false);
    assert_eq!(hits.iter().map(|h| h.session_id.as_str()).collect::<Vec<_>>(), ["s2", "s3"]);
    assert_eq!(more, 1);
}

#[test]
fn one_session_with_several_turns_adds_up_and_points_at_the_biggest_turn() {
    let recs = vec![rec("s1", 1, T - 2 * HOUR, &["a"]), rec("s1", 9, T - HOUR, &["b", "c"])];
    let (hits, _) = match_commit(&recs, T, &files(&["a", "b", "c"]), false);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].turn, 9);
    assert!((hits[0].score - 1.0).abs() < 1e-9);
}

#[test]
fn a_commit_by_hand_gets_no_chip() {
    let recs = vec![rec("s1", 1, T - HOUR, &["src/a.ts"])];
    assert!(match_commit(&recs, T, &files(&["README.md"]), false).0.is_empty());
    assert!(match_commit(&recs, T, &[], false).0.is_empty());
}

#[test]
fn windows_paths_match_without_case() {
    let recs = vec![rec("s1", 1, T - HOUR, &["Src/A.ts"])];
    assert_eq!(match_commit(&recs, T, &files(&["src/a.ts"]), true).0.len(), 1);
    assert!(match_commit(&recs, T, &files(&["src/a.ts"]), false).0.is_empty());
}

#[test]
fn compact_drops_old_turns_keeps_the_last_copy_and_stays_under_the_cap() {
    let now = T;
    let mut a = rec("s1", 1, now - HOUR, &["a"]);
    let old = rec("s0", 1, now - KEEP_MS - 1, &["a"]);
    let b = rec("s1", 1, now - HOUR, &["a", "b"]);
    a.prompt = "first".into();
    let out = compact(vec![old, a, b.clone()], now);
    assert_eq!(out, vec![b]);
    let big: Vec<String> = (0..400).map(|i| format!("src/some/long/folder/name/file-{i:04}.ts")).collect();
    let refs: Vec<&str> = big.iter().map(String::as_str).collect();
    let many: Vec<TurnEdit> = (0..400u64).map(|i| rec("s", i, now - (400 - i as i64) * 1000, &refs)).collect();
    let kept = compact(many, now);
    let size: usize = kept.iter().map(|r| serde_json::to_string(r).unwrap().len() + 1).sum();
    assert!(size <= STORE_MAX, "{size}");
    assert!(kept.len() < 400 && !kept.is_empty());
    assert_eq!(kept.last().unwrap().turn, 399);
}

#[test]
fn paths_are_made_relative_to_the_repo() {
    assert_eq!(repo_path("C:\\w\\api\\src\\a.ts", "C:\\w\\api", "C:/w/api"), Some("src/a.ts".into()));
    assert_eq!(repo_path("c:/W/api/src/a.ts", "C:/w/api", "C:\\w\\api\\"), Some("src/a.ts".into()));
    assert_eq!(repo_path("src/b.ts", "C:/w/api/apps/web", "C:/w/api"), Some("apps/web/src/b.ts".into()));
    assert_eq!(repo_path("../x/c.ts", "/home/u/r/apps", "/home/u/r"), Some("x/c.ts".into()));
    assert_eq!(repo_path("C:/other/a.ts", "C:/w/api", "C:/w/api"), None);
    assert_eq!(repo_path("~/a.ts", "C:/w/api", "C:/w/api"), None);
    assert!(under("C:\\w\\api\\apps", "c:/w/api") && under("/r", "/r/") && !under("/r2", "/r") && !under("", "/r"));
    assert_eq!(store_key("C:\\w\\api\\"), store_key("c:/w/api"));
}

#[test]
fn civil_dates_and_rollout_ids() {
    assert_eq!(civil(0), (1970, 1, 1));
    assert_eq!(civil(parse_iso("2026-11-23T10:00:00.000Z").unwrap().div_euclid(86_400_000)), (2026, 11, 23));
    assert_eq!(civil(parse_iso("2024-02-29T00:00:00.000Z").unwrap() / 86_400_000), (2024, 2, 29));
    let id = "0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b";
    assert_eq!(rollout_id(Path::new(&format!("rollout-2026-11-23T10-00-00-{id}.jsonl"))), Some(id.to_owned()));
    assert_eq!(rollout_id(Path::new("short.jsonl")), None);
}

fn iso(ms: i64) -> String {
    let secs = ms / 1000;
    let (y, m, d) = civil(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", s / 3600, (s / 60) % 60, s % 60)
}

fn line(v: Value) -> String {
    format!("{v}\n")
}

fn user(cwd: &str, at: i64, text: &str) -> String {
    line(json!({ "type": "user", "cwd": cwd, "sessionId": "s", "timestamp": iso(at), "message": { "role": "user", "content": text } }))
}

fn edit(at: i64, id: &str, name: &str, path: &str) -> String {
    line(json!({ "type": "assistant", "cwd": "x", "timestamp": iso(at), "message": { "id": id, "model": "m", "content": [{ "type": "tool_use", "id": id, "name": name, "input": { "file_path": path, "old_string": "a", "new_string": "b", "content": "c" } }] } }))
}

fn result(at: i64, id: &str) -> String {
    line(json!({ "type": "user", "timestamp": iso(at), "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": id, "content": "ok" }] } }))
}

fn base(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-edits-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn ingest_reads_claude_turns_with_the_same_offsets_as_the_chat_and_grows_incrementally() {
    let dir = base("claude");
    let root = dir.join("repo").to_string_lossy().replace('\\', "/");
    let now = crate::time::now_ms() / 1000 * 1000;
    let projects = dir.join("projects");
    let pdir = projects.join(cwd_slug(&root));
    std::fs::create_dir_all(&pdir).unwrap();
    let other = projects.join(format!("{}-old", cwd_slug(&root)));
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("x.jsonl"), user(&format!("{root}-old"), now - HOUR, "elsewhere") + &edit(now - HOUR, "e0", "Edit", &format!("{root}-old/a.ts"))).unwrap();
    let file = pdir.join("sess-1.jsonl");
    let first = user(&root, now - 3 * HOUR, "Fix login with sk-SECRET") + &edit(now - 3 * HOUR + 1000, "e1", "Edit", &format!("{root}/src/a.ts")) + &result(now - 3 * HOUR + 2000, "e1") + &edit(now - 3 * HOUR + 3000, "e2", "Write", &format!("{root}/src/b.ts")) + &edit(now - 3 * HOUR + 4000, "e3", "Write", "C:/elsewhere/c.ts");
    std::fs::write(&file, &first).unwrap();
    let store = Store::new(&dir.join("edits"), &root);
    let src = Sources { projects: projects.clone(), sessions: dir.join("codex") };
    let recs = store.ingest(&src, now, &Plain).unwrap();
    assert_eq!(recs.len(), 1, "{recs:?}");
    let r = &recs[0];
    assert_eq!((r.session_id.as_str(), r.agent.as_str(), r.turn, r.end), ("sess-1", "claude", 0, now - 3 * HOUR + 3000));
    assert_eq!(r.paths, vec!["src/a.ts".to_owned(), "src/b.ts".to_owned()]);
    assert_eq!(r.prompt, "Fix login with ••••");
    assert_eq!(r.label, "repo");
    let second_at = first.len() as u64;
    let mut f = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
    f.write_all((user(&root, now - HOUR, "Add a test") + &edit(now - HOUR + 500, "e4", "Edit", &format!("{root}/src/a.test.ts"))).as_bytes()).unwrap();
    drop(f);
    let recs = store.ingest(&src, now, &Plain).unwrap();
    assert_eq!(recs.iter().map(|r| r.turn).collect::<Vec<_>>(), vec![0, second_at]);
    assert_eq!(store.load().len(), 2);
    let pg = page(&file, Agent::Claude, &root, None, 30, &Plain, false).unwrap();
    let me: Vec<u64> = pg.items.iter().filter(|i| i["k"] == "me").map(|i| i["seq"].as_u64().unwrap()).collect();
    assert_eq!(me, vec![0, second_at]);
    assert_eq!(turn_bounds(&file, Agent::Claude, 0).unwrap(), Some(Some(second_at)));
    assert_eq!(turn_bounds(&file, Agent::Claude, second_at).unwrap(), Some(None));
    assert_eq!(turn_bounds(&file, Agent::Claude, 5).unwrap(), None);
    let commit = recs.iter().map(|r| r.end).max().unwrap() + 30_000;
    let (hits, _) = match_commit(&recs, commit, &files(&["src/a.ts", "src/b.ts"]), fold(&root));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].turn, 0);
}

#[test]
fn ingest_reads_codex_rollouts_in_a_sub_folder_and_skips_other_projects() {
    let dir = base("codex");
    let root = dir.join("mono").to_string_lossy().replace('\\', "/");
    let now = crate::time::now_ms() / 1000 * 1000;
    let folder = |days_ago: i64| {
        let (y, m, d) = civil(now.div_euclid(86_400_000) - days_ago);
        dir.join("sessions").join(format!("{y:04}")).join(format!("{m:02}")).join(format!("{d:02}"))
    };
    let day = folder(0);
    std::fs::create_dir_all(&day).unwrap();
    let id = "0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b";
    let meta = |cwd: &str| line(json!({ "timestamp": iso(now - HOUR), "type": "session_meta", "payload": { "id": id, "cwd": cwd } }));
    let msg = |text: &str| line(json!({ "timestamp": iso(now - HOUR), "type": "response_item", "payload": { "type": "message", "role": "user", "content": [{ "type": "input_text", "text": text }] } }));
    let patch = line(json!({ "timestamp": iso(now - HOUR + 9000), "type": "response_item", "payload": { "type": "custom_tool_call", "name": "apply_patch", "call_id": "c1", "input": "*** Begin Patch\n*** Update File: src/web.ts\n+x\n*** Add File: ../shared/t.ts\n+y\n*** End Patch" } }));
    let body = meta(&format!("{root}/apps/web")) + &msg("Make the page faster") + &patch;
    std::fs::write(day.join(format!("rollout-2026-11-23T09-00-00-{id}.jsonl")), &body).unwrap();
    let other_id = "0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5c";
    std::fs::write(day.join(format!("rollout-2026-11-23T09-00-01-{other_id}.jsonl")), meta("C:/not/here") + &msg("x") + &patch).unwrap();
    let old = folder(60);
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join(format!("rollout-2026-09-01T09-00-00-{id}.jsonl")), &body).unwrap();
    let store = Store::new(&dir.join("edits"), &root);
    let recs = store.ingest(&Sources { projects: dir.join("none"), sessions: dir.join("sessions") }, now, &Plain).unwrap();
    assert_eq!(recs.len(), 1, "{recs:?}");
    assert_eq!((recs[0].agent.as_str(), recs[0].session_id.as_str(), recs[0].label.as_str()), ("codex", id, "web"));
    assert_eq!(recs[0].paths, vec!["apps/shared/t.ts".to_owned(), "apps/web/src/web.ts".to_owned()]);
    let again = store.ingest(&Sources { projects: dir.join("none"), sessions: dir.join("sessions") }, now, &Plain).unwrap();
    assert_eq!(again, recs);
    let later = store.ingest(&Sources { projects: dir.join("none"), sessions: dir.join("sessions") }, now + KEEP_MS + HOUR, &Plain).unwrap();
    assert!(later.is_empty());
    assert!(store.load().is_empty());
}

#[test]
fn transcripts_last_written_before_the_commits_wait_for_a_later_ask() {
    let dir = base("since");
    let root = dir.join("repo").to_string_lossy().replace('\\', "/");
    let now = crate::time::now_ms() / 1000 * 1000;
    let pdir = dir.join("projects").join(cwd_slug(&root));
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::write(pdir.join("s.jsonl"), user(&root, now - HOUR, "Go") + &edit(now - HOUR, "e1", "Edit", &format!("{root}/a.ts"))).unwrap();
    let store = Store::new(&dir.join("edits"), &root);
    let src = Sources { projects: dir.join("projects"), sessions: dir.join("codex") };
    assert!(store.ingest_since(&src, now, now + HOUR, &Plain).unwrap().is_empty());
    assert_eq!(store.ingest_since(&src, now, 0, &Plain).unwrap().len(), 1);
}
