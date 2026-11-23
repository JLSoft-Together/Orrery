use super::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("orrery-stash-{tag}-{}-{}", std::process::id(), crate::util::now_ms() as u64));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn repo(tag: &str) -> PathBuf {
    let dir = tmp(tag);
    sh(&dir, &["init", "-q", "-b", "main"]);
    sh(&dir, &["config", "user.name", "t"]);
    sh(&dir, &["config", "user.email", "t@t"]);
    sh(&dir, &["config", "core.autocrlf", "false"]);
    fs::write(dir.join("a.txt"), "one\n").unwrap();
    sh(&dir, &["add", "-A"]);
    sh(&dir, &["commit", "-q", "-m", "first"]);
    dir
}

fn root(d: &Path) -> String {
    d.to_string_lossy().to_string()
}

fn commit_file(dir: &Path, name: &str, body: &str, msg: &str) {
    fs::write(dir.join(name), body).unwrap();
    sh(dir, &["add", "-A"]);
    sh(dir, &["commit", "-q", "-m", msg]);
}

fn first_hash(v: &Value) -> Value {
    v["stashes"][0]["hash"].clone()
}

#[test]
fn subject_and_list_rows_parse() {
    assert_eq!(split_subject("On main: my work"), ("main".to_string(), "my work".to_string()));
    assert_eq!(split_subject("WIP on feat/x: abc1234 subject"), ("feat/x".to_string(), "abc1234 subject".to_string()));
    assert_eq!(split_subject("odd"), (String::new(), "odd".to_string()));
    let rows = parse_list("h1\u{0}100\u{0}p1 p2\u{0}On main: a\nh2\u{0}90\u{0}p1 p2 p3\u{0}On dev: b\n");
    assert_eq!(rows.len(), 2);
    assert!(!rows[0].untracked && rows[1].untracked);
    assert_eq!((rows[1].index, rows[1].branch.as_str(), rows[1].message.as_str()), (1, "dev", "b"));
}

#[test]
fn push_list_apply_pop_and_drop() {
    let d = repo("flow");
    let r = root(&d);
    assert_eq!(push(&r, None, false)["error"], "nothing");

    fs::write(d.join("a.txt"), "two\n").unwrap();
    let pushed = push(&r, Some(&json!("  my   work\n")), false);
    assert_eq!(pushed["ok"], true, "{pushed}");
    assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "one\n");

    let l = list(&r);
    assert_eq!(l["stashes"].as_array().unwrap().len(), 1);
    assert_eq!(l["stashes"][0]["message"], "my work");
    assert_eq!(l["stashes"][0]["branch"], "main");
    assert_eq!(l["stashes"][0]["untracked"], false);
    let h = first_hash(&l);

    assert_eq!(apply(&r, &h)["ok"], true);
    assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "two\n");
    assert_eq!(list(&r)["stashes"].as_array().unwrap().len(), 1);
    sh(&d, &["checkout", "--", "a.txt"]);

    assert_eq!(pop(&r, &h)["ok"], true);
    assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "two\n");
    assert_eq!(list(&r)["stashes"].as_array().unwrap().len(), 0);

    let again = push(&r, None, false);
    assert_eq!(again["ok"], true);
    let h2 = first_hash(&list(&r));
    assert_eq!(drop_entry(&r, &h2)["ok"], true);
    assert_eq!(list(&r)["stashes"].as_array().unwrap().len(), 0);
    assert_eq!(drop_entry(&r, &h2)["error"], "gone");
    assert_eq!(drop_entry(&r, &json!("zz"))["error"], "denied");
}

#[test]
fn untracked_files_only_go_in_when_asked() {
    let d = repo("untracked");
    let r = root(&d);
    fs::write(d.join("new.txt"), "x\n").unwrap();
    assert_eq!(push(&r, None, false)["error"], "nothing");
    assert!(d.join("new.txt").exists());
    assert_eq!(push(&r, Some(&json!("with new")), true)["ok"], true);
    assert!(!d.join("new.txt").exists());
    let l = list(&r);
    assert_eq!(l["stashes"][0]["untracked"], true);
    assert_eq!(pop(&r, &first_hash(&l))["ok"], true);
    assert!(d.join("new.txt").exists());
}

