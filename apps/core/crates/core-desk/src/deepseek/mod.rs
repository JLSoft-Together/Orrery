pub mod cli;
pub mod parse;
pub mod runner;
pub mod sync;
pub mod templates;

use crate::{arg, claude, js, Ctx, Desk, DeskEnv, DeskError};
use cli::{Codewhale, Env, Step};
use runner::{RunOpts, RunResult, Runner};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sync::SyncPaths;

pub const MODULE: &str = "deepseek";
pub const STATUS_TTL_MS: f64 = 60_000.0;
pub const SYNC_RECORD: &str = "deepseek-sync.json";
const QUICK_TIMEOUT: Duration = Duration::from_secs(30);
const DOCTOR_TIMEOUT: Duration = Duration::from_secs(90);
const INSTALL_TIMEOUT_MS: f64 = 15.0 * 60_000.0;

pub fn feature_on() -> bool {
    std::env::var("CCM_DEEPSEEK_FEATURE").is_ok_and(|v| v == "1")
}

fn result(ok: bool, code: Option<&str>) -> Value {
    match code {
        Some(c) => json!({"ok": ok, "code": c}),
        None => json!({"ok": ok}),
    }
}

fn fail(code: &str) -> Value {
    result(false, Some(code))
}

struct Guard<'a>(&'a Mutex<Option<&'static str>>);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        *self.0.lock().unwrap() = None;
    }
}

pub struct Service {
    env: Env,
    home: String,
    win: bool,
    record_file: PathBuf,
    paths: SyncPaths,
    run: Runner,
    busy: Mutex<Option<&'static str>>,
    abort: Mutex<Option<Arc<AtomicBool>>>,
    cache: Mutex<Option<(f64, Value)>>,
}

impl Service {
    pub fn new(env: Env, home: String, win: bool, data_dir: &Path, run: Runner) -> Self {
        let root = match env.get("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
            Some(d) => d.clone(),
            None => crate::util::path_str(&Path::new(&home).join(".claude")),
        };
        let claude_json = match env.get("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
            Some(d) => crate::util::path_str(&Path::new(d).join(".claude.json")),
            None => crate::util::path_str(&Path::new(&home).join(".claude.json")),
        };
        let codex_home = match env.get("CODEX_HOME").filter(|v| !v.is_empty()) {
            Some(d) => d.clone(),
            None => crate::util::path_str(&Path::new(&home).join(".codex")),
        };
        let paths = SyncPaths { home: home.clone(), codewhale_home: crate::util::path_str(&Path::new(&home).join(".codewhale")), claude_root: root, claude_json, codex_home };
        Service { env, home, win, record_file: data_dir.join(SYNC_RECORD), paths, run, busy: Mutex::new(None), abort: Mutex::new(None), cache: Mutex::new(None) }
    }

    fn find(&self) -> Option<Codewhale> {
        cli::find_codewhale(&self.env, &self.home, self.win)
    }

    fn ready(&self) -> Option<Codewhale> {
        self.find().filter(|c| !c.needs_download)
    }

    fn invalidate(&self) {
        *self.cache.lock().unwrap() = None;
    }

    fn tilde(&self, file: &str) -> String {
        let home = claude::resolve(&self.home);
        let f = claude::resolve(file);
        if claude::is_within(&home, &f, self.win) && claude::fold(&f, self.win) != claude::fold(&home, self.win) {
            let parts: Vec<&str> = f[home.len()..].split(['/', '\\']).filter(|p| !p.is_empty()).collect();
            return format!("~/{}", parts.join("/"));
        }
        file.to_string()
    }

    fn clean(&self, text: &str, secret: Option<&str>) -> String {
        let mut out = match secret {
            Some(s) if !s.is_empty() => text.replace(s, "••••"),
            _ => text.to_string(),
        };
        out = core_sources::mask::mask_secrets(&out, &self.home, if self.win { "win32" } else { "darwin" });
        let t = claude::js_trim(&out);
        if core_transcript::jsstr::units(t) > 400 {
            format!("…{}", claude::tail(t, 400))
        } else {
            t.to_string()
        }
    }

