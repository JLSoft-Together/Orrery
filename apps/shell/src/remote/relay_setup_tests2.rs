use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_remote_proto::pair::QrPayload;
use serde_json::{json, Value};

use super::fake::*;
use crate::remote::cf_api::{migrations, valid_subdomain};
use crate::remote::config::Config;
use crate::remote::keys::{KeyStore, Read};
use crate::remote::relay_setup::TOKEN_SECRET;
use crate::remote::test_util::*;
use crate::remote::{Options, Remote};
use crate::state::lock;

fn deployed(tag: &str, cf: Cf) -> (Rig, Shared) {
    let (rig, shared) = setup(tag, cf);
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["ok"], true);
    (rig, shared)
}

fn allow_delete(rig: &Rig, answer: bool) -> Arc<Mutex<usize>> {
    let asked = Arc::new(Mutex::new(0));
    let a = asked.clone();
    rig.remote.set_delete_confirm(Box::new(move || {
        *lock(&a) += 1;
        answer
    }));
    asked
}

fn deletes(cf: &Shared) -> usize {
    calls(cf).iter().filter(|(m, _)| m == "DELETE").count()
}

#[test]
fn a_dns_that_never_comes_up_still_saves_the_url_with_a_warning() {
    let (rig, _) = setup("cf-dns", Cf { health_ok: false, ..Cf::default() });
    token_ok(&rig);
    let out = rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {});
    assert_eq!((out["ok"].clone(), out["warning"].clone()), (json!(true), json!("health")));
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status["warning"].as_str()), (Some("ready"), Some("health")));
    assert!(Config::load(&rig.dir).relay.url.starts_with("https://orrery-relay."));
    lock(&rig.remote.setup.warning).take();
    assert!(rig.remote.relay_status().get("warning").is_none());
}

#[test]
fn an_unknown_old_tag_is_refused_instead_of_overwritten() {
    let (rig, cf) = setup("cf-oldtag", Cf { subdomain: Some("me".into()), tag: Some("v0".into()), ..Cf::default() });
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "migration");
    assert!(uploads(&cf).is_empty());
}

#[test]
fn migration_steps_follow_the_docs_forms() {
    let one = json!({ "migrations": [{ "tag": "v1", "new_sqlite_classes": ["Room"] }] });
    let two = json!({ "migrations": [{ "tag": "v1", "new_sqlite_classes": ["Room"] }, { "tag": "v2", "new_sqlite_classes": ["Extra"] }] });
    assert_eq!(migrations(None, &one).unwrap(), Some(json!({ "new_tag": "v1", "new_sqlite_classes": ["Room"] })));
    assert_eq!(migrations(Some("v1"), &one).unwrap(), None);
    assert_eq!(migrations(None, &two).unwrap(), Some(json!({ "new_tag": "v2", "steps": [{ "new_sqlite_classes": ["Room"] }, { "new_sqlite_classes": ["Extra"] }] })));
    assert_eq!(migrations(Some("v1"), &two).unwrap(), Some(json!({ "old_tag": "v1", "new_tag": "v2", "steps": [{ "new_sqlite_classes": ["Extra"] }] })));
    assert_eq!(migrations(Some("v2"), &two).unwrap(), None);
    assert_eq!(migrations(Some("v9"), &two), Err("migration"));
    assert_eq!(migrations(None, &json!({})), Err("bundle"));
}

#[test]
fn a_hostile_subdomain_from_the_api_never_reaches_the_url() {
    for bad in ["evil.com/x", "a@b", "UPPER", "-a", "a-", "a.b", "", &"a".repeat(64)] {
        assert!(!valid_subdomain(bad), "{bad}");
    }
    for good in ["a", "orrery-1a2b3c", "jinvu", "a1-b2"] {
        assert!(valid_subdomain(good), "{good}");
    }
    let (rig, cf) = setup("cf-evilsub", Cf { subdomain: Some("evil.com/x".into()), ..Cf::default() });
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "subdomain");
    assert!(uploads(&cf).is_empty());
    assert_eq!(Config::load(&rig.dir).relay.url, "");
}

#[test]
fn accounts_are_read_across_pages() {
    let many: Vec<(String, String)> = (0..5).map(|i| (format!("{i:032x}"), format!("A{i}"))).collect();
    let (rig, _) = setup("cf-pages", Cf { accounts: many, page_size: 2, ..Cf::default() });
    let out = rig.remote.relay_token(&json!(TOKEN));
    assert_eq!(out["accounts"].as_array().unwrap().len(), 5);
}

