use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::super::config::now_secs;
use super::super::devices::Device;
use super::super::fcm::Outcome;
use super::super::fcm_tests::Fake;
use super::super::push_content::{mask_text, LABEL_MAX};
use super::super::push_seal::sealed_plain;
use super::super::push_tests::{ready, TOKEN_A, TOKEN_B};
use super::super::push_watch::Watch;
use super::super::test_util::*;
use super::super::push_web::web_payload;
use super::extra::{menu_of, PAUSE_MAX_MS};
use super::{message, now_ms, title_body, Job, PushState, HOUR, PER_HOUR, PER_HOUR_INFO};
use crate::state::lock;

fn add(rig: &Rig, id: &str, token: &str, seal: bool) -> Phone {
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    assert!(devices.set_push(id, token, "en"));
    devices.set_push_seal(id, seal, seal, now_secs());
    phone
}

fn menu() -> Value {
    json!({ "sig": "1a2b3c4d", "kind": "permission", "agent": "claude", "question": "Allow?", "numbered": true, "confident": true, "since": 1, "items": [
        { "index": 1, "label": "Yes, run npm test --token=abc123 in the whole repository" },
        { "index": 2, "label": "Yes, and do not ask again" },
        { "index": 3, "label": "No" },
        { "index": 4, "label": "Other thing", "other": true },
        { "index": 5, "label": "Fifth" }
    ] })
}

fn summary(menu: Option<Value>) -> Value {
    let mut p1 = json!({ "key": "p1", "terminalId": "t1", "agent": "claude", "title": "✳ Fix login bug" });
    if let Some(m) = menu {
        p1["menu"] = m;
    }
    json!({ "rev": 1, "tabs": [{ "key": "tab", "name": "Backend", "project": "orrery", "panes": [
        p1,
        { "key": "p2", "terminalId": "t2", "agent": "codex", "title": "Refactor the whole payments gateway module" },
        { "key": "p3", "terminalId": "t3", "agent": "shell", "title": "" }
    ] }] })
}

fn job_for(ctx: &super::super::ctx::Ctx, id: &str, pane: &str, kind: &'static str) -> Job {
    let job = ctx.push_targets(|d| d.device_id == id).pop().unwrap();
    Job { pane: pane.into(), terminal: "t1".into(), kind, ..job }
}

#[test]
fn info_quota_is_separate_and_asks_are_never_crowded_out() {
    let mut st = PushState::default();
    let t0 = Instant::now();
    for i in 0..PER_HOUR_INFO {
        assert!(st.admit_kind("d", &format!("p{i}"), "open", t0, 0), "{i}");
    }
    assert!(!st.admit_kind("d", "extra", "open", t0, 0));
    assert!(!st.admit_kind("d", "extra", "running", t0, 0));
    assert!(!st.admit_kind("d", "extra", "done", t0, 0));
    for i in 0..PER_HOUR {
        assert!(st.admit_kind("d", &format!("q{i}"), "question", t0, 0), "{i}");
    }
    assert!(!st.admit_kind("d", "q-over", "question", t0, 0));
    assert!(st.admit_kind("d", "extra", "open", t0 + HOUR + Duration::from_secs(1), 0));
    let mut st = PushState::default();
    for i in 0..PER_HOUR {
        assert!(st.admit_kind("d", &format!("q{i}"), "attention", t0, 0));
    }
    assert!(st.admit_kind("d", "p", "running", t0, 0));
    assert!(st.admit_kind("d", "p", "open", t0, 0));
}

#[test]
fn done_keeps_the_pane_gap_but_running_and_open_do_not() {
    let mut st = PushState::default();
    let t0 = Instant::now();
    assert!(st.admit_kind("d", "p", "done", t0, 0));
    assert!(!st.admit_kind("d", "p", "done", t0 + Duration::from_secs(5), 0));
    assert!(st.admit_kind("d", "p", "open", t0 + Duration::from_secs(5), 0));
    assert!(st.admit_kind("d", "p", "open", t0 + Duration::from_secs(6), 0));
    assert!(st.admit_kind("d", "other", "running", t0, 0));
    assert!(st.admit_kind("d", "other", "done", t0, 0), "the end of a run replaces the ongoing notification");
    assert!(!st.admit_kind("d", "other", "done", t0, 0));
}

