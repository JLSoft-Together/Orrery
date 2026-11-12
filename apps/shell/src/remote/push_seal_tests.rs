use std::time::Instant;

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, X25519};
use core_remote_proto::StaticKeypair;
use serde_json::{json, Value};

use super::config::now_secs;
use super::devices::Device;
use super::fcm::Outcome;
use super::fcm_tests::{script, Fake};
use super::push::{message, title_body, Job};
use super::push_seal::*;
use super::push_tests::{ready, TOKEN_A, TOKEN_B};
use super::test_util::*;
use super::webpush::{b64, unb64};
use crate::state::lock;

const TAG_LEN: usize = 16;
const FIXTURE: &str = include_str!("../../../mobile/test/fixtures/push_seal_v2.json");

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

fn to_hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

#[allow(deprecated)]
fn fixed(raw: &[u8]) -> EphemeralPrivateKey {
    EphemeralPrivateKey::generate(&X25519, &ring::test::rand::FixedSliceRandom { bytes: raw }).unwrap()
}

fn open(private: &[u8], desktop: &[u8], machine: &str, kind: &str, sealed: &str) -> Option<Vec<u8>> {
    let raw = unb64(sealed)?;
    if raw.len() < KEY_LEN + TAG_LEN {
        return None;
    }
    let (ephemeral, body) = raw.split_at(KEY_LEN);
    let me = fixed(private);
    let mine = me.compute_public_key().ok()?.as_ref().to_vec();
    let fixed_ss = StaticKeypair::from_private(private).ok()?.agree(desktop).ok()?;
    let okm = agree_ephemeral(me, &UnparsedPublicKey::new(&X25519, ephemeral), |s| derive(s, &fixed_ss[..], ephemeral, &mine, desktop, machine)).ok()??;
    let key = LessSafeKey::new(UnboundKey::new(&CHACHA20_POLY1305, &okm[..32]).ok()?);
    let mut buf = body.to_vec();
    let plain = key.open_in_place(Nonce::try_assume_unique_for_key(&okm[32..]).ok()?, Aad::from(aad(machine, kind)), &mut buf).ok()?;
    Some(plain.to_vec())
}

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

#[test]
fn the_shared_vector_matches_rfc7748_and_the_dart_fixture_byte_for_byte() {
    let f = fixture();
    let s = |k: &str| f[k].as_str().unwrap().to_owned();
    let device = fixed(&hex(&s("devicePrivate")));
    assert_eq!(to_hex(device.compute_public_key().unwrap().as_ref()), s("devicePublic"));
    let ephemeral = fixed(&hex(&s("ephemeralPrivate")));
    assert_eq!(to_hex(ephemeral.compute_public_key().unwrap().as_ref()), s("ephemeralPublic"));
    let shared = agree_ephemeral(ephemeral, &UnparsedPublicKey::new(&X25519, hex(&s("devicePublic"))), <[u8]>::to_vec).unwrap();
    assert_eq!(to_hex(&shared), s("shared"));
    let desktop = StaticKeypair::from_private(&hex(&s("desktopPrivate"))).unwrap();
    assert_eq!(to_hex(desktop.public()), s("desktopPublic"));
    let fixed_ss = desktop.agree(&hex(&s("devicePublic"))).unwrap();
    assert_eq!(to_hex(&fixed_ss[..]), s("staticShared"));
    let phone_ss = StaticKeypair::from_private(&hex(&s("devicePrivate"))).unwrap().agree(desktop.public()).unwrap();
    assert_eq!(phone_ss, fixed_ss);
    let okm = derive(&shared, &fixed_ss[..], &hex(&s("ephemeralPublic")), &hex(&s("devicePublic")), desktop.public(), &s("machine")).unwrap();
    assert_eq!(to_hex(&okm[..]), s("okm"));
    let sealed = seal_with(fixed(&hex(&s("ephemeralPrivate"))), &desktop, &hex(&s("devicePublic")), &s("machine"), &s("kind"), s("plain").as_bytes()).unwrap();
    assert_eq!(b64(&sealed), s("e"));
    let opened = open(&hex(&s("devicePrivate")), desktop.public(), &s("machine"), &s("kind"), &s("e")).unwrap();
    assert_eq!(String::from_utf8(opened).unwrap().trim_end(), s("plain"));
    let other = StaticKeypair::generate().unwrap();
    assert!(open(&hex(&s("devicePrivate")), other.public(), &s("machine"), &s("kind"), &s("e")).is_none());
}

