use std::net::IpAddr;

use futures_util::SinkExt;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use super::alerts::{blocked_text, paired_text, place_text};
use super::devices::{Device, Devices};
use super::guard::{Guard, BLOCK_SECS, LOG_MAX, STRIKE_MAX};
use super::guard_ip::{key, Cidr};
use super::limits::{Gate, Refused, MAX_PENDING_PER_IP, MAX_PENDING_TOTAL};
use super::test_util::*;
use crate::state::lock;

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn junk() -> Vec<u8> {
    let mut out = vec![0u8, 1];
    out.extend_from_slice(&[9u8; 78]);
    out
}

fn ctl(frame: &[u8]) -> Value {
    assert_eq!(&frame[..2], &[0, 0]);
    serde_json::from_slice(&frame[2..]).unwrap()
}

#[test]
fn cidr_keys_and_ranges_cover_v4_v6_and_mapped_addresses() {
    assert_eq!(key(ip("2001:db8:1:2:aaaa:bbbb:cccc:dddd")), "2001:db8:1:2::/64");
    assert_eq!(key(ip("::ffff:203.0.113.9")), "203.0.113.9");
    let c = Cidr::parse("198.51.100.77/24").unwrap();
    assert_eq!(c.text(), "198.51.100.0/24");
    assert!(c.contains(ip("198.51.100.5")) && !c.contains(ip("198.51.101.5")) && !c.contains(ip("2001:db8::1")));
    assert!(Cidr::parse("10.0.0.0/7").is_none() && Cidr::parse("0.0.0.0/0").is_none() && Cidr::parse("2001::/15").is_none());
    assert!(Cidr::parse("1.2.3.4/33").is_none() && Cidr::parse("1.2.3.4/").is_none() && Cidr::parse("1.2.3").is_none());
    assert!(Cidr::parse("2001:db8::/32").unwrap().overlaps(&Cidr::parse("2001:db8:1:2::/64").unwrap()));
}

#[test]
fn twenty_lan_strikes_block_one_hour_then_a_day_then_a_week_and_survive_a_reload() {
    let dir = temp_dir("guard-steps");
    let g = Guard::load(&dir);
    let a = ip("192.168.1.50");
    let t0 = 1_900_000_000;
    for _ in 0..STRIKE_MAX - 1 {
        assert!(g.strike(a, "code", t0).is_none());
    }
    let row = g.strike(a, "code", t0).unwrap();
    assert_eq!((row.k.as_str(), row.level, row.until, row.n), ("192.168.1.50", 1, t0 + BLOCK_SECS[0], STRIKE_MAX as u32));
    assert!(!g.admits(a, t0 + 10) && g.admits(ip("192.168.1.51"), t0 + 10));
    assert!(g.admits(a, row.until));
    let again = Guard::load(&dir);
    assert!(!again.admits(a, t0 + 10));
    let t1 = row.until + 5;
    let mut next = None;
    for _ in 0..STRIKE_MAX {
        next = again.strike(a, "handshake", t1);
    }
    let r2 = next.take().unwrap();
    assert_eq!((r2.level, r2.until), (2, t1 + BLOCK_SECS[1]));
    let t2 = r2.until + 60;
    for _ in 0..STRIKE_MAX {
        next = again.strike(a, "handshake", t2);
    }
    assert_eq!(next.as_ref().map(|r| (r.level, r.until)), Some((3, t2 + BLOCK_SECS[2])));
    let t3 = next.take().unwrap().until + 8 * 86_400;
    for _ in 0..STRIKE_MAX {
        next = again.strike(a, "handshake", t3);
    }
    assert_eq!(next.map(|r| r.level), Some(1));
}

const STRIKE_WINDOW_STEP: u64 = 40;

#[test]
fn strikes_older_than_ten_minutes_do_not_add_up() {
    let g = Guard::load(&temp_dir("guard-window"));
    let a = ip("10.9.8.7");
    for i in 0..(STRIKE_MAX as u64 * 3) {
        assert!(g.strike(a, "code", 1_000 + i * STRIKE_WINDOW_STEP).is_none());
    }
}

