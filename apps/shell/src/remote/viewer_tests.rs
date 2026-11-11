use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use core_desk::preview::paths::real_or_null;
use core_desk::{Ctx as DeskCtx, Desk, DeskError, DeskIo};
use serde_json::{json, Value};

use super::ctx::Ctx;
use super::devices::Device;
use super::scope;
use super::test_util::*;
use super::viewer_events::{merge, VEvent, WRITE_PER_MINUTE};
use super::viewer_map::{build_args, lookup, outcome, SPECS};
use crate::preview_host::{caps, Gate};
use crate::preview_text::confirm_spec;
use crate::preview_watch::{RootWatcher, Subs};
use crate::state::lock;

struct Io {
    gate: Gate,
    roots: Vec<String>,
}

impl DeskIo for Io {
    fn emit(&self, _event: &str, _payload: Value) {}

    fn host(&self, _req: u64, method: &str, p: Value) -> Result<Value, DeskError> {
        let roots = |_fresh: bool| self.roots.clone();
        let (method, p) = match method {
            "gate.dir" => ("preview.explorerRoot", json!({ "root": p["dir"] })),
            "gate.file" => ("preview.allowed", json!({ "path": p["path"] })),
            m => (m, p),
        };
        if let Some(v) = self.gate.answer(&roots, method, &p) {
            return Ok(v);
        }
        match method {
            "preview.caps" => Ok(caps("")),
            "preview.confirm" => {
                let c = confirm_spec("en", p["kind"].as_str().unwrap_or(""), &p["req"]).ok_or_else(|| DeskError::Host("spec".into()))?;
                Ok(json!(scope::confirm(&c.message, &c.detail, c.ok, c.cancel).unwrap_or(false)))
            }
            m => Err(DeskError::Host(format!("no host for {m}"))),
        }
    }
}

pub(super) struct Engine {
    desk: Desk,
    io: Io,
    pub(super) watcher: RootWatcher,
    subs: Arc<Subs>,
}

impl Engine {
    fn viewer(&self, owner: &str, channel: &str, args: Value, accept: &[String]) -> Value {
        let guard = scope::enter(owner, accept);
        let a0 = args.get(0).cloned().unwrap_or(Value::Null);
        let raw = match channel {
            "ccm:watch" => {
                let roots = |_f: bool| self.io.roots.clone();
                match self.io.gate.allowed(&roots, &a0, false) {
                    Some((real, _)) if self.watcher.add(&real) => {
                        self.subs.hold(owner, &real);
                        Ok(json!(true))
                    }
                    _ => Ok(json!(false)),
                }
            }
            "ccm:unwatch" => match self.subs.release_one(owner, a0.as_str().unwrap_or("")) {
                Some(held) => {
                    self.watcher.remove(&held);
                    Ok(json!(true))
                }
                None => Ok(json!(false)),
            },
            _ => self.desk.dispatch_with(&DeskCtx::new(&self.io, 0), channel, args).map_err(|e| e.to_string()),
        };
        if let Some(need) = guard.finish() {
            return json!({ "needsConfirm": need });
        }
        match raw {
            Ok(v) => json!({ "raw": v }),
            Err(_) => json!({ "error": "internal" }),
        }
    }

    fn release(&self, owner: &str) {
        for root in self.subs.release_all(owner) {
            self.watcher.remove(&root);
        }
    }
}

pub(super) fn real(p: &Path) -> String {
    real_or_null(&p.to_string_lossy()).unwrap()
}

pub(super) fn engine(rig: &Rig, roots: Vec<String>) -> Arc<Engine> {
    let ctx: Weak<Ctx> = Arc::downgrade(&rig.remote.ctx().unwrap());
    let subs = Arc::new(Subs::default());
    let (listen_subs, listen_ctx) = (subs.clone(), ctx);
    let watcher = RootWatcher::new(
        Arc::new(move |root: &str, paths: Vec<String>| {
            let Some(ctx) = listen_ctx.upgrade() else { return };
            for owner in listen_subs.owners_of(root) {
                ctx.viewer.push(&owner, VEvent::Fs { root: root.to_owned(), paths: paths.clone() });
            }
        }),
        Duration::from_millis(60),
    );
    let desk = crate::desk::boot(&crate::desk::env_for(&temp_dir("desk"), "1.0.0")).desk;
    let e = Arc::new(Engine { desk, io: Io { gate: Gate::in_memory(), roots }, watcher, subs });
    let (a, b) = (e.clone(), e.clone());
    *lock(&rig.host.viewer) = Some(Box::new(move |o, c, args, acc| a.viewer(o, c, args, acc)));
    *lock(&rig.host.on_release) = Some(Box::new(move |o| b.release(o)));
    e
}

