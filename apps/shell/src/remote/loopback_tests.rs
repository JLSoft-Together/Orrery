use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use core_remote_proto::pair::PairInitiator;
use futures_util::SinkExt;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

use super::super::config::{new_machine_id, now_secs, Config};
use super::super::link::CLOSE_DEVICE;
use super::super::pair::{PairMeta, PAIR_TTL};
use super::super::test_util::{close_code, connect, decode_qr, pair, runtime, session, temp_dir, MemSecrets, Phone, Rig, TestHost};
use super::super::{Options, Remote};
use super::target;
use crate::state::lock;

fn free_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap().local_addr().unwrap().port()
}

fn usb_rig(tag: &str) -> (Rig, u16) {
    let dir = temp_dir(tag);
    let port = free_port();
    let cfg = Config { machine_id: new_machine_id().unwrap(), port, ..Config::default() };
    assert!(cfg.save(&dir));
    let host = Arc::new(TestHost::default());
    let secrets = MemSecrets::default();
    let opts = Options { addrs: Some(Vec::new()), port: None, mdns: false, name: Some("Test PC".into()) };
    let remote = Remote::new(host.clone(), Box::new(secrets.clone()), dir.clone(), opts);
    (Rig { host, secrets, dir, remote }, port)
}

fn at(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

fn up(port: u16) -> bool {
    TcpStream::connect_timeout(&at(port), Duration::from_millis(400)).is_ok()
}

fn becomes(port: u16, want: bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if up(port) == want {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn usb() -> Value {
    json!({ "via": "usb", "extraLan": ["127.0.0.1"] })
}

fn log_whys(r: &Rig) -> Vec<Value> {
    r.remote.guard_call(&json!({}))["state"]["log"].as_array().cloned().unwrap_or_default().into_iter().map(|e| e["why"].clone()).collect()
}

#[test]
fn the_loopback_listener_lives_only_while_a_usb_code_is_valid() {
    let (r, port) = usb_rig("loop-life");
    r.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    r.remote.enable(true);
    assert!(r.remote.bound().is_empty());
    assert!(!up(port));

    r.remote.call("ccm:remote-pair-start", &[json!(null)]).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    assert!(!up(port), "a QR code must not open the loopback listener");

    let started = r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    assert!(becomes(port, true));
    let qr = decode_qr(&started);
    assert_eq!((qr.lan[0].host.as_str(), qr.lan[0].port), ("127.0.0.1", port));
    let reply = runtime().block_on(pair(at(port), &qr, &Phone::new("p-usb"))).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    assert_eq!(lock(&r.host.pair_asks)[0].source, "usb");
    assert!(becomes(port, false), "pairing done closes the loopback listener");

    r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    assert!(becomes(port, true));
    r.remote.call("ccm:remote-pair-cancel", &[]).unwrap();
    assert!(becomes(port, false), "cancel closes it");

    r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    assert!(becomes(port, true));
    r.remote.call("ccm:remote-pair-start", &[json!(null)]).unwrap();
    assert!(becomes(port, false), "a QR code replacing the usb code closes it");

    r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    assert!(becomes(port, true));
    r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(up(port), "a second usb start keeps it open");
    assert!(r.remote.bound().is_empty(), "the LAN list never holds loopback");
    r.remote.enable(false);
    assert!(becomes(port, false), "turning Remote off closes it");
}

#[test]
fn an_expired_usb_code_closes_the_listener_within_two_seconds() {
    let (r, port) = usb_rig("loop-exp");
    r.remote.enable(true);
    let ctx = r.remote.ctx().unwrap();
    let meta = PairMeta::from_web(&usb());
    let (_, exp) = ctx.pairing.start_with(now_secs() + 3 - PAIR_TTL, None, meta).unwrap();
    assert!(becomes(port, true));
    assert!(becomes(port, false));
    assert!(now_secs() <= exp + 2, "closed {}s after the code expired", now_secs() - exp);
    assert_eq!(ctx.pairing.usb_until(now_secs()), None);
    r.remote.enable(false);
}

#[test]
fn a_loopback_link_only_pairs_with_the_usb_code() {
    let (r, port) = usb_rig("loop-only");
    r.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    r.remote.enable(true);
    let started = r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    assert!(becomes(port, true));
    let qr = decode_qr(&started);
    let ctx = r.remote.ctx().unwrap();
    let rt = runtime();

    let refused = rt.block_on(session(at(port), &r.desktop_key(), &Phone::new("p-known")));
    assert!(refused.is_err(), "a session handshake over loopback is refused");

    let out = rt.block_on(async move {
        let mut s = tokio::net::TcpStream::connect(at(port)).await.unwrap();
        s.write_all(b"GET /install/x/orrery.apk HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").await.unwrap();
        let mut out = Vec::new();
        let _ = tokio::time::timeout(Duration::from_secs(5), s.read_to_end(&mut out)).await;
        out
    });
    assert!(String::from_utf8_lossy(&out).starts_with("HTTP/1.1 403"), "{}", String::from_utf8_lossy(&out));

    let code = rt.block_on(async {
        let mut ws = connect(at(port)).await;
        ctx.pairing.start_with(now_secs(), None, PairMeta::default()).unwrap();
        let (_, msg1) = PairInitiator::start_on(&Phone::new("p-late").key, &qr, "p-late", "Pixel", "android", now_secs()).unwrap();
        ws.send(Message::Binary(msg1.into())).await.unwrap();
        close_code(&mut ws).await
    });
    assert_eq!(code, Some(CLOSE_DEVICE));
    assert!(log_whys(&r).contains(&json!("loopback")), "{:?}", log_whys(&r));
    assert!(lock(&r.host.pair_asks).is_empty());
    assert!(becomes(port, false));
    r.remote.enable(false);
}

#[test]
fn a_busy_port_is_retried_and_a_port_change_moves_the_listener() {
    let (r, port) = usb_rig("loop-busy");
    let blocker = TcpListener::bind(at(port)).unwrap();
    r.remote.enable(true);
    r.remote.call("ccm:remote-pair-start", &[usb()]).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    drop(blocker);
    assert!(becomes(port, true), "the bind is retried while the usb code is valid");

    let ctx = r.remote.ctx().unwrap();
    let moved = free_port();
    lock(&ctx.config).port = moved;
    assert!(becomes(moved, true));
    assert!(becomes(port, false));
    r.remote.enable(false);
    assert!(becomes(moved, false));
}

#[test]
fn the_target_skips_ports_the_lan_listener_already_covers() {
    let (r, port) = usb_rig("loop-target");
    r.remote.enable(true);
    let ctx = r.remote.ctx().unwrap();
    assert_eq!(target(&ctx, false), None);
    assert_eq!(target(&ctx, true), Some(at(port)));
    ctx.set_lan(vec![at(port)]);
    assert_eq!(target(&ctx, true), None);
    ctx.set_lan(Vec::new());
    lock(&ctx.config).port = 0;
    assert_eq!(target(&ctx, true), None);
    r.remote.enable(false);
}
