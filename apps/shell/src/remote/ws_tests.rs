use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::confirm::{command_of, Confirms, CONFIRM_TTL};
use super::devices::Device;
use super::pair::{clean_name, is_hidden};
use super::patch::diff;
use super::phone_ws::{error_code, reply_of, SUMMARY_GET};
use super::test_util::*;
use super::workspace::{Board, WS_PER_MINUTE};
use crate::state::lock;

fn unescape(s: &str) -> String {
    s.replace("~1", "/").replace("~0", "~")
}

fn apply(doc: &Value, ops: &[Value]) -> Value {
    let mut doc = doc.clone();
    for op in ops {
        let path = op["path"].as_str().unwrap();
        if path.is_empty() {
            doc = op["value"].clone();
            continue;
        }
        let parts: Vec<String> = path[1..].split('/').map(unescape).collect();
        let (last, head) = parts.split_last().unwrap();
        let mut node = &mut doc;
        for p in head {
            node = match node {
                Value::Array(a) => &mut a[p.parse::<usize>().unwrap()],
                Value::Object(o) => o.get_mut(p).unwrap(),
                _ => panic!("bad path {path}"),
            };
        }
        match (op["op"].as_str().unwrap(), node) {
            ("remove", Value::Object(o)) => drop(o.remove(last)),
            ("remove", Value::Array(a)) => drop(a.remove(last.parse::<usize>().unwrap())),
            ("add", Value::Array(a)) => a.insert(last.parse::<usize>().unwrap(), op["value"].clone()),
            (_, Value::Object(o)) => drop(o.insert(last.clone(), op["value"].clone())),
            (_, Value::Array(a)) => a[last.parse::<usize>().unwrap()] = op["value"].clone(),
            _ => panic!("bad op {op}"),
        }
    }
    doc
}

fn pane(key: &str, session: &str, state: &str) -> Value {
    json!({ "key": key, "kind": "term", "title": key, "state": state, "rail": "idle", "subagents": 0, "sessionId": session, "dirty": false, "busy": false })
}

fn summary(rev: u64, panes: Vec<Value>) -> Value {
    json!({ "rev": rev, "tabs": [{ "key": "t1", "name": "api", "tree": { "pane": "p1" }, "panes": panes }], "closedCount": 0 })
}

#[test]
fn json_patch_turns_the_old_snapshot_into_the_new_one() {
    let cases = [
        (summary(1, vec![pane("p1", "s1", "running")]), summary(2, vec![pane("p1", "s1", "exited")])),
        (summary(1, vec![pane("p1", "s1", "running")]), summary(2, vec![pane("p1", "s1", "running"), pane("p2", "s2", "starting")])),
        (summary(1, vec![pane("p1", "s1", "running"), pane("p2", "s2", "running"), pane("p3", "s3", "running")]), summary(2, vec![pane("p3", "s3", "running")])),
        (json!({ "rev": 1, "a/b": 1, "c~d": { "x": [1, 2] }, "gone": true }), json!({ "rev": 2, "a/b": 2, "c~d": { "x": [1], "y": null } })),
        (json!({ "rev": 1, "tabs": [] }), json!({ "rev": 2, "tabs": { "now": "object" } })),
        (json!([1, { "a": 1 }]), json!({ "root": "changed" })),
        (summary(5, vec![]), summary(5, vec![])),
    ];
    for (old, new) in cases {
        let ops = diff(&old, &new);
        assert_eq!(apply(&old, &ops), new, "{ops:?}");
    }
    assert!(diff(&summary(5, vec![]), &summary(5, vec![])).is_empty());
}

#[test]
fn the_board_sends_a_snapshot_first_then_patches_and_knows_its_panes() {
    let board = Board::default();
    let mut rx = board.subscribe();
    board.publish(summary(1, vec![pane("p1", "s1", "running")]));
    let first = rx.try_recv().unwrap();
    assert!(first.ops.is_none() && first.base.is_none());
    board.publish(summary(1, vec![pane("p1", "s1", "running")]));
    assert!(rx.try_recv().is_err());
    board.publish(summary(2, vec![pane("p1", "s1", "exited")]));
    let next = rx.try_recv().unwrap();
    assert_eq!((next.base, next.rev), (Some(1), 2));
    assert!(next.ops.is_some());
    assert!(board.has_session("s1"));
    assert!(!board.has_session("s9"));
    let now = Instant::now();
    for _ in 0..WS_PER_MINUTE {
        assert!(board.take_rate("d", now));
    }
    assert!(!board.take_rate("d", now));
    assert!(board.take_rate("other", now));
    assert!(board.take_rate("d", now + Duration::from_secs(61)));
}

