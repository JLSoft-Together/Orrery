use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

use serde_json::{json, Value};

use super::super::super::config::now_secs;
use super::super::super::devices::Device;
use super::super::super::fcm_tests::Fake;
use super::super::super::install::{resolve, Reply, TicketKind, MAX_SLOTS};
use super::super::super::push_seal::sealed_plain;
use super::super::super::push_tests::{ready, TOKEN_A, TOKEN_B};
use super::super::super::push_web::web_payload;
use super::super::super::test_util::*;
use super::super::{message, title_body, PER_HOUR_INFO};
use super::{in_build, sha256_hex, urls_of, vet};
use crate::state::lock;

struct Tree {
    root: PathBuf,
    apk: PathBuf,
}

fn tree(tag: &str) -> Tree {
    let root = temp_dir(tag);
    let out = root.join("app").join("build").join("outputs").join("apk").join("debug");
    std::fs::create_dir_all(&out).unwrap();
    let apk = out.join("app-debug.apk");
    std::fs::write(&apk, b"PK\x03\x04 apk bytes").unwrap();
    Tree { root, apk }
}

fn session(t: &Tree) -> Value {
    json!({ "id": "run-1", "cwd": t.root.to_string_lossy(), "state": "running", "apk": { "path": t.apk.to_string_lossy(), "size": 14, "applicationId": "com.ex.app", "variant": "debug" } })
}

fn add(rig: &Rig, id: &str, token: &str, seal: bool) {
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    assert!(devices.set_push(id, token, "en"));
    devices.set_push_seal(id, seal, seal, now_secs());
}

fn lan() -> Vec<SocketAddr> {
    vec!["192.168.1.20:4321".parse().unwrap(), "10.0.0.5:4321".parse().unwrap(), "8.8.8.8:4321".parse().unwrap()]
}

fn serving(rig: &Rig, sessions: Value) -> std::sync::Arc<super::super::super::ctx::Ctx> {
    let ctx = ready(rig, Fake::ok());
    ctx.set_lan(lan());
    *lock(&rig.host.viewer) = Some(Box::new(move |_, channel, _, _| {
        assert_eq!(channel, "ccm:run-sessions");
        json!({ "raw": sessions })
    }));
    ctx
}

#[test]
fn a_session_without_apk_or_with_a_path_outside_the_build_dir_is_refused() {
    let t = tree("offer-vet");
    assert_eq!(vet(&session(&t)).unwrap().app, "com.ex.app");
    let mut none = session(&t);
    none.as_object_mut().unwrap().remove("apk");
    assert_eq!(vet(&none), Err("noApk"));
    let outside = temp_dir("offer-outside");
    let stray = outside.join("app-debug.apk");
    std::fs::write(&stray, b"PK stray").unwrap();
    let mut s = session(&t);
    s["apk"]["path"] = json!(stray.to_string_lossy());
    assert_eq!(vet(&s), Err("outside"));
    let in_root_not_build = t.root.join("loose.apk");
    std::fs::write(&in_root_not_build, b"PK loose").unwrap();
    s["apk"]["path"] = json!(in_root_not_build.to_string_lossy());
    assert_eq!(vet(&s), Err("outside"));
    let mut dots = session(&t);
    dots["apk"]["path"] = json!(t.root.join("app").join("build").join("outputs").join("..").join("..").join("..").join("loose.apk").to_string_lossy());
    assert_eq!(vet(&dots), Err("outside"));
}

#[test]
fn a_non_apk_an_empty_file_and_a_file_deleted_after_the_build_are_refused() {
    let t = tree("offer-kind");
    let aab = t.apk.with_file_name("app-debug.aab");
    std::fs::write(&aab, b"PK aab").unwrap();
    let mut s = session(&t);
    s["apk"]["path"] = json!(aab.to_string_lossy());
    assert_eq!(vet(&s), Err("notApk"));
    let empty = t.apk.with_file_name("empty.apk");
    std::fs::write(&empty, b"").unwrap();
    s["apk"]["path"] = json!(empty.to_string_lossy());
    assert_eq!(vet(&s), Err("notApk"));
    std::fs::remove_file(&t.apk).unwrap();
    assert_eq!(vet(&session(&t)), Err("gone"));
    let mut bad = session(&t);
    bad["apk"]["path"] = json!("");
    assert_eq!(vet(&bad), Err("noApk"));
    bad["apk"]["path"] = json!(5);
    assert_eq!(vet(&bad), Err("noApk"));
}

