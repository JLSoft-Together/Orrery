use std::path::PathBuf;
use std::time::Duration;

use core_remote_proto::frame::Kind;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::devices::Device;
use super::phone_pty::{strip_claim, CLAIM_FLAG};
use super::test_util::*;
use super::pty_bridge::{spawn, FromHub, Slot, Spec, ToHub};
use super::workspace::MAX_ATTACH;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use crate::pty::PtyHost;
use crate::state::lock;

type Desk = WebSocketStream<MaybeTlsStream<TcpStream>>;
const DESK_ORIGIN: &str = "http://127.0.0.1:1";

fn core_bin() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "orrery-core.exe" } else { "orrery-core" };
    let bin = std::env::var_os("CCM_CORE_BIN").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/target/debug").join(exe));
    bin.is_file().then_some(bin)
}

async fn desk(url: &str, protocol: &str) -> Desk {
    let mut req = url.into_client_request().unwrap();
    req.headers_mut().insert("origin", HeaderValue::from_static(DESK_ORIGIN));
    req.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_str(protocol).unwrap());
    tokio_tungstenite::connect_async(req).await.unwrap().0
}

async fn desk_until(d: &mut Desk, mut want: impl FnMut(&Message) -> bool) {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(20), d.next()).await.expect("desktop wait").expect("desktop closed").unwrap();
        assert!(!matches!(msg, Message::Close(_)), "desktop socket closed");
        if want(&msg) {
            return;
        }
    }
}

async fn phone_output(ws: &mut Ws, t: &mut core_remote_proto::Transport, stream: u32, marker: &str) -> Vec<Value> {
    let mut seen = Vec::new();
    let mut events = Vec::new();
    loop {
        let f = recv_frame(ws, t).await.expect("phone closed");
        match f.kind {
            Kind::PtyOut => {
                assert_eq!(f.stream, stream);
                seen.extend_from_slice(&f.payload);
                if String::from_utf8_lossy(&seen).contains(marker) {
                    return events;
                }
            }
            Kind::Json => events.push(serde_json::from_slice(&f.payload).unwrap()),
            _ => {}
        }
    }
}

#[test]
fn the_claim_bit_is_taken_off_kind_two_only() {
    let mut a = [2u8, CLAIM_FLAG, 0, 0, 0, 1, b'x'];
    assert!(strip_claim(&mut a));
    assert_eq!(a[1], 0);
    let mut b = [0u8, CLAIM_FLAG, 0, 0, 0, 0];
    assert!(!strip_claim(&mut b));
    assert_eq!(b[1], CLAIM_FLAG);
}

#[test]
fn a_phone_views_and_types_into_a_desktop_pane_through_the_real_hub() {
    let Some(bin) = core_bin() else {
        eprintln!("skip: orrery-core debug build not found");
        return;
    };
    let dir = temp_dir("pty-hub");
    let host = PtyHost::new(bin, DESK_ORIGIN.into(), "0.0.0".into(), dir.join("pty.log"));
    let ep = host.endpoint().unwrap();
    let rig = rig("pty-phone", "", false);
    *lock(&rig.host.pty) = Some((ep.url.clone(), ep.protocol.clone()));
    rig.remote.enable(true);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let mut d = desk(&ep.url, &ep.protocol).await;
        d.send(Message::Text(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": dir.to_string_lossy() }).to_string().into())).await.unwrap();
        let mut id = String::new();
        desk_until(&mut d, |m| match m {
            Message::Text(t) => {
                let v: Value = serde_json::from_str(t.as_str()).unwrap();
                if v["t"] == "ready" {
                    id = v["id"].as_str().unwrap().to_owned();
                }
                !id.is_empty()
            }
            _ => false,
        })
        .await;
        d.send(Message::Binary(b"echo desk-first\r".to_vec().into())).await.unwrap();
        desk_until(&mut d, |m| matches!(m, Message::Binary(b) if String::from_utf8_lossy(b).contains("desk-first"))).await;
        let panes = json!([{ "key": "p1", "kind": "term", "title": "sh", "state": "running", "rail": "idle", "subagents": 0, "sessionId": id, "dirty": false, "busy": false }]);
        assert!(rig.remote.publish(&json!({ "rev": 1, "tabs": [{ "key": "t1", "tree": { "pane": "p1" }, "panes": panes }], "closedCount": 0 })));
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello(&mut ws, &mut t).await;
        send_json(&mut ws, &mut t, json!({ "id": "bad", "t": "pty.attach", "stream": 1, "sessionId": "not-a-pane", "cols": 50, "rows": 20 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "bad").await["error"], "notFound");
        send_json(&mut ws, &mut t, json!({ "id": "a", "t": "pty.attach", "stream": 7, "sessionId": id, "cols": 50, "rows": 20 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "a").await["ok"], true);
        let ready = until_json(&mut ws, &mut t, |v| v["t"] == "pty.ready").await;
        assert_eq!((ready["stream"].as_u64(), ready["cols"].as_u64(), ready["rows"].as_u64()), (Some(7), Some(80), Some(24)));
        phone_output(&mut ws, &mut t, 7, "desk-first").await;
        send_raw(&mut ws, &mut t, 2, CLAIM_FLAG, 7, b"echo phone-mark\r").await;
        desk_until(&mut d, |m| matches!(m, Message::Text(t) if t.as_str().contains("\"resized\"") && t.as_str().contains("50"))).await;
        desk_until(&mut d, |m| matches!(m, Message::Binary(b) if String::from_utf8_lossy(b).contains("phone-mark"))).await;
        phone_output(&mut ws, &mut t, 7, "phone-mark").await;
        d.send(Message::Text(json!({ "t": "claim" }).to_string().into())).await.unwrap();
        d.send(Message::Binary(b"echo desk-again\r".to_vec().into())).await.unwrap();
        let events = phone_output(&mut ws, &mut t, 7, "desk-again").await;
        assert!(events.iter().any(|e| e["t"] == "pty.resized" && e["cols"] == 80), "{events:?}");
        for s in 0..MAX_ATTACH as u32 - 1 {
            send_json(&mut ws, &mut t, json!({ "id": format!("m{s}"), "t": "pty.attach", "stream": 20 + s, "sessionId": id })).await;
        }
        send_json(&mut ws, &mut t, json!({ "id": "over", "t": "pty.attach", "stream": 99, "sessionId": id })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "over").await["error"], "limit");
        assert_eq!(ctx.board.attached("p"), MAX_ATTACH);
        send_json(&mut ws, &mut t, json!({ "id": "dt", "t": "pty.detach", "stream": 20 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "dt").await["ok"], true);
        let wait = std::time::Instant::now();
        while ctx.board.attached("p") > MAX_ATTACH - 1 && wait.elapsed() < Duration::from_secs(5) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ctx.board.attached("p"), MAX_ATTACH - 1);
        drop(ws);
        let wait = std::time::Instant::now();
        while ctx.board.attached("p") > 0 && wait.elapsed() < Duration::from_secs(5) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ctx.board.attached("p"), 0);
        d.send(Message::Binary(b"echo still-here\r".to_vec().into())).await.unwrap();
        desk_until(&mut d, |m| matches!(m, Message::Binary(b) if String::from_utf8_lossy(b).contains("still-here"))).await;
        d.send(Message::Text(json!({ "t": "kill", "id": id }).to_string().into())).await.unwrap();
    });
    rig.remote.enable(false);
    host.stop();
}

