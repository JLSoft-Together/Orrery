mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::*;
use core_http::{Hooks, TapError};
use core_store::event::{IdOnly, MonitorEvent};
use futures_util::StreamExt;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

async fn next_text(ws: &mut Ws) -> String {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next()).await.expect("a message in time").expect("open").expect("no error");
        if let Message::Text(t) = msg {
            return t.as_str().to_owned();
        }
    }
}

type Log = Arc<Mutex<Vec<String>>>;

fn recorder() -> (impl Fn(Arc<str>) + Send + Sync + 'static, Log) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    (move |m: Arc<str>| sink.lock().unwrap().push(m.to_string()), got)
}

#[tokio::test]
async fn the_tap_gets_the_snapshot_then_the_same_batches_as_a_ws_client_byte_for_byte() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let mut ws = ws_connect(server.port(), &[]).await.unwrap();
    let ws_snapshot = next_text(&mut ws).await;
    let tap = server.tap();
    let (f, got) = recorder();
    let (snapshot, _guard) = tap.attach(f);
    assert_eq!(snapshot, ws_snapshot);
    assert_eq!(tap.snapshot(), ws_snapshot);
    store.with(|s| s.upsert_terminal(terminal("t1", "one")));
    let first = next_text(&mut ws).await;
    store.with(|s| s.upsert_terminal(terminal("t2", "two")));
    let second = next_text(&mut ws).await;
    wait_for(|| got.lock().unwrap().len() >= 2).await;
    assert_eq!(*got.lock().unwrap(), vec![first, second]);
    server.shutdown().await;
}

#[tokio::test]
async fn forbidden_types_are_denied_and_never_reach_the_dispatch() {
    let (hooks, seen) = collecting();
    let (server, _store) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let tap = server.tap();
    for kind in ["client.presence", "terminal.open", "terminal.focusWindow", "folder.pick", "favorite.open", "notify.update", "sensitive.update", "office.enable"] {
        let msg = json!({ "type": kind, "terminalId": "t", "dir": "d", "requestId": "r", "attentive": true, "app": "vscode", "mode": "new" });
        assert_eq!(tap.send(msg, |_| {}), Err(TapError::Denied), "{kind}");
    }
    assert_eq!(tap.send(json!({ "no": "type" }), |_| {}), Err(TapError::Invalid));
    assert_eq!(tap.send(json!({ "type": "terminal.alias" }), |_| {}), Err(TapError::Invalid));
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(tap.send(json!({ "type": "terminal.alias", "terminalId": "t1", "alias": "x" }), |_| {}), Ok(None));
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(tap.send(json!({ "type": "tool-spans.get" }), |_| {}), Ok(None));
    assert_eq!(seen.lock().unwrap().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn a_direct_reply_goes_back_to_the_tap_that_asked_and_to_nobody_else() {
    let hooks = Hooks {
        on_client_message: Some(Arc::new(|m, ctx| {
            if m == core_http::ClientMessage::HistoryGet {
                ctx.reply(MonitorEvent::TerminalRemoved(IdOnly { id: "answer".into() }));
            }
        })),
        ..Hooks::default()
    };
    let (server, _store) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let mut ws = ws_connect(server.port(), &[]).await.unwrap();
    next_text(&mut ws).await;
    let tap = server.tap();
    let (batches, batch_got) = recorder();
    let _guard = tap.subscribe(batches);
    let (reply, reply_got) = recorder();
    assert_eq!(tap.send(json!({ "type": "history.get" }), reply), Ok(None));
    wait_for(|| reply_got.lock().unwrap().len() == 1).await;
    assert!(reply_got.lock().unwrap()[0].contains("answer"));
    assert!(batch_got.lock().unwrap().is_empty());
    assert!(tokio::time::timeout(Duration::from_millis(200), ws.next()).await.is_err());
    let snapshot = tap.send(json!({ "type": "resync" }), |_| {}).unwrap().unwrap();
    assert!(snapshot.contains("\"snapshot\""));
    server.shutdown().await;
}

#[tokio::test]
async fn dropping_the_guard_stops_batches_and_a_tap_does_not_count_as_a_client() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let tap = server.tap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let guard = tap.subscribe(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    store.with(|s| s.upsert_terminal(terminal("t1", "one")));
    wait_for(|| calls.load(Ordering::SeqCst) == 1).await;
    drop(guard);
    store.with(|s| s.upsert_terminal(terminal("t2", "two")));
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn a_panicking_subscriber_does_not_break_the_store_or_other_subscribers() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let tap = server.tap();
    let _bad = tap.subscribe(|_| panic!("subscriber failure"));
    let (f, got) = recorder();
    let _good = tap.subscribe(f);
    store.with(|s| s.upsert_terminal(terminal("t1", "one")));
    wait_for(|| got.lock().unwrap().len() == 1).await;
    server.shutdown().await;
}