#[test]
fn trusted_ips_and_live_session_ips_are_never_auto_blocked() {
    let g = Guard::load(&temp_dir("guard-spare"));
    g.set_trust(&["10.1.0.0/16".into()]).unwrap();
    let owner = ip("192.168.7.7");
    g.good_open(owner, 1_000);
    for _ in 0..STRIKE_MAX * 2 {
        assert!(g.strike(ip("10.1.2.3"), "code", 1_000).is_none());
        assert!(g.strike(owner, "code", 1_000).is_none());
    }
    g.good_close(owner, 1_000);
    for _ in 0..STRIKE_MAX {
        assert!(g.strike(owner, "code", 1_500).is_none());
    }
    assert!(g.strike(owner, "code", 1_700).is_some());
    assert_eq!(g.set_trust(&["0.0.0.0/0".into()]), Err("invalid"));
}

#[test]
fn manual_blocks_refuse_wide_ranges_and_live_ips_and_unblock_all_always_clears() {
    let g = Guard::load(&temp_dir("guard-manual"));
    let now = 1_000;
    g.good_open(ip("192.168.3.4"), now);
    assert_eq!(g.block("192.168.3.0/24", None, now), Err("live"));
    assert_eq!(g.block("0.0.0.0/0", None, now), Err("invalid"));
    assert_eq!(g.block("10.0.0.1", Some(5), now), Err("invalid"));
    g.block("198.51.100.0/24", None, now).unwrap();
    g.block("2001:db8::/32", Some(3600), now).unwrap();
    assert!(!g.admits(ip("198.51.100.9"), now) && !g.admits(ip("2001:db8::5"), now) && g.admits(ip("2001:db8::5"), now + 3600));
    let state = g.json(now);
    assert_eq!(state["blocked"].as_array().unwrap().len(), 2);
    assert_eq!(state["blocked"][0]["source"], "manual");
    let ops = g.take_ops(now);
    assert_eq!(ctl(&ops[1])["t"], "block.set");
    assert_eq!(ctl(&ops[1])["list"], json!([{ "c": "198.51.100.0/24" }, { "c": "2001:db8::/32", "ttl": 3600 }]));
    assert!(g.unblock(None));
    assert!(g.admits(ip("198.51.100.9"), now));
    let ops: Vec<Value> = g.take_ops(now).iter().map(|f| ctl(f)).collect();
    assert!(ops.contains(&json!({ "t": "block.remove", "all": true })));
    assert!(ops.contains(&json!({ "t": "block.set", "list": [] })));
}

#[test]
fn the_relay_sync_sends_trust_manual_pending_removes_and_asks_for_the_list() {
    let g = Guard::load(&temp_dir("guard-sync"));
    g.set_trust(&["203.0.113.0/24".into()]).unwrap();
    g.unblock(Some("198.51.100.7"));
    let frames: Vec<Value> = g.sync_frames(1_000).iter().map(|f| ctl(f)).collect();
    assert_eq!(frames[0], json!({ "t": "trust.set", "list": ["203.0.113.0/24"] }));
    assert_eq!(frames[1], json!({ "t": "block.set", "list": [] }));
    assert_eq!(frames[2], json!({ "t": "block.remove", "c": "198.51.100.7" }));
    assert_eq!(frames[3], json!({ "t": "block.list" }));
    assert!(g.take_ops(1_000).is_empty());
}

#[test]
fn the_access_log_keeps_the_last_200_lines_and_block_alerts_are_merged_hourly() {
    let g = Guard::load(&temp_dir("guard-log"));
    for i in 0..LOG_MAX + 20 {
        let e = super::guard::Entry { at: i as u64, ip: format!("10.0.0.{}", i % 250), via: "lan", country: None, device: None, result: "fail", why: "code" };
        g.log(e);
    }
    let log = g.json(0)["log"].as_array().unwrap().clone();
    assert_eq!(log.len(), LOG_MAX);
    assert_eq!(log[0]["at"], (LOG_MAX + 19) as u64);
    assert_eq!(g.alert_gate(5_000), Some(0));
    assert_eq!(g.alert_gate(5_100), None);
    assert_eq!(g.alert_gate(6_000), None);
    assert_eq!(g.alert_gate(8_601), Some(2));
    assert!(g.place_gate("d1", 1) && !g.place_gate("d1", 100) && g.place_gate("d2", 100) && g.place_gate("d1", 3_601));
}

