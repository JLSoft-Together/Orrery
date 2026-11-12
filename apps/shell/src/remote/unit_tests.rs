use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use super::config::{Config, DEFAULT_PORT};
use super::dashboard::{client_message, command, events_event, snapshot_event};
use super::devices::{Device, Devices, EXPIRE_SECS};
use super::host::theme;
use super::lan::{is_private_v4, lan_addrs, select};
use super::limits::{Gate, MAX_PENDING_PER_IP};
use super::mdns::{instance, tag, txt};
use super::test_util::*;
use super::{Options, Remote};
use crate::state::lock;

#[test]
fn only_private_ipv4_addresses_are_ever_bound() {
    let ips: Vec<IpAddr> = ["0.0.0.0", "127.0.0.1", "8.8.8.8", "169.254.3.4", "172.32.0.1", "172.15.9.9", "::1", "fe80::1", "10.1.2.3", "172.16.0.5", "172.31.255.1", "192.168.1.217", "192.168.1.217"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let picked = select(ips);
    let want: Vec<Ipv4Addr> = ["10.1.2.3", "172.16.0.5", "172.31.255.1", "192.168.1.217"].iter().map(|s| s.parse().unwrap()).collect();
    assert_eq!(picked, want);
    assert!(!is_private_v4(Ipv4Addr::LOCALHOST));
    assert!(!is_private_v4(Ipv4Addr::UNSPECIFIED));
    for ip in lan_addrs() {
        assert!(is_private_v4(ip) && !ip.is_loopback() && !ip.is_unspecified(), "{ip}");
    }
}

#[test]
fn the_real_listener_lists_only_lan_addresses() {
    let dir = temp_dir("bind");
    prepare(&dir, "", false);
    let host = std::sync::Arc::new(TestHost::default());
    let remote = Remote::new(host.clone(), Box::new(MemSecrets::default()), dir, Options { port: Some(0), ..Options::default() });
    remote.enable(true);
    let bound = remote.bound();
    for addr in &bound {
        match addr.ip() {
            IpAddr::V4(v4) => assert!(is_private_v4(v4) && !v4.is_loopback() && !v4.is_unspecified(), "{addr}"),
            IpAddr::V6(_) => panic!("{addr}"),
        }
    }
    assert_eq!(remote.status()["addresses"].as_array().map(Vec::len), Some(bound.len()));
    assert_eq!(lock(&host.awake).as_slice(), [true]);
    remote.enable(false);
    assert_eq!(lock(&host.awake).as_slice(), [true, false]);
}

#[test]
fn private_keys_never_land_in_a_json_file() {
    let rig = rig("keys", "", false);
    rig.host.answer.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    let qr = decode_qr(&rig.remote.pair_start().unwrap());
    assert_eq!(runtime().block_on(pair(rig.addr(), &qr, &Phone::new("k"))).unwrap()["t"], "pair.ok");
    rig.remote.enable(false);
    let secrets: Vec<String> = lock(&rig.secrets.0).values().cloned().collect();
    assert_eq!(secrets.len(), 3);
    for name in ["config.json", "devices.json"] {
        let text = std::fs::read_to_string(rig.dir.join(name)).unwrap();
        for s in &secrets {
            assert!(!text.contains(s.as_str()), "{name}");
            assert!(!text.contains(s.trim_end_matches('=')), "{name}");
        }
        assert!(!text.to_ascii_lowercase().contains("private"), "{name}");
    }
    let names: Vec<String> = std::fs::read_dir(&rig.dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert!(names.iter().all(|n| n == "config.json" || n == "devices.json"), "{names:?}");
}

#[test]
fn config_defaults_and_relay_timestamps_only_go_up() {
    let dir = temp_dir("cfg");
    let mut cfg = Config::load(&dir);
    assert_eq!((cfg.enabled, cfg.port, cfg.keep_awake), (false, DEFAULT_PORT, true));
    assert_eq!(cfg.machine_id.len(), 22);
    assert_eq!(Config::load(&dir).machine_id, cfg.machine_id);
    assert_eq!(cfg.next_admin_ts(100), 100);
    assert_eq!(cfg.next_admin_ts(100), 101);
    assert_eq!(cfg.next_admin_ts(50), 102);
    assert_eq!(cfg.next_machine_ts(10), 10);
    assert_eq!(cfg.next_machine_ts(10), 11);
}

#[test]
fn devices_authorize_by_key_and_expire_after_thirty_quiet_days() {
    let dir = temp_dir("devices");
    let mut d = Devices::load(&dir);
    let key = [7u8; 32];
    assert!(d.add(Device::new("p1", "Phone", &key, 1_000)));
    assert!(d.authorize("p1", &key, 1_000 + EXPIRE_SECS - 1));
    assert!(!d.authorize("p1", &key, 1_000 + EXPIRE_SECS));
    assert!(!d.authorize("p1", &[8u8; 32], 1_000));
    assert!(!d.authorize("p2", &key, 1_000));
    assert_eq!(d.touch("p1", 1_000 + 86_400 * 2).map(|(_, first)| first), Some(true));
    assert_eq!(d.touch("p1", 1_000 + 86_400 * 2 + 5).map(|(_, first)| first), Some(false));
    assert_eq!(d.expired(1_000 + 86_400 * 2 + 5 + EXPIRE_SECS), vec!["p1".to_owned()]);
    assert_eq!(Devices::load(&dir).list().len(), 1);
    assert_eq!(d.remove(&["p1".to_owned()]), vec!["p1".to_owned()]);
    assert!(Devices::load(&dir).list().is_empty());
}

#[test]
fn the_sweep_revokes_expired_devices_and_queues_them_for_the_relay() {
    let rig = rig("sweep", "https://relay.invalid", true);
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("old", "Old", &[1u8; 32], 10)));
    assert!(lock(&ctx.devices).add(Device::new("new", "New", &[2u8; 32], 10 + EXPIRE_SECS)));
    assert_eq!(ctx.sweep(10 + EXPIRE_SECS), 1);
    assert_eq!(lock(&ctx.devices).ids(), vec!["new".to_owned()]);
    assert_eq!(Config::load(&rig.dir).relay.pending_revoke, vec!["old".to_owned()]);
}

