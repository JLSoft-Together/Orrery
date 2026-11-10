use super::detect::needs_pub_get;
use super::env::{find_flutter_sdk, flutter_executable, flutter_spawn, sdk_settings, FlutterEnvOpts, FlutterSdkLookup};
use super::machine::{MachineClient, MachineHandlers};
use super::pth;
use crate::preview::format::{cmd_shim, Resolved};
use crate::run::args::{expand_args, SubstituteVars};
use crate::run::before::{resolve_for, run_before_launch, run_step};
use crate::run::proc::{kill_pid, pump_lines, spawn, subst_vars, SpawnSpec};
use crate::run::types::{Env, EnvIssue, RunContext, RunResult, Runner, SessionPatch};
use serde_json::{json, Value};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const STOP_TIMEOUT_MS: u64 = 5_000;
const EXIT_GRACE_MS: u64 = 1_500;

#[derive(Debug, Clone, PartialEq)]
pub struct CustomCommand {
    pub file: String,
    pub args: Vec<String>,
    pub verbatim: bool,
    pub mode: String,
}

pub type FindSdkFn = Arc<dyn Fn(&str, &FlutterEnvOpts) -> FlutterSdkLookup + Send + Sync>;
pub type NeedsPubGetFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

#[derive(Clone, Default)]
pub struct RunnerDeps {
    pub find_sdk: Option<FindSdkFn>,
    pub needs_pub_get: Option<NeedsPubGetFn>,
    pub stop_timeout_ms: Option<u64>,
    pub command: Option<CustomCommand>,
}

pub(super) fn is_live(s: &str) -> bool {
    matches!(s, "before-launch" | "building" | "installing" | "launching" | "starting" | "running" | "reloading" | "stopping")
}

#[derive(Default)]
pub(super) struct State {
    pub state: String,
    pub child: Option<u32>,
    pub step: Option<u32>,
    pub app_id: Option<String>,
    pub started: bool,
    pub ended: bool,
    pub stop_requested: bool,
    pub saw_build_error: bool,
    pub device_lost: bool,
    pub stop_called: bool,
}

pub(super) struct Inner {
    pub ctx: RunContext,
    pub deps: RunnerDeps,
    pub mode: String,
    pub cwd: String,
    pub st: Mutex<State>,
    pub machine: Mutex<Option<Arc<MachineClient>>>,
    pub exited: Condvar,
}

pub struct FlutterRunner {
    inner: Arc<Inner>,
}

pub fn sdk_tool(sdk: &str, head: &str, win: bool, env: &Env) -> Option<Resolved> {
    let lower = head.to_lowercase();
    let name = lower.strip_suffix(".bat").or_else(|| lower.strip_suffix(".exe")).unwrap_or(&lower);
    if head.contains('/') || head.contains('\\') || (name != "flutter" && name != "dart") {
        return None;
    }
    let exe = if name == "flutter" { flutter_executable(sdk, win) } else { pth::join(&[sdk, "bin", if win { "dart.bat" } else { "dart" }], win) };
    if win {
        cmd_shim(&exe, &[], env)
    } else {
        Some(Resolved { file: exe, args: Vec::new(), verbatim: false })
    }
}

pub fn flutter_argv(config: &Value, device_id: Option<&str>, vars: &SubstituteVars, flutter_file: &str) -> Vec<String> {
    let s = |k: &str| config.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let device = if s("device") != "selected" { Some(s("device")) } else { device_id.map(String::from) }.filter(|d| !d.is_empty());
    let mut argv = vec![flutter_file.to_string(), "run".into(), "--machine".into()];
    if let Some(d) = device {
        argv.extend(["-d".into(), d]);
    }
    argv.extend(["-t".into(), s("entrypoint")]);
    if !s("flavor").is_empty() {
        argv.extend(["--flavor".into(), s("flavor")]);
    }
    argv.push(format!("--{}", s("mode")));
    argv.extend(expand_args(&s("args"), vars));
    argv
}

