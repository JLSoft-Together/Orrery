use std::net::Ipv4Addr;

use serde_json::json;

use super::ctx::PairAsk;
use super::devices::Devices;
use super::pair_meta::{adb_hash, PairMeta, Via};
use super::test_util::{decode_qr, pair, rig, runtime, Phone};
use crate::state::lock;

#[test]
fn web_pair_options_are_cleaned() {
    let usb = PairMeta::from_web(&json!({ "via": "usb", "extraLan": ["127.0.0.1", "10.0.0.1", "127.0.0.1", "127.0.0.2", "127.0.0.3", "x", 5], "adbSerialHash": "ABCDEF0123456789" }));
    assert_eq!(usb.via, Via::Usb);
    assert_eq!(usb.extra, vec![Ipv4Addr::LOCALHOST, Ipv4Addr::new(127, 0, 0, 2)]);
    assert_eq!(usb.adb, "abcdef0123456789");
    assert_eq!(usb.ip, None);
    assert_eq!(PairMeta::from_web(&json!({ "via": "usb", "adbSerialHash": "abc" })).adb, "");
    for arg in [json!(null), json!({}), json!({ "via": "lan", "extraLan": ["127.0.0.1"] }), json!({ "via": "qr", "adbSerialHash": "0123456789abcdef", "extraLan": ["127.0.0.1"] })] {
        assert_eq!(PairMeta::from_web(&arg), PairMeta::default(), "{arg}");
    }
    assert_eq!(adb_hash("0123456789abcdeg"), None);
    assert_eq!(adb_hash("0123456789abcdef0"), None);
    let lan = PairMeta::lan("::ffff:192.168.1.9".parse().unwrap());
    assert!(lan.admits(Some("192.168.1.9".parse().unwrap())));
    assert!(!lan.admits(Some("192.168.1.10".parse().unwrap())));
    assert!(!lan.admits(None));
    assert!(PairMeta::default().admits(None));
    assert!(!lan.wants_relay() && usb.wants_relay());
}

#[test]
fn a_usb_pairing_puts_loopback_first_and_stores_the_serial_hash() {
    let r = rig("pair-usb", "", false);
    r.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    r.remote.enable(true);
    let started = r.remote.call("ccm:remote-pair-start", &[json!({ "via": "usb", "extraLan": ["127.0.0.1"], "adbSerialHash": "0123456789ABCDEF" })]).unwrap();
    assert_eq!(started["via"], "usb");
    let qr = decode_qr(&started);
    assert_eq!(qr.lan[0].host, "127.0.0.1");
    assert_eq!(qr.lan[0].port, r.addr().port());
    let reply = runtime().block_on(pair(r.addr(), &qr, &Phone::new("p-usb"))).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    assert_eq!(lock(&r.host.pair_asks)[0].source, "usb");
    let dev = &r.remote.status()["devices"][0];
    assert_eq!((dev["pairedVia"].as_str(), dev["adbSerialHash"].as_str()), (Some("usb"), Some("0123456789abcdef")));
    let saved = Devices::load(&r.dir);
    assert_eq!((saved.list()[0].paired_via.as_str(), saved.list()[0].adb_serial_hash.as_str()), ("usb", "0123456789abcdef"));

    let started = r.remote.call("ccm:remote-pair-start", &[json!(null)]).unwrap();
    assert_eq!(started["via"], "qr");
    let reply = runtime().block_on(pair(r.addr(), &decode_qr(&started), &Phone::new("p-qr"))).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    let saved = Devices::load(&r.dir);
    let qr_dev = saved.find("p-qr").unwrap();
    assert_eq!((qr_dev.paired_via.as_str(), qr_dev.adb_serial_hash.as_str()), ("qr", ""));
    r.remote.enable(false);
}

#[test]
fn the_confirm_title_names_where_the_code_came_from() {
    let vi = super::text::remote_text("vi");
    let en = super::text::remote_text("en");
    let mut ask = PairAsk { name: "A33".into(), platform: "android".into(), ip: Some("127.0.0.1".into()), country: None, via: "lan", source: "usb" };
    assert_eq!(vi.pair_title(&ask), "Ghép qua cáp USB từ IP 127.0.0.1: điện thoại android «A33»?");
    assert_eq!(en.pair_title(&ask), "Pair over USB cable from IP 127.0.0.1: android phone “A33”?");
    ask.source = "lan";
    ask.ip = Some("192.168.1.20".into());
    assert_eq!(vi.pair_title(&ask), "Ghép qua trang cài từ IP 192.168.1.20: điện thoại android «A33»?");
    assert_eq!(en.pair_title(&ask), "Pair via the install page from IP 192.168.1.20: android phone “A33”?");
    ask.source = "qr";
    ask.via = "relay";
    assert_eq!(vi.pair_title(&ask), "Ghép qua mã QR (relay) từ IP 192.168.1.20: điện thoại android «A33»?");
    assert_eq!(en.pair_title(&ask), "Pair via QR code (relay) from IP 192.168.1.20: android phone “A33”?");
}
