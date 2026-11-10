use super::*;
use crate::run::types::{BuildError, Hooks, RunDevice, RunSettings};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Default)]
pub(crate) struct Rec {
    pub states: Mutex<Vec<(String, SessionPatch)>>,
    pub logs: Mutex<Vec<(String, String, Option<String>)>>,
    pub errors: Mutex<Vec<BuildError>>,
    pub fails: Mutex<Vec<EnvIssue>>,
}

impl Hooks for Rec {
    fn state(&self, state: &str, patch: SessionPatch) {
        self.states.lock().unwrap().push((state.into(), patch));
    }
    fn fail(&self, f: EnvIssue) {
        self.fails.lock().unwrap().push(f);
    }
    fn log(&self, stream: &str, text: &str, level: Option<&str>, _tag: Option<&str>) {
        self.logs.lock().unwrap().push((stream.into(), text.into(), level.map(String::from)));
    }
    fn error(&self, e: BuildError) {
        self.errors.lock().unwrap().push(e);
    }
    fn confirm(&self, _: &str, _: Option<&str>) -> bool {
        true
    }
}

impl Rec {
    fn list(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (s, _) in self.states.lock().unwrap().iter() {
            if out.last() != Some(s) {
                out.push(s.clone());
            }
        }
        out
    }
    fn last(&self) -> String {
        self.states.lock().unwrap().last().map(|s| s.0.clone()).unwrap_or_default()
    }
    fn has_log(&self, stream: &str, text: &str, level: Option<&str>) -> bool {
        self.logs.lock().unwrap().iter().any(|(s, t, l)| s == stream && t == text && l.as_deref() == level)
    }
}

static N: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ccm-fl-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    d
}

#[cfg(unix)]
pub(crate) fn script(path: &std::path::Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

pub(crate) fn wait_for(what: &str, f: impl Fn() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(10), "timeout waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct H {
    runner: Arc<dyn Runner>,
    rec: Arc<Rec>,
    sdk: PathBuf,
}

impl H {
    fn args(&self) -> String {
        std::fs::read_to_string(self.sdk.join("args")).unwrap_or_default()
    }
    fn sent(&self) -> String {
        std::fs::read_to_string(self.sdk.join("sent")).unwrap_or_default()
    }
    fn start(&self) -> std::thread::JoinHandle<()> {
        let r = self.runner.clone();
        std::thread::spawn(move || r.start())
    }
}

const BOOT: &str = r#"printf '%s\n' '[{"event":"daemon.connected","params":{"version":"0.6.1","pid":42}}]'
printf '%s\n' '[{"event":"app.start","params":{"appId":"app-1","deviceId":"emulator-5554","mode":"debug"}}]'
printf '%s\n' '[{"event":"app.progress","params":{"appId":"app-1","id":"p1","message":"Running Gradle task"}}]'
echo 'lib/main.dart:3:5: Warning: unused import'
printf '%s\n' '[{"event":"app.progress","params":{"appId":"app-1","id":"p1","finished":true}}]'
printf '%s\n' '[{"event":"app.debugPort","params":{"appId":"app-1","port":1,"wsUri":"ws://127.0.0.1:1/abc=/ws"}}]'
printf '%s\n' '[{"event":"app.devTools","params":{"uri":"http://127.0.0.1:9100"}}]'
printf '%s\n' '[{"event":"app.started","params":{"appId":"app-1"}}]'"#;

const LOOP: &str = r#"while read line; do
  echo "$line" >> "$D/sent"
  id=$(echo "$line" | sed -n 's/^\[{"id":\([0-9]*\).*/\1/p')
  case "$line" in
    *'"fullRestart":false'*) printf '[{"id":%s,"result":{"code":0,"message":""}}]\n' "$id" ;;
    *app.restart*) printf '[{"id":%s,"result":{"code":1,"message":"Hot reload was rejected"}}]\n' "$id" ;;
    *app.stop*) printf '%s\n' '[{"event":"app.stop","params":{"appId":"app-1"}}]'; exit 0 ;;
  esac
done"#;