pub(super) fn paired(rig: &Rig, phone: &Phone) {
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
}

pub(super) async fn open(rig: &Rig, id: &str) -> (Ws, core_remote_proto::Transport) {
    let phone = Phone::new(id);
    paired(rig, &phone);
    let (mut ws, mut t) = session(rig.addr(), &rig.desktop_key(), &phone).await.unwrap();
    assert_eq!(hello(&mut ws, &mut t).await["ok"], true);
    (ws, t)
}

pub(super) async fn ask(ws: &mut Ws, t: &mut core_remote_proto::Transport, id: &str, mut cmd: Value) -> Value {
    cmd["id"] = Value::from(id);
    send_json(ws, t, cmd).await;
    let want = id.to_owned();
    until_json(ws, t, move |v| v["re"] == want.as_str()).await
}

pub(super) fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub(super) fn branches(dir: &Path) -> String {
    String::from_utf8_lossy(&Command::new("git").arg("-C").arg(dir).args(["branch", "--list"]).output().unwrap().stdout).into_owned()
}

#[test]
fn the_command_table_is_closed_and_never_reaches_a_native_picker() {
    let banned = ["grant-folder", "request-access", "forget-grants", "full-access", "project-root", "create-entry", "rename-entry", "trash-entry", "search", "backup", "agent", "open"];
    for s in SPECS.iter() {
        assert!(s.channel.starts_with("ccm:"), "{}", s.t);
        assert!(!banned.iter().any(|b| s.channel.contains(b)), "{}", s.channel);
        assert_eq!(lookup(s.t).unwrap().channel, s.channel);
    }
    for t in ["fs.grant", "fs.open", "fs.reveal", "ws.open", "pty.attach", "ccm:read-file", "fs.read ", ""] {
        assert!(lookup(t).is_none(), "{t}");
    }
    let read = lookup("fs.read").unwrap();
    assert_eq!(build_args(read, &json!({ "t": "fs.read", "id": "x", "path": "C:/a" })), Some(json!(["C:/a"])));
    assert_eq!(build_args(read, &json!({ "path": 5 })), None);
    assert_eq!(build_args(read, &json!({ "path": "x".repeat(5000) })), None);
    let write = lookup("fs.write").unwrap();
    assert_eq!(build_args(write, &json!({ "path": "a", "text": "b", "base": 5, "evil": 1 })), Some(json!([{ "path": "a", "text": "b", "expectedMtime": 5 }])));
    let obj = build_args(lookup("git.diff").unwrap(), &json!({ "id": "1", "t": "git.diff", "confirmed": true, "_accept": ["k"], "root": "r" }));
    assert_eq!(obj, Some(json!([{ "root": "r" }])));
}

#[test]
fn desk_results_map_to_phone_outcomes() {
    assert_eq!(outcome(&json!({ "raw": { "kind": "text", "text": "a" } })), json!({ "ok": true, "data": { "kind": "text", "text": "a" } }));
    assert_eq!(outcome(&json!({ "raw": null }))["error"], "denied");
    assert_eq!(outcome(&json!({ "raw": false }))["error"], "denied");
    assert_eq!(outcome(&json!({ "raw": { "kind": "denied" } }))["error"], "denied");
    assert_eq!(outcome(&json!({ "raw": { "ok": false, "error": "denied" } }))["error"], "denied");
    assert_eq!(outcome(&json!({ "raw": { "ok": false, "error": "cancelled" } })), json!({ "ok": true, "cancelled": true }));
    assert_eq!(outcome(&json!({ "raw": { "ok": false, "error": "conflict" } }))["data"]["error"], "conflict");
    assert_eq!(outcome(&json!({ "error": "offline" }))["error"], "offline");
    assert!(outcome(&json!({ "needsConfirm": { "title": "t" } })).get("needsConfirm").is_some());
    assert_eq!(outcome(&json!({}))["error"], "internal");
}

#[test]
fn events_are_merged_per_root_and_keep_their_order() {
    let run = Arc::new(json!({ "type": "log" }));
    let out = merge(vec![
        VEvent::Fs { root: "/a".into(), paths: vec!["/a/1".into(), "/a/2".into()] },
        VEvent::Git { root: "/a".into() },
        VEvent::Fs { root: "/a".into(), paths: vec!["/a/2".into(), "/a/3".into()] },
        VEvent::Git { root: "/a".into() },
        VEvent::Run(run.clone()),
        VEvent::Fs { root: "/b".into(), paths: vec![] },
        VEvent::Run(run),
    ]);
    assert_eq!(out.len(), 5);
    assert_eq!(out[0], json!({ "t": "fs.changed", "root": "/a", "paths": ["/a/1", "/a/2", "/a/3"] }));
    assert_eq!(out[1], json!({ "t": "git.changed", "root": "/a" }));
    assert_eq!(out[2]["t"], "run.event");
    assert_eq!(out[3], json!({ "t": "fs.changed", "root": "/b", "paths": [] }));
}

