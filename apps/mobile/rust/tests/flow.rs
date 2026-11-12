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
    for sealed in channel.seal(0, 0, value.to_string().into_bytes(), false).unwrap() {
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
    let config = read_json(&mut ws, &channel);
    assert_eq!((config["t"].as_str(), config["projectId"].as_str()), (Some("push.config"), Some("fake-project")));

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

    for sealed in channel.seal(2, 7, b"ls".to_vec(), false).unwrap() {
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
        .seal(0, 0, json!({"id": "c9", "t": "nope", "pad": big}).to_string().into_bytes(), false)
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

fn ready_session(gateway: &Gateway) -> (SecureChannel, Ws) {
    let key = generate_static_key().unwrap();
    pair(gateway, &key, &gateway.qr(None));
    let (channel, mut ws) = open_session(gateway, &key).unwrap();
    send_command(&mut ws, &channel, json!({"id": "h", "t": "hello", "proto": 1}));
    for _ in 0..4 {
        read_json(&mut ws, &channel);
    }
    (channel, ws)
}

fn attach(ws: &mut Ws, channel: &SecureChannel, id: &str, stream: u32) -> Value {
    send_command(
        ws,
        channel,
        json!({"id": id, "t": "pty.attach", "stream": stream, "sessionId": "s1", "cols": 80, "rows": 24}),
    );
    let reply = read_json(ws, channel);
    if reply["ok"] == true {
        read_json(ws, channel);
        channel.open(read_binary(ws).unwrap()).unwrap();
    }
    reply
}

fn type_keys(ws: &mut Ws, channel: &SecureChannel, stream: u32, text: &[u8], claim: bool) -> Vec<u8> {
    for sealed in channel.seal(2, stream, text.to_vec(), claim).unwrap() {
        ws.send(Message::Binary(sealed.into())).unwrap();
    }
    channel.open(read_binary(ws).unwrap()).unwrap().unwrap().payload
}

#[test]
fn claim_bit_is_set_only_when_the_caller_asks_and_only_on_pty_input() {
    let gateway = start(300);
    let (channel, mut ws) = ready_session(&gateway);
    assert_eq!(attach(&mut ws, &channel, "a", 3)["stream"], 3);
    assert_eq!(type_keys(&mut ws, &channel, 3, b"x", true), b"claim+echo:x");
    assert_eq!(type_keys(&mut ws, &channel, 3, b"x", false), b"echo:x");
    send_command(&mut ws, &channel, json!({"id": "g", "t": "ws.get"}));
    assert_eq!(read_json(&mut ws, &channel)["re"], "g");
    let flagged = channel.seal(0, 0, b"{}".to_vec(), true).unwrap();
    assert_eq!(flagged.len(), 1);
}

#[test]
fn push_register_stores_a_plain_token_and_unregister_clears_it() {
    let gateway = start(300);
    let (channel, mut ws) = ready_session(&gateway);
    let token = "fcm-token-aaaaaaaaaaaaaaaa:APA91b";
    send_command(&mut ws, &channel, json!({"id": "r1", "t": "push.register", "token": token, "platform": "android", "locale": "vi"}));
    let reply = read_json(&mut ws, &channel);
    assert_eq!((reply["re"].as_str(), reply["ok"].as_bool(), reply["stored"].as_str()), (Some("r1"), Some(true), Some(token)));
    send_command(&mut ws, &channel, json!({"id": "r2", "t": "push.register", "token": "bad token", "platform": "android", "locale": "vi"}));
    assert_eq!(read_json(&mut ws, &channel)["error"], "invalid");
    send_command(&mut ws, &channel, json!({"id": "r3", "t": "push.register", "token": token, "platform": "web", "locale": "vi"}));
    assert_eq!(read_json(&mut ws, &channel)["error"], "invalid");
    send_command(&mut ws, &channel, json!({"id": "u1", "t": "push.unregister"}));
    let cleared = read_json(&mut ws, &channel);
    assert_eq!((cleared["ok"].as_bool(), cleared["stored"].is_null()), (Some(true), true));
}

#[test]
fn attach_rules_stream_zero_duplicate_and_fifth_attach() {
    let gateway = start(300);
    let (channel, mut ws) = ready_session(&gateway);
    assert_eq!(attach(&mut ws, &channel, "z", 0)["error"], "invalid");
    for stream in 1..=4 {
        assert_eq!(attach(&mut ws, &channel, "ok", stream)["stream"], stream);
    }
    assert_eq!(attach(&mut ws, &channel, "dup", 2)["error"], "invalid");
    assert_eq!(attach(&mut ws, &channel, "five", 5)["error"], "limit");
}

#[test]
fn close_needs_confirm_then_patches_the_workspace_on_answer() {
    let gateway = start(300);
    let (channel, mut ws) = ready_session(&gateway);
    send_command(&mut ws, &channel, json!({"id": "x1", "t": "ws.close", "paneKey": "pane1"}));
    let reply = read_json(&mut ws, &channel);
    let cid = reply["confirm"].as_str().unwrap().to_owned();
    let ask = read_json(&mut ws, &channel);
    assert_eq!((ask["t"].as_str(), ask["re"].as_str()), (Some("confirm.ask"), Some("x1")));
    send_command(&mut ws, &channel, json!({"id": "x2", "t": "confirm.answer", "cid": cid, "ok": true}));
    assert_eq!(read_json(&mut ws, &channel)["re"], "x2");
    let patch = read_json(&mut ws, &channel);
    assert_eq!((patch["t"].as_str(), patch["base"].as_u64(), patch["rev"].as_u64()), (Some("ws.patch"), Some(1), Some(2)));
    send_command(&mut ws, &channel, json!({"id": "x3", "t": "confirm.answer", "cid": cid, "ok": true}));
    assert_eq!(read_json(&mut ws, &channel)["error"], "notFound");
}

#[test]
fn declined_confirm_is_cancelled_and_pty_exit_without_code_ends_the_stream() {
    let gateway = start(300);
    let (channel, mut ws) = ready_session(&gateway);
    send_command(&mut ws, &channel, json!({"id": "y1", "t": "ws.close", "paneKey": "pane1"}));
    let cid = read_json(&mut ws, &channel)["confirm"].as_str().unwrap().to_owned();
    read_json(&mut ws, &channel);
    send_command(&mut ws, &channel, json!({"id": "y2", "t": "confirm.answer", "cid": cid, "ok": false}));
    assert_eq!(read_json(&mut ws, &channel)["cancelled"], true);
    send_command(
        &mut ws,
        &channel,
        json!({"id": "v", "t": "pty.attach", "stream": 1, "sessionId": "vanishes", "cols": 80, "rows": 24}),
    );
    assert_eq!(read_json(&mut ws, &channel)["stream"], 1);
    assert_eq!(read_json(&mut ws, &channel)["t"], "pty.ready");
    channel.open(read_binary(&mut ws).unwrap()).unwrap();
    let exit = read_json(&mut ws, &channel);
    assert_eq!(exit["t"], "pty.exit");
    assert!(exit["code"].is_null());
}