#[test]
fn the_gate_counts_pending_connections_per_ip() {
    let gate = std::sync::Arc::new(Gate::default());
    let ip: IpAddr = "192.168.1.9".parse().unwrap();
    let now = std::time::Instant::now();
    let held: Vec<_> = (0..MAX_PENDING_PER_IP).map(|_| gate.admit(ip, now).unwrap()).collect();
    assert!(gate.admit(ip, now).is_none());
    assert!(gate.admit("192.168.1.10".parse().unwrap(), now).is_some());
    drop(held);
    assert_eq!(gate.pending(ip), 0);
    assert!(gate.admit(ip, now).is_some());
}

#[test]
fn phone_messages_map_the_ws_wire() {
    let snap = r#"{"seq":7,"events":[{"type":"snapshot","payload":{"terminals":[]}}]}"#;
    assert_eq!(snapshot_event(snap).unwrap(), (7, json!({ "t": "snapshot", "seq": 7, "data": { "terminals": [] } })));
    let batch = r#"{"seq":8,"events":[{"type":"terminal.created","payload":{}},{"type":"resources","payload":{}},{"type":"office.snapshot","payload":{}}]}"#;
    let (seq, ev) = events_event(batch).unwrap();
    assert_eq!(seq, 8);
    assert_eq!(ev["events"].as_array().unwrap().len(), 1);
    assert_eq!(command(br#"{"id":"1","t":"hello"}"#).unwrap().1, "hello");
    assert_eq!(command(br#"{"t":"hello"}"#).unwrap_err()["error"], "invalid");
    assert_eq!(command(format!(r#"{{"id":"{}","t":"x"}}"#, "a".repeat(33)).as_bytes()).unwrap_err()["re"], Value::Null);
    assert_eq!(command(b"nope").unwrap_err()["error"], "invalid");
    assert_eq!(client_message("terminal.note", &json!({ "id": "1", "t": "terminal.note", "terminalId": "x", "note": "n" })), json!({ "type": "terminal.note", "terminalId": "x", "note": "n" }));
}

#[test]
fn theme_and_mdns_records_carry_only_public_fields() {
    assert_eq!(theme(Some(&json!({ "dark": true, "accent": "#0a84ff" }))), json!({ "mode": "dark", "palette": { "accent": "#0a84ff" } }));
    assert_eq!(theme(None)["mode"], "light");
    let k = tag(&[3u8; 32]);
    assert_eq!(k.len(), 11);
    assert_eq!(instance("Ab_c-12"), "orrery-abc12");
    assert_eq!(txt(&k, "PC"), [("v", "1"), ("k", k.as_str()), ("n", "PC")]);
}

#[test]
fn a_failing_keyring_never_creates_or_overwrites_keys() {
    let rig = rig("keyfail", "", false);
    lock(&rig.secrets.0).insert(super::keys::NOISE_SECRET.into(), "old".into());
    rig.secrets.1.store(true, Ordering::SeqCst);
    rig.remote.enable(true);
    assert_eq!(rig.remote.status()["error"], "keyring");
    assert_eq!(rig.remote.status()["running"], false);
    assert_eq!(lock(&rig.secrets.0).len(), 1);
    assert_eq!(lock(&rig.secrets.0).get(super::keys::NOISE_SECRET).map(String::as_str), Some("old"));
    rig.secrets.1.store(false, Ordering::SeqCst);
    assert!(super::keys::Keys::load_or_create(&rig.secrets).is_none());
    assert_eq!(lock(&rig.secrets.0).get(super::keys::NOISE_SECRET).map(String::as_str), Some("old"));
}

#[test]
fn a_revoke_racing_a_new_session_never_leaves_it_alive() {
    let rig = rig("toctou", "", false);
    let ctx = rig.remote.ctx().unwrap();
    let key = [5u8; 32];
    let now = super::config::now_secs();
    assert!(lock(&ctx.devices).add(Device::new("p", "P", &key, now)));
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let id = ctx.admit_live("p", &key, "lan", tx, now).unwrap();
    assert_eq!(ctx.revoke(&["p".to_owned()]), 1);
    assert_eq!(rx.try_recv().ok(), Some(super::Control::Revoked));
    ctx.drop_live(id);
    let (tx2, _rx2) = tokio::sync::mpsc::channel(4);
    assert_eq!(ctx.admit_live("p", &key, "lan", tx2, now), Err(super::ctx::Admit::Revoked));
    assert_eq!(ctx.live_count(), 0);
}

#[test]
fn the_pre_auth_budget_allows_a_few_small_frames_then_opens_up() {
    use super::link::{Budget, PRE_AUTH_BYTES, PRE_AUTH_FRAMES};
    let b = Budget::default();
    for _ in 0..PRE_AUTH_FRAMES {
        assert!(b.take(100));
    }
    assert!(!b.take(1));
    let big = Budget::default();
    assert!(!big.take(PRE_AUTH_BYTES + 1));
    let open = Budget::default();
    open.open();
    for _ in 0..100 {
        assert!(open.take(65_535));
    }
}

#[test]
fn relay_conn_ids_are_tracked_by_generation() {
    use super::relay::Conns;
    let mut conns = Conns::default();
    let (g1, mut rx1, _) = conns.open(7).unwrap();
    let (g2, mut rx2, b2) = conns.open(7).unwrap();
    assert_ne!(g1, g2);
    assert!(rx1.try_recv().is_err());
    assert!(matches!(rx1.try_recv(), Err(tokio::sync::mpsc::error::TryRecvError::Disconnected)));
    assert!(!conns.current(7, g1));
    assert!(conns.current(7, g2));
    conns.close(7, Some(g1));
    assert!(conns.current(7, g2));
    b2.open();
    conns.data(7, b"x");
    assert_eq!(rx2.try_recv().unwrap(), b"x".to_vec());
    conns.close(7, Some(g2));
    assert!(!conns.current(7, g2));
}

#[test]
fn lan_rebind_finds_gone_and_new_addresses() {
    let a: Ipv4Addr = "192.168.1.5".parse().unwrap();
    let b: Ipv4Addr = "10.0.0.2".parse().unwrap();
    assert_eq!(super::rebind::changed_set(&[a], &[a]), (vec![], vec![]));
    assert_eq!(super::rebind::changed_set(&[a], &[b]), (vec![a], vec![b]));
}

#[test]
fn a_platform_outside_android_and_ios_is_stored_and_shown_as_unknown() {
    let dir = temp_dir("platform");
    let mut devices = Devices::load(&dir);
    let phone = Phone::new("p");
    assert!(devices.add(Device::new("p", "Phone", phone.key.public(), 1)));
    assert_eq!(super::status::device_json(&devices.list()[0], None)["platform"], "unknown");
    devices.set_platform("p", "cli");
    assert_eq!(devices.list()[0].platform, "unknown");
    devices.set_platform("p", "ios");
    assert_eq!(super::status::device_json(&devices.list()[0], None)["platform"], "ios");
    devices.set_platform("p", "android");
    assert_eq!(devices.list()[0].platform, "android");
}

#[test]
fn renaming_a_device_keeps_the_paired_name_and_an_empty_alias_restores_it() {
    let dir = temp_dir("rename");
    let mut d = Devices::load(&dir);
    assert!(d.add(Device::new("p1", "Pixel 7", &[1u8; 32], 1_000)));
    assert_eq!(d.rename("p1", "  Work phone \u{202E}\n"), Some("Work phone".to_owned()));
    assert_eq!(d.find("p1").unwrap().name, "Pixel 7");
    assert_eq!(Devices::load(&dir).find("p1").unwrap().alias, "Work phone");
    assert_eq!(d.touch("p1", 5_000).map(|(name, _)| name), Some("Work phone".to_owned()));
    let long = "\u{1F4F1}".repeat(60);
    assert_eq!(d.rename("p1", &long).unwrap().chars().count(), 40);
    assert_eq!(d.rename("p1", " \t \u{200B}"), Some("Pixel 7".to_owned()));
    assert!(Devices::load(&dir).find("p1").unwrap().alias.is_empty());
    assert_eq!(d.rename("nope", "x"), None);
    assert_eq!(d.rename("p1", "Keep"), Some("Keep".to_owned()));
    assert_eq!(d.remove(&["p1".to_owned()]).len(), 1);
    assert!(d.add(Device::new("p1", "Pixel 7", &[1u8; 32], 9_000)));
    assert!(d.find("p1").unwrap().alias.is_empty());
}

#[test]
fn the_rename_channel_updates_status_and_rejects_bad_input() {
    let rig = rig("rename-ipc", "https://relay.invalid", true);
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new("p1", "Pixel 7", &[1u8; 32], 10)));
    let call = |v: Value| rig.remote.call("ccm:remote-device-rename", &[v]).unwrap();
    assert_eq!(call(json!({"deviceId": "p1", "name": "Desk"})), json!(true));
    let status = rig.remote.call("ccm:remote-status", &[]).unwrap();
    assert_eq!(status["devices"][0]["name"], "Desk");
    assert_eq!(status["devices"][0]["originalName"], "Pixel 7");
    assert_eq!(status["devices"][0]["alias"], "Desk");
    assert_eq!(call(json!({"deviceId": "ghost", "name": "x"})), json!(false));
    assert_eq!(call(json!({"deviceId": "p1"})), json!(false));
    assert_eq!(call(json!("p1")), json!(false));
    assert_eq!(call(json!({"deviceId": "p1", "name": ""})), json!(true));
    assert_eq!(rig.remote.call("ccm:remote-status", &[]).unwrap()["devices"][0]["name"], "Pixel 7");
}