#[cfg(unix)]
#[test]
fn a_symlink_that_leaves_the_build_dir_is_refused() {
    let t = tree("offer-link");
    let outside = temp_dir("offer-link-out");
    let real = outside.join("real.apk");
    std::fs::write(&real, b"PK outside").unwrap();
    let link = t.apk.with_file_name("link.apk");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut s = session(&t);
    s["apk"]["path"] = json!(link.to_string_lossy());
    assert_eq!(vet(&s), Err("outside"));
}

#[test]
fn in_build_needs_build_then_outputs_inside_the_project_root() {
    let root = Path::new("/p");
    assert!(in_build(root, Path::new("/p/app/build/outputs/apk/debug/a.apk")));
    assert!(in_build(root, Path::new("/p/build/outputs/a.apk")));
    assert!(!in_build(root, Path::new("/p/app/outputs/build/a.apk")));
    assert!(!in_build(root, Path::new("/q/app/build/outputs/a.apk")));
    assert!(!in_build(root, Path::new("/p/app/build/a.apk")));
}

#[test]
fn the_digest_and_the_lan_urls_are_built_from_real_data() {
    let t = tree("offer-sha");
    assert_eq!(sha256_hex(&t.apk).unwrap(), {
        let c = ring::digest::digest(&ring::digest::SHA256, b"PK\x03\x04 apk bytes");
        c.as_ref().iter().map(|b| format!("{b:02x}")).collect::<String>()
    });
    assert!(sha256_hex(&t.root.join("nope.apk")).is_none());
    let urls = urls_of(&lan(), "/install/tok/orrery.apk");
    assert_eq!(urls, vec!["http://192.168.1.20:4321/install/tok/orrery.apk", "http://10.0.0.5:4321/install/tok/orrery.apk"]);
    assert!(urls_of(&["8.8.8.8:1".parse().unwrap()], "").is_empty());
}

#[test]
fn offering_checks_the_service_the_id_the_phones_the_network_and_the_session() {
    let rig = rig("offer-gates", "", false);
    assert_eq!(rig.remote.offer_apk(&json!("run-1")), json!({ "ok": false, "error": "off" }));
    let t = tree("offer-gates-t");
    let ctx = serving(&rig, json!([session(&t)]));
    ctx.running.store(false, Ordering::SeqCst);
    assert_eq!(ctx.offer_apk("run-1", Instant::now()), Err("off"));
    ctx.running.store(true, Ordering::SeqCst);
    assert_eq!(ctx.offer_apk("a/b", Instant::now()), Err("invalid"));
    assert_eq!(ctx.offer_apk("run-1", Instant::now()), Err("none"));
    add(&rig, "a", TOKEN_A, false);
    assert_eq!(ctx.offer_apk("run-1", Instant::now()), Err("none"));
    lock(&ctx.devices).set_push_seal("a", true, true, now_secs());
    ctx.set_lan(vec!["8.8.8.8:1".parse().unwrap()]);
    assert_eq!(ctx.offer_apk("run-1", Instant::now()), Err("lan"));
    ctx.set_lan(lan());
    assert_eq!(ctx.offer_apk("missing", Instant::now()), Err("notFound"));
    assert_eq!(ctx.install.len(), 0);
    assert!(ctx.take_push_jobs().is_empty());
}

