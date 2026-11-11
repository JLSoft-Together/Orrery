mod common;

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use common::{now_seconds, Gateway};
use rust_lib_orrery_remote::api::remote::{
    decode_qr, generate_static_key, pair_start, session_start, static_public_key, RemoteErrorKind,
    SecureChannel,
};
use serde_json::{json, Value};
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

fn start(exp_in: i64) -> Arc<Gateway> {
    let gateway = Gateway::new(exp_in);
    gateway.serve(TcpListener::bind("127.0.0.1:0").unwrap());
    std::thread::sleep(std::time::Duration::from_millis(30));
    gateway
}

fn connect(gateway: &Gateway) -> Ws {
    let port = *gateway.port.lock().unwrap();
    tungstenite::connect(format!("ws://127.0.0.1:{port}/remote")).unwrap().0
}

fn read_binary(ws: &mut Ws) -> Result<Vec<u8>, Option<u16>> {
    loop {
        match ws.read() {
            Ok(Message::Binary(bytes)) => return Ok(bytes.to_vec()),
            Ok(Message::Close(frame)) => return Err(frame.map(|f| u16::from(f.code))),
            Ok(_) => continue,
            Err(_) => return Err(None),
        }
    }
}

fn send_command(ws: &mut Ws, channel: &SecureChannel, value: Value) {
    for sealed in channel.seal(0, 0, value.to_string().into_bytes()).unwrap() {
        ws.send(Message::Binary(sealed.into())).unwrap();
    }
}

fn read_json(ws: &mut Ws, channel: &SecureChannel) -> Value {
    loop {
        let cipher = read_binary(ws).unwrap();
        if let Some(message) = channel.open(cipher).unwrap() {
            if message.kind == 0 {
                return serde_json::from_slice(&message.payload).unwrap();
            }
        }
    }
}

fn pair(gateway: &Gateway, key: &[u8], qr: &str) -> (SecureChannel, Ws, Value) {
    let mut ws = connect(gateway);
    let started = pair_start(
        key.to_vec(),
        qr.to_owned(),
        "phone-0001".into(),
        "Test Phone".into(),
        now_seconds(),
    )
    .unwrap();
    ws.send(Message::Binary(started.msg1.clone().into())).unwrap();
    let msg2 = read_binary(&mut ws).unwrap();
    let done = started.handshake.finish(msg2).unwrap();
    ws.send(Message::Binary(done.msg3.clone().into())).unwrap();
    let verdict = read_json(&mut ws, &done.channel);
    (done.channel, ws, verdict)
}

fn open_session(gateway: &Gateway, key: &[u8]) -> Result<(SecureChannel, Ws), Option<u16>> {
    let mut ws = connect(gateway);
    let started = session_start(
        key.to_vec(),
        gateway.desktop.public().to_vec(),
        "phone-0001".into(),
    )
    .unwrap();
    ws.send(Message::Binary(started.msg1.into())).unwrap();
    let msg2 = read_binary(&mut ws)?;
    let channel = started.handshake.finish(msg2).map_err(|_| None)?;
    Ok((channel, ws))
}

#[test]
fn qr_decode_exposes_machine_route_and_key() {
    let gateway = start(300);
    let info = decode_qr(gateway.qr(None)).unwrap();
    assert_eq!(info.machine, "machine-fake-0001");
    assert_eq!(info.host.as_deref(), Some("127.0.0.1"));
    assert_eq!(info.mkey, gateway.desktop.public().to_vec());
    assert_eq!(decode_qr("not a qr".into()).err().unwrap().kind, RemoteErrorKind::BadQr);
}

#[test]
fn static_key_round_trip() {
    let private = generate_static_key().unwrap();
    assert_eq!(private.len(), 32);
    assert_eq!(static_public_key(private.clone()).unwrap().len(), 32);
    assert_eq!(static_public_key(vec![1, 2, 3]).err().unwrap().kind, RemoteErrorKind::BadKey);
}

