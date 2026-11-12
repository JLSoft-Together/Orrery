use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_128_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use serde_json::{json, Value};

use super::config::now_secs;
use super::ctx::Ctx;
use super::devices::{clean_mute, Device, Devices, WebSub};
use super::fcm::Outcome;
use super::push::{title_body, Job, PER_HOUR};
use super::push_web::{parse_sub, subject_for, DEFAULT_SUBJECT};
use super::test_util::*;
use super::webpush::{derive, unb64};
use super::webpush_send::{known_host, topic, valid_endpoint, PushHttp};
use crate::state::lock;

pub const UA_PUBLIC: &str = "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
const UA_PRIVATE: &str = "q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94";
pub const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";
pub const WNS: &str = "https://wns2-sg2p.notify.windows.com/w/?token=BQYAAAB%2bAbC";
const APPLE: &str = "https://web.push.apple.com/QGc1aBcDeF";

type Sent = (String, Vec<(String, String)>, Vec<u8>);

#[derive(Default)]
pub struct FakePush {
    pub calls: Mutex<Vec<Sent>>,
    pub codes: Mutex<VecDeque<Option<u16>>>,
    pub after: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl PushHttp for FakePush {
    fn post(&self, url: &str, headers: &[(&str, String)], body: &[u8]) -> Option<u16> {
        lock(&self.calls).push((url.to_owned(), headers.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect(), body.to_vec()));
        if let Some(f) = lock(&self.after).as_ref() {
            f();
        }
        lock(&self.codes).pop_front().unwrap_or(Some(201))
    }
}

pub fn sub(endpoint: &str) -> WebSub {
    WebSub { endpoint: endpoint.to_owned(), p256dh: UA_PUBLIC.to_owned(), auth: AUTH.to_owned() }
}

#[allow(deprecated)]
pub fn open(body: &[u8]) -> Value {
    let raw = unb64(UA_PRIVATE).unwrap();
    let key = EphemeralPrivateKey::generate(&ECDH_P256, &ring::test::rand::FixedSliceRandom { bytes: &raw }).unwrap();
    assert_eq!(&body[16..20], &4096u32.to_be_bytes());
    assert_eq!(body[20], 65);
    let as_public = &body[21..86];
    let secret = agree_ephemeral(key, &UnparsedPublicKey::new(&ECDH_P256, as_public), |s| s.to_vec()).unwrap();
    let d = derive(&secret, &unb64(AUTH).unwrap(), &unb64(UA_PUBLIC).unwrap(), as_public, &body[..16]);
    let key = LessSafeKey::new(UnboundKey::new(&AES_128_GCM, &d.cek).unwrap());
    let mut buf = body[86..].to_vec();
    let plain = key.open_in_place(Nonce::try_assume_unique_for_key(&d.nonce).unwrap(), Aad::empty(), &mut buf).unwrap();
    let end = plain.iter().rposition(|b| *b != 0).unwrap();
    assert_eq!(plain[end], 2);
    serde_json::from_slice(&plain[..end]).unwrap()
}

fn header<'a>(call: &'a Sent, name: &str) -> &'a str {
    call.1.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).unwrap_or("")
}

pub fn web_rig(tag: &str) -> (Rig, Arc<Ctx>, Arc<FakePush>) {
    let rig = rig(tag, "https://orrery-relay.me.workers.dev", true);
    let ctx = rig.remote.ctx().unwrap();
    ctx.running.store(true, Ordering::SeqCst);
    let http = Arc::new(FakePush::default());
    ctx.webpush.set_http(http.clone());
    *lock(&ctx.webpush.gap) = Duration::ZERO;
    *lock(&ctx.settle) = Duration::ZERO;
    (rig, ctx, http)
}

pub fn add_web(ctx: &Ctx, id: &str, endpoint: &str, locale: &str, content: bool) {
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    assert!(devices.set_web_push(id, sub(endpoint), locale, Vec::new(), content));
}

fn summary() -> Value {
    json!({ "rev": 1, "tabs": [{ "key": "t", "name": "Backend", "project": "D:\\Code\\orrery", "panes": [{ "key": "p1", "agent": "claude", "title": "✳ Fix login", "terminalId": "term-1" }] }] })
}

