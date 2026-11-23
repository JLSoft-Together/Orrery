use super::err::GhErr;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub const MAX_WATCHES: usize = 8;
pub const DONE_TYPE: &str = "ci.done";
const FAILED_MAX: usize = 20;
const HARD_ERRORS: [&str; 6] = ["gh.auth", "gh.scope", "gh.notFound", "gh.noAccess", "gh.noAccount", "invalid"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub root: String,
    pub remote_url: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub sha: String,
    pub pr: Option<u64>,
}

impl Target {
    fn key(&self) -> String {
        format!("{}\n{}", self.root, self.branch)
    }

    pub fn url(&self) -> String {
        match self.pr {
            Some(n) => format!("https://github.com/{}/{}/pull/{n}", self.owner, self.repo),
            None => format!("https://github.com/{}/{}/commit/{}/checks", self.owner, self.repo, self.sha),
        }
    }
}

pub trait Probe: Send + Sync {
    fn status(&self, t: &Target) -> Result<Value, GhErr>;
}

pub type Notify = Arc<dyn Fn(&Value) + Send + Sync>;
pub type Clock = Arc<dyn Fn() -> f64 + Send + Sync>;

#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub interval_ms: f64,
    pub max_ms: f64,
    pub none_max_ms: f64,
    pub background: bool,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { interval_ms: 30_000.0, max_ms: 60.0 * 60_000.0, none_max_ms: 10.0 * 60_000.0, background: true }
    }
}

struct Entry {
    t: Target,
    started: f64,
    next: f64,
}

pub struct CiWatch {
    entries: Mutex<Vec<Entry>>,
    probe: Arc<dyn Probe>,
    notify: Notify,
    now: Clock,
    timing: Timing,
    running: AtomicBool,
    stop: AtomicBool,
    wake: Condvar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Added {
    New,
    Updated,
    Same,
}

impl Added {
    pub fn as_str(self) -> &'static str {
        match self {
            Added::New => "added",
            Added::Updated => "updated",
            Added::Same => "exists",
        }
    }
}

