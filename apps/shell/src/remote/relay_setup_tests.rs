use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::cf_http::API_BASE;
use super::config::Config;
use super::relay_setup::{bundle_hash, TOKEN_SECRET};
use super::test_util::*;
use crate::state::lock;

#[path = "cf_fake.rs"]
mod fake;
#[path = "relay_setup_tests2.rs"]
mod more;

use fake::*;

#[test]
fn a_bad_token_is_rejected_and_never_stored_or_echoed() {
    let (rig, cf) = setup("cf-bad", Cf { verify_status: 401, accounts_status: 401, ..Cf::default() });
    let out = rig.remote.relay_token(&json!(TOKEN));
    assert_eq!(out, json!({ "ok": false, "error": "token_invalid" }));
    assert!(!out.to_string().contains(TOKEN));
    assert!(lock(&rig.secrets.0).get(TOKEN_SECRET).is_none());
    assert_eq!(rig.remote.relay_token(&json!("short"))["error"], "token_invalid");
    assert_eq!(rig.remote.relay_token(&json!("bad token with spaces and more chars!"))["error"], "token_invalid");
    assert_eq!(calls(&cf).len(), 2);
}

#[test]
fn a_token_without_account_read_scope_is_reported() {
    let (rig, _) = setup("cf-scope", Cf { accounts_status: 403, ..Cf::default() });
    assert_eq!(rig.remote.relay_token(&json!(TOKEN))["error"], "token_scope");
    assert!(lock(&rig.secrets.0).get(TOKEN_SECRET).is_none());
}

#[test]
fn an_account_owned_token_passes_through_the_accounts_probe() {
    let (rig, cf) = setup("cf-owned", Cf { user_verify: false, ..Cf::default() });
    assert_eq!(rig.remote.relay_token(&json!(TOKEN))["ok"], true);
    assert!(calls(&cf).iter().any(|(_, p)| p == "/user/tokens/verify"));
}

#[test]
fn several_accounts_are_listed_and_junk_ids_are_dropped() {
    let (rig, _) = setup("cf-many", Cf { accounts: vec![(ACCOUNT.into(), "A".into()), (OTHER.into(), "B".into()), ("../evil".into(), "C".into())], ..Cf::default() });
    let out = rig.remote.relay_token(&json!(TOKEN));
    assert_eq!(out["accounts"], json!([{ "id": ACCOUNT, "name": "A" }, { "id": OTHER, "name": "B" }]));
    assert_eq!(lock(&rig.secrets.0).get(TOKEN_SECRET).map(String::as_str), Some(TOKEN));
}

#[test]
fn no_account_at_all_is_an_error() {
    let (rig, _) = setup("cf-none", Cf { accounts: vec![], ..Cf::default() });
    assert_eq!(rig.remote.relay_token(&json!(TOKEN))["error"], "no_account");
}

#[test]
fn the_first_deploy_creates_the_subdomain_and_sends_bindings_with_migrations() {
    let (rig, cf) = setup("cf-first", Cf::default());
    token_ok(&rig);
    let steps = Mutex::new(Vec::new());
    let out = rig.remote.relay_deploy(&json!(ACCOUNT), &|s| lock(&steps).push(s.to_owned()));
    let url = out["url"].as_str().unwrap().to_owned();
    assert_eq!(out["ok"], true);
    let sub = lock(&cf).subdomain.clone().unwrap();
    assert!(sub.starts_with("orrery-") && sub.len() == 13);
    assert_eq!(url, format!("https://orrery-relay.{sub}.workers.dev"));
    assert_eq!(*lock(&steps), ["verify", "subdomain", "upload", "enable", "health", "done"]);
    let meta = metadata_of(&uploads(&cf)[0]);
    assert_eq!(meta["main_module"], "worker.js");
    assert_eq!(meta["migrations"], json!({ "new_tag": "v1", "new_sqlite_classes": ["Room"] }));
    let kinds: Vec<&str> = meta["bindings"].as_array().unwrap().iter().map(|b| b["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["durable_object_namespace", "ratelimit", "plain_text", "plain_text"]);
    assert_eq!(meta["bindings"][0], json!({ "type": "durable_object_namespace", "name": "ROOM", "class_name": "Room" }));
    assert_eq!(meta["bindings"][1], json!({ "type": "ratelimit", "name": "LIMITER", "namespace_id": "9701", "simple": { "limit": 30, "period": 60 } }));
    assert!(calls(&cf).contains(&("POST".into(), format!("/accounts/{ACCOUNT}/workers/scripts/orrery-relay/subdomain"))));
    let enable = lock(&cf).log.iter().find(|(_, m, _, _, _)| m == "POST").unwrap().3.clone();
    assert_eq!(serde_json::from_slice::<Value>(&enable).unwrap()["enabled"], true);
    let health = lock(&cf).log.iter().filter(|(b, _, _, _, _)| *b == url).count();
    assert_eq!(health, 1);
    let cfg = Config::load(&rig.dir).relay;
    assert_eq!((cfg.url.as_str(), cfg.account_id.as_str(), cfg.deployed_hash.as_str()), (url.as_str(), ACCOUNT, bundle_hash().as_str()));
    assert!(!cfg.registered);
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status["updateAvailable"].as_bool(), status["account"].as_str()), (Some("ready"), Some(false), Some("Jin")));
}

