use super::apk::{find_apk, module_dir};
use super::deps::{EnvDeps, FsEnvDeps};
use super::env::{android_env_with, gradlew_spawn, AndroidEnvOpts};
use super::spare::kill_tree_sparing;
use crate::run::args::{expand_args, SubstituteVars};
use crate::run::before::{resolve_for, run_before_launch};
use crate::run::collect::{collect, Collected, CollectOpts};
use crate::run::parse::{install_failure, parse_build_error, InstallFailure};
use crate::run::proc::{kill_pid, pump_lines, spawn, subst_vars, SpawnSpec};
use crate::run::types::{Env, EnvIssue, RunContext, Runner, SessionPatch};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunnerTiming {
    pub pid_search_ms: u64,
    pub pid_watch_ms: u64,
    pub gone_ms: u64,
    pub adb_ms: u64,
    pub force_stop_ms: u64,
    pub no_pid_notice_ms: u64,
}

impl Default for RunnerTiming {
    fn default() -> Self {
        RunnerTiming { pid_search_ms: 1_500, pid_watch_ms: 3_000, gone_ms: 10_000, adb_ms: 15_000, force_stop_ms: 5_000, no_pid_notice_ms: 60_000 }
    }
}

const BLOCKING: [&str; 5] = ["sdk-missing", "jdk-missing", "jdk-old", "gradlew-missing", "gradlew-not-executable"];
const EXIT_GRACE_MS: u64 = 1_500;

#[derive(Default)]
pub(super) struct State {
    pub phase: String,
    pub ended: bool,
    pub stopping: bool,
    pub stop_done: bool,
    pub adb: Option<String>,
    pub pkg: Option<String>,
    pub children: BTreeMap<u32, bool>,
    pub logcat: Option<u32>,
    pub before: Option<u32>,
}

impl State {
    pub fn halted(&self) -> bool {
        self.ended || self.stopping
    }
}

pub(super) struct Core {
    pub ctx: RunContext,
    pub t: RunnerTiming,
    pub deps: Arc<dyn EnvDeps>,
    pub serial: String,
    pub cwd: String,
    pub st: Mutex<State>,
    pub cv: Condvar,
}

pub struct AndroidRunner {
    core: Arc<Core>,
}

pub fn create(ctx: RunContext) -> Arc<dyn Runner> {
    create_with(ctx, RunnerTiming::default(), Arc::new(FsEnvDeps))
}

pub fn create_with_timing(ctx: RunContext, timing: RunnerTiming) -> Arc<dyn Runner> {
    create_with(ctx, timing, Arc::new(FsEnvDeps))
}

pub fn create_with(ctx: RunContext, timing: RunnerTiming, deps: Arc<dyn EnvDeps>) -> Arc<dyn Runner> {
    let serial = ctx.device.as_ref().map(|d| d.id.clone()).unwrap_or_default();
    let cwd = ctx.cfg_str("cwd");
    let st = State { phase: "idle".into(), ..Default::default() };
    Arc::new(AndroidRunner { core: Arc::new(Core { ctx, t: timing, deps, serial, cwd, st: Mutex::new(st), cv: Condvar::new() }) })
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn android_gradle_args(module: &str, variant: &str, args: &str, vars: &SubstituteVars) -> Vec<String> {
    let mut out = vec![format!("{module}:assemble{}", cap(variant)), "--console=plain".into()];
    out.extend(expand_args(args, vars));
    out
}

fn daemon_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new("GradleDaemon").expect("regex"))
}

impl Core {
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn halted(&self) -> bool {
        self.lock().halted()
    }

    pub fn state(&self, s: &str, patch: SessionPatch) {
        {
            let mut st = self.lock();
            if st.halted() {
                return;
            }
            st.phase = s.to_string();
        }
        self.ctx.hooks.state(s, patch);
    }

    pub fn fail(&self, failure: EnvIssue) {
        {
            let mut st = self.lock();
            if st.halted() {
                return;
            }
            st.ended = true;
        }
        self.kill_all(false);
        self.ctx.hooks.fail(failure);
    }

    pub fn kill_all(&self, sync: bool) {
        let children = {
            let mut st = self.lock();
            st.logcat = None;
            std::mem::take(&mut st.children)
        };
        self.cv.notify_all();
        for (pid, gradle) in children {
            if gradle {
                let hooks = self.ctx.hooks.clone();
                let note: super::spare::Fallback = Box::new(move || hooks.log("build", "Could not read the process list; the Gradle daemon was stopped too.", None, None));
                kill_tree_sparing(pid, daemon_re(), sync, &self.ctx.env, Some(note));
            } else {
                kill_pid(pid, sync);
            }
        }
    }

    pub fn add_child(&self, pid: u32, gradle: bool) -> bool {
        let mut st = self.lock();
        if st.halted() {
            drop(st);
            kill_pid(pid, false);
            return false;
        }
        st.children.insert(pid, gradle);
        true
    }

    pub fn remove_child(&self, pid: u32) {
        self.lock().children.remove(&pid);
    }

    pub fn adb_use(&self) {
        if let Some(f) = &self.ctx.on_adb_use {
            f(&self.serial);
        }
    }

