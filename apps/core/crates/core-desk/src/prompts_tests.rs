use super::*;
use crate::{DeskIo, NoHost};
use std::sync::atomic::{AtomicUsize, Ordering};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ccm-prompts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn project() -> &'static str {
    if cfg!(windows) {
        "C:\\work\\app"
    } else {
        "/work/app"
    }
}

fn item(title: &str, body: &str) -> Value {
    json!({"title": title, "body": body, "scope": "all"})
}

fn desk_in(dir: &Path) -> Desk {
    let env = DeskEnv { data_dir: dir.to_path_buf(), ..DeskEnv::default() };
    let mut desk = Desk::new();
    register(&mut desk, &env);
    desk
}

fn upsert(desk: &Desk, v: Value) -> Value {
    desk.dispatch("ccm:prompts-set", json!([{"op": "upsert", "item": v}])).unwrap()
}

fn titles(v: &Value) -> Vec<String> {
    v["items"].as_array().unwrap().iter().map(|i| i["title"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn a_missing_file_reads_as_an_empty_library() {
    let dir = tmp("missing");
    let desk = desk_in(&dir);
    let got = desk.dispatch("ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(got["ok"], true);
    assert_eq!(got["version"], 1);
    assert_eq!(got["items"], json!([]));
    assert!(!file_for(&dir).exists());
}

#[test]
fn upsert_adds_then_updates_in_place_and_keeps_the_order() {
    let dir = tmp("upsert");
    let desk = desk_in(&dir);
    let a = upsert(&desk, item("A", "first"));
    let id = a["id"].as_str().unwrap().to_owned();
    upsert(&desk, item("B", "second"));
    let edited = upsert(&desk, json!({"id": id, "title": "A2", "body": "changed", "scope": project()}));
    assert_eq!(edited["ok"], true);
    assert_eq!(titles(&edited), ["A2", "B"]);
    assert_eq!(edited["items"][0]["scope"], project());
    assert_eq!(edited["items"][0]["body"], "changed");
    let again = desk.dispatch("ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(titles(&again), ["A2", "B"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_and_reorder() {
    let dir = tmp("reorder");
    let desk = desk_in(&dir);
    let ids: Vec<String> = ["A", "B", "C"].iter().map(|t| upsert(&desk, item(t, "x"))["id"].as_str().unwrap().to_owned()).collect();
    let moved = desk.dispatch("ccm:prompts-set", json!([{"op": "reorder", "ids": [ids[2], "ghost", ids[2], ids[0]]}])).unwrap();
    assert_eq!(titles(&moved), ["C", "A", "B"]);
    let gone = desk.dispatch("ccm:prompts-set", json!([{"op": "delete", "id": ids[0]}])).unwrap();
    assert_eq!(titles(&gone), ["C", "B"]);
    let missing = desk.dispatch("ccm:prompts-set", json!([{"op": "delete", "id": ids[0]}])).unwrap();
    assert_eq!(missing, json!({"ok": false, "error": "not-found"}));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn limits_reject_instead_of_truncating() {
    let dir = tmp("limits");
    let desk = desk_in(&dir);
    let err = |v: Value| upsert(&desk, v)["error"].as_str().unwrap_or("").to_owned();
    assert_eq!(err(item(&"t".repeat(MAX_TITLE + 1), "b")), "invalid-title");
    assert_eq!(err(item("   ", "b")), "invalid-title");
    assert_eq!(err(item("t", "  \n ")), "invalid-body");
    assert_eq!(err(item("t", &"b".repeat(MAX_BODY + 1))), "invalid-body");
    assert_eq!(err(json!({"title": "t", "body": "b", "scope": "relative/dir"})), "invalid-scope");
    assert_eq!(err(json!({"id": "bad id!", "title": "t", "body": "b"})), "invalid-id");
    assert_eq!(upsert(&desk, item(&"t".repeat(MAX_TITLE), &"b".repeat(MAX_BODY)))["ok"], true);
    assert_eq!(desk.dispatch("ccm:prompts-set", json!([{"op": "nope"}])).unwrap()["error"], "invalid-op");
    assert_eq!(desk.dispatch("ccm:prompts-set", Value::Null).unwrap()["error"], "invalid-op");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn emoji_count_as_two_units_like_the_web_length() {
    let mut lib = Library::default();
    let title = "\u{1F600}".repeat(MAX_TITLE / 2);
    assert!(lib.upsert(&item(&title, "b"), 1).is_ok());
    let over = "\u{1F600}".repeat(MAX_TITLE / 2 + 1);
    assert_eq!(lib.upsert(&item(&over, "b"), 1), Err(PromptError::Title));
}

#[test]
fn the_201st_item_is_refused_but_edits_still_work() {
    let mut lib = Library::default();
    for i in 0..MAX_ITEMS {
        lib.upsert(&item(&format!("p{i}"), "x"), 1).unwrap();
    }
    assert_eq!(lib.upsert(&item("extra", "x"), 1), Err(PromptError::Full));
    let id = lib.items[0].id.clone();
    assert!(lib.upsert(&json!({"id": id, "title": "renamed", "body": "x"}), 2).is_ok());
    assert_eq!(lib.items.len(), MAX_ITEMS);
}

#[test]
fn multiline_bodies_keep_their_lines_and_normalize_crlf() {
    let mut lib = Library::default();
    lib.upsert(&item("t", "line one\r\nline two\n\n/slash start"), 1).unwrap();
    assert_eq!(lib.items[0].body, "line one\nline two\n\n/slash start");
    lib.upsert(&item("s", "/review"), 1).unwrap();
    assert_eq!(lib.items[1].body, "/review");
}

#[test]
fn a_broken_file_is_moved_to_bak_and_the_library_starts_empty() {
    let dir = tmp("broken");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(file_for(&dir), "{not json").unwrap();
    let desk = desk_in(&dir);
    let got = desk.dispatch("ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(got["ok"], true);
    assert_eq!(got["recovered"], true);
    assert_eq!(got["items"], json!([]));
    assert_eq!(std::fs::read_to_string(dir.join(BACKUP)).unwrap(), "{not json");
    assert!(!file_for(&dir).exists());
    let added = upsert(&desk, item("fresh", "x"));
    assert_eq!(titles(&added), ["fresh"]);
    let again = desk.dispatch("ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(again["recovered"], false);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_from_a_newer_version_is_never_overwritten() {
    let dir = tmp("newer");
    std::fs::create_dir_all(&dir).unwrap();
    let text = r#"{"version":2,"items":[]}"#;
    std::fs::write(file_for(&dir), text).unwrap();
    let desk = desk_in(&dir);
    assert_eq!(desk.dispatch("ccm:prompts-get", Value::Null).unwrap()["error"], "io");
    assert_eq!(upsert(&desk, item("t", "b"))["error"], "io");
    assert_eq!(std::fs::read_to_string(file_for(&dir)).unwrap(), text);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_rows_are_dropped_and_duplicates_collapse_on_load() {
    let dir = tmp("rows");
    std::fs::create_dir_all(&dir).unwrap();
    let text = json!({"version": 1, "items": [
        {"id": "a", "title": "ok", "body": "b", "scope": "all", "updatedAt": 5},
        {"id": "a", "title": "dup", "body": "b", "scope": "all"},
        {"id": "b", "title": "", "body": "b", "scope": "all"},
        {"id": "c", "title": "t", "body": "b", "scope": "nowhere"},
        "junk",
        {"id": "d", "title": "keep", "body": "b", "scope": "all"}
    ]});
    std::fs::write(file_for(&dir), format!("\u{feff}{text}")).unwrap();
    let loaded = load(&file_for(&dir));
    assert!(loaded.readable && !loaded.recovered);
    let names: Vec<_> = loaded.lib.items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(names, ["ok", "keep"]);
    assert_eq!(loaded.lib.items[0].updated_at, 5);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_adds_with_fresh_ids_and_counts_the_skipped() {
    let dir = tmp("import");
    let desk = desk_in(&dir);
    let out = desk
        .dispatch("ccm:prompts-set", json!([{"op": "import", "items": [item("one", "x"), {"title": "", "body": "x"}, {"id": "keep-me", "title": "two", "body": "y", "scope": "all"}, 7]}]))
        .unwrap();
    assert_eq!((out["added"].as_u64(), out["skipped"].as_u64()), (Some(2), Some(2)));
    let ids: Vec<_> = out["items"].as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap().to_owned()).collect();
    assert!(!ids.contains(&"keep-me".to_owned()));
    assert_eq!(ids[0].len(), 36);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn writes_are_atomic_and_leave_no_temp_file() {
    let dir = tmp("atomic");
    let desk = desk_in(&dir);
    upsert(&desk, item("t", "b"));
    let names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, [FILE]);
    let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(file_for(&dir)).unwrap()).unwrap();
    assert_eq!(on_disk["version"], 1);
    assert_eq!(on_disk["items"][0]["title"], "t");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn concurrent_writers_do_not_lose_items() {
    let dir = tmp("threads");
    let desk = Arc::new(desk_in(&dir));
    let workers: Vec<_> = (0..8)
        .map(|n| {
            let desk = desk.clone();
            std::thread::spawn(move || {
                for i in 0..5 {
                    assert_eq!(upsert(&desk, item(&format!("t{n}-{i}"), "b"))["ok"], true);
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    let got = desk.dispatch("ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(got["items"].as_array().unwrap().len(), 40);
    let _ = std::fs::remove_dir_all(&dir);
}

struct Counter(AtomicUsize);

impl DeskIo for Counter {
    fn emit(&self, event: &str, payload: Value) {
        if event == CHANGED && payload["count"].is_u64() {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn host(&self, req: u64, method: &str, params: Value) -> Result<Value, crate::DeskError> {
        NoHost.host(req, method, params)
    }
}

#[test]
fn a_successful_write_emits_changed_and_a_failed_one_does_not() {
    let dir = tmp("emit");
    let desk = desk_in(&dir);
    let io = Counter(AtomicUsize::new(0));
    let ctx = Ctx::new(&io, 1);
    desk.dispatch_with(&ctx, "ccm:prompts-set", json!([{"op": "upsert", "item": item("t", "b")}])).unwrap();
    desk.dispatch_with(&ctx, "ccm:prompts-set", json!([{"op": "upsert", "item": item("", "b")}])).unwrap();
    desk.dispatch_with(&ctx, "ccm:prompts-get", Value::Null).unwrap();
    assert_eq!(io.0.load(Ordering::SeqCst), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