#[test]
fn a_seal_opens_only_with_the_device_key_desktop_key_machine_and_kind_and_hides_the_length() {
    let phone = Phone::new("a");
    let desk = StaticKeypair::generate().unwrap();
    let dp = *desk.public();
    let private = phone.key.export_private();
    let plain = br#"{"b":"x","t":"y"}"#;
    let one = seal(&desk, phone.key.public(), "m1", "done", plain).unwrap();
    let two = seal(&desk, phone.key.public(), "m1", "done", plain).unwrap();
    assert_ne!(one, two);
    assert_eq!(open(&private[..], &dp, "m1", "done", &one).unwrap(), padded(plain));
    assert_eq!(padded(plain).len(), PAD_STEP);
    assert_eq!(unb64(&one).unwrap().len(), KEY_LEN + PAD_STEP + TAG_LEN);
    assert!(open(&private[..], &dp, "m2", "done", &one).is_none());
    assert!(open(&private[..], &dp, "m1", "question", &one).is_none());
    assert!(open(&Phone::new("b").key.export_private()[..], &dp, "m1", "done", &one).is_none());
    let mut raw = unb64(&one).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 1;
    assert!(open(&private[..], &dp, "m1", "done", &b64(&raw)).is_none());
    assert!(open(&private[..], StaticKeypair::generate().unwrap().public(), "m1", "done", &one).is_none());
    assert!(seal(&desk, &[0u8; 32], "m1", "done", plain).is_none());
    assert!(seal(&desk, &[1u8; 31], "m1", "done", plain).is_none());
    assert_eq!(padded(&[b'a'; 64]).len(), 64);
    assert_eq!(padded(&[b'a'; 65]).len(), 128);
    assert_eq!(padded(b"").len(), 64);
}

fn sealed_job(key: &[u8; 32], seal_ok: bool) -> Job {
    Job { device: "a".into(), token: TOKEN_A.into(), machine: "Qm9yZWFsLW1hY2hpbmUtMQ".into(), pane: "p".repeat(128), kind: "question", locale: "vi", content: true, seal_key: Some(*key), seal_ok, ..Job::default() }
}

#[test]
fn the_worst_case_text_stays_under_the_fcm_4_kb_data_limit() {
    let phone = Phone::new("a");
    let desk = StaticKeypair::generate().unwrap();
    let title = "😀".repeat(120);
    let body = "\"".repeat(120);
    for ok in [true, false] {
        let job = sealed_job(phone.key.public(), ok);
        let out = sealed_fcm(&job, &desk, phone.key.public(), &title, &body).unwrap();
        assert!(data_size(&out) <= DATA_MAX, "{}", data_size(&out));
        assert!(out.to_string().len() < DATA_MAX + 1024);
    }
    let huge = "a".repeat(5000);
    assert!(sealed_fcm(&sealed_job(phone.key.public(), true), &desk, phone.key.public(), &huge, "").is_none());
}

fn sealed_device(rig: &Rig, id: &str, token: &str, seal: bool, ok: bool) -> Phone {
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    assert!(devices.set_push(id, token, "vi"));
    devices.set_push_seal(id, seal, ok, now_secs());
    phone
}

fn sent(http: &Fake) -> Value {
    let calls = http.fcm_calls();
    serde_json::from_str(&calls.last().unwrap().3).unwrap()
}

fn target(ctx: &super::ctx::Ctx, id: &str) -> Job {
    let job = ctx.push_targets(|d| d.device_id == id).pop().unwrap();
    Job { pane: "p1".into(), kind: "done", worked: Some(12 * 60_000), ..job }
}

#[test]
fn a_proven_phone_gets_a_data_only_sealed_message_with_the_content_inside() {
    let rig = rig("seal-proven", "", false);
    let phone = sealed_device(&rig, "a", TOKEN_A, true, true);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    let job = target(&ctx, "a");
    assert_eq!(job.seal_key, Some(*phone.key.public()));
    assert_eq!(ctx.deliver(&job), Outcome::Sent);
    let m = &sent(&http)["message"];
    assert_eq!(m["token"], TOKEN_A);
    assert!(m.get("notification").is_none());
    assert_eq!(m["android"], json!({ "priority": "high" }));
    let data = m["data"].as_object().unwrap();
    let mut keys: Vec<_> = data.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["e", "k", "m", "v"]);
    assert_eq!((data["v"].as_str(), data["k"].as_str()), (Some("2"), Some("done")));
    assert_eq!(data["m"], ctx.machine_id());
    let plain = open(&phone.key.export_private()[..], ctx.keys.noise.public(), &ctx.machine_id(), "done", data["e"].as_str().unwrap()).unwrap();
    let inner: Value = serde_json::from_slice(&plain).unwrap();
    assert_eq!(inner, json!({ "t": ctx.notice(&job).0, "b": "Xong sau 12m", "p": "p1" }));
    assert!(!m.to_string().contains("Xong sau"));
}

#[test]
fn an_unproven_phone_gets_the_fixed_notification_plus_the_sealed_probe() {
    let rig = rig("seal-probe", "", false);
    let phone = sealed_device(&rig, "a", TOKEN_A, true, false);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    let job = target(&ctx, "a");
    assert_eq!(ctx.deliver(&job), Outcome::Sent);
    let m = &sent(&http)["message"];
    let fixed = message(&job);
    assert_eq!(m["notification"], fixed["message"]["notification"]);
    assert_eq!(m["notification"]["title"], title_body("done", "vi").0);
    assert_eq!(m["android"], fixed["message"]["android"]);
    assert_eq!((m["data"]["m"].as_str(), m["data"]["p"].as_str(), m["data"]["k"].as_str(), m["data"]["v"].as_str()), (Some(ctx.machine_id().as_str()), Some("p1"), Some("done"), Some("2")));
    let plain = open(&phone.key.export_private()[..], ctx.keys.noise.public(), &ctx.machine_id(), "done", m["data"]["e"].as_str().unwrap()).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&plain).unwrap()["b"], "Xong sau 12m");
}

