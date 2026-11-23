use std::process::Command;

use super::*;

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn sh(dir: &Path, args: &[&str]) {
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
}

fn repo(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("orrery-wt-{tag}-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
    let dir = base.join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    sh(&dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("a.txt"), "a").unwrap();
    std::fs::write(dir.join(".env"), "SECRET=1").unwrap();
    sh(&dir, &["add", "a.txt"]);
    sh(&dir, &["commit", "-q", "-m", "init"]);
    resolve_cwd(Some(&dir.to_string_lossy()), &std::env::temp_dir()).unwrap()
}

fn ask(dir: &Path, msg: Value, panes: &[PaneDir]) -> Value {
    let mut m = msg;
    m["cwd"] = json!(dir.to_string_lossy());
    answer(&m, panes, &std::env::temp_dir())
}

#[test]
fn porcelain_is_parsed_like_the_electron_broker() {
    let text = "worktree /r/proj\nHEAD abc\nbranch refs/heads/main\n\nworktree /r/proj-wt/x\nHEAD def\nbranch refs/heads/x\nlocked reason\n\nworktree /r/gone\nHEAD 000\ndetached\nprunable gitdir file points to non-existent location\n\n";
    let list = parse_worktrees(text);
    assert_eq!(list.len(), 3);
    assert!(list[0].main && !list[1].main);
    assert_eq!(list[1].branch.as_deref(), Some("x"));
    assert!(list[1].locked && !list[1].prunable);
    assert!(list[2].prunable && list[2].branch.is_none());
    assert_eq!(list[1].to_json()["head"], "def");
    assert!(list[2].to_json().get("branch").is_none());
}

#[test]
fn the_destination_sits_next_to_the_repo_and_flattens_slashes() {
    assert_eq!(worktree_dest("/w/proj", "feat/login"), Path::new("/w").join("proj-wt").join("feat-login"));
}

#[test]
fn inside_checks_whole_path_components() {
    assert!(is_inside("/a/b/c", "/a/b"));
    assert!(is_inside("/a/b", "/a/b"));
    assert!(!is_inside("/a/bc", "/a/b"));
    assert!(!is_inside("/a/b/../c", "/a/b"));
    assert!(!is_inside("/a", "/a/b"));
}

#[test]
fn a_folder_outside_git_lists_nothing_and_refuses_changes() {
    let plain = std::env::temp_dir().join(format!("orrery-wt-plain-{}", std::process::id()));
    std::fs::create_dir_all(&plain).unwrap();
    let plain = resolve_cwd(Some(&plain.to_string_lossy()), &std::env::temp_dir()).unwrap();
    let list = ask(&plain, json!({ "t": "wt.list" }), &[]);
    assert_eq!(list["repo"], Value::Null);
    assert_eq!(ask(&plain, json!({ "t": "wt.add", "branch": "x" }), &[])["code"], "notRepo");
    let no_cwd = answer(&json!({ "t": "wt.list" }), &[], &std::env::temp_dir());
    assert_eq!(no_cwd["code"], "notRepo");
    let _ = std::fs::remove_dir_all(&plain);
}

#[test]
fn a_worktree_is_added_listed_guarded_and_removed() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = repo("flow");
    let base = dir.parent().unwrap().to_path_buf();
    let listed = ask(&dir, json!({ "t": "wt.list" }), &[]);
    assert_eq!(listed["worktrees"].as_array().unwrap().len(), 1);
    assert_eq!(listed["worktrees"][0]["main"], true);
    assert_eq!(ask(&dir, json!({ "t": "wt.add", "branch": "-x" }), &[])["code"], "branch");
    assert_eq!(ask(&dir, json!({ "t": "wt.add", "branch": "main" }), &[])["code"], "exists");
    let added = ask(&dir, json!({ "t": "wt.add", "branch": "feat/a", "copy": [".env*"] }), &[]);
    assert_eq!(added["t"], "wt.added", "{added}");
    let wt = added["path"].as_str().unwrap().to_owned();
    assert!(wt.ends_with("feat-a"));
    assert_eq!(added["copied"], json!([".env"]));
    assert_eq!(std::fs::read_to_string(Path::new(&wt).join(".env")).unwrap(), "SECRET=1");
    assert_eq!(ask(&dir, json!({ "t": "wt.add", "branch": "feat/a" }), &[])["code"], "exists");
    assert_eq!(ask(&dir, json!({ "t": "wt.list" }), &[])["worktrees"].as_array().unwrap().len(), 2);
    let main_path = dir.to_string_lossy().into_owned();
    assert_eq!(ask(&dir, json!({ "t": "wt.remove", "path": main_path }), &[])["code"], "main");
    assert_eq!(ask(&dir, json!({ "t": "wt.remove", "path": base.join("nope").to_string_lossy() }), &[])["code"], "unknown");
    let inside = [PaneDir { cwd: format!("{wt}/sub"), live_cwd: None, exited: false }];
    assert_eq!(ask(&dir, json!({ "t": "wt.remove", "path": wt }), &inside)["code"], "busy");
    let gone = [PaneDir { cwd: wt.clone(), live_cwd: None, exited: true }];
    std::fs::write(Path::new(&wt).join("dirty.txt"), "x").unwrap();
    assert_eq!(ask(&dir, json!({ "t": "wt.remove", "path": wt }), &gone)["code"], "dirty");
    std::fs::remove_file(Path::new(&wt).join("dirty.txt")).unwrap();
    std::fs::remove_file(Path::new(&wt).join(".env")).unwrap();
    let removed = ask(&dir, json!({ "t": "wt.remove", "path": wt }), &gone);
    assert_eq!(removed["t"], "wt.removed", "{removed}");
    assert!(!Path::new(&wt).exists());
    assert_eq!(ask(&dir, json!({ "t": "wt.list" }), &[])["worktrees"].as_array().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_repo_config_overrides_the_copy_patterns_and_flags_setup() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = repo("cfg");
    let base = dir.parent().unwrap().to_path_buf();
    std::fs::write(dir.join("keep.cfg"), "k").unwrap();
    std::fs::create_dir_all(dir.join(".orrery")).unwrap();
    std::fs::write(dir.join(".orrery/worktree.json"), r#"{"copy": ["keep.cfg"], "setup": ["npm install"]}"#).unwrap();
    let added = ask(&dir, json!({ "t": "wt.add", "branch": "feat/cfg", "copy": [".env*"] }), &[]);
    assert_eq!(added["t"], "wt.added", "{added}");
    assert_eq!(added["copied"], json!(["keep.cfg"]));
    assert_eq!(added["setup"], true);
    let wt = added["path"].as_str().unwrap();
    assert!(!Path::new(wt).join(".env").exists());
    std::fs::write(dir.join(".orrery/worktree.json"), r#"{"copy": 5}"#).unwrap();
    let again = ask(&dir, json!({ "t": "wt.add", "branch": "feat/cfg2", "copy": [".env*"] }), &[]);
    assert_eq!(again["copied"], json!([".env"]));
    assert!(again.get("setup").is_none());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn removing_can_drop_the_branch_only_when_git_says_it_is_merged() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = repo("drop");
    let base = dir.parent().unwrap().to_path_buf();
    let merged = ask(&dir, json!({ "t": "wt.add", "branch": "merged" }), &[]);
    let kept = ask(&dir, json!({ "t": "wt.add", "branch": "unmerged" }), &[]);
    let (m, u) = (merged["path"].as_str().unwrap().to_owned(), kept["path"].as_str().unwrap().to_owned());
    std::fs::write(Path::new(&u).join("n.txt"), "n").unwrap();
    sh(Path::new(&u), &["add", "n.txt"]);
    sh(Path::new(&u), &["commit", "-q", "-m", "work"]);
    let gone = |p: &str| [PaneDir { cwd: p.to_owned(), live_cwd: None, exited: true }];
    let a = ask(&dir, json!({ "t": "wt.remove", "path": m, "deleteBranch": true }), &gone(&m));
    assert_eq!(a["t"], "wt.removed", "{a}");
    assert_eq!(a["branch"], "deleted");
    let b = ask(&dir, json!({ "t": "wt.remove", "path": u, "deleteBranch": true }), &gone(&u));
    assert_eq!(b["t"], "wt.removed", "{b}");
    assert_eq!(b["branch"], "kept");
    assert!(!b["branchDetail"].as_str().unwrap_or("").is_empty());
    let list = ask(&dir, json!({ "t": "wt.list" }), &[]);
    assert_eq!(list["worktrees"].as_array().unwrap().len(), 1);
    let plain = ask(&dir, json!({ "t": "wt.add", "branch": "plain" }), &[]);
    let p = plain["path"].as_str().unwrap().to_owned();
    let c = ask(&dir, json!({ "t": "wt.remove", "path": p }), &gone(&p));
    assert!(c.get("branch").is_none());
    let _ = std::fs::remove_dir_all(&base);
}
