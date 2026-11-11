use std::time::{Duration, Instant};

use serde_json::json;

use super::test_util::*;
use super::viewer_map::{build_args, lookup, local_path_ok, pre_needs, Pre};
use super::viewer_tests::{ask, engine, git, open, real};
use crate::state::lock;

#[test]
fn unc_and_device_paths_are_refused_before_any_filesystem_call() {
    for p in ["\\\\host\\share\\x", "//host/share/x", "\\\\?\\UNC\\host\\s\\x", "\\\\.\\pipe\\x", "a\0b"] {
        assert!(!local_path_ok(p), "{p}");
    }
    for p in ["C:\\work\\a", "C:/work/a", "/home/a", "work/a"] {
        assert!(local_path_ok(p), "{p}");
    }
    assert_eq!(build_args(lookup("fs.read").unwrap(), &json!({ "path": "\\\\h\\s\\x" })), None);
    assert_eq!(build_args(lookup("fs.files").unwrap(), &json!({ "root": "//h/x" })), None);
    assert_eq!(build_args(lookup("fs.list").unwrap(), &json!({ "root": "C:/a", "dir": "\\\\h\\s" })), None);
    assert_eq!(build_args(lookup("git.diff").unwrap(), &json!({ "root": "C:/a", "path": "//h/x" })), None);
    assert_eq!(build_args(lookup("log.diff").unwrap(), &json!({ "root": "C:/a", "origPath": "\\\\h\\s" })), None);
    assert_eq!(build_args(lookup("git.diff").unwrap(), &json!({ "root": "C:/a", "path": 5 })), None);
    assert!(build_args(lookup("git.diff").unwrap(), &json!({ "root": "C:/a", "path": "src/a.rs" })).is_some());
}

#[test]
fn a_push_confirm_names_the_remote_target_upstream_and_the_default_branch() {
    let cmd = json!({ "root": "/r", "ref": "refs/heads/feature", "remote": "origin", "branch": "main", "setUpstream": true, "force": true });
    let needs = pre_needs("en", Pre::Push, &cmd);
    let detail = needs["detail"].as_str().unwrap();
    for part in ["refs/heads/feature → origin/main", "Set upstream", "Force push", "default branch main"] {
        assert!(detail.contains(part), "{part} in {detail}");
    }
    let plain = pre_needs("en", Pre::Push, &json!({ "ref": "refs/heads/f", "remote": "origin", "branch": "f" }));
    let detail = plain["detail"].as_str().unwrap();
    assert!(detail.contains("origin/f") && !detail.contains("Warning") && !detail.contains("upstream"), "{detail}");
    let over = pre_needs("vi", Pre::Overwrite, &json!({ "path": "C:/a.txt" }));
    assert_eq!(over["title"], "Ghi đè file?");
}

#[test]
fn a_write_without_a_base_asks_the_phone_before_overwriting() {
    let rig = rig("viewer-overwrite", "", false);
    rig.remote.enable(true);
    let work = temp_dir("ow");
    std::fs::write(work.join("a.txt"), "old").unwrap();
    let (root, a) = (real(&work), real(&work.join("a.txt")));
    engine(&rig, vec![root]);
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "ow").await;
        let first = ask(&mut ws, &mut t, "w", json!({ "t": "fs.write", "path": a, "text": "new" })).await;
        let cid = first["confirm"].as_str().expect("asks first").to_owned();
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!(q["title"], "Overwrite file?");
        assert_eq!(std::fs::read_to_string(work.join("a.txt")).unwrap(), "old");
        let done = ask(&mut ws, &mut t, "y", json!({ "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!(done["ok"], true, "{done}");
        assert_eq!(std::fs::read_to_string(work.join("a.txt")).unwrap(), "new");
    });
    rig.remote.enable(false);
}

