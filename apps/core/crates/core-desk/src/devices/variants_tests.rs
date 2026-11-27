use super::variants::*;
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

type Hook = Box<dyn Fn(&str, &[&str]) -> Option<String> + Send + Sync>;

struct Fake {
    log: Mutex<Vec<String>>,
    hook: Hook,
}

impl Shell for Fake {
    fn sh(&self, id: &str, args: &[&str]) -> Option<String> {
        self.log.lock().unwrap().push(format!("{id}: {}", args.join(" ")));
        (self.hook)(id, args)
    }
}

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-variants-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn device(night: &'static str, font: &'static str) -> Hook {
    Box::new(move |_, a| match a {
        ["cmd", "uimode", "night"] => Some(format!("Night mode: {night}\n")),
        ["settings", "get", "system", "font_scale"] => Some(format!("{font}\n")),
        _ => Some(String::new()),
    })
}

fn make(tag: &str, hook: Hook) -> (Arc<Variants>, Arc<Fake>, PathBuf) {
    let dir = tmp(tag);
    let fake = Arc::new(Fake { log: Mutex::new(Vec::new()), hook });
    (Variants::new(fake.clone(), dir.clone()), fake, dir)
}

fn dark_big() -> Request {
    Request { night: Some(true), font: Some(1.3) }
}

fn writes(f: &Fake) -> Vec<String> {
    f.log.lock().unwrap().iter().filter(|l| l.contains(" put ") || l.contains(" delete ") || l.ends_with("night yes") || l.ends_with("night no") || l.ends_with("night auto")).cloned().collect()
}