#[test]
fn a_pause_mutes_groups_until_the_mark_then_expires_and_is_bounded() {
    let mut st = PushState::default();
    let now = 1_000_000u64;
    let t0 = Instant::now();
    st.set_pause("d", Some(now + 60_000), now);
    assert!(st.paused("d", now) && st.paused("d", now + 59_999) && !st.paused("d", now + 60_000));
    assert!(!st.admit_kind("d", "p", "question", t0, now));
    assert!(!st.admit_kind("d", "p", "done", t0, now));
    assert!(!st.admit_kind("d", "p", "error", t0, now));
    assert!(!st.admit_kind("d", "p", "running", t0, now));
    assert!(st.admit_kind("d", "p", "open", t0, now));
    assert!(st.admit_kind("d", "p", "question", t0, now + 60_000));
    st.set_pause("d", Some(now + PAUSE_MAX_MS + 1), now);
    assert!(!st.paused("d", now));
    st.set_pause("d", Some(now - 1), now);
    assert!(!st.paused("d", now));
    st.set_pause("d", Some(now + 5), now);
    st.set_pause("d", None, now);
    assert!(!st.paused("d", now));
    st.set_pause("d", Some(now + 5), now);
    st.forget_device("d");
    assert!(!st.paused("d", now));
}

#[test]
fn a_paused_device_gets_nothing_queued_or_delivered_but_security_open_and_clear_pass() {
    let rig = rig("g1-pause", "", false);
    add(&rig, "a", TOKEN_A, false);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    ctx.board.publish(summary(None));
    ctx.note_pause("a", Some(&json!(now_ms() + 60_000)));
    assert_eq!(ctx.push_pane("p1", "question", "t1", None, Instant::now()), 0);
    assert_eq!(ctx.deliver(&job_for(&ctx, "a", "p1", "done")), Outcome::Fail("paused"));
    assert!(http.fcm_calls().is_empty());
    assert_eq!(ctx.push_open("p1", Instant::now()), Ok(1));
    assert_eq!(ctx.push_security(None), 1);
    ctx.note_pause("a", Some(&json!(now_ms().saturating_sub(5))));
    assert_eq!(ctx.push_pane("p1", "question", "t1", None, Instant::now()), 1);
    ctx.note_pause("a", Some(&json!("soon")));
    assert!(!lock(&ctx.push).paused("a", now_ms()));
}

