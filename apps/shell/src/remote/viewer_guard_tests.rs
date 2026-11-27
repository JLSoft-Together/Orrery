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

#[test]
fn checklist_ops_force_the_op_and_take_only_a_worktree_path_a_short_id_and_a_bool() {
    let get = lookup("checklist.get").unwrap();
    assert_eq!(build_args(get, &json!({ "dir": "C:/r", "wt": "C:/r-wt", "op": "setScript", "script": "rm -rf /" })), Some(json!([{ "op": "list", "dir": "C:/r", "wt": "C:/r-wt" }])));
    assert_eq!(build_args(get, &json!({ "dir": "C:/r" })), Some(json!([{ "op": "list", "dir": "C:/r" }])));
    assert_eq!(build_args(get, &json!({ "dir": "C:/r", "wt": "//h/s" })), None);
    assert_eq!(build_args(get, &json!({ "dir": "C:/r", "wt": 5 })), None);
    assert_eq!(build_args(get, &json!({ "wt": "C:/w" })), None);
    let toggle = lookup("checklist.toggle").unwrap();
    assert!(toggle.write);
    assert_eq!(build_args(toggle, &json!({ "dir": "C:/r", "id": "t1", "done": true, "op": "remove" })), Some(json!([{ "op": "toggle", "dir": "C:/r", "id": "t1", "done": true }])));
    assert_eq!(build_args(toggle, &json!({ "dir": "C:/r", "id": "t1" })), None);
    assert_eq!(build_args(toggle, &json!({ "dir": "C:/r", "done": true })), None);
    assert_eq!(build_args(toggle, &json!({ "dir": "C:/r", "id": "x".repeat(65), "done": true })), None);
    let run = lookup("checklist.run").unwrap();
    assert_eq!(run.pre, Some(Pre::CheckRun));
    let needs = pre_needs("vi", Pre::CheckRun, &json!({ "dir": "C:/r", "wt": "C:/r-wt" }));
    assert_eq!((needs["title"].clone(), needs["detail"].clone()), (json!("Chạy kiểm trên máy?"), json!("C:/r-wt")));
    assert_eq!(pre_needs("en", Pre::CheckRun, &json!({ "dir": "C:/r" }))["detail"], "C:/r");
}

#[test]
fn a_check_run_asks_the_phone_first_then_hands_the_worktree_to_the_page_once() {
    let rig = rig("viewer-check", "", false);
    rig.remote.enable(true);
    *lock(&rig.host.viewer) = Some(Box::new(|_, c, a, _| {
        assert_eq!(c, "ccm:checklist");
        assert_eq!(a[0]["op"], "list");
        if a[0]["dir"] == "C:/ok" { json!({ "raw": { "ok": true, "items": [] } }) } else { json!({ "raw": { "ok": false, "error": "denied" } }) }
    }));
    *lock(&rig.host.answer_with) = Some(Box::new(|_, _| Some(json!({ "ok": true, "started": true }))));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "chk").await;
        let first = ask(&mut ws, &mut t, "c1", json!({ "t": "checklist.run", "dir": "C:/ok", "wt": "C:/ok-wt" })).await;
        let cid = first["confirm"].as_str().expect("asks first").to_owned();
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!((q["title"].clone(), q["detail"].clone()), (json!("Run the check on the computer?"), json!("C:/ok-wt")));
        assert!(lock(&rig.host.asks).is_empty());
        let done = ask(&mut ws, &mut t, "y1", json!({ "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!((done["ok"].clone(), done["data"]["started"].clone()), (json!(true), json!(true)), "{done}");
        assert_eq!(lock(&rig.host.asks).clone(), vec![("checklist.run".to_owned(), json!({ "dir": "C:/ok", "wt": "C:/ok-wt" }))]);

        let second = ask(&mut ws, &mut t, "c2", json!({ "t": "checklist.run", "dir": "C:/ok" })).await;
        let cid2 = second["confirm"].as_str().unwrap().to_owned();
        until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!(ask(&mut ws, &mut t, "n2", json!({ "t": "confirm.answer", "cid": cid2, "ok": false })).await["cancelled"], true);
        assert_eq!(lock(&rig.host.asks).len(), 1);

        let out = ask(&mut ws, &mut t, "c3", json!({ "t": "checklist.run", "dir": "C:/outside" })).await;
        let cid3 = out["confirm"].as_str().unwrap().to_owned();
        until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        let denied = ask(&mut ws, &mut t, "y3", json!({ "t": "confirm.answer", "cid": cid3, "ok": true })).await;
        assert_eq!(denied["error"], "denied", "{denied}");
        assert_eq!(lock(&rig.host.asks).len(), 1);

        assert_eq!(ask(&mut ws, &mut t, "c4", json!({ "t": "checklist.run", "dir": "//h/s" })).await["error"], "invalid");
    });
    rig.remote.enable(false);
}

#[test]
fn the_checklist_reads_only_inside_the_pane_roots_and_never_takes_a_phone_chosen_op() {
    let rig = rig("viewer-checklist", "", false);
    rig.remote.enable(true);
    let repo = temp_dir("chkrepo");
    let outside = temp_dir("chkout");
    assert!(git(&repo, &["init", "-q", "-b", "main"]) && git(&outside, &["init", "-q", "-b", "main"]));
    let root = real(&repo);
    engine(&rig, vec![root.clone()]);
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "chkget").await;
        let ok = ask(&mut ws, &mut t, "g1", json!({ "t": "checklist.get", "dir": root, "op": "setScript", "script": "calc" })).await;
        assert_eq!((ok["ok"].clone(), ok["data"]["ok"].clone(), ok["data"]["check"]["script"].clone()), (json!(true), json!(true), json!("")), "{ok}");
        let no = ask(&mut ws, &mut t, "g2", json!({ "t": "checklist.get", "dir": real(&outside) })).await;
        assert_eq!(no["error"], "denied", "{no}");
        let gone = ask(&mut ws, &mut t, "g3", json!({ "t": "checklist.get", "dir": root, "wt": real(&outside) })).await;
        assert_eq!(gone["error"], "denied", "{gone}");
    });
    rig.remote.enable(false);
}
