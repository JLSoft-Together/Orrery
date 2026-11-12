use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::config::{now_secs, Config};
use super::devices::{Device, Devices};
use super::fcm_files::PACKAGE;
use super::fcm_tests::{gs_json, sa_json, script, Fake, KEY_A, KEY_B};
use super::push_setup::SECRET;
use super::test_util::*;
use super::{Options, Remote};
use crate::state::lock;

const TOKEN_A: &str = "tokenAAAAAAAAAAAAAAAAAAAAAAAA:APA91bAAAA";
const TOKEN_B: &str = "tokenBBBBBBBBBBBBBBBBBBBBBBBB:APA91bBBBB";

fn files(project: &str, key: &str) -> Value {
    json!({ "googleServices": gs_json(project, PACKAGE), "serviceAccount": sa_json(project, key) })
}

fn add_device(rig: &Rig, id: &str, token: &str) {
    let ctx = rig.remote.ctx().unwrap();
    let phone = Phone::new(id);
    let mut devices = lock(&ctx.devices);
    assert!(devices.add(Device::new(id, id, phone.key.public(), now_secs())));
    if !token.is_empty() {
        assert!(devices.set_push(id, token, "en"));
    }
}

fn with_http(rig: &Rig, http: Arc<Fake>) {
    let ctx = rig.remote.ctx().unwrap();
    ctx.fcm.set_http(http);
    *lock(&ctx.fcm.gap) = Duration::ZERO;
}

fn secret_text(rig: &Rig) -> String {
    lock(&rig.secrets.0).get(SECRET).cloned().unwrap_or_default()
}

fn restarted(rig: &Rig) -> Remote {
    let opts = Options { addrs: None, port: Some(0), mdns: false, name: Some("PC".into()) };
    Remote::new(rig.host.clone(), Box::new(rig.secrets.clone()), rig.dir.clone(), opts)
}

#[test]
fn setup_validates_stores_the_key_in_the_keychain_and_only_options_in_the_config() {
    let rig = rig("pushs-ok", "", false);
    assert_eq!(rig.remote.push_status(), json!({ "configured": false, "tokens": 0 }));
    let out = rig.remote.push_setup(&files("demo-proj", KEY_A));
    assert_eq!(out, json!({ "ok": true, "projectId": "demo-proj" }));
    assert!(secret_text(&rig).contains("PRIVATE KEY"));
    let disk = std::fs::read_to_string(Config::path(&rig.dir)).unwrap();
    assert!(!disk.contains("PRIVATE") && !disk.contains(KEY_A));
    let cfg = Config::load(&rig.dir).firebase;
    assert_eq!((cfg.project_id.as_str(), cfg.sender_id.as_str(), cfg.app_id.as_str()), ("demo-proj", "123456789012", "1:123456789012:android:feedbeef"));
    let status = rig.remote.push_status();
    assert_eq!(status, json!({ "configured": true, "projectId": "demo-proj", "tokens": 0 }));
    let text = status.to_string() + &out.to_string() + &rig.remote.status().to_string();
    assert!(!text.contains("PRIVATE") && !text.contains("iam.gserviceaccount") && !text.contains(KEY_A));
    assert!(rig.host.changes.load(std::sync::atomic::Ordering::SeqCst) > 0);
}

#[test]
fn bad_input_stores_nothing_and_returns_the_error_code() {
    let rig = rig("pushs-bad", "", false);
    let cases = [
        (json!({ "googleServices": gs_json("demo-proj", PACKAGE), "serviceAccount": sa_json("other-proj", KEY_A) }), "mismatch_project"),
        (json!({ "googleServices": gs_json("demo-proj", "com.other"), "serviceAccount": sa_json("demo-proj", KEY_A) }), "no_android_client"),
        (json!({ "googleServices": gs_json("demo-proj", PACKAGE), "serviceAccount": "{\"type\":\"service_account\"}" }), "bad_service_account"),
        (json!({ "googleServices": "nope", "serviceAccount": sa_json("demo-proj", KEY_A) }), "bad_json"),
        (json!({ "googleServices": 5 }), "bad_json"),
        (Value::Null, "bad_json"),
    ];
    for (arg, code) in cases {
        assert_eq!(rig.remote.push_setup(&arg), json!({ "ok": false, "error": code }));
    }
    assert!(secret_text(&rig).is_empty());
    assert!(!Config::load(&rig.dir).firebase.is_set());
    assert_eq!(rig.remote.push_status()["configured"], false);
}