    pub fn wait_ms(&self, ms: u64) -> bool {
        let st = self.lock();
        let (st, _) = self.cv.wait_timeout_while(st, Duration::from_millis(ms), |s| !s.halted()).unwrap_or_else(|e| e.into_inner());
        !st.halted()
    }

    pub fn adb_run(self: &Arc<Self>, args: &[&str], timeout_ms: Option<u64>, log: bool) -> Collected {
        let hooks = &self.ctx.hooks;
        if log {
            hooks.log("build", &format!("> adb {}", args.join(" ")), None, None);
        }
        let Some(adb) = self.lock().adb.clone() else { return Collected::default() };
        let mut argv = vec!["-s".to_string(), self.serial.clone()];
        argv.extend(args.iter().map(|a| a.to_string()));
        let me = self.clone();
        let cell = Arc::new(Mutex::new(None::<u32>));
        let track: Arc<dyn Fn(Option<u32>) + Send + Sync> = Arc::new(move |pid| match pid {
            Some(p) => {
                *cell.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
                me.add_child(p, false);
            }
            None => {
                if let Some(p) = cell.lock().unwrap_or_else(|e| e.into_inner()).take() {
                    me.remove_child(p);
                }
            }
        });
        self.adb_use();
        let r = collect(&adb, &argv, &self.cwd, &self.ctx.env, &CollectOpts { timeout_ms, track: Some(track), ..Default::default() });
        if log {
            for line in super::detect::split_lines(&r.out) {
                if !line.trim().is_empty() {
                    hooks.log("build", line, None, None);
                }
            }
        }
        r
    }

    fn gradle(&self, file: &str, args: &[String], verbatim: bool, env: Env) -> bool {
        let spec = SpawnSpec { file: file.into(), args: args.to_vec(), cwd: self.cwd.clone(), env, verbatim, ..Default::default() };
        let Ok(mut child) = spawn(&spec, &self.ctx.env) else { return false };
        let pid = child.id();
        self.add_child(pid, true);
        let (tx, rx) = mpsc::channel::<()>();
        for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
            let hooks = self.ctx.hooks.clone();
            let cwd = self.cwd.clone();
            let tx = tx.clone();
            let h = pump_lines(pipe, move |text| {
                hooks.log("build", text, None, None);
                if let Some(e) = parse_build_error(text, &cwd) {
                    hooks.error(e);
                }
            });
            std::thread::spawn(move || {
                let _ = h.join();
                let _ = tx.send(());
            });
        }
        drop(tx);
        let code = child.wait().ok().and_then(|s| s.code());
        for _ in 0..2 {
            if rx.recv_timeout(Duration::from_millis(EXIT_GRACE_MS)).is_err() {
                break;
            }
        }
        self.remove_child(pid);
        code == Some(0)
    }

    pub(super) fn install(self: &Arc<Self>, apk: &str) -> Option<&'static str> {
        let r = self.adb_run(&["install", "-r", "-t", apk], None, true);
        if r.code.is_none() {
            return Some("other");
        }
        install_failure(&r.out).map(|f| match f {
            InstallFailure::Signature => "signature",
            InstallFailure::Storage => "storage",
            InstallFailure::Abi => "abi",
            InstallFailure::VersionDowngrade => "version-downgrade",
            InstallFailure::Other => "other",
        })
    }

    fn track_before(&self, pid: Option<u32>) {
        {
            let mut st = self.lock();
            if let Some(o) = st.before.take() {
                st.children.remove(&o);
            }
            st.before = pid;
        }
        if let Some(p) = pid {
            self.add_child(p, false);
        }
    }

    fn run(self: &Arc<Self>) {
        let ctx = &self.ctx;
        if ctx.cfg_str("type") != "android" {
            return self.fail(EnvIssue::new("config-type"));
        }
        let Some(device) = &ctx.device else { return self.fail(EnvIssue::new("device-missing")) };
        if device.state == "unauthorized" {
            return self.fail(EnvIssue::with("device-unauthorized", self.serial.clone()));
        }
        let opts = AndroidEnvOpts { sdk_setting: ctx.settings.android_sdk.clone(), jdk_setting: ctx.settings.jdk.clone(), env: ctx.env.clone(), win: ctx.win, home: ctx.home.clone(), trusted: true, repo_root: None };
        let env = android_env_with(&self.cwd, &opts, self.deps.as_ref());
        if self.halted() {
            return;
        }
        let blocking = env.issues.iter().find(|i| BLOCKING.contains(&i.code.as_str())).cloned();
        if let Some(issue) = blocking.or_else(|| env.adb.is_none().then(|| EnvIssue::new("sdk-missing"))) {
            return self.fail(issue);
        }
        self.lock().adb = env.adb.clone();
        if !ctx.cfg_before().is_empty() {
            self.state("before-launch", SessionPatch::default());
            let ok = run_before_launch(ctx, &|c| resolve_for(ctx, c), &|pid| self.track_before(pid), &|| self.halted());
            if self.halted() {
                return;
            }
            if !ok {
                return self.fail(EnvIssue::new("before-launch"));
            }
        }
        self.state("building", SessionPatch::default());
        let cfg_env = ctx.cfg_env();
        let vars = subst_vars(&self.cwd, &ctx.root, &ctx.env, &cfg_env);
        let (module, variant, extra) = (ctx.cfg_str("module"), ctx.cfg_str("variant"), ctx.cfg_str("args"));
        let gradle_args = android_gradle_args(&module, &variant, &extra, &vars);
        let Some(spec) = gradlew_spawn(&self.cwd, &gradle_args, ctx.win, &ctx.env) else { return self.fail(EnvIssue::new("bad-arg")) };
        let mut gradle_env = Env::new();
        if let Some(j) = &env.jdk {
            gradle_env.insert("JAVA_HOME".into(), j.clone());
        }
        if let Some(s) = &env.sdk {
            gradle_env.insert("ANDROID_HOME".into(), s.clone());
        }
        gradle_env.extend(cfg_env);
        if let Some(note) = &env.jdk_note {
            ctx.hooks.log("build", note, None, None);
        }
        let shown = SubstituteVars { mask_env: true, ..vars };
        ctx.hooks.log("build", &format!("> gradlew {}", android_gradle_args(&module, &variant, &extra, &shown).join(" ")), None, None);
        let built = self.gradle(&spec.file, &spec.args, spec.verbatim, gradle_env);
        if self.halted() {
            return;
        }
        if !built {
            return self.fail(env.issues.iter().find(|i| i.code == "jdk-incompatible").cloned().unwrap_or_else(|| EnvIssue::new("build")));
        }
        let Some(out) = find_apk(&module_dir(&self.cwd, &module), &variant) else {
            if !self.halted() {
                self.fail(EnvIssue::with("install", "apk-missing"));
            }
            return;
        };
        if self.halted() {
            return;
        }
        let pkg = out.application_id.clone();
        self.lock().pkg = Some(pkg.clone());
        self.state("installing", SessionPatch::default());
        let mut failure = self.install(&out.apk);
        if self.halted() {
            return;
        }
        if failure == Some("signature") {
            let yes = ctx.hooks.confirm("reinstall", Some(&pkg));
            if self.halted() {
                return;
            }
            if !yes {
                return self.fail(EnvIssue::with("install", "signature"));
            }
            self.adb_run(&["uninstall", &pkg], Some(self.t.adb_ms * 4), true);
            if self.halted() {
                return;
            }
            failure = self.install(&out.apk);
            if self.halted() {
                return;
            }
        }
        if let Some(f) = failure {
            return self.fail(EnvIssue::with("install", f));
        }
        if self.launch(&pkg) {
            self.state("running", SessionPatch { app_id: Some(Some(pkg.clone())), ..Default::default() });
            self.watch_pid(&pkg);
        }
    }
}

