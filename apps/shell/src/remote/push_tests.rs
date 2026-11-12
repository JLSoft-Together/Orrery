use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::config::now_secs;
use super::devices::{Device, Devices};
use super::fcm::Outcome;
use super::fcm_files::PACKAGE;
use super::fcm_tests::{account, gs_json, sa_json, script, Fake, KEY_A};
use super::push::*;
use super::test_util::*;
use crate::state::lock;

pub(super) const TOKEN_A: &str = "tokenAAAAAAAAAAAAAAAAAAAAAAAA:APA91bAAAA";
pub(super) const TOKEN_B: &str = "tokenBBBBBBBBBBBBBBBBBBBBBBBB:APA91bBBBB";

fn add_device(rig: &Rig, id: &str, token: &str, locale: &str) {
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    if !token.is_empty() {
        assert!(devices.set_push(id, token, locale));
    }
}

pub(super) fn ready(rig: &Rig, http: Arc<Fake>) -> Arc<super::ctx::Ctx> {
    let ctx = rig.remote.ctx().unwrap();
    ctx.running.store(true, std::sync::atomic::Ordering::SeqCst);
    let parsed = super::fcm_files::parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
    lock(&ctx.config).firebase = parsed.firebase;
    ctx.fcm.set_http(http);
    *lock(&ctx.fcm.gap) = Duration::ZERO;
    ctx.fcm.set_account(Some(account()));
    ctx
}

fn job(token: &str, kind: &'static str, locale: &'static str) -> Job {
    Job { device: if token == TOKEN_A { "a" } else { "b" }.into(), token: token.to_owned(), machine: "machine-1".into(), pane: "pane-1".into(), kind, locale, ..Job::default() }
}

#[test]
fn every_kind_has_fixed_text_in_both_locales_and_the_payload_carries_no_content() {
    for kind in ["done", "question", "permission", "attention", "error"] {
        for locale in ["vi", "en"] {
            let (title, body) = title_body(kind, locale);
            assert!(!title.is_empty() && title.chars().count() <= 80);
            assert!(!body.is_empty() && body.chars().count() <= 160);
            let m = message(&job(TOKEN_A, kind_of(kind), locale));
            assert_eq!(m.as_object().unwrap().len(), 1);
            let m = &m["message"];
            assert_eq!(m["token"], TOKEN_A);
            assert_eq!(m["notification"], json!({ "title": title, "body": body }));
            assert_eq!(m["data"], json!({ "m": "machine-1", "p": "pane-1", "k": kind_of(kind) }));
            assert_eq!(m["android"]["priority"], "high");
            assert_eq!(m["android"]["notification"]["channel_id"], "attention");
            assert!(m.to_string().len() < 2048);
        }
    }
    assert_eq!(kind_of("anything else"), "attention");
    assert_ne!(title_body("done", "vi"), title_body("done", "en"));
    assert_eq!(title_body("done", "fr"), title_body("done", "en"));
    let t = message(&Job { pane: String::new(), ..job(TOKEN_A, "test", "vi") });
    assert!(t["message"]["data"].get("p").is_none());
    assert_eq!(t["message"]["data"]["k"], "test");
    assert_eq!(t["message"]["notification"]["body"], test_text("vi").1);
}

#[test]
fn only_plain_fcm_tokens_are_accepted() {
    assert!(valid_token(TOKEN_A));
    assert!(!valid_token("short"));
    assert!(!valid_token(&"a".repeat(513)));
    assert!(!valid_token("aaaaaaaaaaaaaaaa aaaa"));
    assert!(!valid_token("aaaaaaaaaaaaaaaa\"}aaaa"));
    assert!(!valid_token(""));
}

