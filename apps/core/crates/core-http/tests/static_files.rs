mod common;

use common::*;
use core_http::Hooks;

#[tokio::test]
async fn static_server_answers_a_malformed_path_with_400_and_refuses_a_cross_site_origin() {
    let (server, _) = serve(dist_with_index("<!doctype html>ok"), |_| {}, Hooks::default()).await;
    let port = server.port();
    assert_eq!(get(port, "/%E0%A4%A", &[]).await.status, 400);
    assert_eq!(get(port, "/", &[]).await.status, 200);
    assert_eq!(get(port, "/", &[("Origin", "http://localhost:9999")]).await.status, 403);
    assert_eq!(get(port, "/", &[("Host", "evil.example")]).await.status, 403);
    assert_eq!(get(port, "/", &[("Origin", &format!("http://127.0.0.1:{port}"))]).await.status, 200);
    server.shutdown().await;
}

#[tokio::test]
async fn static_paths_never_leave_the_dist_folder() {
    let dist = dist_with_index("<!doctype html>index");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(dist.join("assets").join("index-ABCDEFGH.js"), "console.log(1)").unwrap();
    std::fs::write(dist.join("assets").join("plain.js"), "x").unwrap();
    std::fs::write(dist.parent().unwrap().join("outside-secret.txt"), "secret").unwrap();
    let (server, _) = serve(dist, |_| {}, Hooks::default()).await;
    let port = server.port();
    assert_eq!(get(port, "/..%2foutside-secret.txt", &[]).await.status, 403);
    assert_eq!(get(port, "/%2e%2e%2foutside-secret.txt", &[]).await.status, 403);
    let collapsed = get(port, "/../outside-secret.txt", &[]).await;
    assert_eq!(collapsed.status, 200);
    assert!(!collapsed.text().contains("secret"));
    let encoded = get(port, "/assets/..%5c..%5coutside-secret.txt", &[]).await;
    assert!(!encoded.text().contains("secret"));
    assert_eq!(get(port, "/a%00b", &[]).await.status, 400);
    server.shutdown().await;
}

#[tokio::test]
async fn static_files_carry_content_type_cache_control_nosniff_and_csp() {
    let dist = dist_with_index("<html><script>boot()</script><script src=\"/assets/index-ABCDEFGH.js\"></script></html>");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(dist.join("assets").join("index-ABCDEFGH.js"), "console.log(1)").unwrap();
    std::fs::write(dist.join("assets").join("plain.css"), "a{}").unwrap();
    std::fs::write(dist.join("sw.js"), "self").unwrap();
    let (server, _) = serve(dist, |_| {}, Hooks::default()).await;
    let port = server.port();

    let html = get(port, "/", &[]).await;
    assert_eq!(html.status, 200);
    assert_eq!(html.header("content-type"), Some("text/html; charset=utf-8"));
    assert_eq!(html.header("cache-control"), Some("no-cache"));
    assert_eq!(html.header("x-content-type-options"), Some("nosniff"));
    let csp = html.header("content-security-policy").unwrap();
    assert!(csp.contains("script-src 'self' 'sha256-"), "{csp}");
    assert!(csp.contains("frame-ancestors 'none'"));

    let hashed = get(port, "/assets/index-ABCDEFGH.js", &[]).await;
    assert_eq!(hashed.header("content-type"), Some("text/javascript; charset=utf-8"));
    assert_eq!(hashed.header("cache-control"), Some("public, max-age=31536000, immutable"));
    assert_eq!(hashed.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(hashed.text(), "console.log(1)");
    assert!(hashed.header("content-security-policy").is_none());

    assert_eq!(get(port, "/assets/plain.css", &[]).await.header("cache-control"), Some("no-cache"));
    assert_eq!(get(port, "/sw.js", &[]).await.header("cache-control"), Some("no-cache"));
    let fallback = get(port, "/some/spa/route?x=1", &[]).await;
    assert_eq!(fallback.status, 200);
    assert!(fallback.text().contains("boot()"));
    let head = http(port, "HEAD", "/", &[], None).await.unwrap();
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());
    server.shutdown().await;
}

#[tokio::test]
async fn an_unbuilt_web_dist_answers_404() {
    let (server, _) = serve(temp_dir("empty"), |_| {}, Hooks::default()).await;
    let reply = get(server.port(), "/", &[]).await;
    assert_eq!(reply.status, 404);
    assert_eq!(reply.text(), "Web UI not built. Run: npm run build");
    server.shutdown().await;
}