pub fn create(ctx: RunContext) -> Arc<dyn Runner> {
    create_with(ctx, RunnerDeps::default())
}

pub fn create_with_command(ctx: RunContext, cmd: CustomCommand) -> Arc<dyn Runner> {
    create_with(ctx, RunnerDeps { command: Some(cmd), ..Default::default() })
}

pub fn create_with(ctx: RunContext, deps: RunnerDeps) -> Arc<dyn Runner> {
    Arc::new(FlutterRunner::new(ctx, deps))
}

impl FlutterRunner {
    pub fn new(ctx: RunContext, deps: RunnerDeps) -> Self {
        let mode = match &deps.command {
            Some(c) => c.mode.clone(),
            None if ctx.cfg_str("type") == "flutter" => ctx.cfg_str("mode"),
            None => "debug".into(),
        };
        let cwd = ctx.cfg_str("cwd");
        let st = State { state: "starting".into(), ..Default::default() };
        FlutterRunner { inner: Arc::new(Inner { ctx, deps, mode, cwd, st: Mutex::new(st), machine: Mutex::new(None), exited: Condvar::new() }) }
    }
}

impl Inner {
    pub fn set_state(&self, next: &str, patch: SessionPatch) {
        let mut s = self.st.lock().unwrap();
        if is_live(next) && (s.ended || (s.stop_requested && next != "stopping")) {
            return;
        }
        s.state = next.to_string();
        self.ctx.hooks.state(next, patch);
    }

    pub fn current(&self) -> String {
        self.st.lock().unwrap().state.clone()
    }

    pub fn end(&self, next: &str, failure: Option<EnvIssue>) {
        {
            let mut s = self.st.lock().unwrap();
            if s.ended {
                return;
            }
            s.ended = true;
        }
        let notice = {
            let s = self.st.lock().unwrap();
            next == "stopped" && failure.is_none() && s.device_lost && s.started && !s.stop_requested
        };
        if let Some(f) = failure {
            self.ctx.hooks.fail(f);
        }
        let failure_patch = notice.then(|| Some(EnvIssue::new("device-lost")));
        self.set_state(next, SessionPatch { progress: Some(None), failure: failure_patch, ..Default::default() });
        if let Some(m) = self.machine.lock().unwrap().clone() {
            m.dispose();
        }
        self.exited.notify_all();
    }

    fn stopping(&self) -> bool {
        let s = self.st.lock().unwrap();
        s.stop_requested || s.ended
    }

    fn track_step(&self, pid: Option<u32>) {
        let mut s = self.st.lock().unwrap();
        s.step = pid;
        if let (Some(p), true) = (pid, s.stop_requested) {
            kill_pid(p, false);
        }
    }

    fn pub_get_if_needed(&self, sdk: &str) -> bool {
        let needed = match &self.deps.needs_pub_get {
            Some(f) => f(&self.cwd),
            None => needs_pub_get(&self.cwd),
        };
        if self.st.lock().unwrap().stop_requested {
            self.end("cancelled", None);
            return false;
        }
        if !needed {
            return true;
        }
        self.set_state("before-launch", SessionPatch { progress: Some(Some("flutter pub get".into())), ..Default::default() });
        self.ctx.hooks.log("build", "> flutter pub get", None, None);
        let ok = match flutter_spawn(sdk, &["pub".into(), "get".into()], self.ctx.win, &self.ctx.env) {
            Some(spec) => run_step(&self.ctx, &spec.file, &spec.args, spec.verbatim, "build", &|p| self.track_step(p)),
            None => false,
        };
        self.track_step(None);
        if self.stopping() {
            self.end("cancelled", None);
            return false;
        }
        if !ok {
            self.end("failed", Some(EnvIssue::new("pub-get")));
            return false;
        }
        self.set_state("before-launch", SessionPatch { progress: Some(None), ..Default::default() });
        true
    }

