use std::time::Duration;

use core_remote_proto::pair::{PairInitiator, PairTicket};
use core_remote_proto::relay::{decode, encode_control, encode_data, Control, ControlKind, Envelope};
use core_remote_proto::session::SessionInitiator;
use core_remote_proto::Transport;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::config::now_secs;
use super::ctx::{Ctx, PairAsk};
use super::devices::{Device, Devices};
use super::test_util::*;
use crate::state::lock;

type Relay = WebSocketStream<TcpStream>;

enum Got {
    Data(u16, Vec<u8>),
    Ctl(Value),
}

async fn next(ws: &mut Relay) -> Got {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(10), ws.next()).await.unwrap().unwrap().unwrap();
        let Message::Binary(b) = msg else { continue };
        if b[0] == 0 && b[1] == 0 {
            return Got::Ctl(serde_json::from_slice(&b[2..]).unwrap());
        }
        match decode(&b).unwrap() {
            Envelope::Data { conn, bytes } => return Got::Data(conn, bytes.to_vec()),
            Envelope::Control(_) => unreachable!(),
        }
    }
}

async fn data(ws: &mut Relay, ctls: &mut Vec<Value>) -> (u16, Vec<u8>) {
    loop {
        match next(ws).await {
            Got::Data(conn, bytes) => return (conn, bytes),
            Got::Ctl(v) => ctls.push(v),
        }
    }
}

async fn ctl(ws: &mut Relay, want: impl Fn(&Value) -> bool) -> Value {
    loop {
        if let Got::Ctl(v) = next(ws).await {
            if want(&v) {
                return v;
            }
        }
    }
}

async fn wait_online(ctx: &Ctx) {
    for _ in 0..250 {
        if ctx.relay_state() == "online" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("relay never online");
}

fn open(conn: u16, pairing: bool) -> Vec<u8> {
    let c = Control { ip: Some("203.0.113.7".into()), country: Some("VN".into()), pairing, ..Control::new(ControlKind::Open, conn) };
    encode_control(&c).unwrap()
}

async fn send(ws: &mut Relay, bytes: Vec<u8>) {
    ws.send(Message::Binary(bytes.into())).await.unwrap();
}

async fn accept(listener: &TcpListener) -> Relay {
    let (stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept()).await.unwrap().unwrap();
    tokio_tungstenite::accept_async(stream).await.unwrap()
}

#[test]
fn a_phone_off_the_lan_pairs_through_the_relay_with_the_one_time_ticket() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("pair-relay", &format!("http://127.0.0.1:{port}"), true);
    rig.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    let ctx = rig.remote.ctx().unwrap();
    rig.remote.enable(true);
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        let started = rig.remote.pair_start().unwrap();
        assert_eq!(started["relay"], true);
        let qr = decode_qr(&started);
        let ticket: PairTicket = qr.ticket.clone().expect("ticket in qr");
        assert_eq!(qr.relay.as_deref(), Some(format!("http://127.0.0.1:{port}").as_str()));
        let armed = ctl(&mut ws, |v| v["t"] == "pair.open").await;
        assert_eq!(armed["h"], ticket.hash_text());
        assert!((118..=120).contains(&armed["ttl"].as_u64().unwrap()), "{armed}");
        assert!(armed.get("exp").is_none());
        assert!(!armed.to_string().contains(ticket.as_str()));
        assert_eq!(rig.remote.status()["pairing"]["relay"], true);

        send(&mut ws, open(5, true)).await;
        let phone = Phone::new("phone-r");
        let (init, msg1) = PairInitiator::start_on(&phone.key, &qr, &phone.id, "Pixel Test", "android", now_secs()).unwrap();
        send(&mut ws, encode_data(5, &msg1).unwrap()).await;
        let mut ctls = Vec::new();
        let (conn, msg2) = data(&mut ws, &mut ctls).await;
        assert_eq!(conn, 5);
        let (mut desk, msg3) = init.finish(&msg2).unwrap();
        send(&mut ws, encode_data(5, &msg3).unwrap()).await;
        let (_, sealed) = data(&mut ws, &mut ctls).await;
        let reply = open_json(&mut desk.transport, &sealed).unwrap();
        assert_eq!(reply["t"], "pair.ok");
        assert_eq!(reply["relay"], format!("http://127.0.0.1:{port}"));
        assert!(reply["relayToken"].is_string());
        let closed = ctl(&mut ws, |v| v["t"] == "close").await;
        assert_eq!((closed["conn"].as_u64(), closed["code"].as_u64()), (Some(5), Some(1000)));
        assert!(ctls.iter().any(|v| v["t"] == "pair.close"), "{ctls:?}");
    });
    let ask = PairAsk { name: "Pixel Test".into(), platform: "android".into(), ip: Some("203.0.113.7".into()), country: Some("VN".into()), via: "relay" };
    assert_eq!(lock(&rig.host.pair_asks).as_slice(), [ask]);
    assert_eq!(Devices::load(&rig.dir).list()[0].device_id, "phone-r");
    rig.remote.enable(false);
}