#[test]
fn a_confirm_is_used_once_and_lapses_after_sixty_seconds() {
    let mut c = Confirms::default();
    let now = Instant::now();
    let cmd = command_of(&json!({ "id": "x", "t": "ws.close", "paneKey": "p1", "confirmed": true }));
    assert_eq!(cmd, json!({ "t": "ws.close", "paneKey": "p1" }));
    let cid = c.add(cmd.clone(), now).unwrap();
    assert_eq!(c.take(&cid, now), Some(cmd.clone()));
    assert_eq!(c.take(&cid, now), None);
    let late = c.add(cmd, now).unwrap();
    assert_eq!(c.take(&late, now + CONFIRM_TTL + Duration::from_secs(1)), None);
}

#[test]
fn renderer_results_map_to_phone_replies() {
    assert_eq!(reply_of("a", &json!({ "ok": true, "paneKey": "p2" })), json!({ "re": "a", "ok": true, "paneKey": "p2" }));
    assert_eq!(reply_of("a", &json!({ "ok": false, "error": "busy" }))["error"], "busy");
    assert_eq!(reply_of("a", &json!(null))["error"], "internal");
    assert_eq!(error_code(Some("args")), "invalid");
    assert_eq!(error_code(Some("unavailable")), "offline");
    assert_eq!(error_code(Some("source")), "denied");
    assert_eq!(error_code(Some("weird")), "internal");
}

#[test]
fn hidden_name_characters_include_line_separators_and_every_format_char() {
    for c in ['\u{2028}', '\u{2029}', '\u{00AD}', '\u{180E}', '\u{2060}', '\u{2064}', '\u{E0041}', '\u{FEFF}', '\u{202E}', '\u{0600}', '\u{1D173}'] {
        assert!(is_hidden(c), "{:04X}", c as u32);
    }
    for c in ['a', 'Ă', 'ệ', ' ', '\u{4E2D}', '\u{1F600}'] {
        assert!(!is_hidden(c), "{:04X}", c as u32);
    }
    assert_eq!(clean_name("Pi\u{2028}xel\u{E0041}\u{00AD} 7"), "Pixel 7");
}

#[test]
fn enabling_with_an_unreachable_relay_never_waits_for_the_register_call() {
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = silent.local_addr().unwrap().port();
    let held = Arc::new(AtomicUsize::new(0));
    let count = held.clone();
    let _keep = std::thread::spawn(move || {
        let mut conns = Vec::new();
        for s in silent.incoming().flatten() {
            count.fetch_add(1, Ordering::SeqCst);
            conns.push(s);
        }
    });
    let rig = rig("register-bg", &format!("http://127.0.0.1:{port}"), false);
    let start = Instant::now();
    rig.remote.enable(true);
    assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
    let wait = Instant::now();
    while held.load(Ordering::SeqCst) == 0 && wait.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(held.load(Ordering::SeqCst) > 0);
    let stop = Instant::now();
    rig.remote.enable(false);
    assert!(stop.elapsed() < Duration::from_secs(3), "{:?}", stop.elapsed());
    assert!(!lock(&rig.remote.ctx().unwrap().config).relay.registered);
}

#[test]
fn no_lan_address_shows_the_lan_error_and_it_clears_when_off() {
    let mut rig = rig("no-lan", "", false);
    rig.remote = super::Remote::new(rig.host.clone(), Box::new(rig.secrets.clone()), rig.dir.clone(), super::Options { addrs: Some(Vec::new()), port: Some(0), mdns: false, name: None });
    let on = rig.remote.enable(true);
    assert_eq!(on["error"], "lan");
    let off = rig.remote.enable(false);
    assert!(off["error"].is_null());
}

fn paired(rig: &Rig, phone: &Phone) {
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
}

fn answer(rig: &Rig, f: impl Fn(&str, &Value) -> Option<Value> + Send + Sync + 'static) {
    *lock(&rig.host.answer_with) = Some(Box::new(f));
}