    fn failure(&self, res: &RunResult, secret: Option<&str>) -> Value {
        if res.cancelled {
            return fail("cancelled");
        }
        if res.timed_out {
            return fail("timeout");
        }
        let detail = self.clean(&format!("{}\n{}", res.stderr, res.stdout), secret);
        if detail.is_empty() {
            fail("failed")
        } else {
            json!({"ok": false, "code": "failed", "detail": detail})
        }
    }

    fn cli(&self, cmd: &Codewhale, args: &[&str], timeout: Duration, stdin: Option<String>) -> RunResult {
        let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        (self.run)(&cmd.cmd, &a, &RunOpts { cwd: &self.home, timeout, stdin, on_line: None, cancel: None })
    }

    fn exclusive(&self, kind: &'static str, work: impl FnOnce() -> Result<Value, DeskError>) -> Result<Value, DeskError> {
        {
            let mut b = self.busy.lock().unwrap();
            if b.is_some() {
                return Ok(fail("busy"));
            }
            *b = Some(kind);
        }
        let _guard = Guard(&self.busy);
        work()
    }

    fn cli_part(&self) -> Value {
        let cw = self.find();
        let route = cli::install_plan(&self.env, &self.home, self.win).map(|p| p.route);
        let mut base = json!({
            "installed": cw.as_ref().is_some_and(|c| !c.needs_download),
            "needsDownload": cw.as_ref().is_some_and(|c| c.needs_download),
            "version": null, "loggedIn": null, "telemetry": null,
            "installRoute": route,
            "origin": cw.as_ref().map(|c| c.origin),
        });
        let Some(cw) = cw.filter(|c| !c.needs_download) else { return base };
        let (ver, auth, tele) = std::thread::scope(|s| {
            let v = s.spawn(|| self.cli(&cw, &["--version"], QUICK_TIMEOUT, None));
            let a = s.spawn(|| self.cli(&cw, &["auth", "status"], QUICK_TIMEOUT, None));
            let t = self.cli(&cw, &["config", "get", "telemetry"], QUICK_TIMEOUT, None);
            (v.join().unwrap_or_default(), a.join().unwrap_or_default(), t)
        });
        base["version"] = if ver.code == Some(0) { parse::parse_version(&ver.stdout).into() } else { Value::Null };
        base["loggedIn"] = if auth.code == Some(0) { parse::parse_auth_status(&auth.stdout, "deepseek").into() } else { Value::Null };
        base["telemetry"] = if tele.code == Some(0) { parse::parse_telemetry(&tele.stdout).into() } else { Value::Null };
        base
    }

    pub fn status(&self, fresh: bool) -> Value {
        let busy = *self.busy.lock().unwrap();
        let now = crate::util::now_ms();
        let cached = self.cache.lock().unwrap().clone();
        let mut cli = cached.as_ref().filter(|(at, _)| !fresh && now - at < STATUS_TTL_MS).map(|(_, v)| v.clone());
        if cli.is_none() {
            let held = if matches!(busy, Some("install" | "update")) { cached.as_ref().map(|(_, v)| v.clone()) } else { None };
            let part = held.unwrap_or_else(|| self.cli_part());
            if self.busy.lock().unwrap().is_none() {
                *self.cache.lock().unwrap() = Some((crate::util::now_ms(), part.clone()));
            }
            cli = Some(part);
        }
        let record = sync::read_sync_record(&self.record_file);
        let hash = sync::current_fingerprint(&self.paths);
        let mut out = match cli {
            Some(Value::Object(m)) => m,
            _ => Map::new(),
        };
        out.insert("sync".into(), json!(sync::sync_state(record.as_ref(), &hash)));
        out.insert("syncedAt".into(), record.as_ref().map(|r| json!(r.at)).unwrap_or(Value::Null));
        out.insert("busy".into(), json!(*self.busy.lock().unwrap()));
        Value::Object(out)
    }

    fn confirm(&self, ctx: &Ctx, plan: Value) -> bool {
        matches!(ctx.host("deepseek.confirm", plan), Ok(Value::Bool(true)))
    }