    fn launch_spec(&self, sdk: Option<&str>) -> Result<SpawnSpec, EnvIssue> {
        let ctx = &self.ctx;
        let base = SpawnSpec { cwd: self.cwd.clone(), env: ctx.cfg_env(), stdin_pipe: true, ..Default::default() };
        if let Some(c) = &self.deps.command {
            return Ok(SpawnSpec { file: c.file.clone(), args: c.args.clone(), verbatim: c.verbatim, ..base });
        }
        let Some(sdk) = sdk else { return Err(EnvIssue::with("spawn", "flutter-missing")) };
        let vars = subst_vars(&self.cwd, &ctx.root, &ctx.env, &ctx.cfg_env());
        let file = flutter_executable(sdk, ctx.win);
        let device = ctx.device.as_ref().map(|d| d.id.as_str());
        let argv = flutter_argv(&ctx.config, device, &vars, &file);
        let shown = flutter_argv(&ctx.config, device, &SubstituteVars { mask_env: true, ..vars }, &file);
        ctx.hooks.log("build", &format!("> flutter {}", shown[1..].join(" ")), None, None);
        let spec = flutter_spawn(sdk, &argv[1..], ctx.win, &ctx.env).ok_or_else(|| EnvIssue::new("bad-arg"))?;
        Ok(SpawnSpec { file: spec.file, args: spec.args, verbatim: spec.verbatim, ..base })
    }

    fn on_close(&self, code: Option<i32>) {
        let (ended, stop, started, saw) = {
            let s = self.st.lock().unwrap();
            (s.ended, s.stop_requested, s.started, s.saw_build_error)
        };
        if ended {
            return;
        }
        if stop {
            return self.end(if started { "stopped" } else { "cancelled" }, None);
        }
        if started {
            return self.end("stopped", None);
        }
        if code == Some(0) && !saw {
            return self.end("stopped", None);
        }
        let failure = if saw { EnvIssue::new("build") } else { EnvIssue::with("exit", code.map(|c| c.to_string()).unwrap_or_else(|| "signal".into())) };
        self.end("failed", Some(failure))
    }