async fn desk_spawn(d: &mut Desk, dir: &std::path::Path) -> String {
    d.send(Message::Text(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": dir.to_string_lossy() }).to_string().into())).await.unwrap();
    let mut id = String::new();
    desk_until(d, |m| match m {
        Message::Text(t) => {
            let v: Value = serde_json::from_str(t.as_str()).unwrap();
            if v["t"] == "ready" {
                id = v["id"].as_str().unwrap().to_owned();
            }
            !id.is_empty()
        }
        _ => false,
    })
    .await;
    id
}

#[test]
fn a_claimed_phone_that_stops_reading_never_stalls_the_desktop_pane() {
    let Some(bin) = core_bin() else {
        eprintln!("skip: orrery-core debug build not found");
        return;
    };
    let dir = temp_dir("pty-stall");
    let host = PtyHost::new(bin, DESK_ORIGIN.into(), "0.0.0".into(), dir.join("pty.log"));
    let ep = host.endpoint().unwrap();
    let rig = rig("pty-stall", "", false);
    *lock(&rig.host.pty) = Some((ep.url.clone(), ep.protocol.clone()));
    let ctx = rig.remote.ctx().unwrap();
    runtime().block_on(async {
        let mut d = desk(&ep.url, &ep.protocol).await;
        let id = desk_spawn(&mut d, &dir).await;
        d.send(Message::Binary(b"echo desk-up\r".to_vec().into())).await.unwrap();
        desk_until(&mut d, |m| matches!(m, Message::Binary(b) if String::from_utf8_lossy(b).contains("desk-up"))).await;
        let (back, mut rx) = tokio::sync::mpsc::channel(1);
        let slot = Slot::take(&ctx, "d").unwrap();
        let spec = Spec { re: "a".into(), stream: 1, session: id.clone(), cols: 50, rows: 20, gen: 1, backlog: Arc::new(AtomicUsize::new(0)) };
        let tx = spawn(ctx.host.clone(), spec, slot, back);
        loop {
            match tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.unwrap().unwrap() {
                FromHub::Out(..) => break,
                FromHub::Ended(..) => panic!("bridge ended"),
                FromHub::Json(..) => {}
            }
        }
        tx.send(ToHub::Input(Vec::new(), true)).await.unwrap();
        desk_until(&mut d, |m| matches!(m, Message::Text(t) if t.as_str().contains("\"resized\"") && t.as_str().contains("50"))).await;
        d.send(Message::Binary(b"1..150000 | % { \"flood$_\" }; 'done-' + 'mark'\r".to_vec().into())).await.unwrap();
        let mut last = std::time::Instant::now();
        let mut gap = Duration::ZERO;
        let mut tail = String::new();
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(10), d.next()).await.expect("desktop stalled").unwrap().unwrap();
            if let Message::Binary(b) = msg {
                gap = gap.max(last.elapsed());
                last = std::time::Instant::now();
                tail.push_str(&String::from_utf8_lossy(&b));
                if tail.contains("done-mark") {
                    break;
                }
                let keep = tail.len().saturating_sub(64);
                tail = tail.split_off(tail.char_indices().map(|(i, _)| i).find(|i| *i >= keep).unwrap_or(0));
            }
        }
        assert!(gap <= Duration::from_secs(3), "{gap:?}");
        let mut overflow = false;
        loop {
            match tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.unwrap() {
                Some(FromHub::Ended(_, _, stalled)) => {
                    overflow = stalled;
                    break;
                }
                None => break,
                Some(_) => {}
            }
        }
        assert!(overflow);
        assert_eq!(ctx.board.attached("d"), 0);
        d.send(Message::Text(json!({ "t": "kill", "id": id }).to_string().into())).await.unwrap();
    });
    host.stop();
}
