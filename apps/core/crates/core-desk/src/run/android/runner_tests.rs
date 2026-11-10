use super::*;
use crate::run::android::testutil::{script, tmp_dir, wait_for};
use crate::run::proc::is_alive;
use crate::run::types::{BuildError, Hooks, RunDevice, RunSettings};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

type LogLine = (String, String, Option<String>, Option<String>);

#[derive(Default)]
struct Rec {
    states: Mutex<Vec<(String, SessionPatch)>>,
    fails: Mutex<Vec<EnvIssue>>,
    logs: Mutex<Vec<LogLine>>,
    errors: Mutex<Vec<BuildError>>,
    confirms: Mutex<Vec<(String, Option<String>)>>,
    answer: bool,
}

impl Hooks for Rec {
    fn state(&self, state: &str, patch: SessionPatch) {
        self.states.lock().unwrap().push((state.into(), patch));
    }
    fn fail(&self, failure: EnvIssue) {
        self.fails.lock().unwrap().push(failure);
    }
    fn log(&self, stream: &str, text: &str, level: Option<&str>, tag: Option<&str>) {
        self.logs.lock().unwrap().push((stream.into(), text.into(), level.map(String::from), tag.map(String::from)));
    }
    fn error(&self, error: BuildError) {
        self.errors.lock().unwrap().push(error);
    }
    fn confirm(&self, code: &str, detail: Option<&str>) -> bool {
        self.confirms.lock().unwrap().push((code.into(), detail.map(String::from)));
        self.answer
    }
}

struct T {
    dir: String,
    cwd: String,
    rec: Arc<Rec>,
    runner: Arc<dyn Runner>,
    adb_uses: Arc<AtomicUsize>,
}

struct Opts<'a> {
    adb: &'a str,
    gradle: &'a str,
    with_apk: bool,
    java: &'a str,
    confirm: bool,
    before: Vec<&'a str>,
    timing: RunnerTiming,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Opts { adb: "", gradle: GRADLE_OK, with_apk: true, java: "17.0.9", confirm: true, before: Vec::new(), timing: RunnerTiming { pid_search_ms: 5, pid_watch_ms: 5, gone_ms: 40, adb_ms: 1_000, force_stop_ms: 1_000, no_pid_notice_ms: 60_000 } }
    }
}

const GRADLE_OK: &str = "printf '> Task :app:assembleFreeDebug\\nBUILD SUCCESSFUL in 3s\\n'";
const INSTALL_OK: &str = "*' install -r -t '*) printf 'Performing Streamed Install\\nSuccess\\n';;";
const RESOLVE: &str = "*resolve-activity*) printf 'priority=0 preferredOrder=0\\ncom.ex/.MainActivity\\n';;";
const AM_START: &str = "*' am start -n '*) printf 'Starting: Intent { cmp=com.ex/.MainActivity }\\n';;";
const LOGCAT_LIVE: &str = "*' logcat '*) echo $$ >> $F/logcat.pids; exec sleep 30;;";

fn pid_seq(pids: &[&str]) -> String {
    let mut arms = String::new();
    for (i, p) in pids.iter().enumerate() {
        let body = if p.is_empty() { "exit 1".to_string() } else { format!("echo {p}") };
        let pat = if i + 1 == pids.len() { "*".to_string() } else { i.to_string() };
        arms.push_str(&format!("{pat}) {body};; "));
    }
    format!("*' pidof -s com.ex') n=$(cat $F/pidn 2>/dev/null || echo 0); echo $((n+1)) > $F/pidn; case $n in {arms}esac;;")
}