#[test]
fn stash_ops_find_the_entry_by_hash_even_after_the_list_shifts() {
    let d = repo("shift");
    let r = root(&d);
    fs::write(d.join("a.txt"), "v1\n").unwrap();
    push(&r, Some(&json!("older")), false);
    let older = first_hash(&list(&r));
    fs::write(d.join("a.txt"), "v2\n").unwrap();
    push(&r, Some(&json!("newer")), false);
    assert_eq!(list(&r)["stashes"][0]["message"], "newer");
    assert_eq!(drop_entry(&r, &older)["ok"], true);
    let left = list(&r);
    assert_eq!(left["stashes"].as_array().unwrap().len(), 1);
    assert_eq!(left["stashes"][0]["message"], "newer");
}

#[test]
fn apply_reports_a_conflict_and_keeps_the_stash() {
    let d = repo("conflict");
    let r = root(&d);
    fs::write(d.join("a.txt"), "mine\n").unwrap();
    push(&r, None, false);
    commit_file(&d, "a.txt", "theirs\n", "other");
    let h = first_hash(&list(&r));
    let res = pop(&r, &h);
    assert_eq!(res["error"], "conflict", "{res}");
    assert_eq!(list(&r)["stashes"].as_array().unwrap().len(), 1);
}

#[test]
fn apply_over_local_edits_is_reported_as_dirty() {
    let d = repo("dirty");
    let r = root(&d);
    fs::write(d.join("a.txt"), "stashed\n").unwrap();
    push(&r, None, false);
    fs::write(d.join("a.txt"), "local\n").unwrap();
    let res = apply(&r, &first_hash(&list(&r)));
    assert_eq!(res["error"], "dirty", "{res}");
    assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "local\n");
}

#[test]
fn undo_commit_keeps_the_files_staged_and_returns_the_message() {
    let d = repo("undo");
    let r = root(&d);
    commit_file(&d, "b.txt", "b\n", "second\n\nbody line");
    let head = sh(&d, &["rev-parse", "HEAD"]).trim().to_string();
    let res = undo_commit(&r, &json!(head));
    assert_eq!(res["ok"], true, "{res}");
    assert_eq!(res["subject"], "second");
    assert_eq!(res["message"], "second\n\nbody line");
    assert!(sh(&d, &["status", "--porcelain"]).contains("A  b.txt"));
    assert_eq!(sh(&d, &["log", "-1", "--format=%s"]).trim(), "first");
    assert_eq!(fs::read_to_string(d.join("b.txt")).unwrap(), "b\n");
}

#[test]
fn undo_commit_is_refused_for_a_pushed_stale_root_or_merge_commit() {
    let d = repo("refuse");
    let r = root(&d);
    let first = sh(&d, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(undo_commit(&r, &json!(first))["error"], "root-commit");

    commit_file(&d, "b.txt", "b\n", "second");
    let second = sh(&d, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(undo_commit(&r, &json!(first))["error"], "stale");
    assert_eq!(undo_commit(&r, &json!("nothex!"))["error"], "denied");

    let remote = tmp("remote");
    sh(&remote, &["init", "-q", "--bare", "-b", "main"]);
    sh(&d, &["remote", "add", "origin", &root(&remote)]);
    sh(&d, &["push", "-q", "origin", "main"]);
    sh(&d, &["fetch", "-q", "origin"]);
    let res = undo_commit(&r, &json!(second));
    assert_eq!(res["error"], "pushed", "{res}");
    assert_eq!(sh(&d, &["log", "-1", "--format=%s"]).trim(), "second");

    sh(&d, &["checkout", "-q", "-b", "side"]);
    commit_file(&d, "c.txt", "c\n", "side work");
    sh(&d, &["checkout", "-q", "main"]);
    commit_file(&d, "d.txt", "d\n", "main work");
    sh(&d, &["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    sh(&d, &["update-ref", "-d", "refs/remotes/origin/main"]);
    let merge = sh(&d, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(undo_commit(&r, &json!(merge))["error"], "merge");
    assert_eq!(sh(&d, &["log", "-1", "--format=%s"]).trim(), "merge side");
}

#[test]
fn push_refuses_a_repo_without_commits_and_bad_names() {
    let d = tmp("empty");
    sh(&d, &["init", "-q", "-b", "main"]);
    fs::write(d.join("a.txt"), "x\n").unwrap();
    assert_eq!(push(&root(&d), None, false)["error"], "no-commit");
    let r = repo("names");
    assert_eq!(push(&root(&r), Some(&json!(5)), false)["error"], "denied");
    assert_eq!(push(&root(&r), Some(&json!("a\u{0}b")), false)["error"], "denied");
}