#[test]
fn menu_items_are_masked_cut_capped_at_three_and_other_or_multiselect_menus_are_dropped() {
    let mut s = summary(Some(menu()));
    super::super::chat_src::mask_menus(&mut s);
    let m = menu_of(&s, "p1", &mask_text).unwrap();
    assert_eq!(m["sig"], "1a2b3c4d");
    let items = m["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["index"], 1);
    let label = items[0]["label"].as_str().unwrap();
    assert!(label.chars().count() <= LABEL_MAX && !label.contains("abc123"), "{label}");
    assert_eq!(items[2], json!({ "index": 3, "label": "No" }));
    assert!(m.to_string().find("Other thing").is_none());
    assert!(menu_of(&s, "p2", &mask_text).is_none());
    assert!(menu_of(&s, "missing", &mask_text).is_none());
    let mut multi = summary(Some(menu()));
    multi["tabs"][0]["panes"][0]["menu"]["items"][0]["checked"] = json!(false);
    assert!(menu_of(&multi, "p1", &mask_text).is_none());
    let mut bad = summary(Some(menu()));
    bad["tabs"][0]["panes"][0]["menu"]["sig"] = json!("bad sig!");
    assert!(menu_of(&bad, "p1", &mask_text).is_none());
    let mut hidden = summary(Some(json!({ "sig": "ab", "items": [{ "index": 1, "label": "\u{202E}\u{0007}" }, { "index": 2, "label": "Go\u{202E}od" }] })));
    hidden["tabs"][0]["panes"][0]["menu"]["items"][0]["other"] = json!(true);
    assert_eq!(menu_of(&hidden, "p1", &mask_text).unwrap()["items"], json!([{ "index": 2, "label": "Good" }]));
}

#[test]
fn the_menu_and_waiting_labels_ride_only_inside_the_seal_and_only_with_content_on() {
    let rig = rig("g1-menu", "", false);
    let phone = add(&rig, "a", TOKEN_A, true);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    ctx.board.publish(summary(Some(menu())));
    ctx.set_waiting(vec!["t1".into(), "t2".into(), "t3".into()]);
    let job = Job { content: true, ..job_for(&ctx, "a", "p1", "question") };
    assert_eq!(job.seal_key, Some(*phone.key.public()));
    let rich = ctx.enrich(&job).unwrap();
    assert_eq!(rich.extra.waiting, Some(3));
    assert_eq!(rich.extra.labels, vec!["Fix login bug".to_owned(), "Refactor the whole paym…".to_owned()]);
    assert_eq!(rich.extra.menu.as_ref().unwrap()["items"].as_array().unwrap().len(), 3);
    let plain: Value = serde_json::from_slice(&sealed_plain(&rich, "T", "B")).unwrap();
    assert_eq!(plain["wl"], json!(rich.extra.labels));
    assert_eq!(plain["menu"], rich.extra.menu.clone().unwrap());
    let plain_msg = message(&rich);
    assert_eq!(plain_msg["message"]["data"]["w"], "3");
    let text = plain_msg.to_string();
    assert!(!text.contains("menu") && !text.contains("Fix login"), "{text}");
    let sealed = ctx.fcm_message(&rich);
    assert!(sealed["message"].get("notification").is_none());
    assert_eq!(sealed["message"]["data"]["w"], "3");
    let text = sealed.to_string();
    assert!(!text.contains("Fix login") && !text.contains("\"menu\""), "{text}");
    let web = web_payload(&rich, "T", "B");
    assert_eq!((web["w"].clone(), web["wl"].clone(), web["menu"]["sig"].clone()), (json!(3), json!(rich.extra.labels), json!("1a2b3c4d")));
    let off = ctx.enrich(&Job { content: false, ..job.clone() }).unwrap();
    assert_eq!((off.extra.waiting, off.extra.labels.len(), off.extra.menu.is_none()), (Some(3), 0, true));
    let off_web = web_payload(&off, "T", "B");
    assert!(off_web.get("menu").is_none() && off_web.get("wl").is_none() && off_web["w"] == 3);
    for kind in ["done", "error", "running", "open"] {
        let other = ctx.enrich(&Job { kind, ..job.clone() }).unwrap();
        assert!(other.extra.menu.is_none(), "{kind}");
        assert_eq!(other.extra.waiting, Some(3));
    }
    assert!(ctx.enrich(&Job { kind: "test", ..job.clone() }).unwrap().extra.waiting.is_none());
    assert!(ctx.enrich(&Job { kind: "security", ..job }).unwrap().extra.waiting.is_none());
}

#[test]
fn every_group_push_carries_the_waiting_count_and_running_carries_its_start() {
    let rig = rig("g1-w", "", false);
    add(&rig, "a", TOKEN_A, false);
    let ctx = ready(&rig, Fake::ok());
    ctx.board.publish(summary(None));
    for kind in ["attention", "question", "permission", "done", "error"] {
        let job = ctx.enrich(&job_for(&ctx, "a", "p1", kind)).unwrap();
        let m = message(&job);
        assert_eq!(m["message"]["data"]["w"], "0", "{kind}");
        assert_eq!(m["message"]["android"]["notification"]["tag"], format!("{}:p1", ctx.machine_id()));
        assert_eq!(m["message"]["android"]["notification"]["channel_id"], "attention");
        assert!(m["message"]["data"].get("t0").is_none());
    }
    let running = Job { started: Some(1_764_150_000_000), ..job_for(&ctx, "a", "p1", "running") };
    let m = message(&ctx.enrich(&running).unwrap());
    assert_eq!(m["message"]["data"]["t0"], "1764150000000");
    assert_eq!(m["message"]["android"]["notification"]["channel_id"], "running");
    assert_eq!(m["message"]["notification"]["title"], title_body("running", "en").0);
    ctx.set_waiting((0..1500).map(|i| format!("t{i}")).collect());
    assert_eq!(ctx.enrich(&job_for(&ctx, "a", "p1", "done")).unwrap().extra.waiting, Some(999));
}

#[test]
fn clear_goes_once_to_a_phone_that_was_told_a_count_and_only_while_the_count_is_zero() {
    let rig = rig("g1-clear", "", false);
    add(&rig, "a", TOKEN_A, true);
    add(&rig, "b", TOKEN_B, false);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    ctx.board.publish(summary(None));
    assert!(!ctx.set_waiting(vec![]));
    assert_eq!(ctx.push_clear(Instant::now()), 0);
    assert!(!ctx.set_waiting(vec!["t1".into()]));
    assert_eq!(ctx.deliver(&Job { content: true, ..job_for(&ctx, "a", "p1", "attention") }), Outcome::Sent);
    assert_eq!(lock(&ctx.push).told("a"), 1);
    assert_eq!(lock(&ctx.push).told("b"), 0);
    assert!(ctx.set_waiting(vec![]));
    assert_eq!(ctx.push_clear(Instant::now()), 1);
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.len(), 1);
    let clear = &jobs[0];
    assert_eq!((clear.kind, clear.device.as_str(), clear.pane.as_str(), clear.content, clear.seal_key), ("clear", "a", "", false, None));
    ctx.set_waiting(vec!["t2".into()]);
    assert_eq!(ctx.deliver(clear), Outcome::Fail("superseded"));
    ctx.set_waiting(vec![]);
    assert_eq!(ctx.deliver(clear), Outcome::Sent);
    let calls = http.fcm_calls();
    let sent: Value = serde_json::from_str(&calls.last().unwrap().3).unwrap();
    assert_eq!(sent, json!({ "message": { "token": TOKEN_A, "data": { "m": ctx.machine_id(), "k": "clear", "w": "0" }, "android": { "priority": "normal" } } }));
    assert_eq!(lock(&ctx.push).told("a"), 0);
    assert_eq!(ctx.deliver(clear), Outcome::Fail("nothing"));
    assert_eq!(ctx.push_clear(Instant::now()), 0);
}

