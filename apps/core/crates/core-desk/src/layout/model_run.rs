use super::model::{exists, msg, now_ms, Job, ModelSvc, Resolved};
use super::project_model::{parse_project_model_text, project_model_args};
use crate::preview::paths::root_key;
use crate::run::android::env::{android_env_with, gradlew_spawn, AndroidEnvOpts, FsEnvDeps};
use crate::run::android::pathx;
use crate::run::android::spare::kill_tree_sparing;
use crate::run::proc::{pump_lines, spawn, SpawnSpec};
use crate::run::types::Env;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const BLOCKING: [&str; 5] = ["sdk-missing", "jdk-missing", "jdk-old", "gradlew-missing", "gradlew-not-executable"];

fn daemon_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new("GradleDaemon").expect("regex"))
}

pub struct GradleRun {
    pub code: Option<i32>,
    pub tail: String,
    pub timed_out: bool,
}

impl ModelSvc {
    pub fn on_daemon_state(&self, state: &str) {
        if state != "ready" {
            {
                let mut st = self.lock();
                st.loaded_key = None;
                st.kit_failed = false;
                st.source_failed.clear();
            }
            self.stop_watch();
            return;
        }
        let key = {
            let st = self.lock();
            let wanted = st.wanted.clone();
            wanted.filter(|k| st.loaded_key.is_none() && st.entries.get(k).is_some_and(|e| e.model.is_some() && !e.broken))
        };
        let (Some(key), Some(me)) = (key, self.me.upgrade()) else { return };
        std::thread::spawn(move || {
            let _g = me.serial.lock().unwrap();
            if let Err(err) = me.ensure_loaded(Some(&key), None) {
                me.log("error", &format!("project reload: {}", msg(&err)));
            }
        });
    }

    pub fn refresh_file(&self, file: &str) -> Value {
        if let Resolved::Target { root, module, .. } = self.resolve(file) {
            self.refresh(&root, &module, None, true);
        }
        self.info(file)
    }

    pub fn auto_compile(&self, file: &str) -> bool {
        if !(self.cfg.can_run)() || !self.cfg.settings.get().layout_auto_compile {
            return false;
        }
        let Some(key) = self.entry_for(file) else { return false };
        let Some(e) = self.get(&key) else { return false };
        if e.model.is_some() && !e.broken {
            return false;
        }
        let rk = root_key(&e.root);
        if self.lock().auto_tried.contains(&rk) || self.jobs.lock().unwrap().contains_key(&rk) {
            return false;
        }
        self.lock().auto_tried.insert(rk);
        self.log("info", &format!("project model: auto compile {} (views without classes)", e.module));
        self.refresh_detached(&e.root, &e.module, None, true);
        true
    }

    pub fn after_build(&self, root: &str, module: &str, variant: &str) {
        if let Err(err) = self.prefer(root, module) {
            self.log("warn", &format!("preferred app: {err}"));
        }
        if !(self.cfg.can_run)() {
            return;
        }
        self.refresh_detached(root, module, Some(variant), false);
    }

    fn refresh_detached(&self, root: &str, module: &str, variant: Option<&str>, compile: bool) {
        let Some(me) = self.me.upgrade() else { return };
        let (root, module, variant) = (root.to_string(), module.to_string(), variant.map(String::from));
        std::thread::spawn(move || me.refresh(&root, &module, variant.as_deref(), compile));
    }

    pub fn refresh(&self, root: &str, module: &str, variant: Option<&str>, compile: bool) {
        let key = self.entry(root, module, None);
        let rk = root_key(root);
        let (job, mine) = {
            let mut jobs = self.jobs.lock().unwrap();
            match jobs.get(&rk) {
                Some(j) => (j.clone(), false),
                None => {
                    let j = Arc::new(Job { done: Mutex::new(false), cv: std::sync::Condvar::new() });
                    jobs.insert(rk.clone(), j.clone());
                    (j, true)
                }
            }
        };
        if mine {
            self.run_refresh(&key, variant, compile);
            self.jobs.lock().unwrap().remove(&rk);
            *job.done.lock().unwrap() = true;
            job.cv.notify_all();
        } else {
            let done = job.done.lock().unwrap();
            drop(job.cv.wait_while(done, |d| !*d).unwrap());
        }
    }