#[test]
fn alert_texts_carry_no_secrets_and_clean_the_phone_name() {
    let t = paired_text(true, "Pixel\u{202E} «9»", Some("203.0.113.7"), Some("VN"), "relay");
    assert_eq!(t.body, "«Pixel 9» vừa được ghép từ 203.0.113.7 [VN] qua relay. Không phải bạn? Thu hồi ngay trong Cài đặt → Điều khiển từ xa.");
    let t = blocked_text(false, "2001:db8:1:2::/64", "relay", 3);
    assert_eq!(t.body, "IP 2001:db8:1:2::/64 was blocked on the relay after repeated failures (and 3 more in the last hour). See Settings → Remote.");
    assert!(place_text(false, "P", Some("198.51.100.1"), Some("US")).title.contains("country"));
    for t in [
        paired_text(false, "Pixel 9", Some("203.0.113.7"), Some("VN"), "relay"),
        paired_text(true, "Pixel 9", Some("203.0.113.7"), Some("VN"), "relay"),
        blocked_text(false, "203.0.113.7", "relay", 2),
        place_text(true, "Pixel 9", Some("203.0.113.7"), Some("US")),
    ] {
        assert!(!t.tg.contains("203.0.113") && !t.tg.contains("Pixel"), "{}", t.tg);
    }
    assert!(paired_text(false, "P", Some("1.2.3.4"), Some("VN"), "relay").tg.contains("[VN]"));
}

#[test]
fn a_full_shared_pending_pool_never_counts_as_a_failure_for_the_next_ip() {
    let gate = std::sync::Arc::new(Gate::default());
    let now = std::time::Instant::now();
    let mut held = Vec::new();
    for i in 0..MAX_PENDING_TOTAL / MAX_PENDING_PER_IP {
        for _ in 0..MAX_PENDING_PER_IP {
            held.push(gate.admit(ip(&format!("10.0.0.{i}")), now).ok().unwrap());
        }
    }
    let owner = ip("192.168.1.2");
    for _ in 0..100 {
        assert_eq!(gate.admit(owner, now).err(), Some(Refused::Total));
    }
    drop(held);
    assert!(gate.admit(owner, now).is_ok());
}

#[test]
fn unblocking_one_address_leaves_a_wider_manual_rule_in_place() {
    let g = Guard::load(&temp_dir("guard-exact"));
    g.block("198.51.0.0/16", None, 1_000).unwrap();
    g.block("198.51.100.7", None, 1_000).unwrap();
    assert!(g.unblock(Some("198.51.100.7")));
    assert!(!g.admits(ip("198.51.100.7"), 1_000));
    assert_eq!(g.json(1_000)["blocked"].as_array().unwrap().len(), 1);
    assert!(g.unblock(Some("198.51.0.0/16")));
    assert!(g.admits(ip("198.51.100.7"), 1_000));
}

#[test]
fn devices_remember_countries_and_only_later_ones_are_new() {
    let dir = temp_dir("guard-places");
    let mut d = Devices::load(&dir);
    d.add(Device::new("p1", "Pixel", &[7u8; 32], 1));
    assert!(!d.note_place("p1", Some("VN")));
    assert!(!d.note_place("p1", Some("VN")));
    assert!(!d.note_place("p1", None));
    assert!(d.note_place("p1", Some("US")));
    assert!(!d.note_place("nope", Some("US")));
    let saved = Devices::load(&dir);
    assert_eq!(saved.list()[0].countries, ["VN", "US"]);
    assert_eq!(super::devices::known_platform("web"), "web");
}