#[test]
fn a_valid_offer_issues_one_apk_offer_ticket_and_queues_sealed_jobs_without_a_path() {
    let rig = rig("offer-ok", "", false);
    let t = tree("offer-ok-t");
    add(&rig, "a", TOKEN_A, true);
    add(&rig, "b", TOKEN_B, true);
    let ctx = serving(&rig, json!([session(&t)]));
    assert_eq!(rig.remote.offer_apk(&json!({ "sessionId": "run-1" })), json!({ "ok": true, "sent": 2 }));
    assert_eq!(ctx.install.len(), 1);
    let jobs = ctx.take_push_jobs();
    assert_eq!(jobs.len(), 2);
    for job in &jobs {
        assert_eq!(job.kind, "apk");
        assert!(job.pane.is_empty());
        let apk = job.extra.apk.clone().unwrap();
        assert_eq!(apk["size"], json!(14));
        assert_eq!(apk["app"], json!("com.ex.app"));
        assert_eq!(apk["variant"], json!("debug"));
        assert_eq!(apk["sha256"].as_str().unwrap().len(), 64);
        let url = apk["url"].as_str().unwrap();
        assert!(url.starts_with("http://192.168.1.20:4321/install/") && url.ends_with("/orrery.apk"));
        assert_eq!(apk["urls"].as_array().unwrap().len(), 2);
        let plain = String::from_utf8(sealed_plain(job, "T", "B")).unwrap();
        assert!(plain.contains("\"apk\""));
        let flat = |s: String| s.replace("\\\\", "/").replace('\\', "/");
        let (abs, root) = (flat(t.apk.to_string_lossy().into_owned()), flat(t.root.to_string_lossy().into_owned()));
        let blob = flat(format!("{plain}{}{}", web_payload(job, "T", "B"), message(job)));
        assert!(!blob.contains(&abs) && !blob.contains(&root), "{blob}");
        assert!(!blob.contains("\"path\""));
        assert!(web_payload(job, "T", "B")["apk"].is_object());
        let plain_msg = message(job);
        assert!(!plain_msg.to_string().contains("sha256") && !plain_msg.to_string().contains("/install/"));
        assert_eq!(plain_msg["message"]["data"]["k"], json!("apk"));
        assert_eq!(plain_msg["message"]["notification"]["title"], json!(title_body("apk", "en").0));
    }
}

#[test]
fn the_offer_ticket_serves_the_file_and_lives_next_to_the_install_page_ticket() {
    let rig = rig("offer-ticket", "", false);
    let t = tree("offer-ticket-t");
    add(&rig, "a", TOKEN_A, true);
    let ctx = serving(&rig, json!([session(&t)]));
    let (page, _) = ctx.install.issue_with(TicketKind::ApkPage, t.apk.clone(), "v".into(), now_secs()).unwrap();
    assert_eq!(rig.remote.offer_apk(&json!("run-1")), json!({ "ok": true, "sent": 1 }));
    assert_eq!(ctx.install.len(), 2);
    let job = ctx.take_push_jobs().pop().unwrap();
    let url = job.extra.apk.unwrap()["url"].as_str().unwrap().to_owned();
    let target = url.strip_prefix("http://192.168.1.20:4321").unwrap();
    let head = format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n");
    let ip: std::net::IpAddr = "192.168.1.50".parse().unwrap();
    let canon = std::fs::canonicalize(&t.apk).unwrap();
    assert_eq!(resolve(&ctx.install, ip, &head, now_secs()), Reply::File { path: canon.clone(), head: false, first: true });
    let page_head = format!("GET /install/{page}/ HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(matches!(resolve(&ctx.install, ip, &page_head, now_secs()), Reply::Html { .. }));
    let token = target.trim_start_matches("/install/").split('/').next().unwrap();
    let as_page = format!("GET /install/{token}/ HTTP/1.1\r\nHost: x\r\n\r\n");
    assert_eq!(resolve(&ctx.install, ip, &as_page, now_secs()), Reply::NotFound);
    assert_eq!(resolve(&ctx.install, "8.8.8.8".parse().unwrap(), &head, now_secs()), Reply::Forbidden);
    std::fs::remove_file(&t.apk).unwrap();
    assert_eq!(resolve(&ctx.install, ip, &head, now_secs()), Reply::NotFound);
    for _ in 0..MAX_SLOTS + 2 {
        ctx.install.issue(TicketKind::ApkOffer, canon.clone(), now_secs());
    }
    assert!(ctx.install.len() <= MAX_SLOTS);
}

#[test]
fn a_phone_without_content_or_seal_is_skipped_and_the_info_quota_stops_the_offers() {
    let rig = rig("offer-skip", "", false);
    let t = tree("offer-skip-t");
    add(&rig, "plain", TOKEN_A, false);
    add(&rig, "sealed", TOKEN_B, true);
    let ctx = serving(&rig, json!([session(&t)]));
    assert_eq!(ctx.offer_targets().len(), 1);
    lock(&ctx.devices).set_push_content("sealed", false);
    assert_eq!(ctx.offer_apk("run-1", Instant::now()), Err("none"));
    lock(&ctx.devices).set_push_content("sealed", true);
    let now = Instant::now();
    for i in 0..PER_HOUR_INFO {
        assert_eq!(ctx.offer_apk("run-1", now), Ok(1), "{i}");
    }
    assert_eq!(ctx.offer_apk("run-1", now), Err("limit"));
}