#[test]
fn old_apps_content_off_bad_keys_and_security_alerts_keep_the_fixed_message() {
    let rig = rig("seal-fixed", "", false);
    sealed_device(&rig, "old", TOKEN_A, false, true);
    sealed_device(&rig, "off", TOKEN_B, true, true);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    assert!(!lock(&ctx.devices).find("old").unwrap().push_seal_ok);
    lock(&ctx.devices).set_push_content("off", false);
    for id in ["old", "off"] {
        let job = target(&ctx, id);
        assert_eq!(ctx.deliver(&job), Outcome::Sent);
        assert_eq!(sent(&http), message(&job));
    }
    lock(&ctx.devices).set_push_content("off", true);
    let mut bad = target(&ctx, "off");
    bad.seal_key = Some([0u8; 32]);
    assert_eq!(ctx.fcm_message(&bad), message(&bad));
    assert_eq!(device_key("not-a-key"), None);
    assert_eq!(device_key(&b64(&[7u8; 31])), None);
    assert_eq!(ctx.push_security(None), 2);
    let jobs = ctx.take_push_jobs();
    assert!(jobs.iter().all(|j| !j.content && ctx.fcm_message(j) == message(j)));
}

#[test]
fn mute_push_off_and_a_revoked_phone_still_stop_a_sealed_push() {
    let rig = rig("seal-gates", "", false);
    sealed_device(&rig, "a", TOKEN_A, true, true);
    let http = script(vec![], vec![]);
    let ctx = ready(&rig, http.clone());
    lock(&ctx.devices).set_push_mute("a", TOKEN_A, "vi", vec!["done".into()]);
    assert_eq!(ctx.push_pane("p1", "done", "", None, Instant::now()), 0);
    let job = target(&ctx, "a");
    lock(&ctx.devices).set_push_off("a", true);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    lock(&ctx.devices).set_push_off("a", false);
    lock(&ctx.devices).remove(&["a".into()]);
    assert_eq!(ctx.deliver(&job), Outcome::Fail("no_device"));
    assert!(http.fcm_calls().is_empty());
}

#[test]
fn push_register_stores_the_seal_capability_and_proof_and_an_old_app_clears_them() {
    let rig = rig("seal-ws", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p", "Phone", phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        let state = |ctx: &super::ctx::Ctx| lock(&ctx.devices).find("p").map(|d| (d.push_seal, d.push_seal_ok)).unwrap();
        for (i, (fields, want)) in [
            (json!({ "seal": 2, "sealOk": false }), (true, false)),
            (json!({ "seal": 2, "sealOk": true }), (true, true)),
            (json!({ "seal": 3, "sealOk": true }), (true, true)),
            (json!({ "seal": 1, "sealOk": true }), (false, false)),
            (json!({ "sealOk": true }), (false, false)),
            (json!({ "seal": "2", "sealOk": true }), (false, false)),
            (json!({ "seal": 2 }), (true, false)),
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("r{i}");
            let mut msg = json!({ "id": id, "t": "push.register", "token": TOKEN_A, "platform": "android", "locale": "vi", "content": true });
            msg.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
            send_json(&mut ws, &mut t, msg).await;
            assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == id.as_str()).await["ok"], true);
            assert_eq!(state(&ctx), want, "{fields}");
        }
    });
    rig.remote.enable(false);
}

#[test]
fn the_proof_expires_after_48_hours_and_the_phone_falls_back_to_the_probe() {
    let rig = rig("seal-stale", "", false);
    sealed_device(&rig, "a", TOKEN_A, true, true);
    let http = Fake::ok();
    let ctx = ready(&rig, http.clone());
    let now = now_secs();
    let d = lock(&ctx.devices).find("a").unwrap().clone();
    assert!(d.push_seal_at >= now - 5 && d.push_seal_at <= now);
    assert!(seal_fresh(&d, now + SEAL_OK_TTL - 10));
    assert!(!seal_fresh(&d, now + SEAL_OK_TTL + 10));
    assert!(target(&ctx, "a").seal_ok);
    lock(&ctx.devices).set_push_seal("a", true, true, now - SEAL_OK_TTL - 60);
    let job = target(&ctx, "a");
    assert!(!job.seal_ok);
    assert_eq!(ctx.deliver(&job), Outcome::Sent);
    let m = &sent(&http)["message"];
    assert_eq!(m["notification"]["title"], title_body("done", "vi").0);
    assert_eq!(m["data"]["v"], "2");
    lock(&ctx.devices).set_push_seal("a", true, true, now);
    assert!(target(&ctx, "a").seal_ok);
    lock(&ctx.devices).set_push_seal("a", true, false, now);
    let d = lock(&ctx.devices).find("a").unwrap().clone();
    assert_eq!((d.push_seal_ok, d.push_seal_at), (false, 0));
    assert!(!target(&ctx, "a").seal_ok);
}