    fn fail(&self, key: &str, code: &str, detail: Option<&str>) {
        let mut s = json!({"state": "error", "code": code});
        if let Some(d) = detail.filter(|d| !d.is_empty()) {
            let tail: Vec<char> = d.chars().collect();
            s["detail"] = json!(tail[tail.len().saturating_sub(600)..].iter().collect::<String>());
        }
        s["at"] = json!(now_ms());
        self.set_state(key, s);
    }

    fn run_refresh(&self, key: &str, variant: Option<&str>, compile: bool) {
        self.load_disk(key);
        let Some(e) = self.get(key) else { return };
        let (root, module, hash) = (e.root.clone(), e.module.clone(), e.hash.clone());
        let mut running = json!({"state": "running", "since": now_ms()});
        if compile {
            running["compile"] = json!(true);
        }
        self.set_state(key, running);
        let win = self.cfg.win;
        let settings = self.cfg.settings.get();
        let opts = AndroidEnvOpts { sdk_setting: settings.android_sdk.clone(), jdk_setting: settings.jdk.clone(), env: self.cfg.env.clone(), win, home: self.cfg.home.clone(), trusted: true, repo_root: None };
        let aenv = android_env_with(&root, &opts, &FsEnvDeps);
        if let Some(i) = aenv.issues.iter().find(|i| BLOCKING.contains(&i.code.as_str())) {
            return self.fail(key, &i.code, i.detail.as_deref());
        }
        let _ = std::fs::create_dir_all(&self.cfg.models_dir);
        let out = pathx::join(win, &[&self.cfg.models_dir, &format!("{hash}.next.json")]);
        let _ = std::fs::remove_file(&out);
        let args = match project_model_args(&module, &out, &self.cfg.init_script, variant.unwrap_or(&e.variant), compile, win) {
            Ok(a) => a,
            Err(err) => return self.fail(key, "failed", Some(&err)),
        };
        let Some(spec) = gradlew_spawn(&root, &args, win, &self.cfg.env) else { return self.fail(key, "failed", Some("gradlew wrapper not runnable")) };
        let mut gradle_env = Env::new();
        if let Some(j) = &aenv.jdk {
            gradle_env.insert("JAVA_HOME".into(), j.clone());
        }
        if let Some(s) = &aenv.sdk {
            gradle_env.insert("ANDROID_HOME".into(), s.clone());
        }
        if let Some(n) = &aenv.jdk_note {
            self.log("info", &format!("project model: {n}"));
        }
        self.log("info", &format!("project model: gradlew {} (in {root})", args.join(" ")));
        let started = now_ms();
        let limit = if compile { self.cfg.timing.compile } else { self.cfg.timing.refresh };
        let run = self.run_gradle(&spec.file, spec.args.clone(), spec.verbatim, &root, gradle_env, limit);
        if run.timed_out {
            return self.fail(key, "timeout", None);
        }
        if run.code != Some(0) {
            return match aenv.issues.iter().find(|i| i.code == "jdk-incompatible") {
                Some(i) => self.fail(key, &i.code, i.detail.as_deref()),
                None => self.fail(key, "gradle-failed", Some(&run.tail)),
            };
        }
        let Some(text) = std::fs::read(&out).ok().map(|b| String::from_utf8_lossy(&b).into_owned()) else { return self.fail(key, "no-output", None) };
        let parsed = match parse_project_model_text(&text, &exists, win) {
            Ok(p) => p,
            Err(err) => return self.fail(key, "bad-model", Some(&err)),
        };
        let cache = pathx::join(win, &[&self.cfg.models_dir, &format!("{hash}.json")]);
        if let Err(err) = std::fs::rename(&out, &cache) {
            return self.fail(key, "failed", Some(&err.to_string()));
        }
        self.log("info", &format!("project model {module} ready in {} ms ({} warnings)", now_ms() - started, parsed.warnings.len()));
        for w in parsed.warnings.iter().take(20) {
            self.log("warn", &w.chars().take(500).collect::<String>());
        }
        let parsed = Arc::new(parsed);
        let prev = {
            let mut st = self.lock();
            st.resolved.clear();
            let Some(en) = st.entries.get_mut(key) else { return };
            let prev = en.model.replace(parsed.clone());
            en.variant = parsed.variant.clone();
            en.at = now_ms() as f64;
            en.broken = false;
            if en.code_changed_at < started {
                en.code_changed_at = 0;
            }
            prev
        };
        let ready = self.ready_state(&self.get(key).expect("entry"));
        self.set_state(key, ready);
        {
            let _g = self.serial.lock().unwrap();
            let loaded = self.lock().loaded_key.as_deref() == Some(key);
            let result = if !loaded {
                if let Some(en) = self.lock().entries.get_mut(key) {
                    en.classes_changed = true;
                }
                Ok(())
            } else if prev.as_ref().is_some_and(|p| p.payload.to_value() == parsed.payload.to_value()) {
                self.changed("res", &[]).and_then(|_| self.changed("classes", &[]))
            } else {
                self.load(key)
            };
            if let Err(err) = result {
                self.log("error", &format!("project reload after refresh: {}", msg(&err)));
            }
        }
        self.emit(json!({"type": "project-changed", "root": root, "module": module}));
    }