#[test]
fn a_failed_update_keeps_the_relay_ready_and_shows_the_error() {
    let (rig, cf) = deployed("cf-upd-fail", Cf::default());
    lock(&cf).upload_status = 400;
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "upload");
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status["error"].as_str()), (Some("ready"), Some("upload")));
    assert!(status["url"].as_str().is_some());
}

#[test]
fn a_page_request_alone_never_deletes_the_worker() {
    let (rig, cf) = deployed("cf-confirm", Cf::default());
    assert_eq!(rig.remote.relay_remove(&json!({ "deleteWorker": true }))["error"], "cancelled");
    let asked = allow_delete(&rig, false);
    assert_eq!(rig.remote.relay_remove(&json!({ "deleteWorker": true }))["error"], "cancelled");
    assert_eq!((deletes(&cf), *lock(&asked)), (0, 1));
    assert!(!Config::load(&rig.dir).relay.url.is_empty());
    allow_delete(&rig, true);
    let out = rig.remote.relay_remove(&json!({ "deleteWorker": true }));
    assert_eq!((out["ok"].clone(), out["workerDeleted"].clone()), (json!(true), json!(true)));
    assert!(calls(&cf).contains(&("DELETE".into(), format!("/accounts/{ACCOUNT}/workers/scripts/orrery-relay?force=true"))));
    assert_eq!(Config::load(&rig.dir).relay.url, "");
}

#[test]
fn removing_without_delete_clears_everything_and_needs_no_confirm() {
    let (rig, cf) = deployed("cf-rm", Cf::default());
    let out = rig.remote.relay_remove(&json!({ "deleteWorker": false }));
    assert_eq!((out["ok"].clone(), out["workerDeleted"].clone()), (json!(true), json!(false)));
    assert_eq!(deletes(&cf), 0);
    let cfg = Config::load(&rig.dir).relay;
    assert_eq!((cfg.url.as_str(), cfg.account_id.as_str(), cfg.registered), ("", "", false));
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status["hasToken"].as_bool()), (Some("none"), Some(false)));
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "no_token");
}

#[test]
fn a_failed_worker_delete_keeps_the_relay_and_a_legacy_relay_reports_no_delete() {
    let (rig, cf) = deployed("cf-rmfail", Cf::default());
    allow_delete(&rig, true);
    lock(&cf).delete_status = 500;
    assert_eq!(rig.remote.relay_remove(&json!({ "deleteWorker": true }))["ok"], false);
    assert!(!Config::load(&rig.dir).relay.url.is_empty());
    rig.remote.edit_config(|c| c.relay.account_id.clear());
    let out = rig.remote.relay_remove(&json!({ "deleteWorker": true }));
    assert_eq!((out["ok"].clone(), out["workerDeleted"].clone()), (json!(true), json!(false)));
}

struct KeepToken(MemSecrets);

impl KeyStore for KeepToken {
    fn read(&self, name: &str) -> Read {
        self.0.read(name)
    }
    fn write(&self, name: &str, value: &str) -> bool {
        self.0.write(name, value)
    }
    fn delete(&self, _: &str) -> bool {
        false
    }
}

#[test]
fn a_keychain_that_cannot_delete_the_token_is_reported() {
    let dir = temp_dir("cf-keyring");
    prepare(&dir, "https://orrery-relay.me.workers.dev", true);
    let secrets = MemSecrets::default();
    lock(&secrets.0).insert(TOKEN_SECRET.into(), TOKEN.into());
    let host = Arc::new(TestHost::default());
    let remote = Remote::new(host, Box::new(KeepToken(secrets)), dir.clone(), Options { port: Some(0), ..Options::default() });
    let out = remote.relay_remove(&json!({ "deleteWorker": false }));
    assert_eq!(out["error"], "keyring");
    assert_eq!(Config::load(&dir).relay.url, "");
}