    fn run_install(&self, ctx: &Ctx, steps: &[Step], wrapper_after: impl FnOnce() -> Option<crate::preview::format::Resolved>) -> Value {
        let cancel = Arc::new(AtomicBool::new(false));
        *self.abort.lock().unwrap() = Some(cancel.clone());
        let deadline = crate::util::now_ms() + INSTALL_TIMEOUT_MS;
        let left = || Duration::from_millis((deadline - crate::util::now_ms()).max(1_000.0) as u64);
        let emit_line = |line: &str| ctx.emit("ccm:deepseek-event", json!({"phase": "install", "line": js::slice(&self.clean(line, None), 200)}));
        let out = (|| {
            for step in steps {
                let res = (self.run)(&step.cmd, &[], &RunOpts { cwd: &self.home, timeout: left(), stdin: None, on_line: Some(&emit_line), cancel: Some(cancel.clone()) });
                if res.code != Some(0) {
                    return self.failure(&res, None);
                }
            }
            if let Some(wrapper) = wrapper_after() {
                let on_line = |line: &str| match parse::parse_download_progress(line) {
                    Some(p) => ctx.emit("ccm:deepseek-event", json!({"phase": "download", "mb": p.mb, "totalMb": p.total_mb, "pct": p.pct})),
                    None => emit_line(line),
                };
                let res = (self.run)(&wrapper, &["--version".to_string()], &RunOpts { cwd: &self.home, timeout: left(), stdin: None, on_line: Some(&on_line), cancel: Some(cancel.clone()) });
                if res.code != Some(0) {
                    return self.failure(&res, None);
                }
            }
            if self.find().is_some_and(|c| !c.needs_download) {
                result(true, None)
            } else {
                fail("notFound")
            }
        })();
        *self.abort.lock().unwrap() = None;
        self.invalidate();
        out
    }

    pub fn install(&self, ctx: &Ctx) -> Result<Value, DeskError> {
        self.exclusive("install", || {
            let cw = self.find();
            if cw.as_ref().is_some_and(|c| !c.needs_download) {
                return Ok(fail("installed"));
            }
            let steps = match &cw {
                Some(_) => Vec::new(),
                None => match cli::install_plan(&self.env, &self.home, self.win) {
                    Some(p) => p.steps,
                    None => return Ok(fail("noInstaller")),
                },
            };
            let mut commands: Vec<String> = if cw.is_some() { Vec::new() } else { steps.iter().map(|s| s.display.clone()).collect() };
            commands.push("codewhale --version".into());
            if !self.confirm(ctx, json!({"kind": if cw.is_some() { "download" } else { "install" }, "commands": commands})) {
                return Ok(fail("cancelled"));
            }
            Ok(self.run_install(ctx, &steps, || self.find().filter(|a| a.needs_download).map(|a| a.cmd)))
        })
    }

    pub fn check_update(&self) -> Value {
        let Some(cw) = self.ready() else { return fail("notInstalled") };
        let res = self.cli(&cw, &["doctor", "--check-updates"], DOCTOR_TIMEOUT, None);
        match parse::parse_doctor_updates(&res.stdout) {
            None if res.code == Some(0) => fail("failed"),
            None => self.failure(&res, None),
            Some((current, latest)) => {
                let newer = parse::compare_versions(&latest, &current) > 0;
                json!({"ok": true, "current": current, "latest": latest, "newer": newer})
            }
        }
    }

    pub fn update(&self, ctx: &Ctx) -> Result<Value, DeskError> {
        self.exclusive("update", || {
            let Some(cw) = self.ready() else { return Ok(fail("notInstalled")) };
            let Some(steps) = cli::update_plan(&cw, &self.env, &self.home, self.win) else { return Ok(fail("noInstaller")) };
            let mut commands: Vec<String> = steps.iter().map(|s| s.display.clone()).collect();
            if cw.origin == "npm" {
                commands.push("codewhale --version".into());
            }
            if !self.confirm(ctx, json!({"kind": "update", "commands": commands})) {
                return Ok(fail("cancelled"));
            }
            let wrapper = if cw.origin == "npm" { cw.wrapper.clone() } else { None };
            Ok(self.run_install(ctx, &steps, || wrapper))
        })
    }

    pub fn cancel_value(&self) -> Value {
        Value::Bool(self.cancel())
    }