    fn run_gradle(&self, file: &str, args: Vec<String>, verbatim: bool, cwd: &str, env: Env, limit: Duration) -> GradleRun {
        let spec = SpawnSpec { file: file.into(), args, cwd: cwd.into(), env, verbatim, detached: false, stdin_pipe: false };
        let mut child = match spawn(&spec, &self.cfg.env) {
            Ok(c) => c,
            Err(err) => return GradleRun { code: None, tail: err.to_string(), timed_out: false },
        };
        let pid = child.id();
        self.lock().children.insert(pid);
        let tail = Arc::new(Mutex::new(String::new()));
        let mut pumps = Vec::new();
        let on_line = |tail: Arc<Mutex<String>>, me: Arc<ModelSvc>| {
            move |line: &str| {
                if line.trim().is_empty() {
                    return;
                }
                let mut t = tail.lock().unwrap();
                t.push_str(line);
                t.push('\n');
                let chars: Vec<char> = t.chars().collect();
                if chars.len() > 2000 {
                    *t = chars[chars.len() - 2000..].iter().collect();
                }
                me.log("gradle", &line.chars().take(2000).collect::<String>());
            }
        };
        let me = self.me.upgrade().expect("svc");
        if let Some(o) = child.stdout.take() {
            pumps.push(pump_lines(o, on_line(tail.clone(), me.clone())));
        }
        if let Some(e) = child.stderr.take() {
            pumps.push(pump_lines(e, on_line(tail.clone(), me)));
        }
        let deadline = Instant::now() + limit;
        let mut timed_out = false;
        let code = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) => {}
                Err(_) => break None,
            }
            if Instant::now() >= deadline {
                timed_out = true;
                self.log("error", &format!("project model: Gradle exceeded {} s, killed", limit.as_secs()));
                kill_tree_sparing(pid, daemon_re(), false, &self.cfg.env, None);
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        for _ in 0..20 {
            if pumps.iter().all(|h| h.is_finished()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        self.lock().children.remove(&pid);
        let tail = tail.lock().unwrap().clone();
        GradleRun { code, tail, timed_out }
    }

    pub fn kill_now(&self) {
        let pids: HashSet<u32> = std::mem::take(&mut self.lock().children);
        for pid in pids {
            kill_tree_sparing(pid, daemon_re(), true, &self.cfg.env, None);
        }
        self.stop_watch();
    }
}
