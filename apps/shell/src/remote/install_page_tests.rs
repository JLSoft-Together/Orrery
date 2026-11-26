use std::net::IpAddr;
use std::path::PathBuf;

use core_remote_proto::pair::QrPayload;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;

use super::config::now_secs;
use super::install::{page_path, resolve, serve, url_path, Reply, TicketKind, Tickets, TTL_SECS};
use super::install_page::{is_vi, pair_url, pct, render, PAIR_SCHEME};
use super::install_tickets::Deny;
use super::pair::PairMeta;
use super::test_util::{pair, rig, runtime, temp_dir, Phone};
use crate::state::lock;

const A: &str = "192.168.1.20";
const B: &str = "192.168.1.21";

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn apk(tag: &str) -> PathBuf {
    let path = temp_dir(tag).join("app-release.apk");
    std::fs::write(&path, vec![7u8; 3 * 1024 * 1024]).unwrap();
    path
}

fn get(target: &str, lang: Option<&str>) -> String {
    let lang = lang.map(|l| format!("Accept-Language: {l}\r\n")).unwrap_or_default();
    format!("GET {target} HTTP/1.1\r\nHost: x\r\n{lang}\r\n")
}

fn pair_path(token: &str) -> String {
    format!("{}pair", page_path(token))
}

fn html(reply: Reply) -> String {
    match reply {
        Reply::Html { body, head: false } => body,
        other => panic!("{other:?}"),
    }
}

fn unpct(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

fn qr_of(location: &str) -> QrPayload {
    QrPayload::decode(&unpct(location.strip_prefix(PAIR_SCHEME).unwrap())).unwrap()
}

#[test]
fn the_page_speaks_the_browser_language_and_names_no_ticket() {
    let t = Tickets::default();
    let (token, _) = t.issue_with(TicketKind::ApkPage, apk("page-lang"), "4.0.10".into(), 0).unwrap();
    let (other, _) = t.issue(TicketKind::ApkOffer, apk("page-other"), 0).unwrap();
    let vi = html(resolve(&t, ip(A), &get(&page_path(&token), Some("vi-VN,vi;q=0.9,en;q=0.8")), 1));
    assert!(vi.contains("<html lang=\"vi\">") && vi.contains("Tải APK") && vi.contains("Mở và ghép đôi"), "{vi}");
    assert!(vi.contains("Phiên bản 4.0.10, Dung lượng 3.0 MB"), "{vi}");
    assert!(vi.contains("href=\"orrery.apk\"") && vi.contains("href=\"pair\""));
    let en = html(resolve(&t, ip(A), &get(&page_path(&token), None), 1));
    assert!(en.contains("<html lang=\"en\">") && en.contains("Download APK") && en.contains("Open and pair"), "{en}");
    for body in [&vi, &en] {
        assert!(!body.contains(&token) && !body.contains(&other));
        assert!(!body.contains("<script") && !body.contains("http://") && !body.contains("https://"));
    }
    assert_eq!(resolve(&t, ip(A), &get(&page_path(&other), None), 1), Reply::NotFound);
    assert_eq!(resolve(&t, ip("8.8.8.8"), &get(&page_path(&token), None), 1), Reply::Forbidden);
    assert!(matches!(resolve(&t, ip(A), &format!("HEAD {} HTTP/1.1\r\n\r\n", page_path(&token)), 1), Reply::Html { head: true, .. }));
    assert_eq!(resolve(&t, ip(A), &format!("HEAD {} HTTP/1.1\r\n\r\n", pair_path(&token)), 1), Reply::Method);
    let odd = render(false, "<b>\"1\"</b>", 0);
    assert!(odd.contains("Version b1b, Size 0.0 MB"), "{odd}");
}

#[test]
fn the_language_comes_from_the_first_accept_language_tag() {
    assert!(is_vi("GET / HTTP/1.1\r\naccept-language: vi\r\n\r\n"));
    assert!(is_vi("GET / HTTP/1.1\r\nAccept-Language:  VI-vn;q=1\r\n\r\n"));
    assert!(!is_vi("GET / HTTP/1.1\r\nAccept-Language: en-US,vi;q=0.9\r\n\r\n"));
    assert!(!is_vi("GET / HTTP/1.1\r\n\r\n"));
    assert!(!is_vi("GET /vi HTTP/1.1\r\nX-Accept-Language: vi\r\n\r\n"));
}

#[test]
fn the_page_is_served_with_strict_headers() {
    let t = Tickets::default();
    let (token, _) = t.issue(TicketKind::ApkPage, apk("page-head"), now_secs()).unwrap();
    let out = runtime().block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let request = get(&page_path(&token), Some("vi"));
        let client = tokio::spawn(async move {
            let mut s = TcpStream::connect(addr).await.unwrap();
            s.write_all(request.as_bytes()).await.unwrap();
            let mut out = Vec::new();
            s.read_to_end(&mut out).await.unwrap();
            out
        });
        let (stream, _) = listener.accept().await.unwrap();
        serve(&t, stream, ip(A), Instant::now() + std::time::Duration::from_secs(5)).await;
        String::from_utf8(client.await.unwrap()).unwrap()
    });
    assert!(out.starts_with("HTTP/1.1 200 OK\r\n"), "{out}");
    for header in [
        "Content-Type: text/html; charset=utf-8\r\n",
        "Cache-Control: no-store\r\n",
        "Referrer-Policy: no-referrer\r\n",
        "Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'",
        "X-Content-Type-Options: nosniff\r\n",
    ] {
        assert!(out.contains(header), "{header} in {out}");
    }
    assert!(out.ends_with("</html>"));
}

