use std::sync::atomic::Ordering;

use core_remote_proto::pair::{LanAddr, PairCode, PairTicket};

use super::devices::Devices;
use super::ctx::{PairAsk, MAX_PAIR_TRIES};
use super::link::{CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_RATE};
use super::pair::{clean_name, Pairing, MAX_NAME, MAX_PAIR_FAILS, MAX_QR_FAILS, PAIR_TTL};
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
    assert_eq!(qr.lan, vec![LanAddr::new("127.0.0.1", rig.addr().port())]);
    assert_eq!(&qr.mkey, &rig.desktop_key());
    assert!(qr.relay.is_none());
    assert!(qr.ticket.is_none());
    assert_eq!(started["relay"], false);
    assert_eq!(started["lan"], 1);
    assert!(qr.exp <= super::config::now_secs() + PAIR_TTL && PAIR_TTL == 120);
    assert_eq!(rig.remote.status()["pairing"]["relay"], false);
    let rt = runtime();
    let phone = Phone::new("phone-1");
    let reply = rt.block_on(pair(rig.addr(), &qr, &phone)).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    assert_eq!(reply["name"], "Test PC");
    assert_eq!(reply["machine"].as_str(), Some(qr.machine.as_str()));
    assert_eq!(lock(&rig.host.asked).as_slice(), ["Pixel Test"]);
    let ask = PairAsk { name: "Pixel Test".into(), platform: "android".into(), ip: Some("127.0.0.1".into()), country: None, via: "lan" };
    assert_eq!(lock(&rig.host.pair_asks).as_slice(), [ask]);
    assert_eq!(Devices::load(&rig.dir).list()[0].platform, "android");
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
fn five_wrong_codes_from_one_ip_bar_that_ip_but_keep_the_qr_shown_for_other_sources() {
    let rig = rig("pair-wrong", "", false);
    rig.host.answer.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    let good = decode_qr(&rig.remote.pair_start().unwrap());
    let mut bad = good.clone();
    bad.code = PairCode::generate().unwrap();
    let rt = runtime();
    assert_eq!(MAX_PAIR_TRIES, MAX_PAIR_FAILS);
    for _ in 0..MAX_PAIR_FAILS {
        assert_eq!(rt.block_on(pair(rig.addr(), &bad, &Phone::new("x"))), Err(Some(CLOSE_HANDSHAKE)));
    }
    assert!(!rig.remote.status()["pairing"].is_null());
    assert!(rig.remote.status()["pairEnded"].is_null());
    assert_eq!(rt.block_on(pair(rig.addr(), &good, &Phone::new("y"))), Err(Some(CLOSE_RATE)));
    assert!(!rig.remote.status()["pairing"].is_null());
    assert!(lock(&rig.host.asked).is_empty());
    assert!(Devices::load(&rig.dir).list().is_empty());
    let state = rig.remote.guard_call(&serde_json::json!({ "op": "get" }))["state"].clone();
    let log = state["log"].as_array().unwrap();
    assert_eq!(log.iter().filter(|e| e["why"] == "code").count(), MAX_PAIR_FAILS);
    assert!(log.iter().all(|e| e["ip"] == "127.0.0.1" && e["via"] == "lan" && e["result"] == "fail"));
    let text = state.to_string();
    let b64 = |c: &PairCode| base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, c.as_bytes());
    assert!(!text.contains(&b64(&good.code)) && !text.contains(&b64(&bad.code)));
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
    let (_, exp) = p.start(1_000, None).unwrap();
    assert_eq!(exp, 1_000 + PAIR_TTL);
    assert!(p.ticket(exp - 1).is_some());
    assert!(p.ticket(exp).is_none());
    assert!(p.active(exp).is_none());
    p.start(2_000, None).unwrap();
    let old = p.ticket(2_000).unwrap();
    p.start(2_001, None).unwrap();
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

