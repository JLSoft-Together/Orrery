use super::core::{Cfg, Core};
use crate::{DeskError, Emitter, HostLink};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Server {
    port: u16,
}

fn serve(make: impl FnOnce(u16) -> Vec<(String, Vec<u8>, u64)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = make(port);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let routes = routes.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                if let Some(to) = path.strip_prefix("/redirect") {
                    let _ = write!(stream, "HTTP/1.1 302 Found\r\nLocation: /asset{to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    return;
                }
                match routes.iter().find(|(p, _, _)| *p == path) {
                    Some((_, body, slow)) => {
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        for chunk in body.chunks(8192) {
                            if stream.write_all(chunk).is_err() {
                                return;
                            }
                            if *slow > 0 {
                                std::thread::sleep(Duration::from_millis(*slow));
                            }
                        }
                    }
                    None => {
                        let _ = write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                }
            });
        }
    });
    Server { port }
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

fn release(port: u16, tag: &str, body: &[u8], name: &str, digest: &str) -> Vec<u8> {
    json!({"tag_name": tag, "html_url": format!("http://127.0.0.1:{port}/page"), "name": "Rel", "body": "notes", "published_at": "2026-01-02T03:04:05Z",
        "assets": [{"name": name, "size": body.len(), "browser_download_url": format!("http://127.0.0.1:{port}/redirect"), "digest": digest}]})
    .to_string()
    .into_bytes()
}

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