#[test]
fn pair_links_are_percent_encoded() {
    assert_eq!(pct("a-b_c.d~e"), "a-b_c.d~e");
    assert_eq!(pct("x y/+=?&"), "x%20y%2F%2B%3D%3F%26");
    assert_eq!(pair_url("q=1"), "orrery-remote://pair?q=q%3D1");
    assert_eq!(unpct(&pct("ORR1:ab+/=é")), "ORR1:ab+/=é");
}

#[test]
fn a_pair_claim_needs_arm_the_fetching_ip_and_one_use() {
    let t = Tickets::default();
    let (token, _) = t.issue(TicketKind::ApkPage, apk("claim"), 100).unwrap();
    let (offer, _) = t.issue(TicketKind::ApkOffer, apk("claim-o"), 100).unwrap();
    assert_eq!(t.claim_pair(&token, ip(A), 101), Err(Deny::Unarmed));
    t.arm(true, 100);
    assert_eq!(t.claim_pair(&token, ip(A), 101), Err(Deny::NotFetched));
    assert!(matches!(resolve(&t, ip(A), &get(&url_path(&token), None), 101), Reply::File { first: true, .. }));
    assert!(matches!(resolve(&t, ip(B), &get(&url_path(&token), None), 102), Reply::File { first: false, .. }));
    assert!(matches!(resolve(&t, ip(A), &format!("HEAD {} HTTP/1.1\r\n\r\n", url_path(&offer)), 102), Reply::File { first: false, .. }));
    assert_eq!(t.claim_pair(&token, ip(B), 103), Err(Deny::OtherIp));
    assert_eq!(t.claim_pair(&offer, ip(A), 103), Err(Deny::Missing));
    assert_eq!(t.claim_pair("nope", ip(A), 103), Err(Deny::Missing));
    assert_eq!(t.claim_pair(&token, "::ffff:192.168.1.20".parse().unwrap(), 103), Ok(()));
    assert_eq!(t.claim_pair(&token, ip(A), 104), Err(Deny::Used));
    assert_eq!(t.fetched_json()["ip"], A);
    t.arm(true, 200);
    assert!(t.armed(200 + TTL_SECS - 1) && !t.armed(200 + TTL_SECS));
    t.arm(false, 201);
    assert!(!t.armed(202));
    t.arm(true, 300);
    t.revoke();
    assert!(!t.armed(301) && t.fetched_json().is_null());
}