fn setup(o: Opts) -> T {
    let dir = tmp_dir("arun");
    let cwd = format!("{dir}/proj");
    let out = format!("{cwd}/app/build/outputs/apk/free/debug");
    std::fs::create_dir_all(&out).unwrap();
    if o.with_apk {
        std::fs::write(format!("{out}/output-metadata.json"), r#"{"applicationId":"com.ex","variantName":"freeDebug","elements":[{"outputFile":"app-free-debug.apk"}]}"#).unwrap();
    }
    script(&format!("{dir}/sdk/platform-tools/adb"), &format!("F={dir}\necho \"adb $*\" >> $F/calls\ncase \"$*\" in\n{}\n*) exit 0;;\nesac", o.adb));
    script(&format!("{dir}/jdk/bin/java"), &format!("echo 'openjdk version \"{}\"' >&2", o.java));
    script(&format!("{cwd}/gradlew"), &format!("F={dir}\necho \"gradlew $*\" >> $F/calls\necho \"$JAVA_HOME|$ANDROID_HOME|$FOO\" > $F/gradle.env\n{}", o.gradle));
    let rec = Arc::new(Rec { answer: o.confirm, ..Default::default() });
    let adb_uses = Arc::new(AtomicUsize::new(0));
    let uses = adb_uses.clone();
    let config = json!({"id": "c1", "name": "app", "type": "android", "cwd": cwd, "shared": false, "env": {"FOO": "bar"}, "beforeLaunch": o.before, "module": ":app", "variant": "freeDebug", "device": "selected", "args": ""});
    let device = RunDevice { id: "emulator-5554".into(), name: "Pixel 7".into(), platform: "android".into(), emulator: true, state: "ready".into(), source: "adb".into() };
    let settings = RunSettings { android_sdk: Some(format!("{dir}/sdk")), jdk: Some(format!("{dir}/jdk")), ..RunSettings::new() };
    let env: Env = [("PATH".to_string(), "/usr/bin:/bin".to_string())].into_iter().collect();
    let ctx = RunContext {
        root: cwd.clone(),
        config,
        device: Some(device),
        settings,
        env,
        win: false,
        home: dir.clone(),
        hooks: rec.clone(),
        on_adb_use: Some(Arc::new(move |serial: &str| {
            assert_eq!(serial, "emulator-5554");
            uses.fetch_add(1, Ordering::SeqCst);
        })),
    };
    let runner = create_with_timing(ctx, o.timing);
    T { dir, cwd, rec, runner, adb_uses }
}

impl T {
    fn names(&self) -> Vec<String> {
        self.rec.states.lock().unwrap().iter().map(|s| s.0.clone()).collect()
    }
    fn fails(&self) -> Vec<EnvIssue> {
        self.rec.fails.lock().unwrap().clone()
    }
    fn keys(&self) -> Vec<String> {
        std::fs::read_to_string(format!("{}/calls", self.dir)).unwrap_or_default().lines().map(String::from).collect()
    }
    fn has(&self, key: &str) -> bool {
        self.keys().iter().any(|k| k == key)
    }
    fn logcat_pids(&self) -> Vec<u32> {
        std::fs::read_to_string(format!("{}/logcat.pids", self.dir)).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).collect()
    }
    fn spawn_start(&self) -> std::thread::JoinHandle<()> {
        let r = self.runner.clone();
        std::thread::spawn(move || r.start())
    }
    fn wait_state(&self, s: &str) -> bool {
        wait_for(5_000, || self.names().iter().any(|n| n == s))
    }
}

fn adb(parts: &[&str]) -> String {
    parts.join("\n")
}

#[test]
fn builds_installs_launches_and_follows_logcat() {
    let first_logcat = "*'--pid=1234'*) echo $$ >> $F/logcat.pids; printf '10-15 12:34:56.789  1234  1250 W MyTag   : hello there\\n--------- beginning of main\\n'; exec sleep 30;;";
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, RESOLVE, AM_START, &pid_seq(&["", "1234", "1234", "5678"]), first_logcat, LOGCAT_LIVE]), ..Default::default() });
    let h = t.spawn_start();
    assert!(t.wait_state("running"));
    assert_eq!(t.names(), vec!["building", "installing", "launching", "running"]);
    assert_eq!(t.rec.states.lock().unwrap().last().unwrap().1.app_id, Some(Some("com.ex".into())));
    assert!(t.fails().is_empty());
    assert!(t.has("gradlew :app:assembleFreeDebug --console=plain"));
    assert_eq!(std::fs::read_to_string(format!("{}/gradle.env", t.dir)).unwrap().trim(), format!("{0}/jdk|{0}/sdk|bar", t.dir));
    let apk = format!("{}/app/build/outputs/apk/free/debug/app-free-debug.apk", t.cwd);
    assert!(t.has(&format!("adb -s emulator-5554 install -r -t {apk}")));
    assert!(t.has("adb -s emulator-5554 shell am start -n com.ex/.MainActivity"));
    assert!(wait_for(5_000, || t.has("adb -s emulator-5554 logcat -v threadtime --pid=1234")));
    assert!(wait_for(5_000, || t.rec.logs.lock().unwrap().iter().any(|l| l == &("logcat".into(), "hello there".into(), Some("W".into()), Some("MyTag".into())))));
    assert!(!t.rec.logs.lock().unwrap().iter().any(|l| l.1.contains("beginning of")));
    assert!(wait_for(5_000, || t.has("adb -s emulator-5554 logcat -v threadtime --pid=5678")));
    assert!(wait_for(3_000, || t.logcat_pids().len() == 2));
    let first = t.logcat_pids()[0];
    assert!(wait_for(3_000, || !is_alive(first)));
    t.runner.stop();
    t.runner.stop();
    h.join().unwrap();
    assert!(t.has("adb -s emulator-5554 shell am force-stop com.ex"));
    assert_eq!(t.names()[t.names().len() - 2..], ["stopping".to_string(), "stopped".to_string()]);
    let second = t.logcat_pids()[1];
    assert!(wait_for(3_000, || !is_alive(second)));
    assert!(t.adb_uses.load(Ordering::SeqCst) >= 6);
}