#[test]
fn workspace_commands_go_to_the_renderer_with_gateway_confirm_and_patches() {
    let rig = rig("ws-cmds", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    paired(&rig, &phone);
    answer(&rig, |action, args| match action {
        "ws.open" => Some(json!({ "ok": true, "paneKey": "p2" })),
        "ws.close" if args.get("confirmed") == Some(&Value::Bool(true)) => Some(json!({ "ok": true })),
        "ws.close" => Some(json!({ "needsConfirm": { "title": "Close api?", "detail": "busy", "ok": "Close", "cancel": "Cancel" } })),
        "ws.rename" => {
            std::thread::sleep(Duration::from_secs(7));
            None
        }
        SUMMARY_GET => Some(summary(7, vec![pane("p1", "s1", "running")])),
        _ => None,
    });
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        assert_eq!(hello(&mut ws, &mut t).await["ok"], true);
        send_json(&mut ws, &mut t, json!({ "id": "g", "t": "ws.get" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "g").await["ok"], true);
        let snap = until_json(&mut ws, &mut t, |v| v["t"] == "ws.snapshot").await;
        assert_eq!(snap["rev"], 7);
        let mut doc = snap.as_object().unwrap().clone();
        doc.remove("t");
        let mut doc = Value::Object(doc);
        let next = summary(8, vec![pane("p1", "s1", "running"), pane("p2", "s2", "starting")]);
        assert!(rig.remote.publish(&next));
        let patch = until_json(&mut ws, &mut t, |v| v["t"] == "ws.patch").await;
        assert_eq!((patch["base"].as_u64(), patch["rev"].as_u64()), (Some(7), Some(8)));
        doc = apply(&doc, patch["ops"].as_array().unwrap());
        assert_eq!(doc, next);
        send_json(&mut ws, &mut t, json!({ "id": "o", "t": "ws.open", "cwd": "C:/x", "preset": "shell", "confirmed": true })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "o").await["paneKey"], "p2");
        send_json(&mut ws, &mut t, json!({ "id": "c", "t": "ws.close", "paneKey": "p1", "confirmed": true })).await;
        let pending = until_json(&mut ws, &mut t, |v| v["re"] == "c").await;
        let cid = pending["confirm"].as_str().unwrap().to_owned();
        let ask = until_json(&mut ws, &mut t, |v| v["t"] == "confirm.ask").await;
        assert_eq!((ask["cid"].as_str(), ask["title"].as_str(), ask["re"].as_str()), (Some(cid.as_str()), Some("Close api?"), Some("c")));
        send_json(&mut ws, &mut t, json!({ "id": "a1", "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "a1").await["ok"], true);
        send_json(&mut ws, &mut t, json!({ "id": "a2", "t": "confirm.answer", "cid": cid, "ok": true })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "a2").await["error"], "notFound");
        let started = Instant::now();
        send_json(&mut ws, &mut t, json!({ "id": "r", "t": "ws.rename", "tabKey": "t1", "name": "x" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r").await["error"], "timeout");
        let took = started.elapsed();
        assert!(took >= Duration::from_millis(4900) && took < Duration::from_secs(7), "{took:?}");
        send_json(&mut ws, &mut t, json!({ "id": "k", "t": "ws.keys", "paneKey": "p1", "keys": "approve", "since": 1 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "k").await["error"], "offline");
        send_json(&mut ws, &mut t, json!({ "id": "z", "t": "ws.nope" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "z").await["error"], "denied");
    });
    let asks = lock(&rig.host.asks).clone();
    let mut open = asks.iter().find(|(a, _)| a == "ws.open").unwrap().1.clone();
    assert!(open["rid"].as_str().is_some_and(|r| r.ends_with(".6f")), "{open}");
    open.as_object_mut().unwrap().remove("rid");
    assert_eq!(open, json!({ "device": "p", "cmd": { "t": "ws.open", "cwd": "C:/x", "preset": "shell" } }));
    let closes: Vec<&Value> = asks.iter().filter(|(a, _)| a == "ws.close").map(|(_, v)| v).collect();
    assert_eq!(closes.len(), 2);
    assert!(closes[0].get("confirmed").is_none());
    assert_eq!(closes[1]["confirmed"], true);
    rig.remote.enable(false);
}

#[test]
fn workspace_commands_are_rate_limited_per_device() {
    let rig = rig("ws-rate", "", false);
    rig.remote.enable(true);
    assert!(rig.remote.publish(&summary(1, vec![])));
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        for i in 0..WS_PER_MINUTE {
            send_json(&mut ws, &mut t, json!({ "id": format!("g{i}"), "t": "ws.get" })).await;
            let id = format!("g{i}");
            assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == id.as_str()).await["ok"], true);
        }
        send_json(&mut ws, &mut t, json!({ "id": "last", "t": "ws.get" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "last").await["error"], "rate");
    });
    rig.remote.enable(false);
}

#[test]
fn a_fresh_summary_is_awaited_after_a_connect_then_the_pane_is_refused() {
    let board = Arc::new(Board::default());
    board.publish(summary(1, vec![pane("p1", "s1", "running")]));
    runtime().block_on(async {
        let quick = Instant::now();
        assert!(!board.has_session_fresh("s2", Duration::from_secs(2)).await);
        assert!(quick.elapsed() < Duration::from_millis(500));

        board.mark_connect();
        let b = board.clone();
        let late = tokio::spawn(async move { b.has_session_fresh("s2", Duration::from_secs(2)).await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        board.publish(summary(2, vec![pane("p1", "s1", "running"), pane("p2", "s2", "running")]));
        assert!(late.await.unwrap());

        board.mark_connect();
        let started = Instant::now();
        assert!(!board.has_session_fresh("s9", Duration::from_millis(300)).await);
        assert!(started.elapsed() >= Duration::from_millis(250));
    });
}