#[test]
fn twenty_bad_lan_handshakes_block_the_ip_alert_once_and_the_desktop_can_always_unblock() {
    let rig = rig("guard-lan", "", false);
    rig.remote.enable(true);
    let addr = rig.addr();
    let rt = runtime();
    let state = rt.block_on(async {
        for _ in 0..STRIKE_MAX {
            let mut ws = connect(addr).await;
            ws.send(Message::Binary(junk().into())).await.unwrap();
            assert_eq!(close_code(&mut ws).await, Some(super::link::CLOSE_HANDSHAKE));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let refused = tokio_tungstenite::connect_async(format!("ws://{addr}/remote")).await;
        assert!(refused.is_err());
        let state = rig.remote.guard_call(&json!({ "op": "get" }))["state"].clone();
        assert!(rig.remote.status()["guardRev"].as_u64().unwrap() > 1);
        assert_eq!(rig.remote.guard_call(&json!({ "op": "unblock", "key": "127.0.0.1" }))["ok"], true);
        let mut back = connect(addr).await;
        back.send(Message::Binary(junk().into())).await.unwrap();
        assert_eq!(close_code(&mut back).await, Some(super::link::CLOSE_HANDSHAKE));
        state
    });
    let blocked = state["blocked"].as_array().unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!((blocked[0]["key"].as_str(), blocked[0]["source"].as_str(), blocked[0]["level"].as_u64()), (Some("127.0.0.1"), Some("lan"), Some(1)));
    assert!(state["log"].as_array().unwrap().iter().any(|e| e["result"] == "blocked"));
    let notes = lock(&rig.host.notes).clone();
    assert_eq!(notes.iter().filter(|n| n.contains("IP blocked")).count(), 1, "{notes:?}");
    let tgs = lock(&rig.host.telegrams).clone();
    assert_eq!(tgs.iter().filter(|n| n.contains("IP was blocked")).count(), 1, "{tgs:?}");
    assert!(tgs.iter().all(|n| !n.contains("127.0.0.1")));
    assert_eq!(rig.remote.guard_call(&json!({ "op": "block", "cidr": "0.0.0.0/0" }))["error"], "invalid");
    assert_eq!(rig.remote.guard_call(&json!({ "op": "trust", "list": ["10.0.0.0/8", "fd00::/8"] }))["error"], "invalid");
    assert_eq!(rig.remote.guard_call(&json!({ "op": "trust", "list": ["10.0.0.0/8"] }))["state"]["trusted"], json!(["10.0.0.0/8"]));
    rig.remote.enable(false);
}

#[test]
fn a_new_pairing_alerts_the_desktop_and_telegram_without_the_code() {
    let rig = rig("guard-paired", "", false);
    rig.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    let reply = runtime().block_on(pair(rig.addr(), &qr, &Phone::new("p-new"))).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    let notes = lock(&rig.host.notes).clone();
    assert!(notes.iter().any(|n| n.starts_with("Orrery Remote: new device: “Pixel Test” was just paired from 127.0.0.1 via LAN.")), "{notes:?}");
    let tgs = lock(&rig.host.telegrams).clone();
    assert_eq!(tgs.len(), 1);
    assert!(!tgs[0].contains("127.0.0.1") && !tgs[0].contains("Pixel"), "{tgs:?}");
    let log = rig.remote.guard_call(&json!({}))["state"]["log"].clone();
    assert_eq!(log[0]["why"], "paired");
    assert_eq!(log[0]["device"], "Pixel Test");
    rig.remote.enable(false);
}

#[test]
fn relay_pair_tries_keep_a_shared_ceiling_across_ips() {
    use super::ctx::{MAX_PAIR_TRIES, MAX_RELAY_PAIR_TRIES};
    use super::link::{Budget, Link, Origin, Outbox, Peer};
    let (tx, _keep) = tokio::sync::mpsc::channel(1);
    let (_in, rx) = tokio::sync::mpsc::channel(1);
    let origin = Origin { ip: Some("2001:db8:1:2::9".into()), country: None, pairing: false };
    let link = Link { rx, out: Outbox::Lan(tx), peer: Peer::Relay(3), origin, budget: std::sync::Arc::new(Budget::default()) };
    assert_eq!(super::session::try_keys(&link), [("relay:2001:db8:1:2::/64".to_owned(), MAX_PAIR_TRIES), ("relay".to_owned(), MAX_RELAY_PAIR_TRIES)]);
    let rig = rig("guard-tries", "", false);
    let ctx = rig.remote.ctx().unwrap();
    let now = std::time::Instant::now();
    let keys = |i: usize| [(format!("relay:10.0.0.{i}"), MAX_PAIR_TRIES), ("relay".to_owned(), MAX_RELAY_PAIR_TRIES)];
    for i in 0..MAX_RELAY_PAIR_TRIES {
        assert!(ctx.take_pair_try(&keys(i), now));
    }
    assert!(!ctx.take_pair_try(&keys(99), now));
    for _ in 0..MAX_PAIR_TRIES {
        assert!(ctx.take_pair_try(&[("ip:192.168.0.2".to_owned(), MAX_PAIR_TRIES)], now));
    }
    assert!(!ctx.take_pair_try(&[("ip:192.168.0.2".to_owned(), MAX_PAIR_TRIES)], now));
}