pub fn done_event(t: &Target, state: &str, status: Option<&Value>, timed_out: bool) -> Value {
    let failed: Vec<Value> = status
        .and_then(|s| s.get("runs"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|r| r.get("state").and_then(Value::as_str) == Some("failure"))
        .filter_map(|r| r.get("name").and_then(Value::as_str))
        .take(FAILED_MAX)
        .map(|n| json!(crate::js::slice(n, 120)))
        .collect();
    let mut v = json!({
        "type": DONE_TYPE,
        "root": t.root,
        "branch": t.branch,
        "sha": t.sha,
        "pr": t.pr,
        "state": state,
        "failed": failed,
        "url": t.url(),
    });
    if timed_out {
        v["timedOut"] = json!(true);
    }
    v
}

impl CiWatch {
    pub fn new(probe: Arc<dyn Probe>, notify: Notify, now: Clock, timing: Timing) -> Arc<Self> {
        Arc::new(CiWatch { entries: Mutex::new(Vec::new()), probe, notify, now, timing, running: AtomicBool::new(false), stop: AtomicBool::new(false), wake: Condvar::new() })
    }

    pub fn add(self: &Arc<Self>, t: Target) -> Result<Added, GhErr> {
        let now = (self.now)();
        let added = {
            let mut list = self.entries.lock().unwrap();
            let key = t.key();
            let full = list.len() >= MAX_WATCHES;
            let added = match list.iter_mut().find(|e| e.t.key() == key) {
                Some(e) if e.t.sha == t.sha && e.t.pr == t.pr => Added::Same,
                Some(e) => {
                    let sha_changed = e.t.sha != t.sha;
                    e.t = t;
                    if sha_changed {
                        e.started = now;
                        e.next = now + self.timing.interval_ms;
                    }
                    Added::Updated
                }
                None if full => return Err(GhErr::with_detail("gh.watchMax", MAX_WATCHES.to_string())),
                None => {
                    list.push(Entry { t, started: now, next: now + self.timing.interval_ms });
                    Added::New
                }
            };
            self.wake.notify_all();
            if self.timing.background && !self.stop.load(Ordering::SeqCst) && !self.running.swap(true, Ordering::SeqCst) {
                let me = self.clone();
                if std::thread::Builder::new().name("gh-ci-watch".into()).spawn(move || me.run()).is_err() {
                    self.running.store(false, Ordering::SeqCst);
                }
            }
            added
        };
        Ok(added)
    }

    pub fn remove(&self, root: &str, branch: &str) -> bool {
        let key = format!("{root}\n{branch}");
        let mut list = self.entries.lock().unwrap();
        let before = list.len();
        list.retain(|e| e.t.key() != key);
        let removed = list.len() != before;
        drop(list);
        self.wake.notify_all();
        removed
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.entries.lock().unwrap().clear();
        self.wake.notify_all();
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn list(&self) -> Value {
        let list = self.entries.lock().unwrap();
        Value::Array(list.iter().map(|e| json!({"root": e.t.root, "branch": e.t.branch, "sha": e.t.sha, "pr": e.t.pr, "since": e.started})).collect())
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn finish(&self, key: &str, started: f64) -> bool {
        let mut list = self.entries.lock().unwrap();
        let before = list.len();
        list.retain(|e| !(e.t.key() == key && e.started == started));
        list.len() != before
    }

    fn reschedule(&self, key: &str, started: f64, next: f64) {
        let mut list = self.entries.lock().unwrap();
        if let Some(e) = list.iter_mut().find(|e| e.t.key() == key && e.started == started) {
            e.next = next;
        }
    }

    pub fn tick(&self) -> Vec<Value> {
        let now = (self.now)();
        let due: Vec<(Target, f64)> = {
            let list = self.entries.lock().unwrap();
            list.iter().filter(|e| e.next <= now).map(|e| (e.t.clone(), e.started)).collect()
        };
        let mut out = Vec::new();
        for (t, started) in due {
            let key = t.key();
            if now - started >= self.timing.max_ms {
                if self.finish(&key, started) {
                    let ev = done_event(&t, "pending", None, true);
                    (self.notify)(&ev);
                    out.push(ev);
                }
                continue;
            }
            match self.probe.status(&t) {
                Ok(status) => {
                    let state = status.get("state").and_then(Value::as_str).unwrap_or("none");
                    match state {
                        "pending" => self.reschedule(&key, started, now + self.timing.interval_ms),
                        "none" if now - started < self.timing.none_max_ms => self.reschedule(&key, started, now + self.timing.interval_ms),
                        "none" => {
                            self.finish(&key, started);
                        }
                        done => {
                            if self.finish(&key, started) {
                                let ev = done_event(&t, done, Some(&status), false);
                                (self.notify)(&ev);
                                out.push(ev);
                            }
                        }
                    }
                }
                Err(e) if HARD_ERRORS.contains(&e.code) => {
                    self.finish(&key, started);
                }
                Err(_) => self.reschedule(&key, started, now + self.timing.interval_ms),
            }
        }
        out
    }

    fn wait_ms(&self, list: &[Entry]) -> f64 {
        let now = (self.now)();
        let next = list.iter().map(|e| e.next).fold(f64::INFINITY, f64::min);
        (next - now).clamp(1_000.0, self.timing.interval_ms)
    }

    fn run(self: Arc<Self>) {
        loop {
            {
                let list = self.entries.lock().unwrap();
                if self.stop.load(Ordering::SeqCst) || list.is_empty() {
                    self.running.store(false, Ordering::SeqCst);
                    return;
                }
                let ms = self.wait_ms(&list);
                let (list, res) = self.wake.wait_timeout(list, Duration::from_millis(ms as u64)).unwrap();
                if self.stop.load(Ordering::SeqCst) || list.is_empty() {
                    self.running.store(false, Ordering::SeqCst);
                    return;
                }
                if !res.timed_out() {
                    continue;
                }
            }
            let me = self.clone();
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                me.tick();
            }))
            .is_err()
            {
                eprintln!("[desk] gh-ci-watch tick panicked");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    struct FakeProbe {
        answers: Mutex<Vec<Result<Value, GhErr>>>,
        calls: AtomicU64,
    }

    impl Probe for FakeProbe {
        fn status(&self, _t: &Target) -> Result<Value, GhErr> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut a = self.answers.lock().unwrap();
            if a.len() > 1 {
                a.remove(0)
            } else {
                a[0].clone()
            }
        }
    }

    fn target(branch: &str, sha: &str) -> Target {
        Target { root: "/r".into(), remote_url: "https://github.com/o/r.git".into(), owner: "o".into(), repo: "r".into(), branch: branch.into(), sha: sha.into(), pr: None }
    }

    struct World {
        w: Arc<CiWatch>,
        probe: Arc<FakeProbe>,
        clock: Arc<Mutex<f64>>,
        seen: Arc<Mutex<Vec<Value>>>,
    }

    fn world(answers: Vec<Result<Value, GhErr>>) -> World {
        let probe = Arc::new(FakeProbe { answers: Mutex::new(answers), calls: AtomicU64::new(0) });
        let clock = Arc::new(Mutex::new(1_000_000.0));
        let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
        let (c, s) = (clock.clone(), seen.clone());
        let timing = Timing { background: false, ..Timing::default() };
        let w = CiWatch::new(probe.clone(), Arc::new(move |v: &Value| s.lock().unwrap().push(v.clone())), Arc::new(move || *c.lock().unwrap()), timing);
        World { w, probe, clock, seen }
    }

    fn advance(w: &World, ms: f64) {
        *w.clock.lock().unwrap() += ms;
    }

    #[test]
    fn stops_once_every_run_is_done() {
        let pending = json!({"state": "pending", "runs": [{"name": "build", "state": "pending"}]});
        let failed = json!({"state": "failure", "runs": [{"name": "build", "state": "failure"}, {"name": "lint", "state": "success"}]});
        let w = world(vec![Ok(pending), Ok(failed)]);
        assert_eq!(w.w.add(target("feat/x", "abc1234")).unwrap(), Added::New);
        assert!(w.w.tick().is_empty());
        assert_eq!(w.probe.calls.load(Ordering::SeqCst), 0);
        advance(&w, 30_000.0);
        assert!(w.w.tick().is_empty());
        advance(&w, 10_000.0);
        assert!(w.w.tick().is_empty());
        assert_eq!(w.probe.calls.load(Ordering::SeqCst), 1);
        advance(&w, 20_000.0);
        let done = w.w.tick();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0]["type"], "ci.done");
        assert_eq!(done[0]["state"], "failure");
        assert_eq!(done[0]["failed"], json!(["build"]));
        assert_eq!(done[0]["url"], "https://github.com/o/r/commit/abc1234/checks");
        assert!(w.w.is_empty());
        advance(&w, 30_000.0);
        assert!(w.w.tick().is_empty());
        assert_eq!(w.probe.calls.load(Ordering::SeqCst), 2);
        assert_eq!(w.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn gives_up_after_sixty_minutes() {
        let w = world(vec![Ok(json!({"state": "pending", "runs": []}))]);
        w.w.add(target("main", "abc1234")).unwrap();
        for _ in 0..119 {
            advance(&w, 30_000.0);
            assert!(w.w.tick().is_empty());
        }
        assert_eq!(w.w.len(), 1);
        advance(&w, 30_000.0);
        let done = w.w.tick();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0]["timedOut"], true);
        assert_eq!(done[0]["state"], "pending");
        assert!(w.w.is_empty());
        assert_eq!(w.probe.calls.load(Ordering::SeqCst), 119);
    }

    #[test]
    fn events_and_list_never_carry_the_remote_url() {
        let w = world(vec![Ok(json!({"state": "success", "runs": []}))]);
        let mut t = target("x", "abc1234");
        t.remote_url = "https://u:ghp_SECRETSECRETSECRET0123@github.com/o/r.git".into();
        w.w.add(t).unwrap();
        assert!(!w.w.list().to_string().contains("ghp_"));
        advance(&w, 30_000.0);
        let done = w.w.tick();
        assert_eq!(done[0]["state"], "success");
        assert!(!done[0].to_string().contains("ghp_"));
    }

    #[test]
    fn max_eight_and_no_duplicates() {
        let w = world(vec![Ok(json!({"state": "pending"}))]);
        for i in 0..MAX_WATCHES {
            assert_eq!(w.w.add(target(&format!("b{i}"), "abc1234")).unwrap(), Added::New);
        }
        assert_eq!(w.w.add(target("b0", "abc1234")).unwrap(), Added::Same);
        assert_eq!(w.w.add(target("b0", "def5678")).unwrap(), Added::Updated);
        assert_eq!(w.w.len(), MAX_WATCHES);
        assert_eq!(w.w.add(target("b9", "abc1234")).unwrap_err().code, "gh.watchMax");
        assert!(w.w.remove("/r", "b3"));
        assert!(!w.w.remove("/r", "b3"));
        assert_eq!(w.w.add(target("b9", "abc1234")).unwrap(), Added::New);
    }

    #[test]
    fn remove_and_stop_wake_the_thread_at_once() {
        fn bg() -> World {
            let w = world(vec![Ok(json!({"state": "pending"}))]);
            let timing = Timing { background: true, ..Timing::default() };
            let c = w.clock.clone();
            let probe = w.probe.clone();
            let w2 = CiWatch::new(probe.clone(), Arc::new(|_: &Value| {}), Arc::new(move || *c.lock().unwrap()), timing);
            World { w: w2, probe, clock: w.clock, seen: w.seen }
        }
        fn settle(w: &World, want: bool) -> bool {
            for _ in 0..100 {
                if w.w.is_running() == want {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        }
        let w = bg();
        w.w.add(target("x", "abc1234")).unwrap();
        assert!(settle(&w, true));
        assert!(w.w.remove("/r", "x"));
        assert!(settle(&w, false), "thread must exit after the last remove");
        w.w.add(target("y", "abc1234")).unwrap();
        assert!(settle(&w, true));
        w.w.stop();
        assert!(settle(&w, false), "thread must exit after stop");
        assert!(w.w.is_empty());
        w.w.add(target("z", "abc1234")).unwrap();
        assert!(!w.w.is_running());
    }

    #[test]
    fn hard_errors_stop_and_network_errors_retry() {
        let w = world(vec![Err(GhErr::new("network")), Err(GhErr::new("gh.scope"))]);
        w.w.add(target("x", "abc1234")).unwrap();
        advance(&w, 30_000.0);
        w.w.tick();
        assert_eq!(w.w.len(), 1);
        advance(&w, 30_000.0);
        w.w.tick();
        assert!(w.w.is_empty());
        assert!(w.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn no_checks_at_all_stops_quietly_after_ten_minutes() {
        let w = world(vec![Ok(json!({"state": "none", "runs": []}))]);
        w.w.add(target("x", "abc1234")).unwrap();
        for _ in 0..20 {
            advance(&w, 30_000.0);
            w.w.tick();
        }
        assert!(w.w.is_empty());
        assert!(w.seen.lock().unwrap().is_empty());
    }
}