#[cfg(unix)]
fn harness(cfg: Value, flutter: &str, sdk_found: bool, pub_get: bool) -> H {
    let sdk = tmp_dir("run");
    let body = format!("D='{}'\ncase \"$1\" in pub) {};; esac\necho \"$@\" >> \"$D/args\"\n{flutter}", sdk.display(), if flutter.contains("PUBFAIL") { "exit 1" } else { "exit 0" });
    script(&sdk.join("bin/flutter"), &body);
    script(&sdk.join("bin/dart"), "sleep 10");
    let rec = Arc::new(Rec::default());
    let mut config = json!({"id": "c1", "name": "main", "type": "flutter", "cwd": sdk.to_string_lossy(), "env": {}, "beforeLaunch": [], "entrypoint": "lib/main.dart", "device": "selected", "mode": "debug", "flavor": "", "args": ""});
    for (k, v) in cfg.as_object().unwrap() {
        config[k] = v.clone();
    }
    let ctx = RunContext {
        root: sdk.to_string_lossy().into(),
        config,
        device: Some(RunDevice { id: "emulator-5554".into(), name: "Pixel".into(), platform: "android-x64".into(), emulator: true, state: "ready".into(), source: "flutter".into() }),
        settings: RunSettings::new(),
        env: crate::run::proc::process_env(),
        win: false,
        home: sdk.to_string_lossy().into(),
        hooks: rec.clone(),
        on_adb_use: None,
    };
    let root = sdk.to_string_lossy().to_string();
    let find: FindSdkFn = Arc::new(move |_, _| {
        if sdk_found {
            FlutterSdkLookup { sdk: Some(root.clone()), source: Some("setting"), fvm: None, issues: vec![] }
        } else {
            FlutterSdkLookup { sdk: None, source: None, fvm: None, issues: vec![EnvIssue::new("flutter-missing")] }
        }
    });
    let deps = RunnerDeps { find_sdk: Some(find), needs_pub_get: Some(Arc::new(move |_| pub_get)), stop_timeout_ms: Some(150), command: None };
    H { runner: create_with(ctx, deps), rec, sdk }
}