#[test]
fn a_push_asks_with_the_full_target_and_nothing_runs_until_confirmed() {
    let rig = rig("viewer-push", "", false);
    rig.remote.enable(true);
    let repo = temp_dir("pushrepo");
    assert!(git(&repo, &["init", "-q", "-b", "main"]));
    std::fs::write(repo.join("f.txt"), "1").unwrap();
    assert!(git(&repo, &["add", "."]) && git(&repo, &["commit", "-q", "-m", "one"]));
    let root = real(&repo);
    engine(&rig, vec![root.clone()]);
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "push").await;
        let cmd = json!({ "t": "branch.push", "root": root, "ref": "refs/heads/main", "remote": "origin", "branch": "main", "setUpstream": true });
        let first = ask(&mut ws, &mut t, "p", cmd).await;
        let cid = first["confirm"].as_str().unwrap().to_owned();
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        let detail = q["detail"].as_str().unwrap();
        assert!(detail.contains("origin/main") && detail.contains("Set upstream") && detail.contains("default branch"), "{detail}");
        let no = ask(&mut ws, &mut t, "n", json!({ "t": "confirm.answer", "cid": cid, "ok": false })).await;
        assert_eq!(no["cancelled"], true);
    });
    assert!(lock(&rig.host.views).iter().all(|(_, c, ..)| c != "ccm:git-push"));
    rig.remote.enable(false);
}

#[test]
fn one_slow_git_task_per_root_and_device_and_unc_never_reaches_the_host() {
    let rig = rig("viewer-busy", "", false);
    rig.remote.enable(true);
    *lock(&rig.host.viewer) = Some(Box::new(|_, c, a, _| {
        if c == "ccm:git-fetch" && a[0] == "C:/r1" {
            std::thread::sleep(Duration::from_millis(1500));
        }
        json!({ "raw": { "ok": true } })
    }));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "busy").await;
        for (id, root) in [("f1", "C:/r1"), ("f2", "C:/r1"), ("f3", "C:/r2")] {
            send_json(&mut ws, &mut t, json!({ "id": id, "t": "branch.fetch", "root": root })).await;
        }
        let mut got = std::collections::HashMap::new();
        while got.len() < 3 {
            let v = until_json(&mut ws, &mut t, |v| v["re"].is_string()).await;
            got.insert(v["re"].as_str().unwrap().to_owned(), v);
        }
        assert_eq!(got["f2"]["error"], "busy", "{got:?}");
        assert_eq!((got["f1"]["ok"].clone(), got["f3"]["ok"].clone()), (json!(true), json!(true)));

        assert_eq!(ask(&mut ws, &mut t, "f4", json!({ "t": "branch.fetch", "root": "C:/r1" })).await["ok"], true);
        let before = lock(&rig.host.views).len();
        assert_eq!(ask(&mut ws, &mut t, "u", json!({ "t": "fs.read", "path": "\\\\h\\s\\x" })).await["error"], "invalid");
        assert_eq!(lock(&rig.host.views).len(), before);
    });
    rig.remote.enable(false);
}

#[test]
fn watch_count_is_capped_and_keys_are_normalised() {
    let rig = rig("viewer-cap", "", false);
    rig.remote.enable(true);
    *lock(&rig.host.viewer) = Some(Box::new(|_, _, _, _| json!({ "raw": true })));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "cap").await;
        for i in 0..8 {
            let r = ask(&mut ws, &mut t, &format!("w{i}"), json!({ "t": "fs.watch", "root": format!("C:/r{i}") })).await;
            assert_eq!(r["data"], true, "{r}");
        }
        let calls = lock(&rig.host.views).len();
        assert_eq!(ask(&mut ws, &mut t, "same", json!({ "t": "fs.watch", "root": "C:/r3/" })).await["data"], true);
        assert_eq!(lock(&rig.host.views).len(), calls);
        assert_eq!(ask(&mut ws, &mut t, "nine", json!({ "t": "fs.watch", "root": "C:/r9" })).await["error"], "limit");
        assert_eq!(ask(&mut ws, &mut t, "un", json!({ "t": "fs.unwatch", "root": "C:/never" })).await["ok"], true);
        assert!(lock(&rig.host.views).len() > calls);
    });
    rig.remote.enable(false);
}

#[test]
fn a_watch_that_lands_after_the_session_closed_is_released_again() {
    let rig = rig("viewer-late", "", false);
    rig.remote.enable(true);
    *lock(&rig.host.viewer) = Some(Box::new(|_, _, _, _| {
        std::thread::sleep(Duration::from_millis(800));
        json!({ "raw": true })
    }));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "late").await;
        send_json(&mut ws, &mut t, json!({ "id": "w", "t": "fs.watch", "root": "C:/late" })).await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        drop((ws, t));
        let start = Instant::now();
        let count = |rig: &Rig| lock(&rig.host.released).iter().filter(|o| o.starts_with("remote:late:")).count();
        while count(&rig) < 2 && start.elapsed() < Duration::from_secs(5) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(count(&rig) >= 2, "{}", count(&rig));
    });
    rig.remote.enable(false);
}
