use std::net::IpAddr;
use std::path::PathBuf;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;

use super::install::{classify, file_head, lexical, newest_apk, pick_source, resolve, save_name, serve, url_path, valid_url, Reply, Source, TicketKind, Tickets, FILE, MAX_SLOTS, MIME, TTL_SECS};
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
fn a_symlink_in_place_of_the_apk_is_never_served() {
    let file = apk("inst-link", b"PK-apk");
    assert!(super::install_http::servable(&file));
    assert!(!super::install_http::servable(&file.with_file_name("missing.apk")));
    let link = file.with_file_name("link.apk");
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&file, &link).is_ok();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&file, &link).is_ok();
    if made {
        assert!(!super::install_http::servable(&link));
    }
}

#[test]
fn a_ticket_serves_its_file_until_it_expires() {
    let file = apk("inst-exp", b"PK-apk");
    let t = Tickets::default();
    let (token, exp) = t.issue(TicketKind::ApkPage, file.clone(), 1000).unwrap();
    assert_eq!(exp, 1000 + TTL_SECS);
    assert_eq!(token.len(), 22);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000 + TTL_SECS - 1), Reply::File { path: file.clone(), head: false, first: true });
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000 + TTL_SECS), Reply::NotFound);
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1000), Reply::NotFound);
}

#[test]
fn tickets_live_side_by_side_and_expire_on_their_own() {
    let file = apk("inst-new", b"PK");
    let t = Tickets::default();
    let (first, _) = t.issue(TicketKind::ApkPage, file.clone(), 10).unwrap();
    let (second, exp) = t.issue(TicketKind::ApkOffer, file.clone(), 300).unwrap();
    assert_ne!(first, second);
    assert_eq!(exp, 300 + TicketKind::ApkOffer.ttl());
    assert_eq!(t.len(), 2);
    assert!(matches!(resolve(&t, ip(LAN), &get(&url_path(&first)), 305), Reply::File { .. }));
    assert!(matches!(resolve(&t, ip(LAN), &get(&url_path(&second)), 305), Reply::File { .. }));
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&first)), 10 + TTL_SECS), Reply::NotFound);
    assert!(matches!(resolve(&t, ip(LAN), &get(&url_path(&second)), 10 + TTL_SECS), Reply::File { .. }));
    assert_eq!(t.len(), 1);
    t.revoke();
    assert!(t.is_empty());
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&second)), 11), Reply::NotFound);
}

#[test]
fn a_full_set_of_slots_drops_the_oldest_of_the_same_kind() {
    let file = apk("inst-slots", b"PK");
    let t = Tickets::default();
    let offer = t.issue(TicketKind::ApkOffer, file.clone(), 0).unwrap().0;
    let pages: Vec<String> = (0..MAX_SLOTS - 1).map(|_| t.issue(TicketKind::ApkPage, file.clone(), 0).unwrap().0).collect();
    assert_eq!(t.len(), MAX_SLOTS);
    let newest = t.issue(TicketKind::ApkPage, file.clone(), 0).unwrap().0;
    assert_eq!(t.len(), MAX_SLOTS);
    let live = |token: &str| matches!(resolve(&t, ip(LAN), &get(&url_path(token)), 1), Reply::File { .. });
    assert!(!live(&pages[0]));
    assert!(live(&pages[1]) && live(&pages[2]) && live(&newest) && live(&offer));
    let other = t.issue(TicketKind::ApkOffer, file, 0).unwrap().0;
    assert_eq!(t.len(), MAX_SLOTS);
    assert!(!live(&offer) && live(&other) && live(&pages[1]));
}