#[test]
fn five_failures_bar_one_source_and_twenty_in_total_cancel_the_code_and_disarm_the_relay() {
    let p = Pairing::default();
    let armed = p.armed();
    let ticket = PairTicket::generate().unwrap();
    let (_, exp) = p.start(1_000, Some(&ticket)).unwrap();
    assert_eq!(*armed.borrow(), Some((ticket.hash(), exp)));
    assert_eq!(p.active(1_000), Some((exp, true)));
    let old = p.ticket(1_000).unwrap();
    p.start(1_001, Some(&ticket)).unwrap();
    assert!(!p.fail(&old, "lan:10.0.0.9"));
    let cur = p.ticket(1_001).unwrap();
    for _ in 0..MAX_PAIR_FAILS {
        assert!(!p.fail(&cur, "lan:10.0.0.9"));
    }
    assert!(p.barred(&cur, "lan:10.0.0.9"));
    assert!(!p.barred(&cur, "lan:10.0.0.10"));
    assert!(!p.barred(&cur, "relay:10.0.0.9"));
    assert!(p.active(1_001).is_some());
    assert!(armed.borrow().is_some());
    for i in MAX_PAIR_FAILS..MAX_QR_FAILS - 1 {
        assert!(!p.fail(&cur, &format!("lan:10.0.1.{i}")));
    }
    assert!(p.active(1_001).is_some());
    assert!(p.fail(&cur, "lan:10.0.2.1"));
    assert!(p.active(1_001).is_none());
    assert_eq!(p.ended(), Some("tries"));
    assert_eq!(*armed.borrow(), None);
    assert!(!p.fail(&cur, "lan:10.0.2.1"));
    p.start(1_002, None).unwrap();
    assert_eq!(p.ended(), None);
    assert_eq!(*armed.borrow(), None);
    let t = p.ticket(1_002).unwrap();
    assert!(p.consume(&t));
    assert_eq!(*armed.borrow(), None);
}

#[test]
fn a_denied_confirm_over_lan_shows_the_ip_and_sends_pair_denied() {
    let rig = rig("pair-deny-ask", "", false);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    let reply = runtime().block_on(pair(rig.addr(), &qr, &Phone::new("d2"))).unwrap();
    assert_eq!(reply["t"], "pair.denied");
    let asks = lock(&rig.host.pair_asks).clone();
    assert_eq!(asks.len(), 1);
    assert_eq!((asks[0].via, asks[0].ip.as_deref(), asks[0].platform.as_str()), ("lan", Some("127.0.0.1"), "android"));
    assert!(rig.remote.status()["pairing"].is_null());
    assert!(rig.remote.status()["pairEnded"].is_null());
    rig.remote.enable(false);
}

#[test]
fn the_confirm_title_names_platform_ip_country_and_route() {
    let vi = super::text::remote_text("vi");
    let en = super::text::remote_text("en");
    let mut ask = PairAsk { name: "Pixel 9".into(), platform: "android".into(), ip: Some("203.0.113.7".into()), country: Some("VN".into()), via: "relay" };
    assert_eq!(vi.pair_title(&ask), "Ghép qua relay từ IP 203.0.113.7 [VN]: điện thoại android «Pixel 9»?");
    assert_eq!(en.pair_title(&ask), "Pair via relay from IP 203.0.113.7 [VN]: android phone “Pixel 9”?");
    ask.via = "lan";
    ask.country = Some("vn<".into());
    ask.ip = Some("1.2.3.4\n<b>".into());
    ask.platform = "and roid".into();
    assert_eq!(vi.pair_title(&ask), "Ghép qua LAN từ IP không rõ: điện thoại «Pixel 9»?");
    assert_eq!(en.pair_title(&ask), "Pair via LAN from an unknown IP: phone “Pixel 9”?");
}

#[test]
fn a_hostile_phone_name_cannot_fake_the_route_or_ip_in_the_confirm_title() {
    let vi = super::text::remote_text("vi");
    let en = super::text::remote_text("en");
    let evil = "A» từ IP 192.168.1.5 qua LAN? «B";
    assert!(evil.chars().count() <= MAX_NAME);
    let ask = PairAsk { name: clean_name(evil), platform: "android".into(), ip: Some("203.0.113.7".into()), country: Some("RU".into()), via: "relay" };
    let title = vi.pair_title(&ask);
    assert!(title.starts_with("Ghép qua relay từ IP 203.0.113.7 [RU]: điện thoại android «"), "{title}");
    assert_eq!(title.matches('«').count(), 1);
    assert_eq!(title.matches('»').count(), 1);
    assert!(title.ends_with("«A từ IP 192.168.1.5 qua LAN? B»?"), "{title}");
    let en_evil = PairAsk { name: clean_name("x” from IP 10.0.0.2 via LAN? “y\u{202E}\n\"z'"), ..ask.clone() };
    let title = en.pair_title(&en_evil);
    assert!(title.starts_with("Pair via relay from IP 203.0.113.7 [RU]: android phone “"), "{title}");
    assert_eq!(title.matches('“').count() + title.matches('”').count(), 2);
    assert!(!title.contains('"') && !title.contains('\'') && !title.contains('\n') && !title.contains('\u{202E}'));
    assert_eq!(super::text::shown_name("«»“”\"\u{200B}\t"), "?");
}