#[test]
fn the_watcher_lists_waiting_terminals_oldest_first_and_flags_the_drop_to_zero() {
    let rig = rig("g1-wait", "", false);
    let ctx = ready(&rig, Fake::ok());
    let mut w = Watch::default();
    let ev = |id: &str, status: &str| json!({ "seq": 1, "events": [{ "type": "terminal.updated", "payload": { "id": id, "status": status } }] }).to_string();
    w.apply(&json!({ "seq": 1, "events": [{ "type": "snapshot", "payload": { "terminals": [{ "id": "t1", "status": "working" }, { "id": "t2", "status": "working" }] } }] }).to_string());
    w.apply(&ev("t2", "waiting"));
    std::thread::sleep(Duration::from_millis(5));
    w.apply(&ev("t1", "waiting"));
    assert_eq!(w.waiting_ids(), vec!["t2".to_owned(), "t1".to_owned()]);
    assert!(!ctx.set_waiting(w.waiting_ids()));
    w.apply(&ev("t2", "working"));
    assert_eq!(w.waiting_ids(), vec!["t1".to_owned()]);
    w.apply(&json!({ "seq": 2, "events": [{ "type": "terminal.removed", "payload": { "id": "t1" } }] }).to_string());
    assert!(w.waiting_ids().is_empty());
    assert!(ctx.set_waiting(w.waiting_ids()));
    assert!(!ctx.set_waiting(w.waiting_ids()));
}