#[test]
fn only_the_one_path_is_served() {
    let file = apk("inst-path", b"PK");
    let t = Tickets::default();
    let (token, _) = t.issue(TicketKind::ApkOffer, file, 0).unwrap();
    for target in [
        format!("/install/{token}/other.apk"),
        format!("/install/{token}/orrery.apk?x=1"),
        format!("/install/{token}/../config.json"),
        format!("/install/{token}"),
        format!("/install/{token}/"),
        format!("/install/{token}/pair"),
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
    let (token, _) = t.issue(TicketKind::ApkPage, file, 0).unwrap();
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
    let (token, _) = t.issue(TicketKind::ApkPage, file, 0).unwrap();
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
    let (token, _) = t.issue(TicketKind::ApkPage, txt, 0).unwrap();
    assert_eq!(resolve(&t, ip(LAN), &get(&url_path(&token)), 1), Reply::NotFound);
    let (token, _) = t.issue(TicketKind::ApkPage, dir.join("gone.apk"), 0).unwrap();
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
    let (token, _) = t.issue(TicketKind::ApkPage, file, super::config::now_secs()).unwrap();
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

fn named(tag: &str, name: &str) -> PathBuf {
    let path = temp_dir(tag).join(name);
    std::fs::write(&path, b"PK").unwrap();
    path
}

#[test]
fn the_apk_source_order_is_custom_then_bundled_then_dev() {
    let custom = named("src-custom", "mine.apk");
    let bundled = named("src-bundled", "orrery-remote.apk");
    let dev = named("src-dev", "app-release.apk");
    let b = || Some((bundled.clone(), "4.0.10".to_owned()));
    let found = pick_source(&custom.to_string_lossy(), b(), Some(dev.clone())).unwrap();
    assert_eq!((found.path.clone(), found.source, found.version.as_str()), (custom.clone(), Source::Custom, ""));
    let found = pick_source("", b(), Some(dev.clone())).unwrap();
    assert_eq!((found.path.clone(), found.source, found.version.as_str()), (bundled.clone(), Source::Bundled, "4.0.10"));
    let found = pick_source("/gone/x.apk", b(), Some(dev.clone())).unwrap();
    assert_eq!(found.source, Source::Bundled);
    let found = pick_source("", Some((temp_dir("src-none").join("no.apk"), "1".into())), Some(dev.clone())).unwrap();
    assert_eq!((found.path, found.source), (dev, Source::Dev));
    assert_eq!(pick_source("", None, None), None);
    assert_eq!(Source::Custom.name(), "custom");
    assert_eq!(Source::Bundled.name(), "bundled");
    assert_eq!(Source::Dev.name(), "dev");
}

#[test]
fn get_and_start_share_the_same_apk_source() {
    let r = rig("inst-src", "", false);
    let bundled = named("inst-src-b", "orrery-remote.apk");
    *crate::state::lock(&r.host.bundled) = Some((bundled.clone(), "4.0.10".to_owned()));
    r.remote.enable(true);
    let state = r.remote.install_call(&json!({ "op": "get" }));
    assert_eq!(state["source"], "bundled", "{state}");
    assert_eq!(state["version"], "4.0.10");
    assert_eq!(state["apk"]["path"], bundled.to_string_lossy().as_ref());
    let started = r.remote.install_call(&json!({ "op": "start" }));
    assert_eq!(started["ok"], true, "{started}");
    let custom = named("inst-src-c", "mine.apk");
    assert!(r.remote.set_apk(&custom));
    let state = r.remote.install_call(&json!({ "op": "get" }));
    assert_eq!((state["source"].as_str(), state["version"].as_str()), (Some("custom"), Some("")));
    assert_eq!(state["apk"]["name"], "mine.apk");
    assert_eq!(r.remote.install_call(&json!({ "op": "start" }))["ok"], true);
    r.remote.install_call(&json!({ "op": "clear" }));
    assert_eq!(r.remote.install_call(&json!({ "op": "get" }))["source"], "bundled");
}

#[test]
fn save_copies_the_current_apk_or_reports_why_not() {
    let r = rig("inst-save", "", false);
    let bundled = named("inst-save-b", "orrery-remote.apk");
    *crate::state::lock(&r.host.bundled) = Some((bundled, "4.0.10".to_owned()));
    let dest = temp_dir("inst-save-d").join("out.apk");
    assert_eq!(r.remote.install_call(&json!({ "op": "save" }))["error"], "cancel");
    *crate::state::lock(&r.host.save_to) = Some(dest.clone());
    let saved = r.remote.install_call(&json!({ "op": "save" }));
    assert_eq!(saved["ok"], true, "{saved}");
    assert_eq!(saved["path"], dest.to_string_lossy().as_ref());
    assert_eq!(std::fs::read(&dest).unwrap(), b"PK");
    *crate::state::lock(&r.host.save_to) = Some(temp_dir("inst-save-x").join("missing").join("out.apk"));
    assert_eq!(r.remote.install_call(&json!({ "op": "save" })), json!({ "ok": false, "error": "save" }));
    *crate::state::lock(&r.host.bundled) = None;
    let fresh = rig("inst-save-none", "", false);
    let none = fresh.remote.install_call(&json!({ "op": "save" }));
    assert!(none["ok"] == false && (none["error"] == "apk" || none["error"] == "cancel"), "{none}");
}

#[test]
fn save_names_carry_a_clean_version() {
    assert_eq!(save_name("4.0.10"), "orrery-remote-4.0.10.apk");
    assert_eq!(save_name(""), "orrery-remote.apk");
    assert_eq!(save_name("../x y"), "orrery-remote-..xy.apk");
}