impl Runner for AndroidRunner {
    fn start(&self) {
        let core = self.core.clone();
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| core.run())).is_err() {
            self.core.fail(EnvIssue::new("spawn"));
        }
    }

    fn stop(&self) {
        let c = &self.core;
        let (was_running, adb, pkg) = {
            let mut st = c.lock();
            if st.stopping {
                while !st.stop_done {
                    st = c.cv.wait(st).unwrap_or_else(|e| e.into_inner());
                }
                return;
            }
            st.stopping = true;
            if st.ended {
                st.stop_done = true;
                c.cv.notify_all();
                return;
            }
            let was = st.phase == "running";
            st.phase = "stopping".into();
            (was, st.adb.clone(), st.pkg.clone())
        };
        c.cv.notify_all();
        c.ctx.hooks.state("stopping", SessionPatch::default());
        if let (true, Some(adb), Some(pkg)) = (was_running, adb, pkg) {
            c.adb_use();
            let args: Vec<String> = ["-s", &c.serial, "shell", "am", "force-stop", &pkg].iter().map(|s| s.to_string()).collect();
            collect(&adb, &args, &c.cwd, &c.ctx.env, &CollectOpts { timeout_ms: Some(c.t.force_stop_ms), ..Default::default() });
        }
        c.kill_all(false);
        let phase = if was_running { "stopped" } else { "cancelled" };
        {
            let mut st = c.lock();
            st.ended = true;
            st.phase = phase.into();
            st.stop_done = true;
        }
        c.cv.notify_all();
        c.ctx.hooks.state(phase, SessionPatch::default());
    }

    fn kill(&self) {
        let c = &self.core;
        let force = {
            let mut st = c.lock();
            let f = if !st.ended && st.phase == "running" { st.adb.clone().zip(st.pkg.clone()) } else { None };
            st.ended = true;
            f
        };
        if let Some((adb, pkg)) = force {
            c.adb_use();
            let spec = SpawnSpec { file: adb, args: ["-s", &c.serial, "shell", "am", "force-stop", &pkg].iter().map(|s| s.to_string()).collect(), cwd: c.cwd.clone(), ..Default::default() };
            if let Ok(mut child) = spawn(&spec, &c.ctx.env) {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
        }
        c.kill_all(true);
    }
}

#[cfg(all(test, unix))]
#[path = "runner_tests.rs"]
mod tests;