#[test]
fn a_run_is_announced_once_after_two_minutes_and_a_short_run_never() {
    let mut w = Watch::default();
    let snap = json!({ "seq": 1, "events": [{ "type": "snapshot", "payload": { "terminals": [{ "id": "t1", "status": "working" }, { "id": "t2", "status": "working" }] } }] }).to_string();
    w.apply(&snap);
    let t0 = Instant::now();
    let ms = 1_700_000_000_000u64;
    assert!(w.due_running(t0 + Duration::from_secs(119), ms).is_empty());
    let end = json!({ "seq": 2, "events": [{ "type": "terminal.updated", "payload": { "id": "t2", "status": "idle" } }] }).to_string();
    w.apply(&end);
    let due = w.due_running(t0 + Duration::from_secs(121), ms);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].0, "t1");
    assert!(due[0].1 <= ms && due[0].1 >= ms - 122_000, "{}", due[0].1);
    assert!(w.due_running(t0 + Duration::from_secs(300), ms).is_empty());
    let idle = json!({ "seq": 3, "events": [{ "type": "terminal.updated", "payload": { "id": "t1", "status": "idle" } }] }).to_string();
    w.apply(&idle);
    let again = json!({ "seq": 4, "events": [{ "type": "terminal.updated", "payload": { "id": "t1", "status": "working" }}] }).to_string();
    w.apply(&again);
    assert!(w.due_running(Instant::now(), ms).is_empty());
    assert_eq!(w.due_running(Instant::now() + Duration::from_secs(125), ms).len(), 1);
}

#[test]
fn running_then_the_end_of_the_run_is_at_most_two_pushes_on_the_same_tag() {
    let rig = rig("g1-run", "", false);
    add(&rig, "a", TOKEN_A, false);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    ctx.board.publish(summary(None));
    let now = Instant::now();
    assert_eq!(ctx.push_running("nope", 5, now), 0);
    assert_eq!(ctx.push_running("t1", 1_764_150_000_000, now), 1);
    assert_eq!(ctx.push_terminal("t1", "done", now + Duration::from_secs(3)), 1);
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.iter().map(|j| j.kind).collect::<Vec<_>>(), ["running", "done"]);
    assert_eq!(jobs[0].started, Some(1_764_150_000_000));
    let tags: Vec<_> = jobs.iter().map(|j| message(j)["message"]["android"]["notification"]["tag"].clone()).collect();
    assert_eq!(tags[0], tags[1]);
    assert_eq!(ctx.deliver(&jobs[0]), Outcome::Sent);
    let sent: Value = serde_json::from_str(&http.fcm_calls().last().unwrap().3).unwrap();
    assert_eq!(sent["message"]["data"]["t0"], "1764150000000");
    assert_eq!(ctx.push_terminal("t1", "done", now + Duration::from_secs(4)), 0, "a second end inside the gap stays blocked");
}

#[test]
fn a_run_end_still_replaces_the_ongoing_notification_when_the_pane_gap_or_mute_would_block_it() {
    let rig = rig("g1-ongoing", "", false);
    add(&rig, "a", TOKEN_A, false);
    let ctx = ready(&rig, Fake::ok());
    ctx.board.publish(summary(None));
    let now = Instant::now();
    assert_eq!(ctx.push_running("t1", 1, now), 1);
    lock(&ctx.devices).set_push_mute("a", TOKEN_A, "en", vec!["error".to_owned()]);
    assert_eq!(ctx.push_terminal("t1", "error", now + Duration::from_secs(2)), 1);
    assert_eq!(ctx.push_terminal("t1", "error", now + Duration::from_secs(3)), 0);
}

#[test]
fn a_muted_done_group_also_mutes_running() {
    let rig = rig("g1-mute", "", false);
    add(&rig, "a", TOKEN_A, false);
    let ctx = ready(&rig, Fake::ok());
    ctx.board.publish(summary(None));
    lock(&ctx.devices).set_push_mute("a", TOKEN_A, "en", vec!["done".to_owned()]);
    assert_eq!(ctx.push_running("t1", 1, Instant::now()), 0);
    assert_eq!(ctx.push_open("p1", Instant::now()), Ok(1));
}

