use std::path::PathBuf;
use std::time::Instant;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::{json, Value};

use super::fcm::Outcome;
use super::push_web::parse_sub;
use super::push_web_tests::add_web;
use super::test_util::rig;
use super::webpush::Vapid;
use crate::state::lock;

fn live_dir() -> PathBuf {
    PathBuf::from(std::env::var("ORRERY_WEBPUSH_LIVE").expect("ORRERY_WEBPUSH_LIVE"))
}

#[test]
#[ignore]
fn live_key() {
    let dir = live_dir();
    let (vapid, der) = Vapid::generate().unwrap();
    std::fs::write(dir.join("vapid.key"), B64.encode(der.as_slice())).unwrap();
    std::fs::write(dir.join("vapid.pub"), vapid.public_key()).unwrap();
}

#[test]
#[ignore]
fn live_send() {
    let dir = live_dir();
    let der = B64.decode(std::fs::read_to_string(dir.join("vapid.key")).unwrap().trim()).unwrap();
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("sub.json")).unwrap()).unwrap();
    let sub = parse_sub(&raw).expect("endpoint not accepted by the whitelist");
    let rig = rig("wp-live", "https://orrery-relay.example.workers.dev", true);
    let ctx = rig.remote.ctx().unwrap();
    ctx.webpush.set_vapid(Vapid::from_pkcs8(&der));
    ctx.running.store(true, std::sync::atomic::Ordering::SeqCst);
    lock(&ctx.config).webpush_subject = "mailto:orrery-remote@localhost".into();
    add_web(&ctx, "edge", &sub.endpoint, "vi", true);
    assert!(lock(&ctx.devices).set_web_push("edge", sub.clone(), "vi", Vec::new(), true));
    ctx.publish(json!({ "rev": 1, "tabs": [{ "key": "t", "name": "Backend", "project": "D:\\Code\\orrery", "panes": [{ "key": "p1", "agent": "claude", "title": "✳ Live push", "terminalId": "term-live" }] }] }));
    ctx.note_episode(&json!({ "type": "telegram", "episode": "tw:term-live|0", "reason": "question", "terminalId": "term-live", "name": "orrery", "cwd": "D:\\Code\\orrery", "ask": { "question": "Gửi thật từ Rust tới WNS? API_KEY=sk-test-123", "options": ["Có"], "multi": false } }));
    assert_eq!(ctx.push_terminal_after("term-live", "attention", None, Instant::now()), 1);
    let job = ctx.take_push_jobs().remove(0);
    let first = ctx.deliver(&job);
    let report = json!({ "first": format!("{first:?}"), "endpointHost": super::webpush::audience(&sub.endpoint) });
    std::fs::write(dir.join("result.json"), report.to_string()).unwrap();
    assert_eq!(first, Outcome::Sent);
}

#[test]
#[ignore]
fn live_gone() {
    let dir = live_dir();
    let der = B64.decode(std::fs::read_to_string(dir.join("vapid.key")).unwrap().trim()).unwrap();
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("sub.json")).unwrap()).unwrap();
    let sub = parse_sub(&raw).unwrap();
    let rig = rig("wp-live-gone", "", false);
    let ctx = rig.remote.ctx().unwrap();
    ctx.webpush.set_vapid(Vapid::from_pkcs8(&der));
    add_web(&ctx, "edge", &sub.endpoint, "en", true);
    assert!(lock(&ctx.devices).set_web_push("edge", sub.clone(), "en", Vec::new(), true));
    let job = super::push::Job { device: "edge".into(), machine: "m".into(), kind: "test", locale: "en", web: Some(sub), ..Default::default() };
    let outcome = ctx.deliver(&job);
    std::fs::write(dir.join("gone.json"), json!({ "outcome": format!("{outcome:?}"), "cleared": lock(&ctx.devices).find("edge").unwrap().web_push.is_none() }).to_string()).unwrap();
}