#[test]
fn files_are_read_and_written_only_inside_the_pane_roots_and_unknown_commands_are_denied() {
    let rig = rig("viewer-fs", "", false);
    rig.remote.enable(true);
    let work = temp_dir("work");
    let outside = temp_dir("outside");
    std::fs::write(work.join("a.txt"), "hello").unwrap();
    std::fs::write(outside.join("secret.txt"), "nope").unwrap();
    let (root, a, secret) = (real(&work), real(&work.join("a.txt")), real(&outside.join("secret.txt")));
    engine(&rig, vec![root.clone()]);
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "p").await;
        let r = ask(&mut ws, &mut t, "r1", json!({ "t": "fs.read", "path": a })).await;
        assert_eq!((r["ok"].clone(), r["data"]["kind"].clone(), r["data"]["text"].clone()), (json!(true), json!("text"), json!("hello")));
        assert_eq!(ask(&mut ws, &mut t, "r2", json!({ "t": "fs.read", "path": secret })).await["error"], "denied");
        assert_eq!(ask(&mut ws, &mut t, "r3", json!({ "t": "fs.read", "path": 5 })).await["error"], "invalid");
        let mt = r["data"]["mtimeMs"].clone();
        let w = ask(&mut ws, &mut t, "w1", json!({ "t": "fs.write", "path": a, "text": "changed", "base": mt })).await;
        assert_eq!(w["ok"], true, "{w}");
        let back = ask(&mut ws, &mut t, "r4", json!({ "t": "fs.read", "path": a })).await;
        assert_eq!(back["data"]["text"], "changed");
        assert_eq!(std::fs::read_to_string(work.join("a.txt")).unwrap(), "changed");
        assert_eq!(ask(&mut ws, &mut t, "w2", json!({ "t": "fs.write", "path": secret, "text": "pwned", "base": 1 })).await["error"], "denied");
        assert_eq!(std::fs::read_to_string(outside.join("secret.txt")).unwrap(), "nope");
        let l = ask(&mut ws, &mut t, "l1", json!({ "t": "fs.list", "root": root, "dir": root })).await;
        assert!(l["data"].to_string().contains("a.txt"), "{l}");
        assert_eq!(ask(&mut ws, &mut t, "l2", json!({ "t": "fs.list", "root": root, "dir": real(&outside) })).await["error"], "denied");
        let c = ask(&mut ws, &mut t, "c1", json!({ "t": "run.configs", "root": root })).await;
        assert_eq!(c["ok"], true, "{c}");
        assert!(c["data"].is_array(), "{c}");
        for t_name in ["fs.grant", "fs.open", "fs.reveal", "ccm:read-file", "pty.nope"] {
            let id = format!("n-{t_name}");
            assert_eq!(ask(&mut ws, &mut t, &id, json!({ "t": t_name, "path": a })).await["error"], "denied", "{t_name}");
        }
        let mut ok = 0;
        for i in 0..WRITE_PER_MINUTE + 2 {
            let r = ask(&mut ws, &mut t, &format!("m{i}"), json!({ "t": "fs.write", "path": a, "text": format!("v{i}"), "base": 1 })).await;
            if r["ok"] == true {
                ok += 1;
            } else {
                assert_eq!(r["error"], "rate");
            }
        }
        assert!((WRITE_PER_MINUTE - 3..WRITE_PER_MINUTE).contains(&ok), "{ok}");
    });
    rig.remote.enable(false);
}