#[test]
fn the_second_deploy_reuses_the_subdomain_and_skips_migrations() {
    let (rig, cf) = setup("cf-twice", Cf::default());
    token_ok(&rig);
    let first = rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {});
    lock(&rig.remote.ctx).take();
    let second = rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {});
    assert_eq!(first["url"], second["url"]);
    let ups = uploads(&cf);
    assert_eq!(ups.len(), 2);
    assert!(metadata_of(&ups[0]).get("migrations").is_some());
    assert!(metadata_of(&ups[1]).get("migrations").is_none());
    let puts = calls(&cf).iter().filter(|(m, p)| m == "PUT" && p.ends_with("/workers/subdomain")).count();
    assert_eq!(puts, 1);
}

#[test]
fn deploy_without_a_saved_token_or_with_a_foreign_account_fails() {
    let (rig, cf) = setup("cf-guard", Cf::default());
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "no_token");
    assert_eq!(rig.remote.relay_deploy(&json!("../x"), &|_| {})["error"], "no_account");
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(OTHER), &|_| {})["error"], "no_account");
    assert!(uploads(&cf).is_empty());
    assert_eq!(rig.remote.relay_status()["state"], "error");
}

#[test]
fn a_network_drop_mid_deploy_can_be_retried() {
    let (rig, cf) = setup("cf-drop", Cf { offline_on: Some("upload"), ..Cf::default() });
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "offline");
    assert_eq!(rig.remote.relay_status()["state"], "error");
    lock(&cf).offline_on = None;
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["ok"], true);
    assert_eq!(rig.remote.relay_status()["state"], "ready");
}

#[test]
fn a_rejected_upload_is_an_upload_error_and_a_busy_deploy_is_refused() {
    let (rig, _) = setup("cf-rej", Cf { upload_status: 400, ..Cf::default() });
    token_ok(&rig);
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "upload");
    *lock(&rig.remote.setup.busy) = true;
    assert_eq!(rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {})["error"], "busy");
    assert_eq!(rig.remote.relay_status()["state"], "deploying");
}

#[test]
fn the_token_never_shows_up_in_status_errors_or_the_progress_steps() {
    let (rig, cf) = setup("cf-leak", Cf { upload_status: 403, ..Cf::default() });
    token_ok(&rig);
    let steps = Mutex::new(Vec::new());
    let out = rig.remote.relay_deploy(&json!(ACCOUNT), &|s| lock(&steps).push(s.to_owned()));
    let everything = format!("{out}{}{}{:?}", rig.remote.relay_status(), rig.remote.status(), lock(&steps));
    assert!(!everything.contains(TOKEN));
    assert_eq!(out["error"], "token_scope");
    assert!(!std::fs::read_to_string(Config::path(&rig.dir)).unwrap().contains(TOKEN));
    let health_calls: Vec<_> = lock(&cf).log.iter().filter(|(b, _, _, _, _)| b != API_BASE).map(|e| e.4.clone()).collect();
    assert!(health_calls.iter().all(String::is_empty));
}

#[test]
fn an_older_bundle_hash_means_an_update_is_available() {
    let (rig, _) = setup("cf-upd", Cf::default());
    token_ok(&rig);
    rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {});
    rig.remote.edit_config(|c| c.relay.deployed_hash = "old".into());
    assert_eq!(rig.remote.relay_status()["updateAvailable"], true);
}

#[test]
fn removing_while_connected_stops_the_relay_task() {
    let (rig, _) = setup("cf-live", Cf::default());
    token_ok(&rig);
    rig.remote.enable(true);
    assert_eq!(rig.remote.status()["relayUrl"], "");
    rig.remote.relay_deploy(&json!(ACCOUNT), &|_| {});
    assert!(rig.remote.status()["relayUrl"].as_str().unwrap().starts_with("https://orrery-relay."));
    assert_eq!(rig.remote.status()["running"], true);
    rig.remote.relay_remove(&json!({ "deleteWorker": false }));
    assert_eq!(rig.remote.status()["relayUrl"], "");
    assert_eq!(rig.remote.status()["running"], true);
    rig.remote.enable(false);
}