fn mk(port: u16, kind: &str, dir: &std::path::Path, current: &str, restart: bool, prepare: Value) -> (Arc<Core>, Calls, Arc<Mutex<Vec<Value>>>) {
    let calls: Calls = Arc::new(Mutex::new(Vec::new()));
    let events = Arc::new(Mutex::new(Vec::new()));
    let emitter = Emitter::default();
    let ev = events.clone();
    emitter.set(Arc::new(move |name, payload| {
        assert_eq!(name, "ccm:update-event");
        ev.lock().unwrap().push(payload);
    }));
    let host = HostLink::default();
    let c = calls.clone();
    host.set(Arc::new(move |method, params| {
        c.lock().unwrap().push((method.to_string(), params.clone()));
        Ok(match method {
            "update.prepare" => prepare.clone(),
            "update.restart" => Value::Bool(restart),
            _ => Value::Null,
        })
    }));
    let cfg = Cfg {
        current: current.into(),
        kind: kind.into(),
        asset_kind: "nsis".into(),
        can_download: true,
        arch: "x64".into(),
        feed: format!("http://127.0.0.1:{port}/feed"),
        allow_local: true,
        data_dir: dir.to_path_buf(),
    };
    (Core::new(cfg, emitter, host), calls, events)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("core-desk-upd-{name}-{}", crate::util::uuid()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const NAME: &str = "Orrery-Setup-2.5.0.exe";

fn bytes() -> Vec<u8> {
    (0..40_000u32).map(|i| (i % 251) as u8).collect()
}

fn server(body: &[u8], digest: &str, slow: u64) -> Server {
    let (body, digest) = (body.to_vec(), digest.to_string());
    serve(move |port| vec![("/asset".into(), body.clone(), slow), ("/feed".into(), release(port, "v2.5.0", &body, NAME, &digest), 0)])
}

fn names(calls: &Calls) -> Vec<String> {
    calls.lock().unwrap().iter().map(|(m, _)| m.clone()).collect()
}

#[test]
fn check_download_install_flow() {
    let b = bytes();
    let srv = server(&b, &format!("sha256:{}", sha(&b)), 0);
    let dir = tmp("flow");
    let (core, calls, events)  = mk(srv.port, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    let s = core.check();
    assert_eq!((s["phase"].clone(), s["release"]["version"].clone(), s["error"].clone()), (json!("available"), json!("2.5.0"), Value::Null));
    assert_eq!((s["release"]["asset"]["name"].clone(), s["release"]["asset"]["size"].as_f64()), (json!(NAME), Some(b.len() as f64)));
    assert!(s["checkedAt"].is_number());
    let s = core.download();
    assert_eq!(s["phase"], "ready", "{s}");
    assert_eq!(std::fs::read(dir.join("updates").join(NAME)).unwrap(), b);
    assert!(!dir.join("updates").join(format!("{NAME}.part")).exists());
    assert!(events.lock().unwrap().iter().any(|e| e["phase"] == "downloading" && e["progress"]["total"].as_f64() == Some(b.len() as f64)));
    let s = core.install();
    assert_eq!(s["phase"], "installing");
    assert_eq!(names(&calls), ["update.prepare", "update.restart"]);
    let restart = calls.lock().unwrap()[1].1.clone();
    assert_eq!(restart["version"], "2.5.0");
    assert!(restart["pending"]["file"].as_str().unwrap().ends_with("pending.json"));
    assert_eq!(restart["pending"]["text"], "{\"version\":\"2.5.0\"}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn declined_restart_returns_to_ready_and_readonly_opens_the_file() {
    let b = bytes();
    let srv = server(&b, &format!("sha256:{}", sha(&b)), 0);
    let dir = tmp("decline");
    let (core, calls, _)  = mk(srv.port, "nsis", &dir, "2.4.0", false, json!({"ok": true}));
    core.check();
    core.download();
    assert_eq!(core.install()["phase"], "ready");
    let dir2 = tmp("ro");
    let (core, calls2, _) = core_ro(srv.port, &dir2);
    core.check();
    core.download();
    let s = core.install();
    assert_eq!((s["phase"].clone(), s["error"].clone()), (json!("ready"), json!("mac-readonly")));
    assert_eq!(names(&calls2), ["update.prepare", "update.open"]);
    assert_eq!(names(&calls), ["update.prepare", "update.restart"]);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

fn core_ro(port: u16, dir: &std::path::Path) -> (Arc<Core>, Calls, Arc<Mutex<Vec<Value>>>) {
    mk(port, "nsis", dir, "2.4.0", true, json!({"ok": false, "error": "mac-readonly"}))
}

#[test]
fn bad_digest_and_missing_digest_fail_without_a_file() {
    let b = bytes();
    let srv = server(&b, &format!("sha256:{}", "0".repeat(64)), 0);
    let dir = tmp("digest");
    let (core, _, _)  = mk(srv.port, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    core.check();
    let s = core.download();
    assert_eq!((s["phase"].clone(), s["error"].clone()), (json!("error"), json!("digest")));
    assert!(std::fs::read_dir(dir.join("updates")).unwrap().next().is_none());
    let srv = server(&b, "", 0);
    let dir2 = tmp("nodigest");
    let (core, _, _)  = mk(srv.port, "nsis", &dir2, "2.4.0", true, json!({"ok": true}));
    let s = core.check();
    assert_eq!(s["error"], "no-digest");
    let s = core.download();
    assert_eq!((s["phase"].clone(), s["error"].clone()), (json!("error"), json!("no-digest")));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn no_newer_release_and_skip_and_auto_persist() {
    let b = bytes();
    let srv = server(&b, &format!("sha256:{}", sha(&b)), 0);
    let dir = tmp("none");
    let (core, _, _)  = mk(srv.port, "nsis", &dir, "2.5.0", true, json!({"ok": true}));
    assert_eq!(core.check()["phase"], "none");
    let (core, _, _)  = mk(srv.port, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    core.check();
    assert_eq!(core.skip(&json!("9.9.9"))["skipped"], Value::Null);
    assert_eq!(core.skip(&json!("2.5.0"))["skipped"], "2.5.0");
    assert_eq!(core.set_auto_check(false)["autoCheck"], false);
    let saved: Value = serde_json::from_slice(&std::fs::read(dir.join("update.json")).unwrap()).unwrap();
    assert_eq!(saved, json!({"autoCheck": false, "skipped": "2.5.0"}));
    let (again, _, _)  = mk(srv.port, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    let s = again.state();
    assert_eq!((s["autoCheck"].clone(), s["skipped"].clone()), (json!(false), json!("2.5.0")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cancel_stops_a_running_download() {
    let b: Vec<u8> = vec![7u8; 200_000];
    let srv = server(&b, &format!("sha256:{}", sha(&b)), 40);
    let dir = tmp("cancel");
    let (core, _, _)  = mk(srv.port, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    core.check();
    let c2 = core.clone();
    let t = std::thread::spawn(move || c2.download());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(core.cancel()["phase"], "available");
    t.join().unwrap();
    assert_eq!(core.state()["phase"], "available");
    assert!(std::fs::read_dir(dir.join("updates")).map(|mut d| d.next().is_none()).unwrap_or(true));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn start_clears_leftovers_or_reports_an_unfinished_install() {
    let dir = tmp("start");
    let updates = dir.join("updates");
    std::fs::create_dir_all(&updates).unwrap();
    std::fs::write(updates.join("old.exe"), "x").unwrap();
    let (core, _, _)  = mk(1, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    core.start();
    assert!(!updates.exists());
    std::fs::create_dir_all(&updates).unwrap();
    std::fs::write(updates.join("Orrery-Setup-2.5.0.exe"), "x").unwrap();
    std::fs::write(updates.join("pending.json"), r#"{"version":"2.5.0"}"#).unwrap();
    let (core, _, _)  = mk(1, "nsis", &dir, "2.4.0", true, json!({"ok": true}));
    core.start();
    let s = core.state();
    assert_eq!((s["phase"].clone(), s["error"].clone()), (json!("error"), json!("incomplete")));
    core.dispose();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dev_kind_reveals_instead_of_installing() {
    let b = bytes();
    let srv = server(&b, &format!("sha256:{}", sha(&b)), 0);
    let dir = tmp("dev");
    let (core, calls, _)  = mk(srv.port, "dev", &dir, "2.4.0", true, json!({"ok": true}));
    core.check();
    core.download();
    assert_eq!(core.install()["phase"], "ready");
    assert_eq!(names(&calls), ["update.reveal"]);
    core.open_page();
    assert_eq!(calls.lock().unwrap().last().unwrap().0, "update.open");
    let _ = DeskError::Failed(String::new());
    let _ = std::fs::remove_dir_all(&dir);
}
