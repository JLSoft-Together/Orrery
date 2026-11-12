use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_http::{ClientMessage, Config as HttpConfig, Hooks, RunningServer, StoreHandle};
use core_remote_proto::session::SessionInitiator;
use core_store::{Store, StoreOptions, TerminalSession};
use futures_util::SinkExt;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use super::devices::Device;
use super::link::{CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_PROTO};
use super::limits::MAX_PENDING_PER_IP;
use super::test_util::*;
use crate::state::lock;

type Seen = Arc<Mutex<Vec<ClientMessage>>>;

async fn core_server(dir: &std::path::Path) -> (RunningServer, StoreHandle, Seen) {
    let dist = dir.join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::write(dist.join("index.html"), "ok").unwrap();
    let store = StoreHandle::new(Store::new(StoreOptions { activity_limit: 10, batch_ms: 10 }));
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let hooks = Hooks { on_client_message: Some(Arc::new(move |m, _| sink.lock().unwrap().push(m))), ..Hooks::default() };
    let server = core_http::start(HttpConfig::new(0, dist), store.clone(), hooks).await.unwrap();
    (server, store, seen)
}

fn terminal(id: &str) -> TerminalSession {
    serde_json::from_value(json!({ "id": id, "title": id, "status": "working" })).unwrap()
}

fn paired(rig: &Rig, phone: &Phone) {
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
}

#[test]
fn an_unknown_device_is_closed_with_4401_and_noticed_once() {
    let rig = rig("ik-unknown", "", false);
    rig.remote.enable(true);
    let desktop = rig.desktop_key();
    let rt = runtime();
    for _ in 0..2 {
        assert_eq!(rt.block_on(session(rig.addr(), &desktop, &Phone::new("stranger"))).err(), Some(Some(CLOSE_DEVICE)));
    }
    assert_eq!(lock(&rig.host.notes).len(), 1);
    rig.remote.enable(false);
}

#[test]
fn an_old_protocol_is_closed_with_4426() {
    let rig = rig("ik-proto", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    let code = runtime().block_on(async {
        let mut ws = connect(rig.addr()).await;
        let (_, mut msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
        msg1[0] = 0;
        msg1[1] = 0;
        ws.send(Message::Binary(msg1.into())).await.unwrap();
        close_code(&mut ws).await
    });
    assert_eq!(code, Some(CLOSE_PROTO));
    rig.remote.enable(false);
}

#[test]
fn hello_gets_the_welcome_then_the_ws_snapshot_then_live_events() {
    let rig = rig("ik-hello", "", false);
    let rt = runtime();
    let (server, store, seen) = rt.block_on(core_server(&rig.dir));
    let tap = server.tap();
    *lock(&rig.host.tap) = Some(tap.clone());
    rig.remote.enable(true);
    assert!(rig.remote.publish(&json!({ "rev": 3, "tabs": [], "closedCount": 0 })));
    assert!(!rig.remote.publish(&json!([1])));
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    rt.block_on(async {
        let (mut early, mut et) = session(rig.addr(), &desktop, &phone).await.unwrap();
        send_json(&mut early, &mut et, json!({ "id": "x", "t": "resync" })).await;
        assert_eq!(close_code(&mut early).await, Some(CLOSE_HANDSHAKE));
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        let welcome = hello(&mut ws, &mut t).await;
        assert_eq!(welcome["ok"], true);
        assert_eq!(welcome["name"], "Test PC");
        assert_eq!(welcome["version"], "9.9.9");
        assert_eq!(welcome["minProto"], 1);
        assert_eq!(welcome["theme"]["mode"], "dark");
        assert_eq!(welcome["locale"], "en");
        assert!(welcome.get("relayToken").is_none());
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["t"], "push.config");
        let snap = recv_json(&mut ws, &mut t).await.unwrap();
        let ws_snapshot: Value = serde_json::from_str(&tap.snapshot()).unwrap();
        assert_eq!(snap["t"], "snapshot");
        assert_eq!(snap["seq"], ws_snapshot["seq"]);
        assert_eq!(snap["data"], ws_snapshot["events"][0]["payload"]);
        let wsnap = recv_json(&mut ws, &mut t).await.unwrap();
        assert_eq!((wsnap["t"].as_str(), wsnap["rev"].as_u64()), (Some("ws.snapshot"), Some(3)));
        store.with(|s| s.upsert_terminal(terminal("t1")));
        let ev = recv_json(&mut ws, &mut t).await.unwrap();
        assert_eq!(ev["t"], "events");
        assert!(ev["seq"].as_u64().unwrap() > snap["seq"].as_u64().unwrap());
        assert_eq!(ev["events"][0]["type"], "terminal.created");
        send_json(&mut ws, &mut t, json!({ "id": "a1", "t": "terminal.alias", "terminalId": "t1", "alias": "api" })).await;
        let ok = recv_json(&mut ws, &mut t).await.unwrap();
        assert_eq!((ok["re"].as_str(), ok["ok"].as_bool()), (Some("a1"), Some(true)));
        for kind in ["client.presence", "terminal.open", "terminal.focusWindow", "folder.pick", "favorite.open", "notify.update", "office.enable"] {
            send_json(&mut ws, &mut t, json!({ "id": "d", "t": kind, "attentive": true, "terminalId": "t1" })).await;
            let reply = recv_json(&mut ws, &mut t).await.unwrap();
            assert_eq!(reply["error"], "denied", "{kind}");
        }
        send_json(&mut ws, &mut t, json!({ "id": "r", "t": "resync" })).await;
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["ok"], true);
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["t"], "snapshot");
    });
    let got = lock(&seen).clone();
    assert_eq!(got.len(), 1);
    assert!(format!("{:?}", got[0]).contains("api"));
    rig.remote.enable(false);
    rt.block_on(server.shutdown());
}