    fn start(self: &Arc<Self>) {
        let ctx = &self.ctx;
        let mut sdk: Option<String> = None;
        if self.deps.command.is_none() {
            let (project_setting, setting) = sdk_settings(&ctx.settings, &self.cwd, ctx.win);
            let opts = FlutterEnvOpts { project_setting, setting, env: ctx.env.clone(), win: ctx.win, home: ctx.home.clone(), ..Default::default() };
            let found = match &self.deps.find_sdk {
                Some(f) => f(&self.cwd, &opts),
                None => find_flutter_sdk(&self.cwd, &opts),
            };
            sdk = found.sdk.clone();
            let Some(root) = sdk.as_deref().filter(|_| ctx.cfg_str("type") == "flutter") else {
                let detail = found.issues.first().map(|i| i.code.clone()).unwrap_or_else(|| "flutter-missing".into());
                return self.end("failed", Some(EnvIssue::with("spawn", detail)));
            };
            if !self.pub_get_if_needed(root) {
                return;
            }
        }
        if !ctx.cfg_before().is_empty() {
            self.set_state("before-launch", SessionPatch::default());
            let last_head = Mutex::new(String::new());
            let resolve = |head: &str| -> Option<Resolved> {
                *last_head.lock().unwrap() = head.to_string();
                sdk.as_deref().and_then(|s| sdk_tool(s, head, ctx.win, &ctx.env)).or_else(|| resolve_for(ctx, head))
            };
            let ok = run_before_launch(ctx, &resolve, &|p| self.track_step(p), &|| self.st.lock().unwrap().stop_requested);
            self.track_step(None);
            if self.stopping() {
                return self.end("cancelled", None);
            }
            if !ok {
                let head = last_head.lock().unwrap().clone();
                return self.end("failed", Some(EnvIssue::with("before-launch", pth::basename(&head, ctx.win))));
            }
        }
        self.set_state("starting", SessionPatch::default());
        let spec = match self.launch_spec(sdk.as_deref()) {
            Ok(s) => s,
            Err(e) => return self.end("failed", Some(e)),
        };
        let mut child = match spawn(&spec, &ctx.env) {
            Ok(c) => c,
            Err(e) => return self.end("failed", Some(EnvIssue::with("spawn", e.to_string()))),
        };
        let pid = child.id();
        let stdin = child.stdin.take().map(|w| Box::new(w) as Box<dyn std::io::Write + Send>);
        let client = MachineClient::new(stdin);
        *self.machine.lock().unwrap() = Some(client.clone());
        let kill_now = {
            let mut s = self.st.lock().unwrap();
            s.child = Some(pid);
            s.stop_requested
        };
        if kill_now {
            kill_pid(pid, false);
        }
        let (tx, rx) = mpsc::channel::<()>();
        let (me_e, me_t, me_s) = (self.clone(), self.clone(), self.clone());
        let handlers = MachineHandlers { on_event: Arc::new(move |ev, p| me_e.on_event(ev, p)), on_text: Arc::new(move |l| me_t.plain(l)) };
        let mut pumps = Vec::new();
        if let Some(out) = child.stdout.take() {
            pumps.push(client.attach(out, handlers));
        }
        if let Some(err) = child.stderr.take() {
            pumps.push(pump_lines(err, move |l| me_s.plain(l)));
        }
        for h in pumps {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let _ = h.join();
                let _ = tx.send(());
            });
        }
        drop(tx);
        let code = child.wait().ok().and_then(|s| s.code());
        while rx.recv_timeout(Duration::from_millis(EXIT_GRACE_MS)).is_ok() {}
        self.on_close(code);
    }

    fn stop(&self) {
        let (child, wait) = {
            let mut s = self.st.lock().unwrap();
            if s.stop_called {
                (None, !s.ended && s.child.is_some())
            } else {
                s.stop_called = true;
                s.stop_requested = true;
                if let Some(p) = s.step {
                    kill_pid(p, false);
                }
                (s.child.filter(|_| !s.ended), false)
            }
        };
        let timeout = Duration::from_millis(self.deps.stop_timeout_ms.unwrap_or(STOP_TIMEOUT_MS));
        if wait {
            self.wait_exit(timeout + Duration::from_millis(EXIT_GRACE_MS * 2));
            return;
        }
        let Some(pid) = child else { return };
        self.set_state("stopping", SessionPatch::default());
        let app_id = self.st.lock().unwrap().app_id.clone();
        match (app_id, self.machine.lock().unwrap().clone()) {
            (Some(id), Some(m)) => {
                let _ = m.send("app.stop", json!({"appId": id}));
            }
            _ => kill_pid(pid, false),
        }
        if !self.wait_exit(timeout) {
            kill_pid(pid, false);
            self.wait_exit(Duration::from_millis(EXIT_GRACE_MS * 2));
        }
    }

    fn wait_exit(&self, timeout: Duration) -> bool {
        let s = self.st.lock().unwrap();
        let (s, _) = self.exited.wait_timeout_while(s, timeout, |s| !s.ended).unwrap();
        s.ended
    }

    fn kill(&self) {
        let (step, child) = {
            let mut s = self.st.lock().unwrap();
            s.stop_requested = true;
            (s.step, s.child.filter(|_| !s.ended))
        };
        if let Some(p) = step {
            kill_pid(p, true);
        }
        if let Some(p) = child {
            kill_pid(p, true);
        }
    }
}

impl Runner for FlutterRunner {
    fn start(&self) {
        self.inner.start();
    }

    fn stop(&self) {
        self.inner.stop();
    }

    fn kill(&self) {
        self.inner.kill();
    }

    fn reload(&self, full: bool) -> RunResult {
        self.inner.reload(full)
    }
}

#[path = "runner_events.rs"]
mod events;

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
