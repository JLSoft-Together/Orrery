use std::sync::atomic::Ordering;

use core_remote_proto::pair::PairCode;

use super::devices::Devices;
use super::ctx::MAX_PAIR_TRIES;
use super::link::{CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_RATE};
use super::pair::{clean_name, Pairing, MAX_NAME, PAIR_TTL};
use super::test_util::*;
use crate::state::lock;

#[test]
fn a_phone_pairs_over_localhost_after_the_e2e_confirm_and_then_opens_a_session() {
    std::env::set_var("CCM_E2E_CONFIRM", "ok");
    let rig = rig("pair-ok", "", false);
    rig.host.env_confirm.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    assert_eq!(rig.remote.bound().len(), 1);
    let started = rig.remote.pair_start().unwrap();
    let qr = decode_qr(&started);
    assert_eq!(qr.host.as_deref(), Some("127.0.0.1"));
    assert_eq!(qr.port, Some(rig.addr().port()));
    assert_eq!(&qr.mkey, &rig.desktop_key());
    assert!(qr.relay.is_none());
    let rt = runtime();
    let phone = Phone::new("phone-1");
    let reply = rt.block_on(pair(rig.addr(), &qr, &phone)).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    assert_eq!(reply["name"], "Test PC");
    assert_eq!(reply["machine"].as_str(), Some(qr.machine.as_str()));
    assert_eq!(lock(&rig.host.asked).as_slice(), ["Pixel Test"]);
    let saved = Devices::load(&rig.dir);
    assert_eq!(saved.list().len(), 1);
    assert_eq!(saved.list()[0].device_id, "phone-1");
    assert!(rig.remote.status()["pairing"].is_null());
    let desktop = rig.desktop_key();
    let welcome = rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await
    });
    assert_eq!(welcome["ok"], true);
    assert_eq!(welcome["re"], "h1");
    assert_eq!(Devices::load(&rig.dir).list()[0].platform, "android");
    rig.remote.enable(false);
}

#[test]
fn a_used_code_is_refused_the_second_time() {
    let rig = rig("pair-once", "", false);
    rig.host.answer.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    let rt = runtime();
    assert_eq!(rt.block_on(pair(rig.addr(), &qr, &Phone::new("a"))).unwrap()["t"], "pair.ok");
    assert_eq!(rt.block_on(pair(rig.addr(), &qr, &Phone::new("b"))), Err(Some(CLOSE_DEVICE)));
    assert_eq!(Devices::load(&rig.dir).list().len(), 1);
    rig.remote.enable(false);
}

#[test]
fn wrong_codes_keep_the_qr_but_one_ip_gets_five_tries_a_minute() {
    let rig = rig("pair-wrong", "", false);
    rig.host.answer.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    let good = decode_qr(&rig.remote.pair_start().unwrap());
    let mut bad = good.clone();
    bad.code = PairCode::generate().unwrap();
    let rt = runtime();
    for _ in 0..MAX_PAIR_TRIES {
        assert_eq!(rt.block_on(pair(rig.addr(), &bad, &Phone::new("x"))), Err(Some(CLOSE_HANDSHAKE)));
    }
    assert!(!rig.remote.status()["pairing"].is_null());
    assert_eq!(rt.block_on(pair(rig.addr(), &good, &Phone::new("y"))), Err(Some(CLOSE_RATE)));
    assert!(lock(&rig.host.asked).is_empty());
    assert!(Devices::load(&rig.dir).list().is_empty());
    rig.remote.enable(false);
}

#[test]
fn a_denied_confirm_sends_pair_denied_and_saves_nothing() {
    let rig = rig("pair-deny", "", false);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    let reply = runtime().block_on(pair(rig.addr(), &qr, &Phone::new("d"))).unwrap();
    assert_eq!(reply["t"], "pair.denied");
    assert!(Devices::load(&rig.dir).list().is_empty());
    rig.remote.enable(false);
}

#[test]
fn without_a_shown_qr_or_after_cancel_pairing_is_refused() {
    let rig = rig("pair-none", "", false);
    rig.host.answer.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    rig.remote.pair_cancel();
    assert_eq!(runtime().block_on(pair(rig.addr(), &qr, &Phone::new("c"))), Err(Some(CLOSE_DEVICE)));
    rig.remote.enable(false);
    assert!(rig.remote.pair_start().is_err());
}

#[test]
fn an_expired_code_gives_no_ticket_and_a_code_is_used_once() {
    let p = Pairing::default();
    let (_, exp) = p.start(1_000).unwrap();
    assert_eq!(exp, 1_000 + PAIR_TTL);
    assert!(p.ticket(exp - 1).is_some());
    assert!(p.ticket(exp).is_none());
    assert!(p.active(exp).is_none());
    p.start(2_000).unwrap();
    let old = p.ticket(2_000).unwrap();
    p.start(2_001).unwrap();
    let fresh = p.ticket(2_001).unwrap();
    assert!(!p.consume(&old));
    assert!(p.consume(&fresh));
    assert!(!p.consume(&fresh));
}

#[test]
fn device_names_lose_control_and_bidi_characters_and_are_capped() {
    assert_eq!(clean_name("  Pixel\u{202E}lexiP\u{200B}\n "), "PixellexiP");
    assert_eq!(clean_name("\u{2066}\u{202A}"), "Phone");
    assert_eq!(clean_name(&"x".repeat(100)).chars().count(), MAX_NAME);
}
