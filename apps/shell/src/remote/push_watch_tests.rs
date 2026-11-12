use std::time::{Duration, Instant};

use core_http::{Config as HttpConfig, Hooks, StoreHandle};
use core_store::{Store, StoreOptions, TerminalSession};
use serde_json::{json, Value};

use super::config::now_secs;
use super::devices::Device;
use super::fcm_files::PACKAGE;
use super::fcm_tests::{account, gs_json, sa_json, Fake, KEY_A};
use super::push_watch::{transition, Watch};
use super::test_util::*;
use crate::state::lock;

const TOKEN_A: &str = "tokenAAAAAAAAAAAAAAAAAAAAAAAA:APA91bAAAA";

fn ev(events: Value) -> String {
    json!({ "seq": 1, "events": events }).to_string()
}

fn updated(id: &str, status: &str) -> String {
    ev(json!([{ "type": "terminal.updated", "payload": { "id": id, "status": status } }]))
}

fn seeded() -> Watch {
    let mut w = Watch::default();
    w.apply(&ev(json!([{ "type": "snapshot", "payload": { "terminals": [{ "id": "t1", "status": "working" }, { "id": "t2", "status": "idle" }] } }])));
    w
}

#[test]
fn only_working_to_waiting_or_idle_needs_the_owner() {
    assert_eq!(transition("working", "waiting"), Some("attention"));
    assert_eq!(transition("working", "idle"), Some("done"));
    assert_eq!(transition("working", "error"), Some("error"));
    assert_eq!(transition("waiting", "error"), Some("error"));
    for (a, b) in [("idle", "working"), ("waiting", "working"), ("waiting", "idle"), ("idle", "waiting"), ("working", "working"), ("error", "error"), ("idle", "error"), ("", "idle"), ("stale", "idle")] {
        assert_eq!(transition(a, b), None, "{a}->{b}");
    }
}

#[test]
fn the_watcher_tracks_statuses_from_snapshot_created_updated_removed() {
    let mut w = seeded();
    assert!(w.apply(&updated("t2", "waiting")).is_empty());
    assert_eq!(w.apply(&updated("t1", "waiting")), vec![("t1".to_owned(), "attention")]);
    assert!(w.apply(&updated("t1", "waiting")).is_empty());
    assert!(w.apply(&updated("t1", "idle")).is_empty());
    assert!(w.apply(&updated("t1", "working")).is_empty());
    assert_eq!(w.apply(&updated("t1", "idle")), vec![("t1".to_owned(), "done")]);
    assert!(w.apply(&ev(json!([{ "type": "terminal.created", "payload": { "id": "n", "status": "working" } }]))).is_empty());
    assert_eq!(w.apply(&updated("n", "idle")), vec![("n".to_owned(), "done")]);
    assert!(w.apply(&ev(json!([{ "type": "terminal.removed", "payload": { "id": "n" } }]))).is_empty());
    assert!(w.apply(&updated("n", "idle")).is_empty());
    assert!(w.apply(&ev(json!([{ "type": "terminal.updated", "payload": { "id": "t1", "title": "x" } }]))).is_empty());
    assert!(w.apply("not json").is_empty());
    let both = ev(json!([
        { "type": "terminal.updated", "payload": { "id": "t1", "status": "working" } },
        { "type": "terminal.updated", "payload": { "id": "t1", "status": "waiting" } }
    ]));
    assert_eq!(w.apply(&both), vec![("t1".to_owned(), "attention")]);
}

fn pane_summary(terminal: &str, pane: &str) -> Value {
    json!({ "rev": 1, "closedCount": 0, "tabs": [{ "panes": [{ "key": pane, "terminalId": terminal, "sessionId": "s" }] }] })
}