#[test]
fn the_token_is_stored_with_the_device_and_dies_with_it() {
    let dir = temp_dir("push-store");
    let mut devices = Devices::load(&dir);
    assert!(devices.add(Device::new("a", "A", &[1; 32], 10)));
    assert!(devices.add(Device::new("b", "B", &[2; 32], 10)));
    assert!(devices.set_push("a", TOKEN_A, "vi"));
    assert!(!devices.set_push("missing", TOKEN_B, "vi"));
    let reloaded = Devices::load(&dir);
    assert_eq!(reloaded.find("a").unwrap().push_token, TOKEN_A);
    assert_eq!(reloaded.find("a").unwrap().push_locale, "vi");
    assert!(reloaded.find("b").unwrap().push_token.is_empty());
    assert!(devices.set_push("b", TOKEN_A, "en"));
    assert!(devices.find("a").unwrap().push_token.is_empty());
    assert_eq!(devices.find("b").unwrap().push_token, TOKEN_A);
    devices.clear_push("b");
    assert!(devices.find("b").unwrap().push_token.is_empty());
    assert!(devices.set_push("a", TOKEN_B, "vi"));
    assert!(!devices.clear_push_token(""));
    assert!(devices.clear_push_token(TOKEN_B));
    assert!(!devices.clear_push_token(TOKEN_B));
    assert!(devices.set_push("a", TOKEN_A, "vi"));
    devices.remove(&["a".to_owned()]);
    assert!(std::fs::read_to_string(dir.join(super::config::DEVICES_FILE)).map(|t| !t.contains(TOKEN_A)).unwrap_or(true));
}

#[test]
fn revoking_a_device_removes_its_token_from_disk() {
    let rig = rig("push-revoke", "", false);
    add_device(&rig, "a", TOKEN_A, "en");
    let ctx = rig.remote.ctx().unwrap();
    assert_eq!(ctx.revoke(&["a".to_owned()]), 1);
    assert!(Devices::load(&rig.dir).find("a").is_none());
    let text = std::fs::read_to_string(rig.dir.join(super::config::DEVICES_FILE)).unwrap_or_default();
    assert!(!text.contains(TOKEN_A));
}

#[test]
fn a_pane_gets_one_push_per_30_seconds_and_a_device_30_per_hour() {
    let mut st = PushState::default();
    let t0 = Instant::now();
    assert!(st.admit("d", "p1", t0));
    assert!(!st.admit("d", "p1", t0 + Duration::from_secs(29)));
    assert!(st.admit("d", "p2", t0 + Duration::from_secs(29)));
    assert!(st.admit("other", "p1", t0 + Duration::from_secs(29)));
    assert!(st.admit("d", "p1", t0 + Duration::from_secs(31)));
    let mut st = PushState::default();
    for i in 0..PER_HOUR {
        assert!(st.admit("d", &format!("p{i}"), t0 + Duration::from_secs(i as u64)), "{i}");
    }
    assert!(!st.admit("d", "fresh", t0 + Duration::from_secs(100)));
    assert!(st.admit("d", "fresh", t0 + HOUR + Duration::from_secs(1)));
    let mut st = PushState::default();
    for i in 0..QUEUE_MAX + 5 {
        st.push(Job { pane: format!("p{i}"), ..job(TOKEN_A, "done", "en") });
    }
    let drained = st.drain();
    assert_eq!(drained.len(), QUEUE_MAX);
    assert_eq!(drained[0].pane, "p5");
    assert!(st.drain().is_empty());
}