#[test]
fn the_service_account_survives_a_restart_from_the_keychain() {
    let rig = rig("pushs-restart", "", false);
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_A))["ok"], true);
    let again = restarted(&rig);
    assert_eq!(again.push_status()["configured"], true);
    assert!(again.ctx().unwrap().push_ready());
    let none = rig_without_setup();
    assert!(!none.remote.ctx().unwrap().push_ready());
}

fn rig_without_setup() -> Rig {
    rig("pushs-none", "", false)
}

#[test]
fn replacing_the_account_in_the_same_project_keeps_tokens_but_a_new_project_drops_them() {
    let rig = rig("pushs-move", "", false);
    add_device(&rig, "a", TOKEN_A);
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_A))["ok"], true);
    assert_eq!(rig.remote.push_status()["tokens"], 1);
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_B))["ok"], true);
    assert_eq!(rig.remote.push_status()["tokens"], 1);
    assert_eq!(rig.remote.push_setup(&files("second-proj", KEY_B))["ok"], true);
    assert_eq!(rig.remote.push_status(), json!({ "configured": true, "projectId": "second-proj", "tokens": 0 }));
    assert!(Devices::load(&rig.dir).find("a").unwrap().push_token.is_empty());
}

#[test]
fn test_sends_one_notification_per_registered_device_or_the_chosen_one() {
    let rig = rig("pushs-test", "", false);
    add_device(&rig, "a", TOKEN_A);
    add_device(&rig, "b", TOKEN_B);
    add_device(&rig, "c", "");
    assert_eq!(rig.remote.push_test(&json!({})), json!({ "ok": false, "error": "not_configured" }));
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_A))["ok"], true);
    let http = Fake::ok();
    with_http(&rig, http.clone());
    assert_eq!(rig.remote.push_test(&json!({})), json!({ "ok": true, "sent": 2 }));
    let calls = http.fcm_calls();
    assert_eq!(calls.len(), 2);
    let body: Value = serde_json::from_str(&calls[0].3).unwrap();
    assert_eq!(body["message"]["data"]["k"], "test");
    assert!(body["message"]["data"].get("p").is_none());
    assert_eq!(rig.remote.push_test(&json!({ "deviceId": "b" })), json!({ "ok": true, "sent": 1 }));
    assert_eq!(http.fcm_calls().len(), 3);
    assert_eq!(rig.remote.push_test(&json!({ "deviceId": "c" })), json!({ "ok": false, "error": "no_device" }));
    assert_eq!(rig.remote.push_test(&json!({ "deviceId": "zzz" })), json!({ "ok": false, "error": "no_device" }));
}

#[test]
fn test_reports_fcm_errors_and_dead_tokens() {
    let rig = rig("pushs-test-err", "", false);
    add_device(&rig, "a", TOKEN_A);
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_A))["ok"], true);
    with_http(&rig, script(vec![Some((401, "{}".into()))], vec![]));
    assert_eq!(rig.remote.push_test(&json!({})), json!({ "ok": false, "error": "auth" }));
    assert_eq!(rig.remote.push_status()["error"], "auth");
    with_http(&rig, script(vec![], vec![Some((404, "{}".into()))]));
    assert_eq!(rig.remote.push_test(&json!({})), json!({ "ok": false, "error": "unregistered" }));
    assert_eq!(rig.remote.push_status()["tokens"], 0);
}

#[test]
fn remove_wipes_the_options_the_key_and_the_tokens() {
    let rig = rig("pushs-remove", "", false);
    add_device(&rig, "a", TOKEN_A);
    assert_eq!(rig.remote.push_setup(&files("demo-proj", KEY_A))["ok"], true);
    assert_eq!(rig.remote.push_remove(), json!({ "ok": true }));
    assert_eq!(rig.remote.push_status(), json!({ "configured": false, "tokens": 0 }));
    assert_eq!(secret_text(&rig), "-");
    assert!(!Config::load(&rig.dir).firebase.is_set());
    assert!(!rig.remote.ctx().unwrap().push_ready());
    assert_eq!(rig.remote.push_remove(), json!({ "ok": true }));
    assert!(!restarted(&rig).ctx().unwrap().push_ready());
}

#[test]
fn the_channels_are_routed_through_call() {
    let rig = rig("pushs-call", "", false);
    for c in ["ccm:remote-push-status", "ccm:remote-push-setup", "ccm:remote-push-test", "ccm:remote-push-remove"] {
        assert!(super::handles(c), "{c}");
    }
    assert_eq!(rig.remote.call("ccm:remote-push-status", &[]).unwrap()["configured"], false);
    assert_eq!(rig.remote.call("ccm:remote-push-setup", &[files("demo-proj", KEY_A)]).unwrap()["ok"], true);
    assert_eq!(rig.remote.call("ccm:remote-push-remove", &[]).unwrap()["ok"], true);
}