#[test]
fn a_lan_only_setup_with_telegram_off_still_pushes_with_the_right_kind() {
    let rig = rig("pushw-lan", "", false);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    assert!(lock(&ctx.devices).set_push("p", TOKEN_A, "en"));
    assert!(lock(&ctx.config).relay.url.is_empty());
    ctx.running.store(true, std::sync::atomic::Ordering::SeqCst);
    let parsed = super::fcm_files::parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
    lock(&ctx.config).firebase = parsed.firebase;
    ctx.fcm.set_http(Fake::ok());
    ctx.fcm.set_account(Some(account()));
    let now = Instant::now();
    assert_eq!(ctx.push_terminal("t1", "done", now), 0);
    ctx.publish(pane_summary("t1", "pane-1"));
    assert_eq!(ctx.push_terminal("unknown", "done", now), 0);
    assert_eq!(ctx.push_terminal("t1", "attention", now), 1);
    let jobs = ctx.take_push_jobs();
    assert_eq!((jobs[0].kind, jobs[0].pane.as_str(), jobs[0].device.as_str()), ("attention", "pane-1", "p"));
    assert_eq!(ctx.push_terminal("t1", "attention", now + Duration::from_secs(5)), 0);
    assert_eq!(ctx.push_terminal("t1", "done", now + Duration::from_secs(40)), 1);
    assert_eq!(ctx.take_push_jobs()[0].kind, "done");
}

fn terminal(id: &str, status: &str) -> TerminalSession {
    serde_json::from_value(json!({ "id": id, "title": id, "status": status })).unwrap()
}

#[test]
fn the_running_gateway_pushes_from_a_store_transition_without_any_phone_connected() {
    let rig = rig("pushw-store", "", false);
    let rt = runtime();
    let dist = rig.dir.join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::write(dist.join("index.html"), "ok").unwrap();
    let store = StoreHandle::new(Store::new(StoreOptions { activity_limit: 10, batch_ms: 10 }));
    let server = rt.block_on(core_http::start(HttpConfig::new(0, dist), store.clone(), Hooks::default())).unwrap();
    *lock(&rig.host.tap) = Some(server.tap());
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    assert!(lock(&ctx.devices).set_push("p", TOKEN_A, "vi"));
    let parsed = super::fcm_files::parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
    lock(&ctx.config).firebase = parsed.firebase;
    let http = Fake::ok();
    ctx.fcm.set_http(http.clone());
    ctx.fcm.set_account(Some(account()));
    store.with(|s| s.upsert_terminal(terminal("t1", "working")));
    rig.remote.enable(true);
    assert!(rig.remote.publish(&pane_summary("t1", "pane-1")));
    std::thread::sleep(Duration::from_millis(300));
    store.with(|s| s.upsert_terminal(terminal("t1", "idle")));
    for _ in 0..250 {
        if !http.fcm_calls().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let calls = http.fcm_calls();
    assert_eq!(calls.len(), 1);
    let sent: Value = serde_json::from_str(&calls[0].3).unwrap();
    assert_eq!(sent["message"]["data"], json!({ "m": ctx.machine_id(), "p": "pane-1", "k": "done" }));
    assert_eq!(sent["message"]["notification"]["title"], super::push::title_body("done", "vi").0);
    rig.remote.enable(false);
}

#[test]
fn a_queued_job_is_dropped_when_the_phone_is_connected_by_the_time_it_would_send() {
    let rig = rig("pushw-live", "", false);
    let phone = Phone::new("p");
    rig.remote.enable(true);
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    assert!(lock(&ctx.devices).set_push("p", TOKEN_A, "en"));
    let parsed = super::fcm_files::parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
    lock(&ctx.config).firebase = parsed.firebase;
    let http = Fake::ok();
    ctx.fcm.set_http(http.clone());
    ctx.fcm.set_account(Some(account()));
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        lock(&ctx.push).push(super::push::Job { device: "p".into(), token: TOKEN_A.into(), machine: "m".into(), pane: "p1".into(), kind: "done", locale: "en" });
        ctx.push_wake.notify_one();
        tokio::time::sleep(Duration::from_millis(500)).await;
    });
    assert!(http.fcm_calls().is_empty());
    rig.remote.enable(false);
}