#[test]
fn watch_events_reach_only_the_watching_device_and_the_watcher_goes_with_the_session() {
    let rig = rig("viewer-watch", "", false);
    rig.remote.enable(true);
    let work = temp_dir("watch");
    let root = real(&work);
    let e = engine(&rig, vec![root.clone()]);
    runtime().block_on(async {
        let (mut ws_a, mut ta) = open(&rig, "dev-a").await;
        let (mut ws_b, mut tb) = open(&rig, "dev-b").await;
        assert_eq!(ask(&mut ws_a, &mut ta, "w", json!({ "t": "fs.watch", "root": root })).await["data"], true);
        assert_eq!(ask(&mut ws_a, &mut ta, "w-again", json!({ "t": "fs.watch", "root": root })).await["data"], true);
        assert_eq!(ask(&mut ws_b, &mut tb, "none", json!({ "t": "fs.unwatch", "root": root })).await["error"], "denied");
        assert!(e.watcher.has(&root));
        tokio::time::sleep(Duration::from_millis(400)).await;
        std::fs::write(work.join("new.txt"), "x").unwrap();
        let ev = until_json(&mut ws_a, &mut ta, |v| v["t"] == "fs.changed").await;
        assert_eq!(ev["root"], root.as_str());
        assert!(ev["paths"].to_string().contains("new.txt"), "{ev}");
        let other = tokio::time::timeout(Duration::from_millis(1500), until_json(&mut ws_b, &mut tb, |v| v["t"] == "fs.changed")).await;
        assert!(other.is_err());
        drop((ws_a, ta));
        let start = Instant::now();
        while e.watcher.has(&root) && start.elapsed() < Duration::from_secs(8) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(!e.watcher.has(&root));
    });
    assert!(lock(&rig.host.released).iter().any(|o| o.starts_with("remote:dev-a:")));
    rig.remote.enable(false);
}

#[test]
fn run_events_go_only_to_devices_that_asked_for_run_sessions() {
    let rig = rig("viewer-run", "", false);
    rig.remote.enable(true);
    let work = temp_dir("run");
    engine(&rig, vec![real(&work)]);
    let ctx = rig.remote.ctx().unwrap();
    runtime().block_on(async {
        let (mut ws_a, mut ta) = open(&rig, "run-a").await;
        let (mut ws_b, mut tb) = open(&rig, "run-b").await;
        let s = ask(&mut ws_a, &mut ta, "s", json!({ "t": "run.sessions" })).await;
        assert!(s["data"].is_array(), "{s}");
        assert_eq!(ask(&mut ws_b, &mut tb, "c", json!({ "t": "run.configs", "root": real(&work) })).await["ok"], true);
        ctx.viewer.push_run(&json!({ "type": "log", "sessionId": "x" }));
        let ev = until_json(&mut ws_a, &mut ta, |v| v["t"] == "run.event").await;
        assert_eq!(ev["event"]["sessionId"], "x");
        assert!(tokio::time::timeout(Duration::from_millis(800), until_json(&mut ws_b, &mut tb, |v| v["t"] == "run.event")).await.is_err());
    });
    rig.remote.enable(false);
}

#[test]
fn branch_delete_asks_the_sending_phone_and_never_a_native_box() {
    let rig = rig("viewer-branch", "", false);
    rig.remote.enable(true);
    let repo = temp_dir("repo");
    assert!(git(&repo, &["init", "-q", "-b", "main"]));
    std::fs::write(repo.join("f.txt"), "1").unwrap();
    assert!(git(&repo, &["add", "."]) && git(&repo, &["commit", "-q", "-m", "one"]));
    assert!(git(&repo, &["branch", "b2"]) && git(&repo, &["branch", "b3"]));
    let root = real(&repo);
    engine(&rig, vec![root.clone()]);
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "git-phone").await;
        let st = ask(&mut ws, &mut t, "st", json!({ "t": "git.status", "root": root })).await;
        assert_eq!(st["ok"], true, "{st}");
        let first = ask(&mut ws, &mut t, "d1", json!({ "t": "branch.delete", "root": root, "ref": "refs/heads/b2" })).await;
        let cid = first["confirm"].as_str().expect("confirm cid").to_owned();
        let asked = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!((asked["cid"].as_str(), asked["re"].as_str()), (Some(cid.as_str()), Some("d1")));
        assert!(asked["title"].as_str().is_some_and(|s| s.contains("b2")), "{asked}");
        assert!(branches(&repo).contains("b2"));
        let done = ask(&mut ws, &mut t, "a1", json!({ "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!(done["ok"], true, "{done}");
        assert!(!branches(&repo).contains("b2"), "{}", branches(&repo));
        let again = ask(&mut ws, &mut t, "d2", json!({ "t": "branch.delete", "root": root, "ref": "refs/heads/b3" })).await;
        let cid2 = again["confirm"].as_str().unwrap().to_owned();
        let no = ask(&mut ws, &mut t, "a2", json!({ "t": "confirm.answer", "cid": cid2, "ok": false })).await;
        assert_eq!(no["cancelled"], true);
        assert!(branches(&repo).contains("b3"));
        std::fs::write(repo.join("f.txt"), "2").unwrap();
        let dis = ask(&mut ws, &mut t, "x1", json!({ "t": "git.discard", "root": root, "path": "f.txt" })).await;
        let cid3 = dis["confirm"].as_str().expect("discard asks first").to_owned();
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask" && v["cid"] == cid3.as_str()).await;
        assert_eq!(q["title"], "Discard changes?");
        let no = ask(&mut ws, &mut t, "x2", json!({ "t": "confirm.answer", "cid": cid3, "ok": false })).await;
        assert_eq!(no["cancelled"], true);
        assert_eq!(std::fs::read_to_string(repo.join("f.txt")).unwrap(), "2");
    });
    assert!(lock(&rig.host.asked).is_empty());
    let views = lock(&rig.host.views).clone();
    assert!(views.iter().all(|(owner, ..)| owner.starts_with("remote:git-phone:")));
    assert!(views.iter().filter(|(_, c, ..)| c == "ccm:git-branch-delete").all(|(.., acc)| acc.len() <= 1));
    rig.remote.enable(false);
}

