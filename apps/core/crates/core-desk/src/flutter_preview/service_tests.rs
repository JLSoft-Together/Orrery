use super::service::{Deps, FlutterPreview, OnPubspec, OnSave};
use super::watcher::Unwatch;
use crate::run::proc::process_env;
use crate::run::types::RunSettings;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PUBSPEC: &str = "name: demo\ndependencies:\n  flutter:\n    sdk: flutter\n";
const ID: &str = "lib/a.dart#a";
const WIDGET_LINE: &str = r#"printf '%s\n' '[{"event":"app.log","params":{"log":"I/flutter ( 4242): ccm.error {\"id\":\"lib/a.dart#a\",\"message\":\"Bad state: sk-aaaaaaaaaaaaaaaaaaaa at /home/u/app\\nnext\"}"}}]'"#;
const EARLY_LINE: &str = r#"printf '%s\n' '[{"event":"app.log","params":{"log":"flutter: ccm.error {\"id\":\"lib/a.dart#a\",\"message\":\"early\"}"}}]'"#;

fn script() -> String {
    format!(
        r#"#!/bin/sh
PATH=/usr/bin:/bin
D="$(dirname "$0")/.."
echo "$@" >> "$D/calls.log"
if [ "$1" = pub ]; then
  [ -f "$D/pub_fail" ] && {{ echo 'Error: version solving failed.'; exit 1; }}
  [ -f "$D/pub_slow" ] && sleep 1
  [ -f "$D/pub_hang" ] && sleep 30
  exit 0
fi
[ -f "$D/build_fail" ] && {{ echo 'lib/a.dart:3:5: Error: Undefined name x.'; exit 1; }}
[ -f "$D/early_err" ] && {EARLY_LINE}
echo '[{{"event":"app.start","params":{{"appId":"app1"}}}}]'
echo '[{{"event":"app.started","params":{{"appId":"app1"}}}}]'
[ -f "$D/lost" ] && {{ echo 'Lost connection to device.'; exit 0; }}
[ -f "$D/die" ] && exit 1
[ -f "$D/widget_err" ] && {WIDGET_LINE}
while IFS= read -r line; do
  echo "$line" >> "$D/reqs.log"
  id=$(printf '%s' "$line" | sed 's/^\[{{"id":\([0-9]*\),.*/\1/')
  case "$line" in
    *app.stop*) echo "[{{\"id\":$id,\"result\":true}}]"; [ -f "$D/ignore_stop" ] || exit 0;;
    *app.restart*)
      if [ -f "$D/reload_err" ]; then echo '[{{"event":"daemon.logMessage","params":{{"level":"error","message":"lib/a.dart:4:2: Error: Expected a type."}}}}]'; fi
      if [ -f "$D/reload_err" ] || [ -f "$D/reload_fail" ]; then echo "[{{\"id\":$id,\"result\":{{\"code\":1,\"message\":\"Reload rejected\"}}}}]"; else echo "[{{\"id\":$id,\"result\":{{\"code\":0}}}}]"; fi;;
    *callServiceExtension*)
      if [ -f "$D/show_unknown" ]; then echo "[{{\"id\":$id,\"result\":{{\"ok\":false}}}}]"; else echo "[{{\"id\":$id,\"result\":{{\"ok\":true}}}}]"; fi;;
  esac
done
"#
    )
}

type Slot<T> = Arc<Mutex<Option<T>>>;

struct H {
    svc: FlutterPreview,
    root: PathBuf,
    sdk: PathBuf,
    save: Slot<OnSave>,
    pubspec: Slot<OnPubspec>,
    emitted: Arc<Mutex<Vec<Value>>>,
}