#[test]
fn attention_queues_one_job_per_registered_device_with_its_locale() {
    let rig = rig("push-attn", "", false);
    add_device(&rig, "vi-phone", TOKEN_A, "vi");
    add_device(&rig, "en-phone", TOKEN_B, "en");
    add_device(&rig, "no-token", "", "");
    let ctx = rig.remote.ctx().unwrap();
    let now = Instant::now();
    assert_eq!(ctx.push_attention("p1", "question", now), 0);
    let ctx = ready(&rig, Fake::ok());
    ctx.running.store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(ctx.push_attention("p1", "question", now), 0);
    ctx.running.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(ctx.push_attention("a/b", "question", now), 0);
    assert_eq!(ctx.push_attention("p1", "question", now), 2);
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.len(), 2);
    let vi = jobs.iter().find(|j| j.token == TOKEN_A).unwrap();
    let en = jobs.iter().find(|j| j.token == TOKEN_B).unwrap();
    assert_eq!((vi.locale, en.locale), ("vi", "en"));
    assert_eq!(message(vi)["message"]["notification"]["title"], title_body("question", "vi").0);
    assert_eq!(message(en)["message"]["notification"]["title"], title_body("question", "en").0);
    assert_eq!(vi.machine, ctx.machine_id());
    assert_eq!(vi.pane, "p1");
    assert_eq!(ctx.push_attention("p1", "question", now + Duration::from_secs(5)), 0);
    assert_eq!(ctx.push_attention("p2", "done", now + Duration::from_secs(5)), 2);
    assert!(ctx.take_push_jobs().len() == 2 && ctx.take_push_jobs().is_empty());
}

#[test]
fn no_push_without_firebase_options_or_without_the_service_account() {
    let rig = rig("push-unready", "", false);
    add_device(&rig, "a", TOKEN_A, "en");
    let ctx = ready(&rig, Fake::ok());
    ctx.fcm.set_account(None);
    assert!(!ctx.push_ready());
    assert_eq!(ctx.push_attention("p1", "done", Instant::now()), 0);
    ctx.fcm.set_account(Some(account()));
    lock(&ctx.config).firebase = Default::default();
    assert!(!ctx.push_ready());
    assert_eq!(ctx.push_attention("p1", "done", Instant::now()), 0);
    assert_eq!(ctx.deliver(&job(TOKEN_A, "done", "en")), Outcome::Fail("not_configured"));
}

#[test]
fn a_dead_token_from_fcm_clears_the_device_token_and_others_stay() {
    let rig = rig("push-dead", "", false);
    add_device(&rig, "a", TOKEN_A, "en");
    add_device(&rig, "b", TOKEN_B, "en");
    let unregistered = json!({ "error": { "details": [{ "errorCode": "UNREGISTERED" }] } }).to_string();
    let ctx = ready(&rig, script(vec![], vec![Some((404, unregistered))]));
    assert_eq!(ctx.deliver(&job(TOKEN_A, "done", "en")), Outcome::Dead);
    assert!(lock(&ctx.devices).find("a").unwrap().push_token.is_empty());
    assert_eq!(lock(&ctx.devices).find("b").unwrap().push_token, TOKEN_B);
    assert!(Devices::load(&rig.dir).find("a").unwrap().push_token.is_empty());
    assert_eq!(ctx.deliver(&job(TOKEN_B, "done", "en")), Outcome::Sent);
    assert_eq!(lock(&ctx.devices).find("b").unwrap().push_token, TOKEN_B);
}

#[test]
fn a_transient_fcm_failure_keeps_the_token() {
    let rig = rig("push-transient", "", false);
    add_device(&rig, "a", TOKEN_A, "en");
    let ctx = ready(&rig, script(vec![], vec![Some((503, "{}".into())), Some((503, "{}".into())), Some((503, "{}".into()))]));
    assert_eq!(ctx.deliver(&job(TOKEN_A, "done", "en")), Outcome::Fail("unavailable"));
    assert_eq!(lock(&ctx.devices).find("a").unwrap().push_token, TOKEN_A);
    assert_eq!(ctx.fcm.last_error(), Some("unavailable"));
}