#[test]
fn only_https_endpoints_of_known_push_services_are_accepted() {
    for ok in [WNS, APPLE, "https://fcm.googleapis.com/fcm/send/abc:APA91b", "https://updates.push.services.mozilla.com/wpush/v2/gAAA", "https://web.push.apple.com:443/x"] {
        assert!(valid_endpoint(ok), "{ok}");
    }
    for bad in [
        "http://web.push.apple.com/x",
        "https://web.push.apple.com",
        "https://web.push.apple.com/",
        "https://evil.com/web.push.apple.com/x",
        "https://web.push.apple.com.evil.com/x",
        "https://user:pw@web.push.apple.com/x",
        "https://web.push.apple.com:8443/x",
        "https://notify.windows.com/x",
        "https://.notify.windows.com/x",
        "https://a..notify.windows.com/x",
        "https://WEB.PUSH.APPLE.COM/x",
        "https://127.0.0.1/x",
        "https://web.push.apple.com/x y",
        "https://web.push.apple.com/x#frag",
    ] {
        assert!(!valid_endpoint(bad), "{bad}");
    }
    assert!(!valid_endpoint(&format!("https://web.push.apple.com/{}", "a".repeat(2100))));
    assert!(known_host("db5p.notify.windows.com") && !known_host("x.notify.windows.com.evil"));
}

#[test]
fn subscriptions_need_a_p256_key_and_a_16_byte_secret() {
    let good = json!({ "endpoint": APPLE, "p256dh": UA_PUBLIC, "auth": AUTH });
    assert_eq!(parse_sub(&good), Some(sub(APPLE)));
    assert_eq!(parse_sub(&json!({ "endpoint": APPLE, "p256dh": format!("{UA_PUBLIC}="), "auth": format!("{AUTH}==") })), Some(sub(APPLE)));
    let mut off = unb64(UA_PUBLIC).unwrap();
    off[40] ^= 0xff;
    for bad in [
        json!({ "endpoint": "https://evil.com/x", "p256dh": UA_PUBLIC, "auth": AUTH }),
        json!({ "endpoint": APPLE, "p256dh": super::webpush::b64(&off), "auth": AUTH }),
        json!({ "endpoint": APPLE, "p256dh": UA_PUBLIC, "auth": "c2hvcnQ" }),
        json!({ "endpoint": APPLE, "p256dh": "!!", "auth": AUTH }),
        json!({ "endpoint": APPLE, "auth": AUTH }),
        json!({ "endpoint": 5, "p256dh": UA_PUBLIC, "auth": AUTH }),
    ] {
        assert_eq!(parse_sub(&bad), None, "{bad}");
    }
    assert_eq!(subject_for("mailto:me@x.dev", "https://r.dev/"), "mailto:me@x.dev");
    assert_eq!(subject_for("", "https://orrery-relay.me.workers.dev/"), "https://orrery-relay.me.workers.dev");
    assert_eq!(subject_for("bogus", "http://r.dev"), DEFAULT_SUBJECT);
}

#[test]
fn a_web_push_is_aes128gcm_encrypted_to_the_subscription_with_vapid_and_carries_the_content() {
    let (_rig, ctx, http) = web_rig("wp-content");
    add_web(&ctx, "ip", WNS, "vi", true);
    ctx.publish(summary());
    ctx.note_episode(&json!({ "type": "telegram", "episode": "tw:term-1|0", "reason": "question", "terminalId": "term-1", "name": "api", "cwd": "D:\\Code\\orrery", "ask": { "question": "Dùng Postgres hay SQLite? token=abc123", "options": ["a"], "multi": false } }));
    assert_eq!(ctx.push_terminal_after("term-1", "attention", None, Instant::now()), 1);
    let job = ctx.take_push_jobs().remove(0);
    assert_eq!(ctx.deliver(&job), Outcome::Sent);
    let calls = lock(&http.calls).clone();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.0, WNS);
    assert_eq!(header(call, "content-encoding"), "aes128gcm");
    assert_eq!(header(call, "ttl"), "3600");
    assert_eq!(header(call, "urgency"), "high");
    assert_eq!(header(call, "topic"), topic(&ctx.machine_id(), "p1", AUTH));
    assert_eq!(header(call, "topic").len(), 32);
    let auth = header(call, "authorization");
    let (jwt, k) = auth.strip_prefix("vapid t=").unwrap().split_once(", k=").unwrap();
    assert_eq!(Some(k.to_owned()), ctx.webpush.public_key());
    let claims: Value = serde_json::from_slice(&unb64(jwt.split('.').nth(1).unwrap()).unwrap()).unwrap();
    assert_eq!(claims["aud"], "https://wns2-sg2p.notify.windows.com");
    assert_eq!(claims["sub"], "https://orrery-relay.me.workers.dev");
    let exp = claims["exp"].as_u64().unwrap();
    assert!(exp > now_secs() && exp <= now_secs() + 12 * 3600);
    let payload = open(&call.2);
    assert_eq!(payload["title"], "Claude · Fix login · orrery");
    assert_eq!(payload["body"], "Dùng Postgres hay SQLite? token=••••");
    assert_eq!(payload["k"], "question");
    assert_eq!(payload["m"], ctx.machine_id());
    assert_eq!(payload["p"], "p1");
    assert_eq!(payload["tag"], format!("{}:p1", ctx.machine_id()));
    assert!(!String::from_utf8_lossy(&call.2).contains("Postgres"));
    assert_eq!(call.2.len() % 64, (86 + 16) % 64);
}