    pub fn cancel(&self) -> bool {
        match self.abort.lock().unwrap().as_ref() {
            Some(a) => {
                a.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn login(&self, raw: &Value) -> Result<Value, DeskError> {
        self.exclusive("login", || {
            let Some(key) = parse::clean_api_key(raw) else { return Ok(fail("badInput")) };
            let Some(cw) = self.ready() else { return Ok(fail("notInstalled")) };
            let res = self.cli(&cw, &["auth", "set", "--provider", "deepseek", "--api-key-stdin"], QUICK_TIMEOUT, Some(format!("{key}\n")));
            self.invalidate();
            Ok(if res.code == Some(0) { result(true, None) } else { self.failure(&res, Some(&key)) })
        })
    }

    pub fn logout(&self, ctx: &Ctx) -> Result<Value, DeskError> {
        self.exclusive("logout", || {
            let Some(cw) = self.ready() else { return Ok(fail("notInstalled")) };
            if !self.confirm(ctx, json!({"kind": "logout"})) {
                return Ok(fail("cancelled"));
            }
            let res = self.cli(&cw, &["auth", "clear", "--provider", "deepseek"], QUICK_TIMEOUT, Some("y\n".into()));
            self.invalidate();
            Ok(if res.code == Some(0) { result(true, None) } else { self.failure(&res, None) })
        })
    }

    pub fn telemetry(&self, on: &Value) -> Result<Value, DeskError> {
        self.exclusive("telemetry", || {
            let Some(on) = on.as_bool() else { return Ok(fail("badInput")) };
            let Some(cw) = self.ready() else { return Ok(fail("notInstalled")) };
            let res = self.cli(&cw, &["config", "set", "telemetry", if on { "true" } else { "false" }], QUICK_TIMEOUT, None);
            self.invalidate();
            Ok(if res.code == Some(0) { result(true, None) } else { self.failure(&res, None) })
        })
    }

    pub fn sync(&self, ctx: &Ctx) -> Result<Value, DeskError> {
        self.exclusive("sync", || {
            let p = sync::plan_sync(&self.paths).map_err(|e| DeskError::Failed(e.to_string()))?;
            let files: Vec<Value> = p
                .files
                .iter()
                .map(|f| {
                    let mut m = Map::new();
                    m.insert("id".into(), json!(f.id));
                    m.insert("file".into(), json!(self.tilde(&f.file)));
                    m.insert("action".into(), json!(f.action.name()));
                    if let Some(r) = f.reason {
                        m.insert("reason".into(), json!(r));
                    }
                    Value::Object(m)
                })
                .collect();
            let writes: Vec<Value> = files.iter().filter(|f| matches!(f["action"].as_str(), Some("create" | "update"))).map(|f| json!({"file": f["file"], "action": f["action"]})).collect();
            if !writes.is_empty() && !self.confirm(ctx, json!({"kind": "sync", "files": writes, "mcpAdded": p.mcp_added, "outboxAdded": p.outbox_added})) {
                return Ok(fail("cancelled"));
            }
            if let Err(e) = sync::apply_sync(&p, &self.record_file, crate::util::now_ms()) {
                return Ok(json!({"ok": false, "code": "failed", "detail": self.clean(&e.to_string(), None)}));
            }
            Ok(json!({"ok": true, "files": files, "mcpAdded": p.mcp_added}))
        })
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    if !feature_on() {
        return;
    }
    let sys = claude::Sys::from_env();
    let vars: Env = std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
    let child = crate::extensions::status::child_env(&vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>());
    let svc = Arc::new(Service::new(vars, sys.home.clone(), sys.win, &env.data_dir, runner::default_runner(child)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-status", move |_, args| Ok(s.status(arg(args, 0) == &Value::Bool(true))));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-install", move |ctx, _| s.install(ctx));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-cancel", move |_, _| Ok(Value::Bool(s.cancel())));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-login", move |_, args| s.login(arg(args, 0)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-logout", move |ctx, _| s.logout(ctx));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-telemetry", move |_, args| s.telemetry(arg(args, 0)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-sync", move |ctx, _| s.sync(ctx));
    let s = svc.clone();
    desk.on(MODULE, "ccm:deepseek-check-update", move |_, _| Ok(s.check_update()));
    desk.on(MODULE, "ccm:deepseek-update", move |ctx, _| svc.update(ctx));
}

#[cfg(test)]
mod tests;