#[test]
fn a_removal_during_a_deploy_cancels_it_and_the_url_never_comes_back() {
    let (rig, _) = setup("cf-race", Cf { health_ok: false, ..Cf::default() });
    {
        let mut env = lock(&rig.remote.setup.env);
        env.health_wait = Duration::from_secs(30);
        env.health_gap = Duration::from_millis(10);
    }
    token_ok(&rig);
    let (deploy, remove) = std::thread::scope(|s| {
        let d = s.spawn(|| rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {}));
        while !*lock(&rig.remote.setup.busy) {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(100));
        let r = rig.remote.relay_remove(&json!({ "deleteWorker": false }));
        (d.join().unwrap(), r)
    });
    assert_eq!(deploy["error"], "cancelled");
    assert_eq!(remove["ok"], true);
    assert_eq!(Config::load(&rig.dir).relay.url, "");
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status.get("error"), status["hasToken"].as_bool()), (Some("none"), None, Some(false)));
}

#[test]
fn a_cancel_flag_raised_between_steps_stops_before_the_config_is_written() {
    let (rig, _) = setup("cf-cancel", Cf::default());
    token_ok(&rig);
    let out = rig.remote.relay_deploy(&json!(ACCOUNT), &|step| {
        if step == "health" {
            rig.remote.setup.cancel.store(true, Ordering::SeqCst);
        }
    });
    assert_eq!(out["error"], "cancelled");
    assert_eq!(Config::load(&rig.dir).relay.url, "");
    assert_eq!(rig.remote.relay_status()["state"], "none");
}

#[test]
fn a_second_remove_or_a_token_check_while_busy_is_refused() {
    let (rig, _) = setup("cf-busy", Cf::default());
    *lock(&rig.remote.setup.busy) = true;
    rig.remote.setup.removing.store(true, Ordering::SeqCst);
    assert_eq!(rig.remote.relay_remove(&json!({ "deleteWorker": false }))["error"], "busy");
    assert_eq!(rig.remote.relay_token(&json!(TOKEN))["error"], "busy");
}

#[test]
fn a_gateway_the_user_turned_off_is_not_started_by_a_deploy() {
    let (rig, _) = setup("cf-off", Cf::default());
    token_ok(&rig);
    rig.remote.enable(true);
    let out = rig.remote.relay_deploy(&json!(ACCOUNT), &|step| {
        if step == "health" {
            rig.remote.enable(false);
        }
    });
    assert_eq!(out["ok"], true);
    assert_eq!(rig.remote.status()["running"], false);
    assert_eq!(rig.remote.status()["enabled"], false);
}

#[test]
fn an_empty_relay_url_means_lan_only_and_an_old_config_keeps_its_url() {
    assert_eq!(Config::default().relay.url, "");
    let dir = temp_dir("cf-cfg");
    std::fs::write(Config::path(&dir), r#"{"machineId":"AAAAAAAAAAAAAAAAAAAAAA","relay":{"url":"https://orrery-relay.jinvu.workers.dev","registered":true}}"#).unwrap();
    let cfg = Config::load(&dir);
    assert_eq!((cfg.relay.url.as_str(), cfg.relay.registered), ("https://orrery-relay.jinvu.workers.dev", true));
    let bare = temp_dir("cf-cfg2");
    std::fs::write(Config::path(&bare), r#"{"machineId":"AAAAAAAAAAAAAAAAAAAAAA"}"#).unwrap();
    assert_eq!(Config::load(&bare).relay.url, "");
    let rig = rig("cf-lan", "", false);
    rig.remote.enable(true);
    let status = rig.remote.status();
    assert_eq!((status["relay"].as_str(), status["relayUrl"].as_str()), (Some("off"), Some("")));
    let qr = QrPayload::decode(rig.remote.pair_start().unwrap()["qr"].as_str().unwrap()).unwrap();
    assert_eq!(qr.relay, None);
    assert!(qr.host.is_some());
    rig.remote.enable(false);
    let with = self::rig("cf-lan2", "https://orrery-relay.me.workers.dev", false);
    with.remote.enable(true);
    let qr = QrPayload::decode(with.remote.pair_start().unwrap()["qr"].as_str().unwrap()).unwrap();
    assert_eq!(qr.relay.as_deref(), Some("https://orrery-relay.me.workers.dev"));
    with.remote.enable(false);
}

#[test]
fn the_progress_steps_and_the_busy_status_are_visible_during_a_deploy() {
    let (rig, _) = setup("cf-prog", Cf::default());
    token_ok(&rig);
    let seen: Mutex<Vec<Value>> = Mutex::new(Vec::new());
    rig.remote.relay_deploy(&json!(ACCOUNT), &|_| lock(&seen).push(rig.remote.relay_status()));
    assert!(lock(&seen).iter().all(|s| s["state"] == "deploying"));
}