#[test]
fn a_pairing_conn_refuses_ik_with_4400_even_for_a_paired_phone() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("pair-relay-ik", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new("phone-k");
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, &phone.id, phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    rig.remote.enable(true);
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        let (_init, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
        send(&mut ws, open(7, true)).await;
        send(&mut ws, encode_data(7, &msg1).unwrap()).await;
        let closed = ctl(&mut ws, |v| v["t"] == "close").await;
        assert_eq!((closed["conn"].as_u64(), closed["code"].as_u64()), (Some(7), Some(4400)));
        send(&mut ws, open(8, false)).await;
        let (init, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
        send(&mut ws, encode_data(8, &msg1).unwrap()).await;
        let (conn, msg2) = data(&mut ws, &mut Vec::new()).await;
        assert_eq!(conn, 8);
        let _t: Transport = init.finish(&msg2).unwrap();
    });
    rig.remote.enable(false);
}

#[test]
fn cancel_disarms_the_relay_and_an_offline_relay_gives_a_qr_without_ticket() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("pair-relay-cancel", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    let early = decode_qr(&{
        rig.remote.enable(true);
        let v = rig.remote.pair_start().unwrap();
        assert_eq!(v["relay"], false);
        v
    });
    assert!(early.ticket.is_none());
    assert!(early.relay.is_some());
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        let qr = decode_qr(&rig.remote.pair_start().unwrap());
        assert!(qr.ticket.is_some());
        ctl(&mut ws, |v| v["t"] == "pair.open").await;
        rig.remote.pair_cancel();
        ctl(&mut ws, |v| v["t"] == "pair.close").await;
    });
    rig.remote.enable(false);
}

async fn quiet(ws: &mut Relay, ms: u64) -> Vec<Value> {
    let mut seen = Vec::new();
    while let Ok(Some(Ok(msg))) = tokio::time::timeout(Duration::from_millis(ms), ws.next()).await {
        if let Message::Binary(b) = msg {
            if b.len() > 2 && b[0] == 0 && b[1] == 0 {
                seen.push(serde_json::from_slice::<Value>(&b[2..]).unwrap());
            }
        }
    }
    seen
}

#[test]
fn a_ticket_the_relay_spent_is_not_armed_again_when_the_desktop_reconnects() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("pair-relay-spent", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    rig.remote.enable(true);
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        decode_qr(&rig.remote.pair_start().unwrap());
        ctl(&mut ws, |v| v["t"] == "pair.open").await;
        let _ = ws.close(None).await;
        let mut again = accept(&listener).await;
        let first = ctl(&mut again, |v| v["t"] == "pair.open" || v["t"] == "pair.close").await;
        assert_eq!(first["t"], "pair.open", "an unused ticket is armed again after a reconnect");

        send(&mut again, open(4, true)).await;
        ctl(&mut again, |v| v["t"] == "pair.close").await;
        assert_eq!(rig.remote.status()["pairing"]["relay"], false);
        assert!(!rig.remote.status()["pairing"].is_null());
        let _ = again.close(None).await;
        let mut third = accept(&listener).await;
        wait_online(&ctx).await;
        let seen = quiet(&mut third, 1500).await;
        assert!(seen.iter().all(|v| v["t"] != "pair.open"), "{seen:?}");
    });
    rig.remote.enable(false);
}