#[test]
fn without_include_content_the_web_push_keeps_the_fixed_text_and_done_shows_the_time() {
    let (_rig, ctx, http) = web_rig("wp-fixed");
    add_web(&ctx, "plain", WNS, "en", false);
    add_web(&ctx, "rich", APPLE, "vi", true);
    ctx.publish(summary());
    assert_eq!(ctx.push_terminal_after("term-1", "done", Some(12 * 60_000), Instant::now()), 2);
    for job in ctx.take_push_jobs() {
        assert_eq!(ctx.deliver(&job), Outcome::Sent);
    }
    let calls = lock(&http.calls).clone();
    let plain = open(&calls.iter().find(|c| c.0 == WNS).unwrap().2);
    let rich = open(&calls.iter().find(|c| c.0 == APPLE).unwrap().2);
    assert_eq!((plain["title"].as_str(), plain["body"].as_str()), (Some(title_body("done", "en").0), Some(title_body("done", "en").1)));
    assert_eq!(rich["body"], "Xong sau 12m");
    assert_eq!(rich["k"], "done");
}

#[test]
fn security_alerts_reach_web_phones_with_fixed_text_and_skip_the_new_device() {
    let (rig, ctx, http) = web_rig("wp-security");
    add_web(&ctx, "old", APPLE, "vi", true);
    add_web(&ctx, "new", WNS, "en", true);
    ctx.alert_paired("new", "Evil Phone", Some("203.0.113.9"), Some("VN"), "relay");
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(ctx.deliver(&jobs[0]), Outcome::Sent);
    let payload = open(&lock(&http.calls)[0].2);
    assert_eq!((payload["title"].as_str(), payload["body"].as_str()), (Some(title_body("security", "vi").0), Some(title_body("security", "vi").1)));
    assert!(payload.get("p").is_none());
    let text = payload.to_string();
    assert!(!text.contains("203.0.113") && !text.contains("Evil"));
    assert!(lock(&rig.host.notes).iter().any(|n| n.contains("Evil Phone")));
}

#[test]
fn gone_subscriptions_are_dropped_and_busy_services_are_retried() {
    for code in [404, 410] {
        let (rig, ctx, http) = web_rig("wp-gone");
        add_web(&ctx, "a", APPLE, "en", true);
        add_web(&ctx, "b", WNS, "en", true);
        *lock(&http.codes) = VecDeque::from([Some(code)]);
        let job = Job { device: "a".into(), machine: "m".into(), pane: "p1".into(), kind: "done", locale: "en", web: Some(sub(APPLE)), content: true, ..Job::default() };
        assert_eq!(ctx.deliver(&job), Outcome::Dead);
        assert!(lock(&ctx.devices).find("a").unwrap().web_push.is_none());
        assert!(lock(&ctx.devices).find("b").unwrap().web_push.is_some());
        assert!(Devices::load(&rig.dir).find("a").unwrap().web_push.is_none());
    }
    let (_rig, ctx, http) = web_rig("wp-busy");
    add_web(&ctx, "a", APPLE, "en", true);
    let job = Job { device: "a".into(), machine: "m".into(), pane: "p1".into(), kind: "done", locale: "en", web: Some(sub(APPLE)), ..Job::default() };
    *lock(&http.codes) = VecDeque::from([Some(429), Some(503), Some(500)]);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("unavailable"));
    assert_eq!(lock(&http.calls).len(), 3);
    assert!(lock(&ctx.devices).find("a").unwrap().web_push.is_some());
    *lock(&http.codes) = VecDeque::from([None, Some(502), Some(201)]);
    assert_eq!(ctx.deliver(&job), Outcome::Sent);
    *lock(&http.codes) = VecDeque::from([Some(403)]);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("denied"));
    *lock(&http.codes) = VecDeque::from([Some(413)]);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("rejected"));
    assert_eq!(lock(&http.calls).len(), 8);
    assert!(lock(&ctx.devices).find("a").unwrap().web_push.is_some());
}