#[test]
fn reports_kotlin_errors_and_fails_build() {
    let t = setup(Opts { gradle: "printf 'e: file:///C:/src/app/Main.kt:12:5 Unresolved reference: foo\\nBUILD FAILED\\n'; exit 1", ..Default::default() });
    t.runner.start();
    let errors = t.rec.errors.lock().unwrap().clone();
    assert_eq!(errors.len(), 1);
    assert_eq!((errors[0].line, errors[0].col, errors[0].message.as_str(), errors[0].severity.as_str()), (12, 5, "Unresolved reference: foo", "error"));
    assert_eq!(t.fails(), vec![EnvIssue::new("build")]);
    assert!(!t.keys().iter().any(|k| k.contains("install")));
}

#[test]
fn blocking_env_issue_before_spawning() {
    let t = setup(Opts { java: "11.0.2", ..Default::default() });
    t.runner.start();
    assert_eq!(t.fails(), vec![EnvIssue::with("jdk-old", "11")]);
    assert!(t.keys().is_empty());
}

#[test]
fn missing_output_metadata() {
    let t = setup(Opts { with_apk: false, ..Default::default() });
    t.runner.start();
    assert_eq!(t.fails(), vec![EnvIssue::with("install", "apk-missing")]);
}

#[test]
fn before_launch_failure() {
    let t = setup(Opts { before: vec!["/bin/sh -c 'exit 3'"], ..Default::default() });
    t.runner.start();
    assert_eq!(t.names(), vec!["before-launch"]);
    assert_eq!(t.fails(), vec![EnvIssue::new("before-launch")]);
    assert!(t.keys().is_empty());
}

const SIG_THEN_OK: &str = "*' install -r -t '*) if [ -f $F/installed ]; then printf 'Success\\n'; else touch $F/installed; printf 'Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: signatures do not match]\\n'; exit 1; fi;;";

#[test]
fn reinstalls_on_signature_mismatch_when_confirmed() {
    let t = setup(Opts { adb: &adb(&[SIG_THEN_OK, RESOLVE, AM_START, &pid_seq(&[""])]), ..Default::default() });
    let h = t.spawn_start();
    assert!(t.wait_state("running"));
    assert_eq!(*t.rec.confirms.lock().unwrap(), vec![("reinstall".to_string(), Some("com.ex".to_string()))]);
    assert!(t.has("adb -s emulator-5554 uninstall com.ex"));
    t.runner.kill();
    h.join().unwrap();
}

#[test]
fn signature_mismatch_declined() {
    let t = setup(Opts { adb: "*' install -r -t '*) printf 'Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE]\\n'; exit 1;;", confirm: false, ..Default::default() });
    t.runner.start();
    assert_eq!(t.fails(), vec![EnvIssue::with("install", "signature")]);
    assert!(!t.keys().iter().any(|k| k.contains("uninstall")));
}

#[test]
fn maps_other_install_failures() {
    let t = setup(Opts { adb: "*' install -r -t '*) printf 'Failure [INSTALL_FAILED_INSUFFICIENT_STORAGE]\\n'; exit 1;;", ..Default::default() });
    t.runner.start();
    assert_eq!(t.fails(), vec![EnvIssue::with("install", "storage")]);
}