#[test]
fn the_running_remote_sends_a_queued_push_to_fcm_in_the_background() {
    let rig = rig("push-run", "", false);
    add_device(&rig, "a", TOKEN_A, "vi");
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    rig.remote.enable(true);
    assert_eq!(ctx.push_attention("p1", "permission", Instant::now()), 1);
    for _ in 0..200 {
        if !http.fcm_calls().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let calls = http.fcm_calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].0.ends_with("/projects/demo-proj/messages:send"));
    let sent: Value = serde_json::from_str(&calls[0].3).unwrap();
    assert_eq!(sent["message"]["token"], TOKEN_A);
    assert_eq!(sent["message"]["data"], json!({ "m": ctx.machine_id(), "p": "p1", "k": "permission" }));
    assert_eq!(sent["message"]["notification"]["title"], title_body("permission", "vi").0);
    rig.remote.enable(false);
}

#[test]
fn a_phone_registers_and_unregisters_over_a_real_session_gets_push_config_and_no_push_while_connected() {
    let rig = rig("push-ws", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    assert!(lock(&rig.remote.ctx().unwrap().devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    let ctx = rig.remote.ctx().unwrap();
    let rt = runtime();
    rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        let cfg = until_json(&mut ws, &mut t, |v| v["t"] == "push.config").await;
        assert_eq!(cfg, json!({ "t": "push.config" }));
        let parsed = super::fcm_files::parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
        lock(&ctx.config).firebase = parsed.firebase;
        ctx.push_config_changed();
        let cfg = until_json(&mut ws, &mut t, |v| v["t"] == "push.config").await;
        assert_eq!(cfg, json!({ "t": "push.config", "apiKey": "AIzaSyTestKey_0123456789-abc", "appId": "1:123456789012:android:feedbeef", "senderId": "123456789012", "projectId": "demo-proj" }));
        assert!(!cfg.to_string().contains("PRIVATE") && !cfg.to_string().contains("iam.gserviceaccount"));
        send_json(&mut ws, &mut t, json!({ "id": "r1", "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "vi" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r1").await["ok"], true);
        assert_eq!(lock(&ctx.devices).find("p").unwrap().push_token, TOKEN_A);
        assert_eq!(lock(&ctx.devices).find("p").unwrap().push_locale, "vi");
        for bad in [json!("short"), json!("with space aaaaaaaaaaaa"), Value::Null, json!(5)] {
            send_json(&mut ws, &mut t, json!({ "id": "bad", "t": "push.register", "token": bad, "platform": "android", "locale": "vi" })).await;
            let reply = until_json(&mut ws, &mut t, |v| v["re"] == "bad").await;
            assert_eq!((reply["ok"].as_bool(), reply["error"].as_str()), (Some(false), Some("invalid")));
        }
        assert_eq!(lock(&ctx.devices).find("p").unwrap().push_token, TOKEN_A);
        assert!(ctx.is_live("p"));
        ctx.fcm.set_account(Some(account()));
        assert_eq!(ctx.push_attention("p1", "done", Instant::now()), 0);
        send_json(&mut ws, &mut t, json!({ "id": "u1", "t": "push.unregister" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "u1").await["ok"], true);
        assert!(lock(&ctx.devices).find("p").unwrap().push_token.is_empty());
    });
    rig.remote.enable(false);
}

#[test]
fn a_muted_group_is_stored_cleaned_and_skipped() {
    use super::devices::{clean_mute, push_group};
    assert_eq!(clean_mute(["error", "bogus", "done", "error"]), vec!["done".to_owned(), "error".to_owned()]);
    assert_eq!(push_group("question"), "waiting");
    assert_eq!(push_group("permission"), "waiting");
    assert_eq!(push_group("attention"), "waiting");
    let dir = temp_dir("push-mute");
    let mut devices = Devices::load(&dir);
    assert!(devices.add(Device::new("a", "A", &[1; 32], 10)));
    assert!(devices.set_push_mute("a", TOKEN_A, "vi", clean_mute(["error"])));
    let reloaded = Devices::load(&dir);
    let d = reloaded.find("a").unwrap();
    assert!(d.wants("done") && d.wants("question") && !d.wants("error"));
    assert!(devices.set_push("a", TOKEN_A, "vi"));
    assert!(devices.find("a").unwrap().wants("error"));
}