#[test]
fn web_and_fcm_phones_share_the_pane_and_hourly_limits_and_honour_mute_and_the_desktop_switch() {
    let (rig, ctx, _http) = web_rig("wp-limits");
    add_web(&ctx, "w", APPLE, "en", true);
    let now = Instant::now();
    assert_eq!(ctx.push_pane("p1", "done", "", None, now), 1);
    assert_eq!(ctx.push_pane("p1", "question", "", None, now + Duration::from_secs(29)), 0);
    assert_eq!(ctx.push_pane("p1", "done", "", None, now + Duration::from_secs(31)), 1);
    for i in 0..PER_HOUR {
        ctx.push_pane(&format!("q{i}"), "done", "", None, now + Duration::from_secs(40));
    }
    assert_eq!(ctx.push_pane("zz", "done", "", None, now + Duration::from_secs(41)), 0);
    ctx.take_push_jobs();
    lock(&ctx.devices).set_web_push("w", sub(APPLE), "en", clean_mute(["done", "waiting", "error"]), true);
    let later = now + Duration::from_secs(7200);
    assert_eq!(ctx.push_pane("m1", "done", "", None, later), 0);
    lock(&ctx.devices).set_web_push("w", sub(APPLE), "en", Vec::new(), true);
    assert_eq!(rig.remote.push_device(&json!({ "deviceId": "w", "on": false })), json!({ "ok": true, "on": false }));
    assert_eq!(ctx.push_pane("m2", "done", "", None, later), 0);
    assert_eq!(ctx.push_security(None), 0);
    let status = rig.remote.status();
    let d = status["devices"].as_array().unwrap().iter().find(|d| d["deviceId"] == "w").unwrap().clone();
    assert_eq!((d["push"].as_str(), d["pushOn"].as_bool(), d["pushContent"].as_bool()), (Some("web"), Some(false), Some(true)));
    assert!(Devices::load(&rig.dir).find("w").unwrap().push_off);
    assert_eq!(rig.remote.push_device(&json!({ "deviceId": "w", "on": true }))["ok"], true);
    assert_eq!(ctx.push_pane("m3", "done", "", None, later), 1);
    assert_eq!(rig.remote.push_device(&json!({ "deviceId": "zz", "on": true })), json!({ "ok": false, "error": "no_device" }));
    assert_eq!(rig.remote.push_device(&json!({ "deviceId": "w" })), json!({ "ok": false, "error": "invalid" }));
    assert_eq!(rig.remote.push_test(&json!({ "deviceId": "w" })), json!({ "ok": true, "sent": 1 }));
}