fn log_whys(r: &super::test_util::Rig) -> Vec<Value> {
    r.remote.guard_call(&json!({}))["state"]["log"].as_array().cloned().unwrap_or_default().into_iter().map(|e| e["why"].clone()).collect()
}

#[test]
fn the_pair_route_only_mints_a_code_for_the_ip_that_fetched_and_only_once() {
    let r = rig("inst-pair", "", false);
    r.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    r.remote.enable(true);
    assert!(r.remote.set_apk(&apk("inst-pair-apk")));
    let started = r.remote.install_call(&json!({ "op": "start" }));
    let token = started["path"].as_str().unwrap().trim_start_matches("/install/").trim_end_matches("/orrery.apk").to_owned();
    assert_eq!(started["page"], page_path(&token));
    let ctx = r.remote.ctx().unwrap();
    let now = now_secs();
    let qr = super::test_util::decode_qr(&r.remote.pair_start().unwrap());
    let go = |peer: &str, target: &str| ctx.install_reply(resolve(&ctx.install, ip(peer), &get(target, None), now), ip(peer));

    assert_eq!(go(A, &pair_path(&token)), Reply::Forbidden);
    assert!(matches!(go(A, &url_path(&token)), Reply::File { first: true, .. }));
    let events = lock(&r.host.events).clone();
    assert_eq!(events, vec![("ccm:remote-install-fetched".to_owned(), json!({ "ip": A }))]);
    assert_eq!(r.remote.status()["installFetched"]["ip"], A);
    assert!(matches!(go(A, &url_path(&token)), Reply::File { first: false, .. }));
    assert_eq!(lock(&r.host.events).len(), 1);
    assert_eq!(go(A, &pair_path(&token)), Reply::Forbidden);
    assert_eq!(r.remote.install_call(&json!({ "op": "arm" }))["ok"], true);
    assert_eq!(go(B, &pair_path(&token)), Reply::Forbidden);
    assert_eq!(go("10.9.9.9", &pair_path(&token)), Reply::Forbidden);
    let whys = log_whys(&r);
    assert!(whys.contains(&json!("install_unarmed")) && whys.contains(&json!("install_ip")), "{whys:?}");

    let first = runtime().block_on(pair(r.addr(), &qr, &Phone::new("p-qr")));
    assert_eq!(first.unwrap()["t"], "pair.ok", "a refused /pair must not replace the running pairing slot");

    let Reply::Redirect(location) = go(A, &pair_path(&token)) else { panic!("no redirect") };
    let bound = qr_of(&location);
    assert_eq!(go(A, &pair_path(&token)), Reply::Forbidden);
    assert!(log_whys(&r).contains(&json!("install_used")));
    let refused = runtime().block_on(pair(r.addr(), &bound, &Phone::new("p-other-ip")));
    assert_eq!(refused.err(), Some(Some(4401)));
    assert!(log_whys(&r).contains(&json!("pair_ip")));
    assert_eq!(lock(&r.host.asked).len(), 1);
    r.remote.enable(false);
}

#[test]
fn a_code_bound_to_the_phone_ip_pairs_from_that_ip() {
    let r = rig("inst-bound", "", false);
    r.host.answer.store(true, std::sync::atomic::Ordering::SeqCst);
    r.remote.enable(true);
    let ctx = r.remote.ctx().unwrap();
    let started = ctx.start_pair(PairMeta::lan(ip("127.0.0.1"))).unwrap();
    assert_eq!(started["via"], "lan");
    assert_eq!(started["relay"], false);
    let qr = super::test_util::decode_qr(&started);
    let reply = runtime().block_on(pair(r.addr(), &qr, &Phone::new("p-lan"))).unwrap();
    assert_eq!(reply["t"], "pair.ok");
    let asks = lock(&r.host.pair_asks).clone();
    assert_eq!(asks[0].source, "lan");
    let dev = &r.remote.status()["devices"][0];
    assert_eq!(dev["pairedVia"], "lan");
    assert!(dev.get("adbSerialHash").is_none());
    r.remote.enable(false);
}
