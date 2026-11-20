use std::net::IpAddr;
use std::path::PathBuf;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;

use super::install::{classify, file_head, lexical, newest_apk, resolve, serve, url_path, valid_url, Reply, Tickets, FILE, MIME, TTL_SECS};
use super::test_util::{rig, runtime, temp_dir};

const LAN: &str = "192.168.1.20";

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn apk(tag: &str, bytes: &[u8]) -> PathBuf {
    let path = temp_dir(tag).join("app-release.apk");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn get(target: &str) -> String {
    format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n")
}

#[test]
fn a_ticket_serves_its_file_until_it_expires() {
    let file = apk("inst-exp", b"PK-apk");
    let t = Tickets::default();
    let (token, exp) = t.issue(file.clone(), 1000).unwrap();
    assert_eq!(exp, 1000 + TTL_SECS);
    assert_eq!(token.len(), 22);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000 + TTL_SECS - 1), Reply::File { path: file.clone(), head: false });
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000 + TTL_SECS), Reply::NotFound);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000), Reply::NotFound);
}

#[test]
fn a_new_ticket_replaces_the_old_one() {
    let file = apk("inst-new", b"PK");
    let t = Tickets::default();
    let (old, _) = t.issue(file.clone(), 10).unwrap();
    let (new, _) = t.issue(file.clone(), 10).unwrap();
    assert_ne!(old, new);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&old)), 11), Reply::NotFound);
    assert!(matches!(resolve(&t, ip(LAN), &get(&url_path(&new)), 11), Reply::File { .. }));
    t.revoke();
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&new)), 11), Reply::NotFound);
}

#[test]
fn only_the_one_path_is_served() {
    let file = apk("inst-path", b"PK");
    let t = Tickets::default();
    let (token, _) = t.issue(file, 0).unwrap();
    for target in [
        format!("/install/{token}/other.apk"),
        format!("/install/{token}/orrery.apk?x=1"),
        format!("/install/{token}/../config.json"),
        format!("/install/{token}"),
        format!("/install/{token}/"),
        format!("/install/x/{token}/orrery.apk"),
        "/install//orrery.apk".to_owned(),
        "/remote".to_owned(),
        format!("/install/{token}%2F/orrery.apk"),
        format!("/INSTALL/{token}/orrery.apk"),
    ] {
        assert_eq!(resolve(&t, ip(LAN), &get(&target), 1), Reply::NotFound, "{target}");
    }
    assert_eq!(resolve(&t, ip(LAN), &format!("POST {} HTTP/1.1\r\n\r\n", url_path(&token)), 1), Reply::Method);
    assert!(matches!(resolve(&t, ip(LAN), &format!("HEAD {} HTTP/1.1\r\n\r\n", url_path(&token)), 1), Reply::File { head: true, .. }));
    assert_eq!(resolve(&t, ip(LAN), &format!("GET {} HTTP/2\r\n\r\n", url_path(&token)), 1), Reply::NotFound);
}

#[test]
fn a_wrong_token_is_not_found() {
    let file = apk("inst-wrong", b"PK");
    let t = Tickets::default();
    let (token, _) = t.issue(file, 0).unwrap();
    let mut wrong = token.clone().into_bytes();
    wrong[0] = if wrong[0] == b'A' { b'B' } else { b'A' };
    let wrong = String::from_utf8(wrong).unwrap();
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&wrong)), 1), Reply::NotFound);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token[..10])), 1), Reply::NotFound);
    assert_eq!(resolve(&Tickets::default(), ip(LAN), &get(&url_path(&token)), 1), Reply::NotFound);
}

#[test]
fn non_private_peers_are_refused() {
    let file = apk("inst-ip", b"PK");
    let t = Tickets::default();
    let (token, _) = t.issue(file, 0).unwrap();
    for peer in ["127.0.0.1", "8.8.8.8", "100.64.1.2", "172.32.0.1", "::1", "fe80::1"] {
        assert_eq!(resolve(&t, ip(peer), &get(&url_path(&token)), 1), Reply::Forbidden, "{peer}");
    }
    for peer in ["10.0.0.5", "172.16.3.4", "192.168.0.9", "::ffff:192.168.0.9"] {
        assert!(matches!(resolve(&t, ip(peer), &get(&url_path(&token)), 1), Reply::File { .. }), "{peer}");
    }
}

#[test]
fn a_missing_or_non_apk_file_is_not_served() {
    let dir = temp_dir("inst-kind");
    let txt = dir.join("notes.txt");
    std::fs::write(&txt, b"secret").unwrap();
    let t = Tickets::default();
    let (token, _) = t.issue(txt, 0).unwrap();
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1), Reply::NotFound);
    let (token, _) = t.issue(dir.join("gone.apk"), 0).unwrap();
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1), Reply::NotFound);
}