fn dir(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!("fwp-svc-{name}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(file: &Path, text: &str) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

fn harness(name: &str, pubspec: &str, use_sdk: bool) -> H {
    use std::os::unix::fs::PermissionsExt;
    let base = dir(name);
    let root = base.join("proj");
    let sdk = base.join("sdk");
    put(&root.join("pubspec.yaml"), pubspec);
    put(&root.join("lib/a.dart"), "@Preview()\nWidget a() => const Text('a');\n");
    put(&sdk.join("bin/flutter"), &script());
    std::fs::set_permissions(sdk.join("bin/flutter"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let (save, pubspec_slot): (Slot<OnSave>, Slot<OnPubspec>) = Default::default();
    let emitted = Arc::new(Mutex::new(Vec::new()));
    let (s, p, e) = (save.clone(), pubspec_slot.clone(), emitted.clone());
    let sdk_s = sdk.to_string_lossy().to_string();
    let mut env = process_env();
    env.insert("PATH".into(), String::new());
    let deps = Deps {
        env,
        win: false,
        platform: "linux",
        home: "/home/u".into(),
        settings: Arc::new(move || RunSettings { flutter_sdk: use_sdk.then(|| sdk_s.clone()), ..RunSettings::new() }),
        emit: Arc::new(move |v| e.lock().unwrap().push(v)),
        watch: Arc::new(move |_, cb| {
            *s.lock().unwrap() = Some(cb);
            let s2 = s.clone();
            Box::new(move || *s2.lock().unwrap() = None) as Unwatch
        }),
        watch_pubspec: Arc::new(move |_, cb| {
            *p.lock().unwrap() = Some(cb);
            let p2 = p.clone();
            Box::new(move || *p2.lock().unwrap() = None) as Unwatch
        }),
        stop_timeout: Duration::from_millis(300),
        log_flush: Duration::from_millis(5),
        pub_get_timeout: Duration::from_secs(10),
    };
    H { svc: FlutterPreview::new(deps), root, sdk, save, pubspec: pubspec_slot, emitted }
}

impl H {
    fn root(&self) -> String {
        self.root.to_string_lossy().to_string()
    }

    fn flag(&self, name: &str) {
        put(&self.sdk.join(name), "");
    }

    fn file(&self, name: &str) -> String {
        std::fs::read_to_string(self.sdk.join(name)).unwrap_or_default()
    }

    fn state(&self) -> Value {
        self.svc.get()
    }

    fn start(&self, flavor: Option<&str>) -> Value {
        self.svc.start(&self.root(), "emu-1", flavor)
    }

    fn save(&self, changed: &[&str]) {
        let cb = self.save.lock().unwrap().clone().expect("watching lib");
        cb(changed.iter().map(|s| s.to_string()).collect());
    }

    fn pubspec_changed(&self) {
        let cb = self.pubspec.lock().unwrap().clone().expect("watching pubspec");
        cb();
    }

    fn host_file(&self) -> String {
        std::fs::read_to_string(self.root.join(".dart_tool/ccm_preview/main.dart")).unwrap_or_default()
    }
}

fn wait(what: &str, mut ok: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(15);
    while Instant::now() < end {
        if ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    panic!("timeout: {what}");
}

fn wait_state(h: &H, state: &str) {
    wait(state, || h.state()["state"] == state);
}

#[test]
fn start_runs_flutter_on_the_host_then_show_calls_the_extension() {
    let h = harness("start", PUBSPEC, true);
    assert_eq!(h.svc.show(ID), json!({"ok": true}));
    assert_eq!(h.state()["current"], ID);
    assert_eq!(h.start(Some("dev")), json!({"ok": true}));
    wait_state(&h, "running");
    assert!(h.emitted.lock().unwrap().iter().any(|v| v["state"] == "starting") && h.emitted.lock().unwrap().iter().any(|v| v["state"] == "running"));
    let calls = h.file("calls.log");
    assert!(calls.starts_with(&format!("run --machine -t {}/.dart_tool/ccm_preview/main.dart -d emu-1 --debug --flavor dev", h.root())), "{calls}");
    assert!(h.host_file().contains("'lib/a.dart#a': () => p0.a(),"));
    let s = h.state();
    assert_eq!((s["appId"].clone(), s["current"].clone(), s["entries"].as_array().map(Vec::len)), (json!("app1"), json!(ID), Some(1)));
    assert!(h.save.lock().unwrap().is_some() && h.pubspec.lock().unwrap().is_some());
    wait("show on start", || h.file("reqs.log").contains(r#""methodName":"ext.ccm.show""#));
    h.flag("show_unknown");
    assert_eq!(h.svc.show("lib/a.dart#other"), json!({"ok": false, "error": {"code": "unknown-id"}}));
    h.svc.stop();
}

#[test]
fn saving_a_dart_file_rescans_rewrites_host_and_reloads() {
    let h = harness("save", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    h.save(&["/proj/lib/readme.md"]);
    std::thread::sleep(Duration::from_millis(150));
    assert!(!h.file("reqs.log").contains("app.restart"));
    put(&h.root.join("lib/b.dart"), "@Preview()\nWidget b() => const Text('b');\n");
    h.save(&["/proj/lib/b.dart"]);
    wait("restart", || h.file("reqs.log").contains("app.restart"));
    wait("entries", || h.state()["state"] == "running" && h.state()["entries"].as_array().map(Vec::len) == Some(2));
    assert!(h.host_file().contains("p1.b()"));
    assert!(h.file("reqs.log").contains(r#""fullRestart":false"#) && h.file("reqs.log").contains(r#""reason":"save""#));
    h.svc.stop();
}

#[test]
fn stop_sends_app_stop_and_ends_stopped() {
    let h = harness("stop", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    h.svc.stop();
    assert_eq!(h.state()["state"], "stopped");
    assert!(h.file("reqs.log").contains("app.stop"));
    assert!(h.save.lock().unwrap().is_none());
}

#[test]
fn stop_kills_after_the_timeout_when_app_stop_is_ignored() {
    let h = harness("kill", PUBSPEC, true);
    h.flag("ignore_stop");
    h.start(None);
    wait_state(&h, "running");
    let t = Instant::now();
    h.svc.stop();
    assert!(t.elapsed() >= Duration::from_millis(250));
    assert_eq!(h.state()["state"], "stopped");
}

#[test]
fn build_error_before_start_fails_with_the_parsed_error() {
    let h = harness("build", PUBSPEC, true);
    h.flag("build_fail");
    h.start(None);
    wait_state(&h, "failed");
    let s = h.state();
    assert_eq!(s["error"]["code"], "build");
    assert_eq!((s["error"]["build"]["line"].clone(), s["error"]["build"]["col"].clone(), s["error"]["build"]["severity"].clone()), (json!(3), json!(5), json!("error")));
    assert!(s["log"].as_array().unwrap().iter().any(|l| l == "lib/a.dart:3:5: Error: Undefined name x."));
}

#[test]
fn missing_sdk_and_non_flutter_fail_before_spawning() {
    let none = harness("nosdk", PUBSPEC, false);
    let r = none.start(None);
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["code"], "flutter-missing");
    assert_eq!(none.state()["state"], "failed");
    let dart = harness("dart", "name: pure\n", true);
    assert_eq!(dart.start(None), json!({"ok": false, "error": {"code": "not-flutter"}}));
    assert_eq!(dart.file("calls.log"), "");
    assert_eq!(none.start(Some("bad flavor"))["error"]["code"], "bad-arg");
}

#[test]
fn host_wrappers_go_into_the_host_and_the_log() {
    let h = harness("wrap", &format!("{PUBSPEC}  flutter_riverpod: ^3.0.0\n"), true);
    h.start(None);
    wait_state(&h, "running");
    assert!(h.host_file().contains("ccm_riverpod.ProviderScope("));
    assert!(h.state()["log"].as_array().unwrap().iter().any(|l| l == "> preview wrappers: ProviderScope"));
    h.svc.stop();
}

#[test]
fn hot_reload_error_surfaces_and_clears_on_the_next_good_reload() {
    let h = harness("reload", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    let restarts = || h.file("reqs.log").matches("app.restart").count();
    h.flag("reload_err");
    h.save(&["/p/lib/a.dart"]);
    wait("build error", || h.state()["state"] == "running" && h.state()["error"]["code"] == "build");
    assert_eq!((h.state()["error"]["build"]["line"].clone(), h.state()["error"]["build"]["col"].clone()), (json!(4), json!(2)));
    std::fs::remove_file(h.sdk.join("reload_err")).unwrap();
    h.flag("reload_fail");
    h.save(&["/p/lib/a.dart"]);
    wait("reload-failed", || restarts() == 2 && h.state()["state"] == "running" && h.state()["error"]["code"] == "reload-failed");
    assert_eq!(h.state()["error"], json!({"code": "reload-failed", "detail": "Reload rejected"}));
    std::fs::remove_file(h.sdk.join("reload_fail")).unwrap();
    h.save(&["/p/lib/a.dart"]);
    wait("clear", || restarts() == 3 && h.state()["state"] == "running" && h.state()["error"].is_null());
    h.svc.stop();
}

#[test]
fn unexpected_exit_reports_device_lost_or_exited() {
    let lost = harness("lost", PUBSPEC, true);
    lost.flag("lost");
    lost.start(None);
    wait("device-lost", || lost.state()["state"] == "stopped" && lost.state()["error"]["code"] == "device-lost");
    let gone = harness("gone", PUBSPEC, true);
    gone.flag("die");
    gone.start(None);
    wait("exited", || gone.state()["error"] == json!({"code": "exited", "detail": "1"}));
    assert_eq!(gone.state()["state"], "stopped");
}

#[test]
fn show_while_not_running_remembers_the_selection() {
    let h = harness("idle", PUBSPEC, true);
    assert_eq!(h.svc.show(ID), json!({"ok": true}));
    assert_eq!(h.state()["current"], ID);
}

#[test]
fn kill_now_ends_the_session() {
    let h = harness("killnow", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    h.svc.kill_now();
    wait_state(&h, "stopped");
    assert!(h.save.lock().unwrap().is_none());
}

#[test]
fn widget_error_marker_becomes_a_masked_panel_error_cleared_by_the_next_show() {
    let h = harness("widget", PUBSPEC, true);
    h.flag("early_err");
    h.flag("widget_err");
    h.start(None);
    wait("early log", || h.state()["log"].as_array().unwrap().iter().any(|l| l.as_str().is_some_and(|l| l.contains("early"))));
    wait("widget error", || h.state()["error"]["code"] == "widget");
    let s = h.state();
    assert_eq!(s["error"]["entry"], ID);
    assert_eq!(s["error"]["detail"], "Bad state: [masked] at ~/app next");
    assert!(!s["log"].to_string().contains("sk-aaaa"));
    assert_eq!(s["state"], "running");
    h.svc.show("lib/a.dart#b");
    assert!(h.state()["error"].is_null());
    h.svc.stop();
}

#[test]
fn pubspec_change_runs_pub_get_then_restarts_the_session() {
    let h = harness("pubget", PUBSPEC, true);
    h.flag("pub_slow");
    h.start(Some("dev"));
    wait_state(&h, "running");
    h.pubspec_changed();
    wait_state(&h, "syncing");
    assert!(h.file("calls.log").contains("pub get"));
    h.save(&["/p/lib/a.dart"]);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!h.file("reqs.log").contains("app.restart"));
    wait("second run", || h.file("calls.log").lines().filter(|l| l.starts_with("run ")).count() == 2);
    assert!(h.file("calls.log").lines().filter(|l| l.starts_with("run ")).all(|l| l.contains("-d emu-1") && l.ends_with("--flavor dev")));
    assert!(h.file("reqs.log").contains("app.stop"));
    wait_state(&h, "running");
    h.svc.stop();
}

#[test]
fn failed_pub_get_keeps_the_old_session_with_an_error() {
    let h = harness("pubfail", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    h.flag("pub_fail");
    h.pubspec_changed();
    wait("pub-get error", || h.state()["state"] == "running" && h.state()["error"]["code"] == "pub-get");
    assert_eq!(h.file("calls.log").lines().filter(|l| l.starts_with("run ")).count(), 1);
    h.svc.stop();
}

#[test]
fn stop_during_pub_get_kills_it() {
    let h = harness("pubstop", PUBSPEC, true);
    h.start(None);
    wait_state(&h, "running");
    h.flag("pub_hang");
    h.pubspec_changed();
    wait_state(&h, "syncing");
    let t = Instant::now();
    h.svc.stop();
    assert!(t.elapsed() < Duration::from_secs(10));
    assert_eq!(h.state()["state"], "stopped");
    assert!(h.pubspec.lock().unwrap().is_none());
}
