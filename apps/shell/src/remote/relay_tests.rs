use std::io::{Read, Write};
use std::sync::atomic::Ordering;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use core_remote_proto::relay::{decode, encode_control, encode_data, verify_token, Control, ControlKind, Envelope, RelayKey};
use core_remote_proto::session::SessionInitiator;
use core_remote_proto::Transport;
use futures_util::{SinkExt, StreamExt};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use super::config::now_secs;
use super::devices::Device;
use super::relay::{register, ws_url, MACHINE_DOMAIN, REGISTER_DOMAIN};
use super::test_util::*;
use crate::state::lock;

fn verify(pubkey: &str, domain: &str, machine: &str, ts: u64, sig: &str) -> bool {
    let key = URL_SAFE_NO_PAD.decode(pubkey).unwrap();
    let sig = URL_SAFE_NO_PAD.decode(sig).unwrap();
    UnparsedPublicKey::new(&ED25519, key).verify(format!("{domain}{machine}{ts}").as_bytes(), &sig).is_ok()
}

fn query(q: &str, key: &str) -> String {
    q.split('&').find_map(|kv| kv.strip_prefix(&format!("{key}="))).unwrap_or_default().to_owned()
}

async fn recv_env(ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) -> (u16, Vec<u8>) {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(10), ws.next()).await.unwrap().unwrap().unwrap();
        if let Message::Binary(b) = msg {
            match decode(&b).unwrap() {
                Envelope::Data { conn, bytes } => return (conn, bytes.to_vec()),
                Envelope::Control(_) => continue,
            }
        }
    }
}

fn open_reply(t: &mut Transport, bytes: &[u8]) -> Value {
    open_json(t, bytes).expect("a json frame for this connection")
}

#[test]
#[allow(clippy::result_large_err)]
fn the_relay_socket_is_signed_and_two_conn_ids_get_separate_sessions() {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("relay", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    let a = Phone::new("phone-a");
    let b = Phone::new("phone-b");
    for p in [&a, &b] {
        assert!(lock(&ctx.devices).add(Device::new(&p.id, &p.id, p.key.public(), now_secs())));
    }
    let desktop = rig.desktop_key();
    let machine = ctx.machine_id();
    let pubkey = ctx.keys.sign_public();
    let relay_key = RelayKey::from_bytes(ctx.keys.relay.as_bytes()).unwrap();
    rig.remote.enable(true);
    rt.block_on(async {
        let (stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept()).await.unwrap().unwrap();
        let mut seen = String::new();
        let mut ws = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, res: Response| {
            seen = format!("{}?{}", req.uri().path(), req.uri().query().unwrap_or(""));
            Ok(res)
        })
        .await
        .unwrap();
        let (path, q) = seen.split_once('?').unwrap();
        assert_eq!(path, format!("/m/{machine}"));
        let ts: u64 = query(q, "ts").parse().unwrap();
        assert!(verify(&pubkey, MACHINE_DOMAIN, &machine, ts, &query(q, "sig")));
        let mut sessions = Vec::new();
        for (conn, phone) in [(1u16, &a), (2u16, &b)] {
            ws.send(Message::Binary(encode_control(&Control { t: ControlKind::Open, conn }).unwrap().into())).await.unwrap();
            let (init, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
            ws.send(Message::Binary(encode_data(conn, &msg1).unwrap().into())).await.unwrap();
            sessions.push((conn, init));
        }
        let mut transports = std::collections::HashMap::new();
        while transports.len() < 2 {
            let (conn, msg2) = recv_env(&mut ws).await;
            let idx = sessions.iter().position(|(c, _)| *c == conn).unwrap();
            let (_, init) = sessions.remove(idx);
            transports.insert(conn, init.finish(&msg2).unwrap());
        }
        for conn in [1u16, 2] {
            let t = transports.get_mut(&conn).unwrap();
            let hello = json!({ "id": format!("h{conn}"), "t": "hello", "proto": 1 });
            ws.send(Message::Binary(encode_data(conn, &seal_json(t, &hello)).unwrap().into())).await.unwrap();
        }
        let mut got: std::collections::HashMap<u16, Vec<Value>> = std::collections::HashMap::new();
        while got.values().map(Vec::len).sum::<usize>() < 4 {
            let (conn, bytes) = recv_env(&mut ws).await;
            let reply = open_reply(transports.get_mut(&conn).unwrap(), &bytes);
            got.entry(conn).or_default().push(reply);
        }
        for (conn, msgs) in &got {
            assert_eq!(msgs[0]["re"], format!("h{conn}"));
            assert_eq!(msgs[0]["relay"], format!("http://127.0.0.1:{port}"));
            assert_eq!(msgs[1]["t"], "relay.token");
            let device = if *conn == 1 { "phone-a" } else { "phone-b" };
            assert_eq!(verify_token(&relay_key, &machine, msgs[1]["token"].as_str().unwrap(), now_secs()).unwrap(), device);
        }
        assert!(lock(&ctx.devices).find("phone-a").unwrap().token_exp > now_secs());
        for _ in 0..50 {
            if ctx.live_count() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ctx.live_count(), 2);
        ws.send(Message::Binary(encode_control(&Control { t: ControlKind::Close, conn: 1 }).unwrap().into())).await.unwrap();
        for _ in 0..50 {
            if ctx.live_count() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ctx.live_count(), 1);
        assert_eq!(ctx.relay_state(), "online");
    });
    rig.remote.enable(false);
    assert_eq!(ctx.relay_state(), "off");
}

#[test]
fn register_posts_a_signed_tofu_body_and_marks_the_machine_registered() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("register", &format!("http://127.0.0.1:{port}"), false);
    let ctx = rig.remote.ctx().unwrap();
    let server = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = s.read(&mut buf) {
            raw.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let len = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                if body.len() >= len {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        s.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 27\r\n\r\n{\"ok\":true,\"created\":true}\n").unwrap();
        String::from_utf8_lossy(&raw).into_owned()
    });
    assert_eq!(register(&ctx), Ok(()));
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /register HTTP/1.1"));
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    let machine = ctx.machine_id();
    assert_eq!(body["machineId"], machine);
    assert_eq!(body["relayKey"], ctx.keys.relay_key());
    let ts = body["ts"].as_u64().unwrap();
    assert!(verify(body["pubkey"].as_str().unwrap(), REGISTER_DOMAIN, &machine, ts, body["sig"].as_str().unwrap()));
    assert!(lock(&ctx.config).relay.registered);
    assert!(rig.host.changes.load(Ordering::SeqCst) == 0);
}

#[test]
fn the_relay_socket_url_follows_the_base_scheme() {
    assert_eq!(ws_url("https://r.example.dev/", "m1", 5, "s").as_deref(), Some("wss://r.example.dev/m/m1?ts=5&sig=s"));
    assert_eq!(ws_url("http://127.0.0.1:9", "m1", 5, "s").as_deref(), Some("ws://127.0.0.1:9/m/m1?ts=5&sig=s"));
    assert_eq!(ws_url("ftp://x", "m1", 5, "s"), None);
}