#[test]
fn sniffing_tells_install_requests_from_the_websocket() {
    assert_eq!(classify(b"GET /install/abc"), Some(true));
    assert_eq!(classify(b"HEAD /install/ab"), Some(true));
    assert_eq!(classify(b"GET /remote HTTP"), Some(false));
    assert_eq!(classify(b"GET /"), None);
    assert_eq!(classify(b"G"), None);
    assert_eq!(classify(b"POST /install/x"), Some(false));
}

#[test]
fn urls_must_be_https() {
    assert!(valid_url("https://drive.example.com/file/d/abc/view?usp=sharing"));
    assert!(!valid_url("http://example.com/a.apk"));
    assert!(!valid_url("https://"));
    assert!(!valid_url("https://user@evil.example/a.apk"));
    assert!(!valid_url("https://exa mple.com/a.apk"));
    assert!(!valid_url("javascript:alert(1)"));
    assert!(!valid_url(&format!("https://e.com/{}", "a".repeat(3000))));
}

#[test]
fn the_newest_apk_is_suggested() {
    let dir = temp_dir("inst-new-apk");
    std::fs::write(dir.join("old.apk"), b"1").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(dir.join("new.apk"), b"2").unwrap();
    std::fs::write(dir.join("later.txt"), b"3").unwrap();
    assert_eq!(newest_apk(&dir).unwrap().file_name().unwrap(), "new.apk");
    assert_eq!(newest_apk(&dir.join("none")), None);
    let p = lexical(&dir.join("a").join("..").join("new.apk"));
    assert_eq!(p, dir.join("new.apk"));
}

async fn exchange(t: &Tickets, peer: &str, request: String) -> Vec<u8> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::spawn(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(request.as_bytes()).await.unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).await.unwrap();
        out
    });
    let (stream, _) = listener.accept().await.unwrap();
    serve(t, stream, ip(peer), Instant::now() + std::time::Duration::from_secs(5)).await;
    client.await.unwrap()
}

#[test]
fn serve_streams_the_apk_with_its_headers() {
    let body = b"PK\x03\x04 fake apk body";
    let file = apk("inst-serve", body);
    let t = Tickets::default();
    let (token, _) = t.issue(file, super::config::now_secs()).unwrap();
    runtime().block_on(async {
        let out = exchange(&t, LAN, get(&url_path(&token))).await;
        let text = String::from_utf8_lossy(&out);
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"), "{text}");
        assert!(text.contains(&format!("Content-Type: {MIME}\r\n")));
        assert!(text.contains(&format!("Content-Disposition: attachment; filename=\"{FILE}\"\r\n")));
        assert!(text.contains(&format!("Content-Length: {}\r\n", body.len())));
        assert!(out.ends_with(body));
        assert!(file_head(1).ends_with("\r\n\r\n"));

        let out = exchange(&t, "8.8.8.8", get(&url_path(&token))).await;
        assert!(String::from_utf8_lossy(&out).starts_with("HTTP/1.1 403"));
        let out = exchange(&t, LAN, get("/install/nope/orrery.apk")).await;
        assert!(String::from_utf8_lossy(&out).starts_with("HTTP/1.1 404"));
    });
}

#[test]
fn the_lan_listener_routes_install_requests_and_refuses_loopback() {
    let r = rig("inst-lan", "", false);
    let file = apk("inst-lan-apk", b"PK");
    r.remote.enable(true);
    assert!(r.remote.set_apk(&file));
    let started = r.remote.install_call(&json!({ "op": "start" }));
    assert_eq!(started["ok"], true, "{started}");
    let path = started["path"].as_str().unwrap().to_owned();
    assert!(path.starts_with("/install/") && path.ends_with("/orrery.apk"));
    assert_eq!(started["port"].as_u64().unwrap(), u64::from(r.addr().port()));
    let addr = r.addr();
    let out = runtime().block_on(async move {
        let mut s = TcpStream::connect(addr).await.unwrap();
        s.write_all(get(&path).as_bytes()).await.unwrap();
        let mut out = Vec::new();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), s.read_to_end(&mut out)).await;
        out
    });
    assert!(String::from_utf8_lossy(&out).starts_with("HTTP/1.1 403"), "{}", String::from_utf8_lossy(&out));
    let state = r.remote.install_call(&json!({ "op": "get" }));
    assert_eq!(state["apk"]["name"], "app-release.apk");
    assert_eq!(r.remote.install_call(&json!({ "op": "url", "url": "http://x.com/a.apk" }))["error"], "url");
    assert_eq!(r.remote.install_call(&json!({ "op": "url", "url": "https://x.com/a.apk" }))["url"], "https://x.com/a.apk");
    r.remote.enable(false);
    assert_eq!(r.remote.install_call(&json!({ "op": "start" }))["error"], "off");
}