#[test]
fn pair_then_ik_session_with_snapshot_and_pty_echo() {
    let gateway = start(300);
    let key = generate_static_key().unwrap();
    let (_channel, _ws, verdict) = pair(&gateway, &key, &gateway.qr(None));
    assert_eq!(verdict["t"], "pair.ok");
    assert_eq!(verdict["machine"], "machine-fake-0001");
    assert_eq!(gateway.device_count(), 1);

    let (channel, mut ws) = open_session(&gateway, &key).unwrap();
    send_command(&mut ws, &channel, json!({"id": "c1", "t": "hello", "proto": 1}));
    let reply = read_json(&mut ws, &channel);
    assert_eq!(reply["re"], "c1");
    assert_eq!(reply["ok"], true);
    assert_eq!(read_json(&mut ws, &channel)["t"], "snapshot");
    assert_eq!(read_json(&mut ws, &channel)["t"], "ws.snapshot");

    send_command(
        &mut ws,
        &channel,
        json!({"id": "c2", "t": "pty.attach", "stream": 7, "sessionId": "s", "cols": 80, "rows": 24}),
    );
    assert_eq!(read_json(&mut ws, &channel)["re"], "c2");
    assert_eq!(read_json(&mut ws, &channel)["t"], "pty.ready");
    let replay = channel.open(read_binary(&mut ws).unwrap()).unwrap().unwrap();
    assert_eq!((replay.kind, replay.stream), (1, 7));
    assert_eq!(replay.payload, b"replay\r\n");

    for sealed in channel.seal(2, 7, b"ls".to_vec()).unwrap() {
        ws.send(Message::Binary(sealed.into())).unwrap();
    }
    let echoed = channel.open(read_binary(&mut ws).unwrap()).unwrap().unwrap();
    assert_eq!((echoed.kind, echoed.stream), (1, 7));
    assert_eq!(echoed.payload, b"echo:ls");
}

#[test]
fn large_json_is_chunked_and_reassembled() {
    let gateway = start(300);
    let key = generate_static_key().unwrap();
    pair(&gateway, &key, &gateway.qr(None));
    let (channel, mut ws) = open_session(&gateway, &key).unwrap();
    let big = "x".repeat(150_000);
    let chunks = channel
        .seal(0, 0, json!({"id": "c9", "t": "nope", "pad": big}).to_string().into_bytes())
        .unwrap();
    assert!(chunks.len() >= 3);
    for sealed in chunks {
        ws.send(Message::Binary(sealed.into())).unwrap();
    }
    let reply = read_json(&mut ws, &channel);
    assert_eq!(reply["re"], "c9");
    assert_eq!(reply["error"], "notFound");
}

#[test]
fn desktop_key_mismatch_stops_pairing_before_msg3() {
    let gateway = start(300);
    let key = generate_static_key().unwrap();
    let other = static_public_key(generate_static_key().unwrap()).unwrap();
    let mut forged = [0u8; 32];
    forged.copy_from_slice(&other);
    let qr = gateway.qr(Some(forged));
    let mut ws = connect(&gateway);
    let started = pair_start(key, qr, "phone-0001".into(), "P".into(), now_seconds()).unwrap();
    ws.send(Message::Binary(started.msg1.into())).unwrap();
    let msg2 = read_binary(&mut ws).unwrap();
    let error = started.handshake.finish(msg2).err().unwrap();
    assert_eq!(error.kind, RemoteErrorKind::KeyMismatch);
    assert_eq!(gateway.device_count(), 0);
}

#[test]
fn expired_qr_is_rejected_before_any_traffic() {
    let gateway = start(-10);
    let key = generate_static_key().unwrap();
    let error = pair_start(key, gateway.qr(None), "phone-0001".into(), "P".into(), now_seconds())
        .err()
        .unwrap();
    assert_eq!(error.kind, RemoteErrorKind::Expired);
}

#[test]
fn unpaired_phone_is_closed_with_4401() {
    let gateway = start(300);
    let key = generate_static_key().unwrap();
    assert_eq!(open_session(&gateway, &key).err(), Some(Some(4401)));
}

#[test]
fn pairing_code_works_only_once() {
    let gateway = start(300);
    let key = generate_static_key().unwrap();
    let qr = gateway.qr(None);
    pair(&gateway, &key, &qr);
    let mut ws = connect(&gateway);
    let started =
        pair_start(key, qr, "phone-0002".into(), "P".into(), now_seconds()).unwrap();
    ws.send(Message::Binary(started.msg1.into())).unwrap();
    assert_eq!(read_binary(&mut ws).err(), Some(Some(4400)));
}