#[test]
fn monkey_fallback_reports_missing_launcher() {
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, "*resolve-activity*) printf 'No activity found\\n';;", "*' monkey '*) printf '** No activities found to run, monkey aborted.\\n'; exit 252;;"]), ..Default::default() });
    t.runner.start();
    assert!(t.has("adb -s emulator-5554 shell monkey -p com.ex -c android.intent.category.LAUNCHER 1"));
    assert_eq!(t.fails(), vec![EnvIssue::with("launch", "no-launcher")]);
}

#[test]
fn stop_during_build_kills_gradle() {
    let t = setup(Opts { gradle: "echo $$ > $F/gradle.pid; exec sleep 30", ..Default::default() });
    let h = t.spawn_start();
    let pid_file = format!("{}/gradle.pid", t.dir);
    assert!(wait_for(5_000, || std::fs::read_to_string(&pid_file).map(|s| !s.trim().is_empty()).unwrap_or(false)));
    let pid: u32 = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
    t.runner.stop();
    h.join().unwrap();
    assert!(wait_for(3_000, || !is_alive(pid)));
    assert_eq!(t.names(), vec!["building", "stopping", "cancelled"]);
    assert!(t.fails().is_empty());
    assert!(!t.keys().iter().any(|k| k.contains("force-stop")));
}

#[test]
fn ends_when_app_process_gone() {
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, RESOLVE, AM_START, &pid_seq(&["1234", ""]), LOGCAT_LIVE]), ..Default::default() });
    t.runner.start();
    assert_eq!(t.names().last().map(String::as_str), Some("stopped"));
    assert!(wait_for(3_000, || t.logcat_pids().len() == 1));
    let pid = t.logcat_pids()[0];
    assert!(wait_for(3_000, || !is_alive(pid)));
}

#[test]
fn kill_tree_kills_children_synchronously() {
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, RESOLVE, AM_START, &pid_seq(&["1234"]), LOGCAT_LIVE]), ..Default::default() });
    let h = t.spawn_start();
    assert!(wait_for(5_000, || t.logcat_pids().len() == 1));
    t.runner.kill();
    h.join().unwrap();
    let pid = t.logcat_pids()[0];
    assert!(wait_for(3_000, || !is_alive(pid)));
    assert!(wait_for(3_000, || t.has("adb -s emulator-5554 shell am force-stop com.ex")));
}

#[test]
fn ps_fallback_when_pidof_missing() {
    let ps = "*' ps -A -o PID,NAME') printf '  PID NAME\\n  901 com.ex:remote\\n  877 com.ex\\n  612 system_server\\n';;";
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, RESOLVE, AM_START, "*' pidof -s '*) printf '/system/bin/sh: pidof: not found\\n'; exit 127;;", ps, LOGCAT_LIVE]), ..Default::default() });
    let h = t.spawn_start();
    assert!(wait_for(5_000, || t.has("adb -s emulator-5554 logcat -v threadtime --pid=877")));
    t.runner.kill();
    h.join().unwrap();
}

#[test]
fn logs_once_when_no_process_shows_up() {
    let t = setup(Opts { adb: &adb(&[INSTALL_OK, RESOLVE, AM_START, &pid_seq(&[""])]), timing: RunnerTiming { no_pid_notice_ms: 20, ..Opts::default().timing }, ..Default::default() });
    let h = t.spawn_start();
    let count = || t.rec.logs.lock().unwrap().iter().filter(|l| l.1.contains("logcat is not attached")).count();
    assert!(wait_for(5_000, || count() == 1));
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(count(), 1);
    assert_eq!(t.names().last().map(String::as_str), Some("running"));
    t.runner.kill();
    h.join().unwrap();
}

#[test]
fn gradle_args_and_config_checks() {
    let vars = SubstituteVars { project_dir: "/p".into(), workspace_dir: "/w".into(), env: [("K".to_string(), "v".to_string())].into_iter().collect(), mask_env: false };
    assert_eq!(android_gradle_args(":app", "freeDebug", "-Px=${env:K}", &vars), vec![":app:assembleFreeDebug", "--console=plain", "-Px=v"]);
    let masked = SubstituteVars { mask_env: true, ..vars };
    assert_eq!(android_gradle_args(":app", "debug", "-Px=${env:K}", &masked)[2], "-Px=***");
}