#[test]
fn a_web_phone_gets_the_vapid_key_registers_its_subscription_and_gets_no_push_while_open() {
    let rig = rig("wp-ws", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("w");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("w", "iPhone", phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    let rt = runtime();
    rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        send_json(&mut ws, &mut t, json!({ "id": "h1", "t": "hello", "proto": 1, "app": "pwa", "platform": "web", "caps": [] })).await;
        let cfg = until_json(&mut ws, &mut t, |v| v["t"] == "webpush.config").await;
        assert_eq!(cfg["key"].as_str().map(str::to_owned), ctx.webpush.public_key());
        assert_eq!(unb64(cfg["key"].as_str().unwrap()).unwrap().len(), 65);
        let reg = |id: &str, endpoint: &str, p256dh: &str| json!({ "id": id, "t": "push.register", "kind": "webpush", "endpoint": endpoint, "p256dh": p256dh, "auth": AUTH, "locale": "vi", "mute": ["error"], "content": false });
        for (id, bad) in [("b1", reg("b1", "https://evil.example/x", UA_PUBLIC)), ("b2", reg("b2", APPLE, "BAAA")), ("b3", reg("b3", "http://web.push.apple.com/x", UA_PUBLIC))] {
            send_json(&mut ws, &mut t, bad).await;
            let reply = until_json(&mut ws, &mut t, |v| v["re"] == id).await;
            assert_eq!((reply["ok"].as_bool(), reply["error"].as_str()), (Some(false), Some("invalid")));
        }
        assert!(lock(&ctx.devices).find("w").unwrap().web_push.is_none());
        send_json(&mut ws, &mut t, reg("r1", APPLE, UA_PUBLIC)).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r1").await["ok"], true);
        let d = lock(&ctx.devices).find("w").unwrap().clone();
        assert_eq!(d.web_push, Some(sub(APPLE)));
        assert_eq!((d.push_locale.as_str(), d.push_mute.clone(), d.push_content), ("vi", vec!["error".to_owned()], false));
        assert!(ctx.is_live("w"));
        assert_eq!(ctx.push_pane("p1", "done", "", None, Instant::now()), 0);
        send_json(&mut ws, &mut t, json!({ "id": "u1", "t": "push.unregister" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "u1").await["ok"], true);
        assert!(lock(&ctx.devices).find("w").unwrap().web_push.is_none());
    });
    rig.remote.enable(false);
}

#[test]
fn an_android_hello_gets_no_vapid_event_and_fcm_registration_drops_a_web_subscription() {
    let rig = rig("wp-android", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("a");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("a", "Pixel", phone.key.public(), now_secs())));
    lock(&ctx.devices).set_web_push("a", sub(APPLE), "en", Vec::new(), true);
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        send_json(&mut ws, &mut t, json!({ "id": "e", "t": "echo.none" })).await;
        let mut seen = Vec::new();
        loop {
            let v = until_json(&mut ws, &mut t, |_| true).await;
            if v["re"] == "e" {
                break;
            }
            seen.push(v["t"].as_str().unwrap_or("").to_owned());
        }
        assert!(!seen.iter().any(|s| s == "webpush.config"), "{seen:?}");
        send_json(&mut ws, &mut t, json!({ "id": "r", "t": "push.register", "token": "tokenAAAAAAAAAAAAAAAAAAAAAAAA:APA91bAAAA", "platform": "android", "locale": "en", "content": false })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "r").await["ok"], true);
    });
    let d = lock(&ctx.devices).find("a").unwrap().clone();
    assert!(d.web_push.is_none() && !d.push_token.is_empty() && !d.push_content);
    assert_eq!(d.push_channel(), "fcm");
    rig.remote.enable(false);
}

#[test]
fn a_queued_push_is_dropped_when_the_phone_was_revoked_switched_off_or_resubscribed_meanwhile() {
    let (rig, ctx, http) = web_rig("wp-recheck");
    add_web(&ctx, "a", APPLE, "en", true);
    let job = Job { device: "a".into(), machine: "m".into(), pane: "p1".into(), kind: "done", locale: "en", web: Some(sub(APPLE)), ..Job::default() };
    lock(&ctx.devices).set_push_off("a", true);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    lock(&ctx.devices).set_push_off("a", false);
    lock(&ctx.devices).set_web_push("a", WebSub { auth: "AAAAAAAAAAAAAAAAAAAAAA".into(), ..sub(APPLE) }, "en", Vec::new(), true);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    lock(&ctx.devices).set_web_push("a", sub(APPLE), "en", Vec::new(), true);
    ctx.revoke(&["a".to_owned()]);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    assert!(lock(&http.calls).is_empty());
    add_web(&ctx, "b", WNS, "en", true);
    let job = Job { device: "b".into(), web: Some(sub(WNS)), ..job };
    *lock(&http.codes) = VecDeque::from([Some(503), Some(201)]);
    let c = ctx.clone();
    *lock(&http.after) = Some(Box::new(move || {
        lock(&c.devices).set_push_off("b", true);
    }));
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    assert_eq!(lock(&http.calls).len(), 1);
    let _ = rig;
}

#[test]
fn the_topic_is_stable_per_pane_and_subscription_but_differs_across_phones() {
    let a = topic("m", "p1", AUTH);
    assert_eq!(a, topic("m", "p1", AUTH));
    assert_ne!(a, topic("m", "p1", "AAAAAAAAAAAAAAAAAAAAAA"));
    assert_ne!(a, topic("m", "p2", AUTH));
    assert_eq!(a.len(), 32);
    assert!(a.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
}

#[test]
fn the_vapid_audience_is_the_origin_without_the_default_port() {
    use super::webpush::audience;
    assert_eq!(audience("https://web.push.apple.com:443/x").as_deref(), Some("https://web.push.apple.com"));
    assert_eq!(audience("https://wns2-pn1p.notify.windows.com/w/?token=a").as_deref(), Some("https://wns2-pn1p.notify.windows.com"));
}