#[cfg(unix)]
#[test]
fn boot_logs_reload_and_stop() {
    let h = harness(json!({}), &format!("{BOOT}\n{LOOP}"), true, false);
    let t = h.start();
    wait_for("running", || h.rec.last() == "running");
    assert_eq!(h.args().trim(), "run --machine -d emulator-5554 -t lib/main.dart --debug");
    assert_eq!(h.rec.list(), vec!["starting", "running"]);
    let states = h.rec.states.lock().unwrap().clone();
    assert!(states.iter().any(|(_, p)| p.app_id == Some(Some("app-1".into()))));
    assert!(states.iter().any(|(_, p)| p.vm_service_uri == Some(Some("ws://127.0.0.1:1/abc=/ws".into()))));
    assert!(states.iter().any(|(_, p)| p.devtools_uri == Some(Some("http://127.0.0.1:9100".into()))));
    assert!(states.iter().any(|(_, p)| p.progress == Some(Some("Running Gradle task".into()))));
    let e = h.rec.errors.lock().unwrap().clone();
    assert_eq!((e.len(), e[0].line, e[0].col, e[0].severity.as_str()), (1, 3, 5, "warning"));
    assert!(h.rec.has_log("build", "lib/main.dart:3:5: Warning: unused import", None));
    assert!(h.rec.has_log("build", "flutter daemon connected (pid 42)", None));
    assert_eq!(h.runner.reload(false), RunResult::ok());
    assert_eq!(h.rec.last(), "running");
    assert!(h.rec.list().contains(&"reloading".to_string()));
    assert_eq!(h.runner.reload(true), RunResult::Err(EnvIssue::with("reload-failed", "Hot reload was rejected")));
    assert!(h.rec.has_log("app", "Hot reload was rejected", Some("E")));
    let sent = h.sent();
    assert!(sent.contains(r#""method":"app.restart","params":{"appId":"app-1","fullRestart":false,"pause":false,"reason":"manual"}"#));
    assert!(sent.contains(r#"[{"id":2,"#));
    h.runner.stop();
    assert!(h.sent().contains(r#""method":"app.stop","params":{"appId":"app-1"}"#));
    t.join().unwrap();
    h.runner.stop();
    assert_eq!(h.rec.list().iter().rev().take(2).cloned().collect::<Vec<_>>(), vec!["stopped", "stopping"]);
}

#[cfg(unix)]
#[test]
fn app_log_lines_and_plain_text() {
    let body = format!(
        "{BOOT}\nprintf '%s\\n' '[{{\"event\":\"app.log\",\"params\":{{\"appId\":\"app-1\",\"log\":\"flutter: hello\"}}}}]'\nprintf '%s\\n' '[{{\"event\":\"app.log\",\"params\":{{\"appId\":\"app-1\",\"log\":\"boom\",\"error\":true}}}}]'\necho 'plain after start'\n{LOOP}"
    );
    let h = harness(json!({}), &body, true, false);
    let t = h.start();
    wait_for("plain", || h.rec.has_log("app", "plain after start", None));
    assert!(h.rec.has_log("app", "flutter: hello", None));
    assert!(h.rec.has_log("app", "boom", Some("E")));
    h.runner.kill();
    t.join().unwrap();
    assert_eq!(h.rec.last(), "stopped");
}

#[cfg(unix)]
#[test]
fn late_started_after_stop_does_not_revive() {
    let body = r#"printf '%s\n' '[{"event":"app.start","params":{"appId":"app-1"}}]'
read line
printf '%s\n' '[{"event":"app.progress","params":{"appId":"app-1","message":"Syncing files"}}]'
printf '%s\n' '[{"event":"app.started","params":{"appId":"app-1"}}]'
sleep 0.2
exit 0"#;
    let h = harness(json!({}), body, true, false);
    let t = h.start();
    wait_for("app.start", || h.rec.states.lock().unwrap().iter().any(|(_, p)| p.app_id.is_some()));
    h.runner.stop();
    t.join().unwrap();
    assert_eq!(h.rec.list(), vec!["starting", "stopping", "stopped"]);
}

#[cfg(unix)]
#[test]
fn masks_env_in_logged_command() {
    let h = harness(json!({"args": "--dart-define=TOKEN=${env:SECRET}", "env": {"SECRET": "s3cr3t"}}), "sleep 10", true, false);
    let t = h.start();
    wait_for("args", || h.args().contains("TOKEN"));
    assert!(h.args().contains("--dart-define=TOKEN=s3cr3t"));
    let logged = h.rec.logs.lock().unwrap().iter().find(|l| l.1.starts_with("> flutter")).unwrap().1.clone();
    assert!(logged.contains("--dart-define=TOKEN=***") && !logged.contains("s3cr3t"));
    h.runner.kill();
    t.join().unwrap();
}

#[cfg(unix)]
#[test]
fn exits_before_start() {
    let h = harness(json!({}), "echo 'lib/main.dart:10:3: Error: Expected a type.' 1>&2\nexit 1", true, false);
    h.runner.start();
    let e = h.rec.errors.lock().unwrap()[0].clone();
    assert_eq!((e.line, e.col, e.severity.as_str()), (10, 3, "error"));
    assert_eq!(*h.rec.fails.lock().unwrap(), vec![EnvIssue::new("build")]);
    assert_eq!(h.rec.list(), vec!["starting", "failed"]);
    let h2 = harness(json!({}), "exit 3", true, false);
    h2.runner.start();
    assert_eq!(*h2.rec.fails.lock().unwrap(), vec![EnvIssue::with("exit", "3")]);
    let h3 = harness(json!({}), "exit 0", false, false);
    h3.runner.start();
    assert_eq!(h3.args(), "");
    assert_eq!(*h3.rec.fails.lock().unwrap(), vec![EnvIssue::with("spawn", "flutter-missing")]);
    assert_eq!(h3.rec.list(), vec!["failed"]);
}

#[cfg(unix)]
#[test]
fn reload_unsupported_outside_debug_and_stop_timeout() {
    let body = format!("{BOOT}\ntrap '' TERM\nwhile true; do sleep 1; done");
    let h = harness(json!({"mode": "release"}), &body, true, false);
    let t = h.start();
    wait_for("running", || h.rec.last() == "running");
    assert_eq!(h.runner.reload(false), RunResult::err("reload-unsupported"));
    h.runner.stop();
    t.join().unwrap();
    assert_eq!(h.rec.last(), "stopped");
}

#[cfg(unix)]
#[test]
fn device_lost_after_running_ends_stopped() {
    let h = harness(json!({}), &format!("{BOOT}\necho 'Lost connection to device.' 1>&2\nexit 1"), true, false);
    h.runner.start();
    assert!(h.rec.fails.lock().unwrap().is_empty());
    assert_eq!(h.rec.last(), "stopped");
}

#[cfg(unix)]
#[test]
fn pub_get_first_and_failure() {
    let h = harness(json!({}), "exit 0", true, true);
    h.runner.start();
    assert_eq!(h.rec.states.lock().unwrap()[0], ("before-launch".to_string(), SessionPatch { progress: Some(Some("flutter pub get".into())), ..Default::default() }));
    assert!(h.rec.has_log("build", "> flutter pub get", None));
    assert!(h.args().contains("--machine"));
    let h2 = harness(json!({}), "PUBFAIL", true, true);
    h2.runner.start();
    assert_eq!(*h2.rec.fails.lock().unwrap(), vec![EnvIssue::new("pub-get")]);
    assert_eq!(h2.args(), "");
}

#[cfg(unix)]
#[test]
fn before_launch_uses_sdk_and_stop_cancels() {
    let h = harness(json!({"beforeLaunch": ["flutter pub get", "dart run build_runner build"]}), "PUBFAIL", true, false);
    h.runner.start();
    assert_eq!(h.rec.states.lock().unwrap()[0].0, "before-launch");
    assert_eq!(*h.rec.fails.lock().unwrap(), vec![EnvIssue::with("before-launch", "flutter")]);
    assert_eq!(h.rec.last(), "failed");
    let h2 = harness(json!({"beforeLaunch": ["dart --version"]}), "exit 0", true, false);
    let t = h2.start();
    wait_for("dart step", || h2.rec.has_log("build", "> dart --version", None));
    std::thread::sleep(Duration::from_millis(100));
    h2.runner.stop();
    t.join().unwrap();
    assert!(h2.rec.fails.lock().unwrap().is_empty());
    assert_eq!(h2.rec.last(), "cancelled");
}

#[test]
fn sdk_tool_and_argv() {
    let env = Env::new();
    assert_eq!(sdk_tool("/sdk", "flutter", false, &env), Some(Resolved { file: "/sdk/bin/flutter".into(), args: vec![], verbatim: false }));
    assert_eq!(sdk_tool("/sdk", "dart.exe", false, &env).unwrap().file, "/sdk/bin/dart");
    assert_eq!(sdk_tool("/sdk", "./flutter", false, &env), None);
    assert_eq!(sdk_tool("/sdk", "melos", false, &env), None);
    let w = sdk_tool("C:\\sdk", "Flutter.BAT", true, &env).unwrap();
    assert_eq!((w.file.as_str(), w.verbatim), ("C:\\Windows\\System32\\cmd.exe", true));
    assert!(w.args[3].contains("C:\\sdk\\bin\\flutter.bat"));
    let cfg = json!({"device": "chrome", "entrypoint": "lib/main_dev.dart", "flavor": "dev", "mode": "profile", "args": "--x \"a b\""});
    let vars = SubstituteVars::default();
    assert_eq!(flutter_argv(&cfg, Some("emu"), &vars, "f"), vec!["f", "run", "--machine", "-d", "chrome", "-t", "lib/main_dev.dart", "--flavor", "dev", "--profile", "--x", "a b"]);
    let sel = json!({"device": "selected", "entrypoint": "lib/main.dart", "mode": "debug"});
    assert_eq!(flutter_argv(&sel, None, &vars, "f"), vec!["f", "run", "--machine", "-t", "lib/main.dart", "--debug"]);
}
