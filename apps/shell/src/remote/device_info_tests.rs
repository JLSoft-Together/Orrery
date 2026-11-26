use serde_json::{json, Value};

use super::device_info::{clean_text, parse_caps};
use super::devices::{Device, Devices};
use super::status::device_json;
use super::test_util::*;
use crate::state::lock;

fn device() -> Device {
    Device::new("p", "Phone", &[7u8; 32], 1)
}

#[test]
fn a_device_file_without_the_new_fields_still_loads() {
    let dir = temp_dir("info-old");
    let old = json!({ "devices": [{ "deviceId": "p", "name": "Pixel", "platform": "android", "pubkey": "k", "pairedAt": 1, "lastSeen": 2 }] });
    std::fs::write(dir.join(super::config::DEVICES_FILE), old.to_string()).unwrap();
    let devices = Devices::load(&dir);
    let d = &devices.list()[0];
    assert_eq!((d.app_version.clone(), d.app_build.clone(), d.model.clone(), d.os.clone()), (None, None, None, None));
    let shown = device_json(d, None);
    for key in ["appVersion", "appBuild", "model", "os"] {
        assert!(shown.get(key).is_none(), "{key}");
    }
}

#[test]
fn hello_fields_are_stored_saved_and_shown() {
    let dir = temp_dir("info-set");
    let mut devices = Devices::load(&dir);
    assert!(devices.add(device()));
    devices.set_info("p", &json!({ "appVersion": "4.0.10", "appBuild": "7", "model": "Samsung SM-A336B", "os": "Android 14" }));
    let shown = device_json(&devices.list()[0], None);
    assert_eq!(shown["appVersion"], "4.0.10");
    assert_eq!(shown["appBuild"], "7");
    assert_eq!(shown["model"], "Samsung SM-A336B");
    assert_eq!(shown["os"], "Android 14");
    let again = Devices::load(&dir);
    assert_eq!(again.list()[0].model.as_deref(), Some("Samsung SM-A336B"));
}

#[test]
fn a_numeric_build_is_accepted_and_a_missing_field_keeps_the_old_value() {
    let mut d = device();
    assert!(d.apply_hello(&json!({ "appVersion": "1.0.0", "appBuild": 12, "model": "Pixel 7" })));
    assert_eq!(d.app_build.as_deref(), Some("12"));
    assert!(!d.apply_hello(&json!({ "platform": "android", "caps": [] })));
    assert_eq!(d.model.as_deref(), Some("Pixel 7"));
    assert!(d.apply_hello(&json!({ "model": "Pixel 8" })));
    assert_eq!(d.model.as_deref(), Some("Pixel 8"));
    assert_eq!(d.app_version.as_deref(), Some("1.0.0"));
}

#[test]
fn too_long_or_control_or_hidden_text_is_dropped() {
    let mut d = device();
    let long_version = "1".repeat(17);
    let long_model = "m".repeat(41);
    let hello = json!({ "appVersion": long_version, "appBuild": "1\u{0}2", "model": long_model, "os": "Android\n14" });
    assert!(!d.apply_hello(&hello));
    assert_eq!((d.app_version.clone(), d.app_build.clone(), d.model.clone(), d.os.clone()), (None, None, None, None));
    assert!(!d.apply_hello(&json!({ "appVersion": ["1"], "model": { "a": 1 }, "os": true, "appBuild": "" })));
    assert_eq!(clean_text("a\u{202e}b", 10), None);
    assert_eq!(clean_text("  ok  ", 10).as_deref(), Some("ok"));
    assert_eq!(clean_text(&"x".repeat(16), 16).map(|s| s.len()), Some(16));
    assert_eq!(clean_text(&"é".repeat(40), 40).map(|s| s.chars().count()), Some(40));
}

#[test]
fn caps_keep_only_short_distinct_plain_strings() {
    assert_eq!(parse_caps(&json!({ "caps": ["chat", "chat", "pty.snapshot", 5, "", "bad cap", "x".repeat(33)] })), vec!["chat", "pty.snapshot"]);
    assert!(parse_caps(&json!({})).is_empty());
    assert!(parse_caps(&json!({ "caps": "chat" })).is_empty());
    let many: Vec<String> = (0..40).map(|i| format!("c{i}")).collect();
    assert_eq!(parse_caps(&json!({ "caps": many })).len(), 32);
}

fn paired(rig: &Rig, phone: &Phone) {
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
}

async fn say_hello(rig: &Rig, id: &str, hello: Value) -> Value {
    let phone = Phone::new(id);
    paired(rig, &phone);
    let (mut ws, mut t) = session(rig.addr(), &rig.desktop_key(), &phone).await.unwrap();
    send_json(&mut ws, &mut t, hello).await;
    recv_json(&mut ws, &mut t).await.unwrap()
}

#[test]
fn a_hello_with_the_new_fields_fills_the_device_and_the_welcome_lists_desktop_caps() {
    let rig = rig("info-hello", "", false);
    rig.remote.enable(true);
    let welcome = runtime().block_on(say_hello(&rig, "p1", json!({ "id": "h1", "t": "hello", "proto": 1, "app": "orrery-remote", "platform": "android", "caps": ["chat"], "appVersion": "4.0.10", "appBuild": "3", "model": "Samsung SM-A336B", "os": "Android 14" })));
    assert_eq!(welcome["ok"], true);
    assert_eq!(welcome["caps"], json!(["devinfo", "checklist", "clip"]));
    let devices = rig.remote.status()["devices"].clone();
    let shown = devices.as_array().unwrap().iter().find(|d| d["deviceId"] == "p1").unwrap().clone();
    assert_eq!(shown["appVersion"], "4.0.10");
    assert_eq!(shown["appBuild"], "3");
    assert_eq!(shown["model"], "Samsung SM-A336B");
    assert_eq!(shown["os"], "Android 14");
    rig.remote.enable(false);
}

#[test]
fn an_old_hello_without_the_new_fields_still_works() {
    let rig = rig("info-old-hello", "", false);
    rig.remote.enable(true);
    let welcome = runtime().block_on(say_hello(&rig, "p2", json!({ "id": "h1", "t": "hello", "proto": 1, "app": "old", "platform": "android" })));
    assert_eq!(welcome["ok"], true);
    assert_eq!(welcome["caps"], json!(["devinfo", "checklist", "clip"]));
    let devices = rig.remote.status()["devices"].clone();
    let shown = devices.as_array().unwrap().iter().find(|d| d["deviceId"] == "p2").unwrap().clone();
    assert!(shown.get("model").is_none());
    rig.remote.enable(false);
}
