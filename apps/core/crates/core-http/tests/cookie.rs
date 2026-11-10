mod common;

use common::*;
use core_http::Hooks;

#[tokio::test]
async fn the_key_in_the_url_becomes_a_cookie_that_opens_pages_and_the_websocket() {
    let (server, _) = serve(dist_with_index("ok"), |c| c.app_key = Some("s3cret".into()), Hooks::default()).await;
    let port = server.port();

    let login = get(port, "/?k=s3cret", &[]).await;
    assert_eq!(login.status, 302);
    assert_eq!(login.header("location"), Some("/"));
    assert_eq!(login.header("set-cookie"), Some("ccm_app=s3cret; HttpOnly; SameSite=Strict; Path=/"));
    assert_eq!(login.header("cache-control"), Some("no-store"));

    let wrong = get(port, "/?k=nope", &[]).await;
    assert_eq!(wrong.status, 403);
    assert!(wrong.header("set-cookie").is_none());
    assert_eq!(get(port, "/index.html?k=s3cret", &[]).await.status, 403);
    assert_eq!(http(port, "POST", "/?k=s3cret", &[], None).await.unwrap().status, 403);

    assert_eq!(get(port, "/", &[("Cookie", "theme=dark; ccm_app=s3cret")]).await.status, 200);
    assert_eq!(get(port, "/", &[("Cookie", "ccm_app=s3cre")]).await.status, 403);
    assert_eq!(get(port, "/snooze?t=a&m=5", &[("Cookie", "ccm_app=s3cret")]).await.status, 404);
    assert_eq!(get(port, "/", &[("x-ccm-app", "s3cret")]).await.status, 200);

    assert!(ws_connect(port, &[("cookie", "ccm_app=wrong!")]).await.is_err());
    let mut ws = ws_connect(port, &[("cookie", "ccm_app=s3cret")]).await.unwrap();
    assert_eq!(next_json(&mut ws).await["events"][0]["type"], "snapshot");
    server.shutdown().await;
}

#[tokio::test]
async fn the_compact_window_keeps_its_query_after_the_redirect() {
    let (server, _) = serve(dist_with_index("ok"), |c| c.app_key = Some("abc".into()), Hooks::default()).await;
    let r = get(server.port(), "/?mode=compact&k=abc", &[]).await;
    assert_eq!(r.status, 302);
    assert_eq!(r.header("location"), Some("/?mode=compact"));
    server.shutdown().await;
}