#[test]
fn apply_saves_the_old_values_to_disk_before_changing_anything() {
    let dir = tmp("order");
    let seen = Arc::new(Mutex::new(Vec::<bool>::new()));
    let (s, d) = (seen.clone(), dir.clone());
    let hook: Hook = Box::new(move |id, a| match a {
        ["cmd", "uimode", "night"] => Some("Night mode: no\n".into()),
        ["settings", "get", "system", "font_scale"] => Some("1.0\n".into()),
        _ => {
            s.lock().unwrap().push(d.join(file_name(id)).exists());
            Some(String::new())
        }
    });
    let fake = Arc::new(Fake { log: Mutex::new(Vec::new()), hook });
    let v = Variants::new(fake.clone(), dir.clone());
    assert_eq!(v.apply("R58M", &dark_big())["ok"], json!(true));
    assert_eq!(*seen.lock().unwrap(), vec![true, true]);
    let saved = v.pending();
    assert_eq!(saved, vec![Saved { serial: "R58M".into(), night: Some("no".into()), font: Some(Some("1.0".into())) }]);
    assert_eq!(writes(&fake), vec!["R58M: cmd uimode night yes", "R58M: settings put system font_scale 1.3"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn restore_gives_back_the_old_values_and_removes_the_file() {
    let (v, fake, dir) = make("restore", device("no", "1.15"));
    v.apply("R58M", &dark_big());
    let r = v.restore("R58M");
    assert_eq!(r, json!({"ok": true, "restored": true}));
    let w = writes(&fake);
    assert_eq!(&w[2..], ["R58M: cmd uimode night no", "R58M: settings put system font_scale 1.15"]);
    assert!(v.pending().is_empty());
    assert!(!dir.join(file_name("R58M")).exists());
    assert_eq!(v.restore("R58M"), json!({"ok": true, "restored": false}));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unset_font_scale_is_deleted_again_and_custom_night_becomes_auto() {
    let (v, fake, dir) = make("unset", device("custom", "null"));
    v.apply("emu-5554", &dark_big());
    v.restore("emu-5554");
    let w = writes(&fake);
    assert_eq!(&w[2..], ["emu-5554: cmd uimode night auto", "emu-5554: settings delete system font_scale"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_second_apply_keeps_the_first_originals() {
    let (v, _, dir) = make("twice", device("no", "1.0"));
    v.apply("R58M", &Request { night: Some(true), font: None });
    v.apply("R58M", &Request { night: Some(false), font: Some(1.3) });
    let saved = v.pending();
    assert_eq!(saved[0].night.as_deref(), Some("no"));
    assert_eq!(saved[0].font, Some(Some("1.0".into())));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn restore_on_startup_only_for_ready_devices_not_in_use_by_this_process() {
    let (first, _, dir) = make("boot", device("yes", "1.0"));
    first.apply("A1", &dark_big());
    first.apply("B2", &dark_big());
    let fake2 = Arc::new(Fake { log: Mutex::new(Vec::new()), hook: device("yes", "1.3") });
    let second = Variants::new(fake2.clone(), dir.clone());
    let ready = vec!["A1".to_string()];
    assert_eq!(second.restore_ready(&ready, Instant::now()), vec!["A1".to_string()]);
    assert_eq!(second.pending().iter().map(|s| s.serial.clone()).collect::<Vec<_>>(), vec!["B2".to_string()]);
    assert!(writes(&fake2).iter().all(|l| l.starts_with("A1")));
    assert!(first.restore_ready(&["B2".to_string()], Instant::now()).is_empty());
    let later = Instant::now() + ACTIVE_MAX + std::time::Duration::from_secs(1);
    assert_eq!(first.restore_ready(&["B2".to_string()], later), vec!["B2".to_string()]);
    assert!(first.pending().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_restore_keeps_the_file_for_the_next_time() {
    let (v, _, dir) = make("keep", Box::new(|_, a| if a.contains(&"put") && a.contains(&"1.0") { None } else { device("no", "1.0")("x", a) }));
    v.apply("R58M", &dark_big());
    let r = v.restore("R58M");
    assert_eq!(r["error"], json!("restore-failed"));
    assert_eq!(v.pending().len(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_change_rolls_back_and_a_failed_read_writes_nothing() {
    let (v, fake, dir) = make("fail", Box::new(|_, a| if a.contains(&"1.3") { None } else { device("no", "1.0")("x", a) }));
    let r = v.apply("R58M", &dark_big());
    assert_eq!(r["error"], json!("apply-failed"));
    assert_eq!(r["detail"], json!("restored"));
    assert!(v.pending().is_empty());
    assert_eq!(writes(&fake).last().map(String::as_str), Some("R58M: settings put system font_scale 1.0"));
    let (w, _, dir2) = make("noread", Box::new(|_, _| None));
    assert_eq!(w.apply("R58M", &dark_big())["error"], json!("read-failed"));
    assert!(!dir2.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rejects_odd_requests() {
    assert_eq!(clean_request(&json!({"night": true})), Some(Request { night: Some(true), font: None }));
    assert_eq!(clean_request(&json!({"fontScale": 1.3})), Some(Request { night: None, font: Some(1.3) }));
    assert_eq!(clean_request(&json!({"fontScale": 1.234})), Some(Request { night: None, font: Some(1.23) }));
    assert_eq!(clean_request(&json!({"fontScale": 0.4})), None);
    assert_eq!(clean_request(&json!({"fontScale": 3.1})), None);
    assert_eq!(clean_request(&json!({"fontScale": "1.3"})), None);
    assert_eq!(clean_request(&json!({"night": "yes"})), None);
    assert_eq!(clean_request(&json!({"night": 1})), None);
    assert_eq!(clean_request(&json!({})), None);
    assert_eq!(clean_request(&json!({"night": null, "fontScale": null})), None);
    assert_eq!(clean_request(&json!("yes")), None);
}

#[test]
fn file_names_are_safe_for_any_serial() {
    assert_eq!(file_name("R58M"), "R58M.json");
    assert_eq!(file_name("192.168.1.5:5555"), "192~2e168~2e1~2e5~3a5555.json");
    assert_eq!(file_name("adb-X-abc._adb-tls-connect._tcp"), "adb-X-abc~2e_adb-tls-connect~2e_tcp.json");
    assert!(!file_name("a/b\\c:d").contains(['/', '\\', ':']));
}

#[test]
fn a_corrupt_or_foreign_file_is_ignored_then_dropped_on_restore() {
    let (v, _, dir) = make("corrupt", device("no", "1.0"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(file_name("R58M")), "{not json").unwrap();
    std::fs::write(dir.join("other.json"), r#"{"serial":"X1","night":"maybe"}"#).unwrap();
    assert!(v.pending().is_empty());
    assert_eq!(v.restore("R58M")["error"], json!("corrupt"));
    assert!(!dir.join(file_name("R58M")).exists());
    std::fs::write(dir.join(file_name("R58M")), r#"{"serial":"OTHER","night":"yes"}"#).unwrap();
    assert_eq!(v.restore("R58M")["error"], json!("corrupt"));
    let _ = std::fs::remove_dir_all(dir);
}
