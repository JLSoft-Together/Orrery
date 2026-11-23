use super::*;
use std::process::Command;

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("orrery-cl-{tag}-{}-{}", std::process::id(), crate::util::now_ms() as u64));
    fs::create_dir_all(&dir).unwrap();
    dir
}

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

#[test]
fn the_key_ignores_case_slashes_and_a_trailing_separator() {
    let a = key("C:\\Work\\Proj");
    assert_eq!(a.len(), 16);
    if cfg!(windows) {
        assert_eq!(a, key("c:/work/proj/"));
    }
    assert_ne!(a, key("C:\\Work\\Other"));
}

#[test]
fn items_are_added_toggled_and_removed_per_worktree() {
    let data = tmp("crud");
    let (a, b) = ("/wt/a", "/wt/b");
    let one = add(&data, a, "owner", "  write the   tests ", 5.0).unwrap();
    assert_eq!(one["text"], "write the tests");
    assert_eq!(one["by"], "owner");
    assert_eq!(one["done"], false);
    let two = add(&data, a, "agent", "update docs", 6.0).unwrap();
    assert_ne!(one["id"], two["id"]);
    assert_eq!(list(&data, a)["items"].as_array().unwrap().len(), 2);
    assert!(list(&data, b)["items"].as_array().unwrap().is_empty());
    let id = one["id"].as_str().unwrap();
    assert_eq!(set_done(&data, a, id, None).unwrap()["done"], true);
    assert_eq!(set_done(&data, a, id, None).unwrap()["done"], false);
    assert_eq!(set_done(&data, a, id, Some(true)).unwrap()["done"], true);
    assert_eq!(set_done(&data, a, "nope", None), Err("unknown"));
    remove(&data, a, id, "owner").unwrap();
    assert_eq!(list(&data, a)["items"].as_array().unwrap().len(), 1);
    let id3 = add(&data, a, "owner", "again", 7.0).unwrap()["id"].clone();
    assert_ne!(id3, one["id"]);
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn an_agent_cannot_remove_what_the_owner_wrote_but_the_owner_can_remove_both() {
    let data = tmp("perm");
    let mine = add(&data, "/wt/a", "owner", "owner item", 1.0).unwrap();
    let theirs = add(&data, "/wt/a", "agent", "agent item", 2.0).unwrap();
    assert_eq!(remove(&data, "/wt/a", mine["id"].as_str().unwrap(), "agent"), Err("owner"));
    assert_eq!(list(&data, "/wt/a")["items"].as_array().unwrap().len(), 2);
    remove(&data, "/wt/a", theirs["id"].as_str().unwrap(), "agent").unwrap();
    remove(&data, "/wt/a", mine["id"].as_str().unwrap(), "owner").unwrap();
    assert!(list(&data, "/wt/a")["items"].as_array().unwrap().is_empty());
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn text_and_size_are_limited() {
    let data = tmp("limit");
    assert_eq!(add(&data, "/wt/a", "owner", "   ", 1.0), Err("text"));
    assert_eq!(add(&data, "/wt/a", "owner", "a\u{0}b\nc", 1.0).unwrap()["text"], "a b c");
    let long = "x".repeat(500);
    assert_eq!(add(&data, "/wt/a", "owner", &long, 1.0).unwrap()["text"].as_str().unwrap().chars().count(), 200);
    for i in 0..MAX_ITEMS - 2 {
        add(&data, "/wt/a", "owner", &format!("i{i}"), 1.0).unwrap();
    }
    assert_eq!(add(&data, "/wt/a", "owner", "one too many", 1.0), Err("full"));
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn the_check_keeps_its_script_state_and_head() {
    let data = tmp("check");
    let fresh = list(&data, "/wt/a");
    assert_eq!(fresh["check"]["state"], "idle");
    set_script(&data, "/wt/a", "npm run qa").unwrap();
    set_check(&data, "/wt/a", "pass", "abc1234", 9.0).unwrap();
    let c = list(&data, "/wt/a")["check"].clone();
    assert_eq!((c["script"].as_str(), c["state"].as_str(), c["head"].as_str(), c["at"].as_f64()), (Some("npm run qa"), Some("pass"), Some("abc1234"), Some(9.0)));
    assert_eq!(set_check(&data, "/wt/a", "weird", "abc1234", 1.0), Err("state"));
    assert_eq!(set_check(&data, "/wt/a", "pass", "not a head", 1.0), Err("head"));
    set_script(&data, "/wt/a", "npm test").unwrap();
    let c = list(&data, "/wt/a")["check"].clone();
    assert_eq!((c["script"].as_str(), c["state"].as_str(), c["head"].as_str()), (Some("npm test"), Some("idle"), Some("")));
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn a_broken_or_oversized_file_reads_as_empty_and_is_replaced_on_write() {
    let data = tmp("broken");
    let path = file_of(&data, "/wt/a");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{not json").unwrap();
    assert!(list(&data, "/wt/a")["items"].as_array().unwrap().is_empty());
    add(&data, "/wt/a", "owner", "ok", 1.0).unwrap();
    assert_eq!(list(&data, "/wt/a")["items"].as_array().unwrap().len(), 1);
    fs::write(&path, vec![b' '; (MAX_FILE + 1) as usize]).unwrap();
    assert!(list(&data, "/wt/a")["items"].as_array().unwrap().is_empty());
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn the_ipc_reply_wraps_each_op_and_stamps_the_head() {
    let data = tmp("ipc");
    let repo = tmp("repo");
    sh(&repo, &["init", "-q", "-b", "main"]);
    fs::write(repo.join("a.txt"), "a").unwrap();
    sh(&repo, &["add", "a.txt"]);
    sh(&repo, &["commit", "-q", "-m", "init"]);
    let root = repo.to_string_lossy().into_owned();
    let r = reply(&data, &root, &json!({ "op": "add", "text": "ship it" }));
    assert_eq!(r["ok"], true);
    let head = r["head"].as_str().unwrap().to_string();
    assert_eq!(head.len(), 40);
    let id = r["items"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(reply(&data, &root, &json!({ "op": "toggle", "id": id }))["items"][0]["done"], true);
    assert_eq!(reply(&data, &root, &json!({ "op": "setScript", "script": "qa" }))["check"]["script"], "qa");
    let c = reply(&data, &root, &json!({ "op": "setCheck", "state": "fail", "head": head }));
    assert_eq!(c["check"]["state"], "fail");
    assert_eq!(reply(&data, &root, &json!({ "op": "setCheck", "state": "x", "head": "abcd" }))["error"], "state");
    assert_eq!(reply(&data, &root, &json!({ "op": "remove", "id": id }))["items"].as_array().unwrap().len(), 0);
    assert_eq!(reply(&data, &root, &json!({ "op": "bogus" }))["ok"], false);
    fs::write(repo.join("b.txt"), "b").unwrap();
    sh(&repo, &["add", "b.txt"]);
    sh(&repo, &["commit", "-q", "-m", "more"]);
    let after = reply(&data, &root, &json!({ "op": "list" }));
    assert_ne!(after["head"].as_str().unwrap(), head);
    assert_eq!(after["check"]["head"], head);
    let _ = fs::remove_dir_all(&data);
    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn the_toplevel_of_a_subfolder_is_the_worktree_root() {
    let repo = tmp("top");
    sh(&repo, &["init", "-q", "-b", "main"]);
    let sub = repo.join("src").join("deep");
    fs::create_dir_all(&sub).unwrap();
    let top = toplevel(&sub.to_string_lossy());
    assert_eq!(normalize(&top), normalize(&repo.to_string_lossy()));
    let plain = tmp("plain");
    assert_eq!(toplevel(&plain.to_string_lossy()), plain.to_string_lossy());
    let _ = fs::remove_dir_all(&repo);
    let _ = fs::remove_dir_all(&plain);
}

#[test]
fn only_a_listed_worktree_of_the_repo_is_accepted() {
    let base = tmp("wtof");
    let repo = base.join("proj");
    fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    fs::write(repo.join("a.txt"), "a").unwrap();
    sh(&repo, &["add", "a.txt"]);
    sh(&repo, &["commit", "-q", "-m", "init"]);
    let wt = base.join("proj-wt").join("feat");
    sh(&repo, &["worktree", "add", "-q", "-b", "feat", &wt.to_string_lossy()]);
    let other = base.join("other");
    fs::create_dir_all(&other).unwrap();
    let root = repo.to_string_lossy().into_owned();
    let found = worktree_of(&root, &wt.to_string_lossy()).unwrap();
    assert_eq!(normalize(&found), normalize(&wt.to_string_lossy()));
    assert!(worktree_of(&root, &other.to_string_lossy()).is_none());
    assert!(worktree_of(&root, &repo.join("..").join("proj-wt").join("feat").to_string_lossy()).is_some());
    let _ = fs::remove_dir_all(&base);
}
