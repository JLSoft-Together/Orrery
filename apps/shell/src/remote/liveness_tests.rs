use std::time::Duration;

use core_remote_proto::frame::{Frame, Kind};
use core_remote_proto::relay::{decode, encode_control, encode_data, Control, ControlKind, Envelope};
use core_remote_proto::session::SessionInitiator;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use super::config::now_secs;
use super::devices::Device;
use super::test_util::*;
use crate::state::lock;

struct Run {
    pings: usize,
    closed: bool,
    live: usize,
}

fn drive(every_ms: u64, dead_ms: u64, echo: bool, watch_ms: u64) -> Run {
    let rt = runtime();
    let listener = rt.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let rig = rig("liveness", &format!("http://127.0.0.1:{port}"), true);
    let ctx = rig.remote.ctx().unwrap();
    *lock(&ctx.keepalive) = (Duration::from_millis(every_ms), Duration::from_millis(dead_ms));
    let phone = Phone::new("phone-a");
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, &phone.id, phone.key.public(), now_secs())));
    let desktop = rig.desktop_key();
    rig.remote.enable(true);
    let run = rt.block_on(async {
        let (stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept()).await.unwrap().unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        ws.send(Message::Binary(encode_control(&Control::new(ControlKind::Open, 1)).unwrap().into())).await.unwrap();
        let (init, msg1) = SessionInitiator::start(&phone.key, &desktop, &phone.id).unwrap();
        ws.send(Message::Binary(encode_data(1, &msg1).unwrap().into())).await.unwrap();
        let mut t = loop {
            let msg = tokio::time::timeout(Duration::from_secs(10), ws.next()).await.unwrap().unwrap().unwrap();
            if let Message::Binary(b) = msg {
                if let Ok(Envelope::Data { bytes, .. }) = decode(&b) {
                    break init.finish(bytes).unwrap();
                }
            }
        };
        let hello = seal_json(&mut t, &json!({ "id": "h1", "t": "hello", "proto": 1 }));
        ws.send(Message::Binary(encode_data(1, &hello).unwrap().into())).await.unwrap();
        let mut run = Run { pings: 0, closed: false, live: 0 };
        let end = tokio::time::Instant::now() + Duration::from_millis(watch_ms);
        while let Ok(Some(Ok(Message::Binary(b)))) = tokio::time::timeout_at(end, ws.next()).await {
            match decode(&b) {
                Ok(Envelope::Control(Control { t: ControlKind::Close, .. })) => run.closed = true,
                Ok(Envelope::Data { bytes, .. }) => {
                    let ping = t.open(bytes).ok().and_then(|p| Frame::decode(&p).ok()).is_some_and(|f| f.kind == Kind::Ping);
                    if ping {
                        run.pings += 1;
                        if echo {
                            let sealed = t.seal(&Frame::new(Kind::Ping, 0, Vec::new()).encode().unwrap()).unwrap();
                            ws.send(Message::Binary(encode_data(1, &sealed).unwrap().into())).await.unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
        run.live = ctx.live_count();
        run
    });
    rig.remote.enable(false);
    run
}

#[test]
fn a_relay_phone_that_stops_answering_is_dropped_and_the_relay_is_told_to_close_it() {
    let run = drive(100, 400, false, 2000);
    assert!(run.pings >= 2, "pings {}", run.pings);
    assert!(run.closed);
    assert_eq!(run.live, 0);
}

#[test]
fn a_relay_phone_that_echoes_every_ping_stays_live_past_the_dead_deadline() {
    let run = drive(100, 400, true, 1500);
    assert!(!run.closed);
    assert_eq!(run.live, 1);
    assert!(run.pings >= 8 && run.pings <= 16, "pings {}", run.pings);
}
