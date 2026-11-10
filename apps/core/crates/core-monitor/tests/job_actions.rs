use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use core_monitor::job_stop::JobStopResult;
use core_monitor::script::ScriptDeps;
use core_monitor::{Monitor, MonitorConfig, MonitorStores};
use core_store::model::ActivityKind;
use core_store::persist::json_file::JsonFile;
use core_store::{Store, StoreOptions};

const SID: &str = "0e1d2c3b-4a59-4687-9a1b-2c3d4e5f6a7b";

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

struct Fx {
    root: PathBuf,
    deps: Arc<ScriptDeps>,
}

impl Fx {
    fn new(name: &str) -> Fx {
        let root = std::env::temp_dir().join(format!("ccm-jobact-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let deps = Arc::new(ScriptDeps { now: AtomicI64::new(now()), dead: Mutex::new(Vec::new()), stop_result: Mutex::new(JobStopResult::Ok), stops: Mutex::new(Vec::new()) });
        Fx { root, deps }
    }

    fn write_job(&self, id: &str, state: &str, mtime: i64) {
        let dir = self.root.join("jobs").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("state.json");
        std::fs::write(&file, format!(r#"{{"state":"{state}","name":"{id}","sessionId":"{SID}","intent":"secret"}}"#)).unwrap();
        let f = std::fs::File::options().write(true).open(&file).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_millis(mtime as u64)).unwrap();
    }

    fn monitor(&self, dismissed: Option<JsonFile>) -> Monitor {
        let mut config = MonitorConfig::new(self.root.clone(), self.root.clone());
        config.verify_processes = false;
        let mut stores = MonitorStores::memory(self.deps.clone());
        stores.dismissed_jobs = dismissed;
        Monitor::new(config, self.deps.clone(), stores, None)
    }

    fn set_stop(&self, r: JobStopResult) {
        *self.deps.stop_result.lock().unwrap() = r;
    }
}

impl Drop for Fx {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn store() -> Store {
    Store::new(StoreOptions { activity_limit: 100, batch_ms: 1 })
}

fn ids(s: &Store) -> Vec<String> {
    let mut v: Vec<String> = s.job_list().iter().map(|j| j.id.clone()).collect();
    v.sort();
    v
}

fn stop(m: &mut Monitor, s: &mut Store, id: &str) -> Option<JobStopResult> {
    if !m.stop_job(id) {
        return None;
    }
    m.wait_background(s, Duration::from_secs(5));
    let snap = s.snapshot();
    let last = snap.activity.iter().rev().find(|a| matches!(a.kind, ActivityKind::JobStopped | ActivityKind::JobStopFailed))?;
    let err = last.data.as_ref().and_then(|d| d.error.clone());
    Some(match err.as_deref() {
        None => JobStopResult::Ok,
        Some("notRunning") => JobStopResult::NotRunning,
        Some("notFound") => JobStopResult::NotFound,
        Some("unconfirmed") => JobStopResult::Unconfirmed,
        Some("noCli") => JobStopResult::NoCli,
        _ => JobStopResult::Failed,
    })
}

#[test]
fn dismisses_jobs_until_their_state_changes_again_and_clear_all_only_takes_done_ones() {
    let fx = Fx::new("dismiss");
    let t = now();
    fx.write_job("done0001", "done", t - 60_000);
    fx.write_job("done0002", "done", t - 50_000);
    fx.write_job("work0001", "working", t - 40_000);
    let (mut m, mut s) = (fx.monitor(None), store());
    m.poll_extras(&mut s);
    assert_eq!(ids(&s), ["done0001", "done0002", "work0001"]);
    assert_eq!(s.job_list()[0].session_id.as_deref(), Some(SID));
    assert!(!serde_json::to_string(s.job_list()).unwrap().contains("secret"));
    assert_eq!(m.dismiss_jobs(&mut s, Some("done0001")), 1);
    assert_eq!(ids(&s), ["done0002", "work0001"]);
    m.poll_extras(&mut s);
    assert_eq!(ids(&s), ["done0002", "work0001"]);
    assert_eq!(m.dismiss_jobs(&mut s, None), 1);
    assert_eq!(ids(&s), ["work0001"]);
    assert_eq!(m.dismiss_jobs(&mut s, Some("work0001")), 1);
    assert!(ids(&s).is_empty());
    m.poll_extras(&mut s);
    assert!(ids(&s).is_empty());
    fx.write_job("done0001", "working", t - 10_000);
    fx.write_job("work0001", "blocked", t - 5_000);
    m.poll_extras(&mut s);
    assert_eq!(ids(&s), ["done0001", "work0001"]);
}

#[test]
fn stops_only_known_running_jobs_and_reports_the_result_as_activity() {
    let fx = Fx::new("stop");
    let t = now();
    fx.write_job("work0001", "working", t - 40_000);
    fx.write_job("done0001", "done", t - 60_000);
    let (mut m, mut s) = (fx.monitor(None), store());
    m.poll_extras(&mut s);
    assert_eq!(stop(&mut m, &mut s, "nope0001"), None);
    assert_eq!(stop(&mut m, &mut s, "done0001"), None);
    assert!(m.stop_job("work0001"));
    assert!(!m.stop_job("work0001"));
    m.wait_background(&mut s, Duration::from_secs(5));
    assert_eq!(fx.deps.stops.lock().unwrap().as_slice(), ["work0001"]);
    let snap = s.snapshot();
    let a = snap.activity.iter().find(|a| a.kind == ActivityKind::JobStopped).unwrap();
    assert_eq!(a.data.as_ref().map(|d| (d.title.clone(), d.job_id.clone())), Some((Some("work0001".into()), Some("work0001".into()))));
    fx.set_stop(JobStopResult::NoCli);
    assert_eq!(stop(&mut m, &mut s, "work0001"), Some(JobStopResult::NoCli));
}

#[test]
fn keeps_dismissed_jobs_hidden_across_a_restart_until_the_job_changes() {
    let fx = Fx::new("restart");
    let t = now();
    fx.write_job("blck0001", "blocked", t - 40_000);
    let file = fx.root.join("dismissed-jobs.json");
    let (mut first, mut s) = (fx.monitor(Some(JsonFile::new(&file, 60_000))), store());
    first.poll_extras(&mut s);
    assert_eq!(first.dismiss_jobs(&mut s, Some("blck0001")), 1);
    first.flush_persist();
    assert!(Path::new(&file).exists());
    let (mut second, mut s2) = (fx.monitor(Some(JsonFile::new(&file, 60_000))), store());
    second.poll_extras(&mut s2);
    assert!(ids(&s2).is_empty());
    fx.write_job("blck0001", "working", t - 5_000);
    let (mut third, mut s3) = (fx.monitor(Some(JsonFile::new(&file, 60_000))), store());
    third.poll_extras(&mut s3);
    assert_eq!(ids(&s3), ["blck0001"]);
}

#[test]
fn hides_a_job_once_it_is_stopped_or_already_gone() {
    let fx = Fx::new("gone");
    let t = now();
    fx.write_job("blck0001", "blocked", t - 40_000);
    fx.write_job("blck0002", "blocked", t - 30_000);
    let (mut m, mut s) = (fx.monitor(None), store());
    m.poll_extras(&mut s);
    assert_eq!(stop(&mut m, &mut s, "blck0001"), Some(JobStopResult::Ok));
    fx.set_stop(JobStopResult::NotFound);
    assert_eq!(stop(&mut m, &mut s, "blck0002"), Some(JobStopResult::NotFound));
    assert!(ids(&s).is_empty());
    m.poll_extras(&mut s);
    assert!(ids(&s).is_empty());
}

#[test]
fn treats_an_unconfirmed_stop_as_not_running_when_the_claude_daemon_is_gone() {
    let fx = Fx::new("daemon");
    let t = now();
    fx.write_job("blck0001", "blocked", t - 40_000);
    std::fs::create_dir_all(fx.root.join("daemon")).unwrap();
    std::fs::write(fx.root.join("daemon").join("roster.json"), r#"{"proto":1,"supervisorPid":4242,"workers":{}}"#).unwrap();
    fx.set_stop(JobStopResult::Unconfirmed);
    let (mut m, mut s) = (fx.monitor(None), store());
    m.poll_extras(&mut s);
    assert_eq!(stop(&mut m, &mut s, "blck0001"), Some(JobStopResult::Unconfirmed));
    assert_eq!(ids(&s), ["blck0001"]);
    fx.deps.dead.lock().unwrap().push(4242);
    assert_eq!(stop(&mut m, &mut s, "blck0001"), Some(JobStopResult::NotRunning));
    assert!(ids(&s).is_empty());
}