#[test]
fn the_relay_gets_trust_and_manual_blocks_on_connect_and_its_blocks_reach_the_desktop_list_and_alerts() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("guard-relay", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    assert_eq!(rig.remote.guard_call(&serde_json::json!({ "op": "trust", "list": ["198.51.100.0/24"] }))["ok"], true);
    assert_eq!(rig.remote.guard_call(&serde_json::json!({ "op": "block", "cidr": "192.0.2.0/24", "ttl": 3600 }))["ok"], true);
    rig.remote.enable(true);
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        let trust = ctl(&mut ws, |v| v["t"] == "trust.set").await;
        assert_eq!(trust["list"], serde_json::json!(["198.51.100.0/24"]));
        let set = ctl(&mut ws, |v| v["t"] == "block.set").await;
        assert_eq!(set["list"][0]["c"], "192.0.2.0/24");
        assert!((3590..=3600).contains(&set["list"][0]["ttl"].as_u64().unwrap()), "{set}");
        ctl(&mut ws, |v| v["t"] == "block.list").await;
        wait_online(&ctx).await;
        let rev = rig.remote.status()["guardRev"].as_u64().unwrap();
        let row = r#"{"k":"203.0.113.66","until":4102444800,"level":2,"n":20,"at":4102358400,"why":"ticket"}"#;
        send(&mut ws, [&[0u8, 0][..], format!(r#"{{"t":"block.list","total":1,"items":[{row}]}}"#).as_bytes()].concat()).await;
        send(&mut ws, [&[0u8, 0][..], format!(r#"{{"t":"blocked","row":{}}}"#, row.replace("203.0.113.66", "203.0.113.67")).as_bytes()].concat()).await;
        for _ in 0..100 {
            if rig.remote.guard_call(&serde_json::json!({}))["state"]["blocked"].as_array().unwrap().len() == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(rig.remote.status()["guardRev"].as_u64().unwrap() > rev);
        let state = rig.remote.guard_call(&serde_json::json!({}))["state"].clone();
        let keys: Vec<(String, String)> = state["blocked"].as_array().unwrap().iter().map(|b| (b["key"].as_str().unwrap().to_owned(), b["source"].as_str().unwrap().to_owned())).collect();
        assert_eq!(keys, [("192.0.2.0/24".to_owned(), "manual".to_owned()), ("203.0.113.67".into(), "relay".into()), ("203.0.113.66".into(), "relay".into())]);
        assert_eq!(rig.remote.guard_call(&serde_json::json!({ "op": "unblock", "key": "203.0.113.66" }))["ok"], true);
        let removed = ctl(&mut ws, |v| v["t"] == "block.remove").await;
        assert_eq!(removed["c"], "203.0.113.66");
    });
    let notes = lock(&rig.host.notes).clone();
    assert_eq!(notes.iter().filter(|n| n.contains("203.0.113.67") && n.contains("relay")).count(), 1, "{notes:?}");
    rig.remote.enable(false);
}

#[test]
fn a_paired_phone_on_the_relay_from_a_new_country_alerts_once_and_is_remembered() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("guard-place", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new("phone-w");
    {
        let mut devices = lock(&ctx.devices);
        assert!(devices.add(Device::new(&phone.id, "Pixel W", phone.key.public(), now_secs())));
        devices.note_place(&phone.id, Some("US"));
    }
    let desktop = rig.desktop_key();
    rig.remote.enable(true);
    let mut seen = Vec::new();
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        for conn in [11u16, 12] {
            send(&mut ws, open(conn, false)).await;
            let (init, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
            send(&mut ws, encode_data(conn, &msg1).unwrap()).await;
            let msg2 = loop {
                let (c, b) = data(&mut ws, &mut seen).await;
                if c == conn {
                    break b;
                }
            };
            assert!(!seen.iter().any(|v| v["t"] == "good" && v["conn"] == conn), "good before hello: {seen:?}");
            let mut t: Transport = init.finish(&msg2).unwrap();
            let hello = seal_json(&mut t, &serde_json::json!({ "id": "h1", "t": "hello", "proto": 1, "app": "test", "platform": "android", "caps": [] }));
            send(&mut ws, encode_data(conn, &hello).unwrap()).await;
            loop {
                let (c, b) = data(&mut ws, &mut seen).await;
                if c == conn && open_json(&mut t, &b).is_some_and(|v| v["re"] == "h1") {
                    break;
                }
            }
            send(&mut ws, encode_control(&Control::new(ControlKind::Close, conn)).unwrap()).await;
        }
    });
    let notes = lock(&rig.host.notes).clone();
    assert_eq!(notes.iter().filter(|n| n.contains("new country") && n.contains("203.0.113.7 [VN]")).count(), 1, "{notes:?}");
    assert!([11u16, 12].iter().all(|c| seen.iter().any(|v| v["t"] == "good" && v["conn"] == *c)), "{seen:?}");
    let saved = Devices::load(&rig.dir);
    assert_eq!(saved.list()[0].countries, ["US", "VN"]);
    rig.remote.enable(false);
}

#[test]
fn a_strike_still_reaches_the_relay_when_the_phone_left_and_manual_blocks_apply_on_an_old_relay() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("guard-gone", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    rig.remote.enable(true);
    rt.block_on(async {
        let mut ws = accept(&listener).await;
        wait_online(&ctx).await;
        let mut junk = vec![0u8, 1];
        junk.extend_from_slice(&[7u8; 78]);
        send(&mut ws, open(21, false)).await;
        send(&mut ws, encode_data(21, &junk).unwrap()).await;
        send(&mut ws, encode_control(&Control::new(ControlKind::Close, 21)).unwrap()).await;
        let strike = ctl(&mut ws, |v| v["t"] == "strike").await;
        assert_eq!(strike, serde_json::json!({ "t": "strike", "conn": 21, "why": "handshake" }));

        assert_eq!(rig.remote.guard_call(&serde_json::json!({ "op": "block", "cidr": "203.0.113.0/24", "ttl": null }))["ok"], true);
        send(&mut ws, open(31, false)).await;
        let closed = ctl(&mut ws, |v| v["t"] == "close" && v["conn"] == 31).await;
        assert_eq!(closed["code"], 4429);
    });
    let log = rig.remote.guard_call(&serde_json::json!({}))["state"]["log"].clone();
    assert_eq!((log[0]["result"].as_str(), log[0]["why"].as_str(), log[0]["ip"].as_str()), (Some("blocked"), Some("manual"), Some("203.0.113.7")));
    rig.remote.enable(false);
}