#[test]
fn a_reply_too_big_for_one_message_becomes_a_limit_error_and_the_session_survives() {
    let rig = rig("viewer-big", "", false);
    rig.remote.enable(true);
    *lock(&rig.host.viewer) = Some(Box::new(|_, _, _, _| json!({ "raw": { "kind": "text", "text": "x".repeat(950_000) } })));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "big").await;
        assert_eq!(ask(&mut ws, &mut t, "b", json!({ "t": "fs.read", "path": "C:/x" })).await["error"], "limit");
        assert_eq!(ask(&mut ws, &mut t, "z", json!({ "t": "nope" })).await["error"], "denied");
    });
    rig.remote.enable(false);
}

#[test]
fn one_merged_event_stays_under_64_kb_and_says_more() {
    let long = "x".repeat(2000);
    let paths: Vec<String> = (0..150).map(|i| format!("/a/{i}/{long}")).collect();
    let out = merge(vec![VEvent::Fs { root: "/a".into(), paths }]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["more"], true);
    assert!(serde_json::to_vec(&out[0]).unwrap().len() <= 64 * 1024);
    assert!(!out[0]["paths"].as_array().unwrap().is_empty());
    let small = merge(vec![VEvent::Fs { root: "/a".into(), paths: vec!["/a/1".into()] }]);
    assert!(small[0].get("more").is_none());
}

#[test]
fn a_run_confirm_from_a_phone_command_goes_to_that_phone_and_the_answer_wakes_the_runner() {
    let rig = rig("viewer-ask", "", false);
    rig.remote.enable(true);
    let work = temp_dir("ask");
    let root = real(&work);
    let ctx = rig.remote.ctx().unwrap();
    let hub = ctx.clone();
    *lock(&rig.host.viewer) = Some(Box::new(move |owner, _, _, _| {
        let runner = {
            let (hub, owner) = (hub.clone(), owner.to_owned());
            std::thread::spawn(move || hub.viewer.ask(&owner, &json!({ "title": "Reinstall?", "detail": "com.app", "ok": "Yes", "cancel": "No" }), Duration::from_secs(10)))
        };
        json!({ "raw": runner.join().unwrap() })
    }));
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "ask-phone").await;
        send_json(&mut ws, &mut t, json!({ "id": "r", "t": "run.start", "root": root, "configId": "c" })).await;
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!((q["title"].clone(), q["ok"].clone()), (json!("Reinstall?"), json!("Yes")));
        let cid = q["cid"].as_str().unwrap().to_owned();
        let done = ask(&mut ws, &mut t, "y", json!({ "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!(done["ok"], true, "{done}");
        let r = until_json(&mut ws, &mut t, |v| v["re"] == "r").await;
        assert_eq!((r["ok"].clone(), r["data"].clone()), (json!(true), json!(true)), "{r}");

        send_json(&mut ws, &mut t, json!({ "id": "r2", "t": "run.start", "root": root, "configId": "c" })).await;
        let q = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        let cid = q["cid"].as_str().unwrap().to_owned();
        let done = ask(&mut ws, &mut t, "n", json!({ "t": "confirm.answer", "cid": cid, "ok": false })).await;
        assert_eq!(done["cancelled"], true, "{done}");
        let r = until_json(&mut ws, &mut t, |v| v["re"] == "r2").await;
        assert_eq!(r["error"], "denied", "{r}");
    });
    rig.remote.enable(false);
}

#[test]
fn a_run_confirm_with_no_open_phone_session_is_refused_at_once() {
    let rig = rig("viewer-ask-none", "", false);
    rig.remote.enable(true);
    let ctx = rig.remote.ctx().unwrap();
    let started = Instant::now();
    assert!(!ctx.viewer.ask("remote:gone:x", &json!({}), Duration::from_secs(10)));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!ctx.viewer.answer("nope", true));
    rig.remote.enable(false);
}
