use super::before::run_step;
use super::manager::Inner;
use super::proc::kill_pid;
use super::types::{BuildError, EnvIssue, Hooks, RunContext, RunResult, SessionPatch};
use super::flutter;
use crate::preview::paths::is_within;
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

struct PubHooks {
    first: Arc<Mutex<Option<String>>>,
    m: Arc<Inner>,
}

impl Hooks for PubHooks {
    fn state(&self, _: &str, _: SessionPatch) {}
    fn fail(&self, _: EnvIssue) {}
    fn log(&self, _: &str, text: &str, _: Option<&str>, _: Option<&str>) {
        let mut f = self.first.lock().unwrap();
        if f.is_none() && text.to_lowercase().contains("error") {
            *f = Some(crate::js::slice(&(self.m.mask)(text), 300));
        }
    }
    fn error(&self, _: BuildError) {}
    fn confirm(&self, _: &str, _: Option<&str>) -> bool {
        false
    }
}

impl Inner {
    pub fn pub_get(self: &Arc<Self>, dir: &str) -> RunResult {
        let resolved = crate::preview::paths::resolve_str(dir);
        let key = if self.win { resolved.to_lowercase() } else { resolved };
        let (slot, leader) = {
            let mut g = self.state.lock().unwrap();
            match g.pub_gets.get(&key) {
                Some(s) => (s.clone(), false),
                None => {
                    let s = Arc::new((Mutex::new(None), Condvar::new()));
                    g.pub_gets.insert(key.clone(), s.clone());
                    (s, true)
                }
            }
        };
        if leader {
            let result = self.run_pub_get(dir);
            *slot.0.lock().unwrap() = Some(result);
            slot.1.notify_all();
            self.state.lock().unwrap().pub_gets.remove(&key);
        }
        let mut g = slot.0.lock().unwrap();
        while g.is_none() {
            g = slot.1.wait(g).unwrap();
        }
        g.clone().unwrap()
    }

    fn run_pub_get(self: &Arc<Self>, dir: &str) -> RunResult {
        let settings = self.get_settings();
        let opts = flutter::env::env_opts(&settings, dir, &self.env, self.win, &self.home);
        let Some(sdk) = flutter::env::find_flutter_sdk(dir, &opts).sdk else { return RunResult::err("flutter-missing") };
        let Some(spec) = flutter::env::flutter_spawn(&sdk, &["pub".to_string(), "get".to_string()], self.win, &self.env) else { return RunResult::err("bad-arg") };
        let first = Arc::new(Mutex::new(None));
        let ctx = RunContext {
            root: dir.to_string(),
            config: json!({"cwd": dir, "env": {}}),
            device: None,
            settings,
            env: self.env.clone(),
            win: self.win,
            home: self.home.clone(),
            hooks: Arc::new(PubHooks { first: first.clone(), m: self.clone() }),
            on_adb_use: None,
        };
        let pid = Arc::new(AtomicU32::new(0));
        let (tx, rx) = mpsc::channel::<bool>();
        let (p2, m2) = (pid.clone(), self.clone());
        std::thread::spawn(move || {
            let ok = run_step(&ctx, &spec.file, &spec.args, spec.verbatim, "build", &|p| match p {
                Some(v) => {
                    p2.store(v, Ordering::SeqCst);
                    m2.state.lock().unwrap().pub_pids.insert(v);
                }
                None => {
                    let v = p2.swap(0, Ordering::SeqCst);
                    m2.state.lock().unwrap().pub_pids.remove(&v);
                }
            });
            let _ = tx.send(ok);
        });
        let result = rx.recv_timeout(Duration::from_millis(self.pub_get_timeout_ms));
        if result.is_err() {
            let p = pid.load(Ordering::SeqCst);
            if p != 0 {
                kill_pid(p, false);
            }
        }
        let roots: Vec<String> = self.state.lock().unwrap().detected.keys().filter(|r| is_within(r, dir)).cloned().collect();
        for root in roots {
            let m = self.clone();
            std::thread::spawn(move || {
                m.detect(&root, true);
            });
        }
        match result {
            Ok(true) => RunResult::ok(),
            Ok(false) => match first.lock().unwrap().clone() {
                Some(d) => RunResult::Err(EnvIssue::with("pub-get", d)),
                None => RunResult::err("pub-get"),
            },
            Err(_) => RunResult::Err(EnvIssue::with("pub-get", "timeout")),
        }
    }
}
