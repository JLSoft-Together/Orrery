mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::*;
use core_http::{start, ClientMessage, Config, Hooks, StartError};
use core_store::event::FocusWindowResult;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Message;

fn as_values(got: &Arc<Mutex<Vec<ClientMessage>>>) -> Vec<Value> {
    got.lock().unwrap().iter().map(|m| serde_json::to_value(m).unwrap()).collect()
}

#[tokio::test]
async fn health_needs_no_key_but_everything_else_does() {
    let (server, store) = serve(dist_with_index("ok"), |c| c.app_key = Some("s3cret".into()), Hooks::default()).await;
    let port = server.port();
    let health = get(port, "/health", &[]).await;
    assert_eq!(health.status, 200);
    assert_eq!(health.header("content-type"), Some("application/json"));
    assert_eq!(health.text(), r#"{"ok":true,"seq":0}"#);
    store.with(|s| s.set_loading(true));
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(get(port, "/health", &[]).await.text(), r#"{"ok":true,"seq":1}"#);
    assert_eq!(get(port, "/health", &[("Origin", "http://localhost:9999")]).await.status, 403);

    let denied = get(port, "/", &[]).await;
    assert_eq!(denied.status, 403);
    assert_eq!(denied.text(), "Orrery only opens in its desktop app.");
    assert_eq!(get(port, "/", &[("x-ccm-app", "wrong!")]).await.status, 403);
    assert_eq!(get(port, "/", &[("x-ccm-app", "s3cre")]).await.status, 403);
    assert_eq!(get(port, "/", &[("x-ccm-app", "s3cret")]).await.status, 200);
    assert_eq!(get(port, "/snooze?t=a&m=5", &[]).await.status, 403);
    assert_eq!(http(port, "POST", "/focus", &[], Some("{}")).await.unwrap().status, 403);

    assert!(ws_connect(port, &[]).await.is_err());
    assert!(ws_connect(port, &[("x-ccm-app", "wrong!")]).await.is_err());
    let mut ws = ws_connect(port, &[("x-ccm-app", "s3cret")]).await.unwrap();
    assert_eq!(next_json(&mut ws).await["events"][0]["type"], "snapshot");
    server.shutdown().await;
}

#[tokio::test]
async fn the_websocket_refuses_another_origin_host_and_path() {
    let (server, _) = serve(dist_with_index("ok"), |c| c.dev_origin_ports = vec![5173], Hooks::default()).await;
    let port = server.port();
    assert!(ws_connect(port, &[("origin", "http://localhost:9999")]).await.is_err());
    assert!(ws_connect(port, &[("host", "evil.example")]).await.is_err());
    assert!(ws_connect(port, &[("origin", "http://127.0.0.1:5173")]).await.is_ok());
    assert!(ws_connect(port, &[("origin", &format!("http://localhost:{port}"))]).await.is_ok());

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut raw = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    raw.write_all(format!("GET /other HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), raw.read_to_end(&mut buf)).await;
    assert!(buf.is_empty());
    server.shutdown().await;
}

#[tokio::test]
async fn the_websocket_echoes_the_first_requested_subprotocol() {
    let (server, _) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let port = server.port();
    let mut raw = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    raw.write_all(format!("GET /ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: chat, other\r\n\r\n").as_bytes()).await.unwrap();
    let mut buf = vec![0u8; 512];
    let n = tokio::time::timeout(Duration::from_secs(2), raw.read(&mut buf)).await.unwrap().unwrap();
    let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
    assert!(head.starts_with("http/1.1 101"), "{head}");
    assert!(head.contains("sec-websocket-accept: s3pplmbitxaq9kygzzhzrbk+xoo="), "{head}");
    assert!(head.contains("sec-websocket-protocol: chat\r\n"), "{head}");
    server.shutdown().await;
}

#[tokio::test]
async fn client_messages_are_validated_by_shape_before_the_hook_sees_them() {
    let (hooks, got) = collecting();
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let mut ws = ws_connect(server.port(), &[]).await.unwrap();
    for bad in [
        json!({"type": "toolSpans.get", "since": "nope"}),
        json!({"type": "toolSpans.get", "since": "x".repeat(41)}),
        json!({"type": "toolSpans.get", "terminalId": 5}),
        json!({"type": "toolstats.get", "days": 14, "project": null}),
        json!({"type": "toolstats.get", "days": 7}),
        json!({"type": "toolstats.get", "days": 30, "project": ""}),
        json!({"type": "toolstats.get", "days": "7", "project": null}),
        json!({"type": "nope"}),
        json!([1]),
    ] {
        send_json(&mut ws, bad).await;
    }
    ws.send(Message::text("not json {")).await.unwrap();
    ws.send(Message::text("42")).await.unwrap();
    send_json(&mut ws, json!({"type": "toolSpans.get"})).await;
    send_json(&mut ws, json!({"type": "toolSpans.get", "terminalId": "x", "since": "2026-10-09T00:00:00.000Z"})).await;
    send_json(&mut ws, json!({"type": "toolstats.get", "days": 7, "project": null})).await;
    send_json(&mut ws, json!({"type": "toolstats.get", "days": 30, "project": "D:\\work\\demo", "extra": 1})).await;
    ws.send(Message::binary(json!({"type": "history.get"}).to_string().into_bytes())).await.unwrap();
    wait_for(|| got.lock().unwrap().len() >= 5).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        as_values(&got),
        vec![
            json!({"type": "toolSpans.get"}),
            json!({"type": "toolSpans.get", "terminalId": "x", "since": "2026-10-09T00:00:00.000Z"}),
            json!({"type": "toolstats.get", "days": 7, "project": null}),
            json!({"type": "toolstats.get", "days": 30, "project": "D:\\work\\demo"}),
            json!({"type": "history.get"}),
        ]
    );
    server.shutdown().await;
}

#[tokio::test]
async fn favorite_open_keeps_terminal_and_iterm_and_drops_any_other_terminal_app() {
    let (hooks, got) = collecting();
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let mut ws = ws_connect(server.port(), &[]).await.unwrap();
    for app in [json!("iterm"), json!("terminal"), json!("warp"), json!(5), Value::Null] {
        let mut m = json!({"type": "favorite.open", "dir": "/x", "mode": "new"});
        if !app.is_null() {
            m["terminalApp"] = app;
        }
        send_json(&mut ws, m).await;
    }
    wait_for(|| got.lock().unwrap().len() >= 5).await;
    let values = as_values(&got);
    assert_eq!(values[0], json!({"type": "favorite.open", "dir": "/x", "mode": "new", "terminalApp": "iterm"}));
    assert_eq!(values[1]["terminalApp"], "terminal");
    for m in &values[2..] {
        assert!(m.get("terminalApp").is_none(), "{m}");
    }
    server.shutdown().await;
}

#[tokio::test]
async fn listen_before_the_first_pass_one_payload_for_every_client_in_seq_order() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    store.with(|s| s.set_loading(true));
    tokio::time::sleep(Duration::from_millis(60)).await;
    let port = server.port();
    let mut a = ws_connect(port, &[]).await.unwrap();
    let mut b = ws_connect(port, &[]).await.unwrap();
    let snap_a = next_json(&mut a).await;
    let snap_b = next_json(&mut b).await;
    assert_eq!(snap_a, snap_b);
    assert_eq!(snap_a["events"][0]["type"], "snapshot");
    assert_eq!(snap_a["events"][0]["payload"]["loading"], true);
    assert_eq!(snap_a["events"][0]["payload"]["terminals"], json!([]));

    store.with(|s| {
        s.upsert_terminal(terminal("t", "A"));
        s.upsert_terminal(terminal("t", "B"));
        s.set_loading(false);
    });
    let next_a = next_json(&mut a).await;
    let next_b = next_json(&mut b).await;
    assert_eq!(next_a["seq"].as_i64().unwrap(), snap_a["seq"].as_i64().unwrap() + 1);
    assert_eq!(next_a["events"], json!([{"type": "terminal.created", "payload": {"id": "t", "title": "B", "status": "working"}}, {"type": "sessions.loading", "payload": {"loading": false}}]));
    assert_eq!(next_a, next_b);
    server.shutdown().await;
}

#[tokio::test]
async fn resync_answers_with_a_snapshot_after_pending_events_and_a_connect_flushes_first() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let port = server.port();
    let mut a = ws_connect(port, &[]).await.unwrap();
    let first = next_json(&mut a).await;
    store.with(|s| s.upsert_terminal(terminal("t1", "one")));
    send_json(&mut a, json!({"type": "resync"})).await;
    let broadcast = next_json(&mut a).await;
    assert_eq!(broadcast["events"][0]["type"], "terminal.created");
    let snap = next_json(&mut a).await;
    assert_eq!(snap["events"][0]["type"], "snapshot");
    assert_eq!(snap["events"][0]["payload"]["terminals"][0]["id"], "t1");
    assert_eq!(snap["seq"], broadcast["seq"]);
    assert!(snap["seq"].as_i64().unwrap() > first["seq"].as_i64().unwrap());

    store.with(|s| s.upsert_terminal(terminal("t2", "two")));
    let mut late = ws_connect(port, &[]).await.unwrap();
    let late_snap = next_json(&mut late).await;
    let ids: Vec<&str> = late_snap["events"][0]["payload"]["terminals"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["t1", "t2"]);
    let echo = next_json(&mut a).await;
    assert_eq!(echo["events"][0]["payload"]["id"], "t2");
    server.shutdown().await;
}

#[tokio::test]
async fn a_reply_goes_to_the_asking_socket_only_outside_the_broadcast_sequence() {
    let hooks = Hooks {
        on_client_message: Some(Arc::new(|m, ctx| {
            if matches!(m, ClientMessage::HistoryGet) {
                let event = serde_json::from_value(json!({"type": "history.data", "payload": {"sessions": []}})).unwrap();
                ctx.reply(event);
            }
        })),
        ..Hooks::default()
    };
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let port = server.port();
    let mut a = ws_connect(port, &[]).await.unwrap();
    let mut b = ws_connect(port, &[]).await.unwrap();
    next_json(&mut a).await;
    next_json(&mut b).await;
    send_json(&mut a, json!({"type": "history.get"})).await;
    let reply = next_json(&mut a).await;
    assert_eq!(reply, json!({"seq": -1, "direct": true, "events": [{"type": "history.data", "payload": {"sessions": []}}]}));
    assert!(tokio::time::timeout(Duration::from_millis(200), b.next()).await.is_err());
    server.shutdown().await;
}

#[tokio::test]
async fn a_second_server_on_the_same_port_reports_port_in_use() {
    let (first, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let config = Config::new(first.port(), temp_dir("dup"));
    let err = start(config, store, Hooks::default()).await.err().expect("an error");
    assert!(matches!(err, StartError::PortInUse(_)), "{err}");
    assert_eq!(core_http::PORT_IN_USE_EXIT, 3);
    first.shutdown().await;
}

#[tokio::test]
async fn it_refuses_to_bind_a_non_loopback_host() {
    let store = new_store(10);
    for host in ["0.0.0.0", "192.168.1.5", "example.com", "::"] {
        let mut config = Config::new(0, temp_dir("host"));
        config.host = host.into();
        let err = start(config, store.clone(), Hooks::default()).await.err().expect("an error");
        assert!(matches!(err, StartError::NotLoopback(_)), "{host}: {err}");
    }
}

#[tokio::test]
async fn a_silent_socket_is_terminated_by_the_heartbeat_and_a_live_one_is_kept() {
    let counts = Arc::new(Mutex::new(Vec::<usize>::new()));
    let sink = counts.clone();
    let closed = Arc::new(AtomicUsize::new(0));
    let closed_sink = closed.clone();
    let hooks = Hooks {
        on_client_count: Some(Arc::new(move |n| sink.lock().unwrap().push(n))),
        on_client_close: Some(Arc::new(move |_| {
            closed_sink.fetch_add(1, Ordering::SeqCst);
        })),
        ..Hooks::default()
    };
    let (server, _) = serve(dist_with_index("ok"), |c| c.heartbeat = Duration::from_millis(80), hooks).await;
    let port = server.port();
    let mut alive = ws_connect(port, &[]).await.unwrap();
    let silent = ws_connect(port, &[]).await.unwrap();
    wait_for(|| counts.lock().unwrap().contains(&2)).await;
    let keep = tokio::spawn(async move { while alive.next().await.is_some() {} });
    wait_for(|| closed.load(Ordering::SeqCst) == 1).await;
    assert_eq!(*counts.lock().unwrap().last().unwrap(), 1);
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(closed.load(Ordering::SeqCst), 1);
    drop(silent);
    keep.abort();
    server.shutdown().await;
}

#[tokio::test]
async fn a_message_over_128_kib_closes_the_socket_with_1009_and_never_reaches_the_hook() {
    let (hooks, got) = collecting();
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let mut ws = ws_connect(server.port(), &[]).await.unwrap();
    next_json(&mut ws).await;
    let big = json!({"type": "favorite.reorder", "dirs": ["a".repeat(200 * 1024)]}).to_string();
    let _ = ws.send(Message::text(big)).await;
    let mut code = None;
    while let Ok(Some(Ok(msg))) = tokio::time::timeout(Duration::from_secs(3), ws.next()).await {
        if let Message::Close(Some(frame)) = msg {
            code = Some(frame.code);
            break;
        }
    }
    assert_eq!(code, Some(CloseCode::Size));
    assert!(got.lock().unwrap().is_empty());

    let mut ok = ws_connect(server.port(), &[]).await.unwrap();
    next_json(&mut ok).await;
    let dirs: Vec<String> = (0..100).map(|_| "d".repeat(1000)).collect();
    send_json(&mut ok, json!({"type": "favorite.reorder", "dirs": dirs})).await;
    wait_for(|| got.lock().unwrap().len() == 1).await;
    server.shutdown().await;
}

#[tokio::test]
async fn snooze_links_need_a_terminal_a_whole_positive_number_and_the_hook_to_agree() {
    let seen = Arc::new(Mutex::new(Vec::<(String, Option<String>, f64)>::new()));
    let sink = seen.clone();
    let hooks = Hooks {
        on_snooze_link: Some(Arc::new(move |t, k, m| {
            sink.lock().unwrap().push((t.to_owned(), k.map(str::to_owned), m));
            t != "refused"
        })),
        ..Hooks::default()
    };
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let port = server.port();
    let ok = get(port, "/snooze?t=12%3A34&m=30&k=key+1", &[]).await;
    assert_eq!(ok.status, 204);
    assert_eq!(ok.header("cache-control"), Some("no-store"));
    assert_eq!(seen.lock().unwrap().as_slice(), [("12:34".to_owned(), Some("key 1".to_owned()), 30.0)]);
    assert_eq!(get(port, "/snooze?t=a&m=5", &[]).await.status, 204);
    assert_eq!(seen.lock().unwrap().last().unwrap(), &("a".to_owned(), None, 5.0));
    for bad in ["/snooze?m=5", "/snooze?t=&m=5", "/snooze?t=a", "/snooze?t=a&m=0", "/snooze?t=a&m=-5", "/snooze?t=a&m=1.5", "/snooze?t=a&m=abc", "/snooze?t=refused&m=5"] {
        assert_eq!(get(port, bad, &[]).await.status, 404, "{bad}");
    }
    assert_eq!(seen.lock().unwrap().len(), 3);
    let long = format!("/snooze?t={}&m=5", "x".repeat(257));
    assert_eq!(get(port, &long, &[]).await.status, 404);
    assert_eq!(seen.lock().unwrap().len(), 3);
    let other_method = http(port, "POST", "/snooze?t=a&m=5", &[], None).await.unwrap();
    assert_eq!(other_method.status, 200);
    server.shutdown().await;
}

#[tokio::test]
async fn focus_posts_run_the_hook_only_for_a_valid_body() {
    let calls = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let sink = calls.clone();
    let hooks = Hooks {
        on_focus_run: Some(Arc::new(move |t, k| {
            sink.lock().unwrap().push((t.clone(), k));
            Box::pin(async move { (t != "gone").then_some(FocusWindowResult::Ok) })
        })),
        ..Hooks::default()
    };
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let port = server.port();
    let ok = http(port, "POST", "/focus", &[("content-type", "application/json")], Some(r#"{"t":"term","k":"key"}"#)).await.unwrap();
    assert_eq!(ok.status, 200);
    assert_eq!(ok.header("content-type"), Some("application/json"));
    assert_eq!(ok.header("cache-control"), Some("no-store"));
    assert_eq!(ok.text(), r#"{"result":"ok"}"#);
    assert_eq!(calls.lock().unwrap().as_slice(), [("term".to_owned(), "key".to_owned())]);

    let rejected = r#"{"result":"rejected"}"#;
    for body in [r#"{"t":"gone","k":"key"}"#, r#"{"t":"term"}"#, r#"{"t":5,"k":"k"}"#, "not json", "[]", "", r#"{"k":"k"}"#] {
        let reply = http(port, "POST", "/focus", &[], Some(body)).await.unwrap();
        assert_eq!(reply.status, 404, "{body}");
        assert_eq!(reply.text(), rejected, "{body}");
    }
    let long = format!(r#"{{"t":"{}","k":"k"}}"#, "x".repeat(257));
    assert_eq!(http(port, "POST", "/focus", &[], Some(&long)).await.unwrap().status, 404);
    assert_eq!(calls.lock().unwrap().len(), 2);

    let huge = format!(r#"{{"t":"a","k":"{}"}}"#, "k".repeat(3000));
    assert!(http(port, "POST", "/focus", &[], Some(&huge)).await.is_none());
    assert_eq!(calls.lock().unwrap().len(), 2);
    server.shutdown().await;
}

#[tokio::test]
async fn client_context_reports_the_open_socket_count_and_an_id() {
    let seen = Arc::new(Mutex::new(Vec::<(usize, u64)>::new()));
    let sink = seen.clone();
    let hooks = Hooks { on_client_message: Some(Arc::new(move |_, ctx| sink.lock().unwrap().push((ctx.clients, ctx.client_id)))), ..Hooks::default() };
    let (server, _) = serve(dist_with_index("ok"), |_| {}, hooks).await;
    let port = server.port();
    let mut a = ws_connect(port, &[]).await.unwrap();
    let mut b = ws_connect(port, &[]).await.unwrap();
    next_json(&mut a).await;
    next_json(&mut b).await;
    send_json(&mut a, json!({"type": "history.get"})).await;
    send_json(&mut b, json!({"type": "history.get"})).await;
    wait_for(|| seen.lock().unwrap().len() == 2).await;
    let mut got = seen.lock().unwrap().clone();
    got.sort_by_key(|(_, id)| *id);
    assert_eq!(got, [(2, 1), (2, 2)]);
    server.shutdown().await;
}
