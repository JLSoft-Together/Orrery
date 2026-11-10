mod common;

use std::time::{Duration, Instant};

use common::*;
use core_http::Hooks;

#[tokio::test]
async fn a_long_store_lock_does_not_stall_health_or_the_runtime() {
    let (server, store) = serve(dist_with_index("ok"), |_| {}, Hooks::default()).await;
    let port = server.port();
    let mut ws = ws_connect(port, &[]).await.unwrap();
    assert_eq!(next_json(&mut ws).await["events"][0]["type"], "snapshot");
    let held = store.clone();
    let holder = std::thread::spawn(move || {
        held.with(|s| {
            s.set_loading(true);
            std::thread::sleep(Duration::from_millis(900));
        })
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    assert_eq!(get(port, "/health", &[]).await.status, 200);
    assert!(started.elapsed() < Duration::from_millis(500), "health waited {:?}", started.elapsed());
    let joining = tokio::spawn(async move {
        let mut late = ws_connect(port, &[]).await.unwrap();
        next_json(&mut late).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(get(port, "/health", &[]).await.status, 200);
    assert!(started.elapsed() < Duration::from_millis(700), "health waited {:?}", started.elapsed());
    tokio::task::spawn_blocking(move || holder.join().unwrap()).await.unwrap();
    let snapshot = joining.await.unwrap();
    assert_eq!(snapshot["events"][0]["payload"]["loading"], true);
    let next = next_json(&mut ws).await;
    assert_eq!(next["events"][0]["type"], "sessions.loading");
    assert_eq!(get(port, "/health", &[]).await.text(), r#"{"ok":true,"seq":1}"#);
}