#[test]
fn more_than_eight_pending_connections_from_one_ip_are_refused() {
    let rig = rig("dos", "", false);
    rig.remote.enable(true);
    let addr = rig.addr();
    let ctx = rig.remote.ctx().unwrap();
    runtime().block_on(async {
        let mut held = Vec::new();
        for _ in 0..MAX_PENDING_PER_IP {
            held.push(tokio::net::TcpStream::connect(addr).await.unwrap());
        }
        for _ in 0..50 {
            if ctx.gate.pending(addr.ip()) == MAX_PENDING_PER_IP {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ctx.gate.pending(addr.ip()), MAX_PENDING_PER_IP);
        assert!(tokio_tungstenite::connect_async(format!("ws://{addr}/remote")).await.is_err());
        drop(held);
        for _ in 0..100 {
            if ctx.gate.pending(addr.ip()) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(tokio_tungstenite::connect_async(format!("ws://{addr}/remote")).await.is_ok());
        assert!(tokio_tungstenite::connect_async(format!("ws://{addr}/other")).await.is_err());
    });
    rig.remote.enable(false);
}

#[test]
fn revoking_a_live_device_sends_device_revoked_and_closes_with_4401() {
    let rig = rig("revoke", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    let rt = runtime();
    rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["t"], "push.config");
        assert_eq!(rig.remote.revoke(&["p".to_owned()]).unwrap(), serde_json::json!(true));
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["t"], "device.revoked");
        assert_eq!(close_code(&mut ws).await, Some(CLOSE_DEVICE));
        assert_eq!(session(rig.addr(), &desktop, &phone).await.err(), Some(Some(CLOSE_DEVICE)));
    });
    rig.remote.enable(false);
}

#[test]
fn turning_the_gateway_off_tells_live_phones_remote_off() {
    let rig = rig("off", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    let rt = runtime();
    let (mut ws, mut t) = rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        assert_eq!(recv_json(&mut ws, &mut t).await.unwrap()["t"], "push.config");
        (ws, t)
    });
    rig.remote.enable(false);
    let got = rt.block_on(recv_json(&mut ws, &mut t)).unwrap();
    assert_eq!(got["t"], "remote.off");
    assert_eq!(rig.remote.status()["enabled"], false);
    assert!(rig.remote.bound().is_empty());
}

#[test]
fn a_replayed_ik_msg1_never_touches_last_seen_or_takes_a_session_slot() {
    let rig = rig("replay", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    let seen = super::config::now_secs() - 3 * 86_400;
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), seen)));
    let desktop = rig.desktop_key();
    let addr = rig.addr();
    runtime().block_on(async {
        let (_, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
        let mut replays = Vec::new();
        for _ in 0..MAX_PENDING_PER_IP - 1 {
            let mut ws = connect(addr).await;
            ws.send(Message::Binary(msg1.clone().into())).await.unwrap();
            assert!(next_binary(&mut ws).await.is_ok());
            replays.push(ws);
        }
        assert_eq!(ctx.live_count(), 0);
        assert_eq!(lock(&ctx.devices).find("p").unwrap().last_seen, seen);
        assert!(lock(&rig.host.notes).is_empty());
        let (mut ws, mut t) = session(addr, &desktop, &phone).await.unwrap();
        assert_eq!(hello(&mut ws, &mut t).await["ok"], true);
        assert_eq!(ctx.live_count(), 1);
        assert!(lock(&ctx.devices).find("p").unwrap().last_seen > seen);
        assert_eq!(close_code(&mut replays[0]).await, Some(CLOSE_HANDSHAKE));
    });
    rig.remote.enable(false);
}
