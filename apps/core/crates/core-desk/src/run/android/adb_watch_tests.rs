use super::*;
use crate::run::android::testutil::{script, tmp_dir, wait_for};
use crate::run::parse::{parse_adb_rows, AdbRow};
use crate::run::proc::{is_alive, kill_pid_only, process_env};
use std::sync::atomic::{AtomicU32, Ordering};

pub fn frame(body: &str) -> String {
    format!("{:04x}{body}", body.len())
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn fake_adb(cases: &str) -> (String, String) {
    let dir = tmp_dir("adbw");
    let adb = format!("{dir}/adb");
    let log = format!("{dir}/calls");
    script(&adb, &format!("echo \"$*\" >> {log}\ncase \"$*\" in\n{cases}\n*) exit 0;;\nesac"));
    (adb, log)
}

fn calls(log: &str) -> Vec<String> {
    std::fs::read_to_string(log).unwrap_or_default().lines().map(String::from).collect()
}

fn watch(adb: Option<String>, timing: AdbWatchTiming, kill: Option<fn(u32)>) -> AdbWatch<AdbRow> {
    AdbWatch::new(AdbWatchOpts { cwd: "/".into(), env: process_env(), adb: Arc::new(move || adb.clone()), parse: Arc::new(parse_adb_rows), timing, kill })
}

#[test]
fn publishes_state_changes_in_same_set() {
    let frames = format!("{}{}", frame("R58\tunauthorized\n"), frame("R58\tdevice\n"));
    let (adb, _) = fake_adb(&format!("'track-devices -l') printf '%s' {}; exec sleep 30;;", sh_quote(&frames)));
    let w = watch(Some(adb), AdbWatchTiming::default(), Some(kill_pid_only));
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let s = seen.clone();
    w.on_change(Arc::new(move |rows: Vec<AdbRow>| s.lock().unwrap().push(rows[0].state.clone())));
    assert!(wait_for(3_000, || seen.lock().unwrap().len() >= 2 || seen.lock().unwrap().last().map(String::as_str) == Some("ready")));
    let got = seen.lock().unwrap().clone();
    assert_eq!(got.last().map(String::as_str), Some("ready"));
    assert!(got == ["unauthorized", "ready"] || got == ["ready"]);
    assert_eq!(w.list().len(), 1);
    w.stop(false);
}

#[test]
fn restarts_tracker_then_polls() {
    let (adb, log) = fake_adb("'track-devices -l') exit 1;;\n'devices -l') printf 'List of devices attached\\nR58M  device usb:1 model:SM_G991B\\n';;");
    let w = watch(Some(adb), AdbWatchTiming { poll_ms: 5, ..Default::default() }, None);
    let seen = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let s = seen.clone();
    w.on_change(Arc::new(move |rows: Vec<AdbRow>| s.lock().unwrap().push(rows.iter().map(|r| r.id.clone()).collect())));
    assert!(wait_for(5_000, || !seen.lock().unwrap().is_empty()));
    assert_eq!(*seen.lock().unwrap(), vec![vec!["R58M".to_string()]]);
    assert_eq!(calls(&log).iter().filter(|c| c.contains("track-devices")).count(), 4);
    assert!(wait_for(3_000, || calls(&log).iter().filter(|c| c.ends_with("devices -l") && !c.contains("track")).count() > 1));
    w.stop(false);
}

static TRACK_KILLED: AtomicU32 = AtomicU32::new(0);
fn record_track(pid: u32) {
    TRACK_KILLED.store(pid, Ordering::SeqCst);
    kill_pid_only(pid);
}

#[test]
fn stop_uses_injected_kill() {
    let (adb, log) = fake_adb("'track-devices -l') exec sleep 30;;");
    let w = watch(Some(adb), AdbWatchTiming::default(), Some(record_track));
    w.on_change(Arc::new(|_| {}));
    assert!(wait_for(3_000, || !calls(&log).is_empty() && lock(&w.inner).tracker.is_some()));
    let pid = lock(&w.inner).tracker.unwrap();
    w.stop(true);
    assert_eq!(TRACK_KILLED.load(Ordering::SeqCst), pid);
    assert!(wait_for(2_000, || !is_alive(pid)));
}

static POLL_KILLED: AtomicU32 = AtomicU32::new(0);
fn record_poll(pid: u32) {
    POLL_KILLED.store(pid, Ordering::SeqCst);
    kill_pid_only(pid);
}

#[test]
fn poll_timeout_uses_injected_kill() {
    let (adb, _) = fake_adb("'track-devices -l') exit 1;;\n'devices -l') exec sleep 30;;");
    let w = watch(Some(adb), AdbWatchTiming { max_restarts: 0, poll_ms: 5, poll_timeout_ms: 50, ..Default::default() }, Some(record_poll));
    w.on_change(Arc::new(|_| {}));
    assert!(wait_for(3_000, || POLL_KILLED.load(Ordering::SeqCst) != 0));
    w.stop(false);
}

#[test]
fn list_is_empty_without_adb() {
    let w = watch(None, AdbWatchTiming { first_list_ms: 10, ..Default::default() }, None);
    let t = Instant::now();
    assert!(w.list().is_empty());
    assert!(t.elapsed() < Duration::from_millis(2_000));
    w.stop(false);
}