#[test]
fn open_checks_the_pane_the_service_and_the_phones_and_stops_at_the_info_quota() {
    let rig = rig("g1-open", "", false);
    let none = rig.remote.send_pane(&json!({ "paneKey": "p1" }));
    assert_eq!(none, json!({ "ok": false, "error": "off" }));
    let ctx = ready(&rig, Fake::ok());
    ctx.running.store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(ctx.push_open("p1", Instant::now()), Err("off"));
    ctx.running.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(ctx.push_open("a/b", Instant::now()), Err("invalid"));
    assert_eq!(ctx.push_open("p1", Instant::now()), Err("notFound"));
    ctx.board.publish(summary(None));
    assert_eq!(ctx.push_open("p9", Instant::now()), Err("notFound"));
    assert_eq!(ctx.push_open("p1", Instant::now()), Err("none"));
    add(&rig, "a", TOKEN_A, false);
    add(&rig, "b", TOKEN_B, false);
    assert_eq!(rig.remote.send_pane(&json!("p1")), json!({ "ok": true, "sent": 2 }));
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|j| j.kind == "open" && j.pane == "p1"));
    let m = message(&jobs[0]);
    assert_eq!(m["message"]["data"], json!({ "m": ctx.machine_id(), "k": "open", "p": "p1" }));
    assert_eq!(m["message"]["notification"]["title"], title_body("open", "en").0);
    for _ in 1..PER_HOUR_INFO {
        assert!(ctx.push_open("p1", Instant::now()).is_ok());
    }
    assert_eq!(ctx.push_open("p1", Instant::now()), Err("limit"));
    assert_eq!(rig.remote.send_pane(&json!({ "paneKey": "p1" })), json!({ "ok": false, "error": "limit" }));
    assert_eq!(ctx.push_pane("p2", "question", "t2", None, Instant::now()), 2, "a question still gets through when info is full");
}

#[test]
fn open_content_text_is_masked_and_a_connected_phone_still_receives_it() {
    let rig = rig("g1-open-text", "", false);
    add(&rig, "a", TOKEN_A, true);
    let ctx = ready(&rig, Fake::ok());
    ctx.board.publish(summary(None));
    let job = Job { content: true, ..job_for(&ctx, "a", "p2", "open") };
    let (title, body) = ctx.notice(&job);
    assert_eq!(title, "Codex, Refactor the whole payments gateway module, orrery");
    assert_eq!(body, "Opened from your computer");
    let running = Job { content: true, started: Some(now_ms() - 125_000), ..job_for(&ctx, "a", "p2", "running") };
    assert_eq!(ctx.notice(&running).1, "Running for 2m");
}

#[test]
fn push_register_stores_pause_until_per_device_and_a_register_without_it_resumes() {
    let rig = rig("g1-register", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    let rt = runtime();
    rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        let until = now_ms() + 3_600_000;
        send_json(&mut ws, &mut t, json!({ "id": "r1", "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "en", "mute": ["waiting", "done", "error"], "pauseUntil": until })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r1").await["ok"], true);
        assert!(lock(&ctx.push).paused("p", now_ms()));
        send_json(&mut ws, &mut t, json!({ "id": "r2", "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "en" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r2").await["ok"], true);
        assert!(!lock(&ctx.push).paused("p", now_ms()));
        send_json(&mut ws, &mut t, json!({ "id": "r3", "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "en", "pauseUntil": now_ms() + 3 * 24 * 3_600_000u64 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r3").await["ok"], true);
        assert!(!lock(&ctx.push).paused("p", now_ms()));
        send_json(&mut ws, &mut t, json!({ "id": "r4", "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "en", "pauseUntil": until })).await;
        until_json(&mut ws, &mut t, |v| v["re"] == "r4").await;
        send_json(&mut ws, &mut t, json!({ "id": "u1", "t": "push.unregister" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "u1").await["ok"], true);
        assert!(!lock(&ctx.push).paused("p", now_ms()));
    });
    rig.remote.enable(false);
}
