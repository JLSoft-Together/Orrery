pub mod commands;
pub mod parse;
pub mod status;

use crate::claude::{self, check_dir, proc, DirArg, Sys};
use crate::{arg, Ctx, Desk, DeskEnv, DeskError};
use commands::CliStep;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const MODULE: &str = "extensions";
const LIST_TIMEOUT: u64 = 60_000;
const MCP_LIST_TIMEOUT: u64 = 120_000;
const MUTATE_TIMEOUT: u64 = 180_000;
const LOGIN_TIMEOUT: u64 = 300_000;
const MCP_TTL: f64 = 30_000.0;
const OUT_CAP: usize = 48 * 1024 * 1024;

pub struct CliRun {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

pub type Runner = Box<dyn Fn(&str, &[String], &str, u64) -> CliRun + Send + Sync>;

pub struct ExtService {
    pub sys: Sys,
    pub env: BTreeMap<String, String>,
    pub child: BTreeMap<String, String>,
    pub run: Runner,
    mcp: Mutex<HashMap<String, Value>>,
    busy: AtomicBool,
}

pub fn default_runner(child: BTreeMap<String, String>) -> Runner {
    Box::new(move |file, args, cwd, timeout| {
        let out = proc::run(proc::RunOpts { file, args, cwd, env: &child, stdin: None, timeout: Duration::from_millis(timeout), tree: false, cap: OUT_CAP });
        match out {
            Some(o) => CliRun { code: o.code, stdout: o.stdout, stderr: o.stderr, timed_out: o.timed_out },
            None => CliRun { code: None, stdout: String::new(), stderr: String::new(), timed_out: false },
        }
    })
}

struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn fail(code: &str) -> Value {
    json!({"ok": false, "code": code})
}

fn strip(res: Value) -> Value {
    if res["ok"] == true {
        return json!({"ok": true});
    }
    let mut out = Map::new();
    out.insert("ok".into(), Value::Bool(false));
    out.insert("code".into(), res["code"].clone());
    if let Some(d) = res.get("detail").and_then(Value::as_str).filter(|d| !d.is_empty()) {
        out.insert("detail".into(), Value::String(d.into()));
    }
    Value::Object(out)
}

fn has_any(text: &str, words: &[&str]) -> bool {
    let low = text.to_lowercase();
    words.iter().any(|w| low.contains(w))
}

impl ExtService {
    pub fn new(sys: Sys, env: Vec<(String, String)>, run: Option<Runner>) -> Self {
        let child = status::child_env(&env);
        let run = run.unwrap_or_else(|| default_runner(child.clone()));
        ExtService { sys, env: env.into_iter().collect(), child, run, mcp: Mutex::new(HashMap::new()), busy: AtomicBool::new(false) }
    }

    fn take(&self) -> Option<Busy<'_>> {
        self.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| Busy(&self.busy))
    }

    fn cli(&self) -> Option<String> {
        status::find_claude_cli(&self.env, &self.sys)
    }

    fn clean(&self, text: &str, secrets: &[String]) -> String {
        let mut out = text.to_string();
        for s in secrets.iter().filter(|s| !s.is_empty()) {
            out = out.replace(s.as_str(), "••••");
        }
        let masked = core_sources::mask::mask_secrets(&out, &self.sys.home, self.sys.platform());
        let t = claude::js_trim(&masked);
        if core_transcript::jsstr::units(t) > 600 {
            format!("…{}", claude::tail(t, 600))
        } else {
            t.to_string()
        }
    }

    fn marketplaces(&self, cli: &str, cwd: &str) -> Vec<String> {
        let r = (self.run)(cli, &["plugin".into(), "marketplace".into(), "list".into(), "--json".into()], cwd, LIST_TIMEOUT);
        if r.code == Some(0) {
            parse::parse_marketplaces(&r.stdout)
        } else {
            Vec::new()
        }
    }

    fn run_steps(&self, cli: &str, steps: &[CliStep], cwd: &str, timeout: u64) -> Value {
        let mut output = String::new();
        for step in steps {
            let res = (self.run)(cli, &step.args, cwd, timeout);
            output = format!("{}\n{}", res.stdout, res.stderr);
            if res.code == Some(0) {
                continue;
            }
            let detail = self.clean(&output, &step.secrets);
            if res.timed_out {
                return json!({"ok": false, "code": "timeout", "detail": detail});
            }
            if has_any(&output, &["showncommand", "--accept-command", "require -y", "requires -y", "require --yes", "requires --yes", "not a tty"]) {
                return json!({"ok": false, "code": "needsTerminal", "detail": detail});
            }
            if step.args.get(1).map(String::as_str) == Some("marketplace") && step.args.get(2).map(String::as_str) == Some("add") && has_any(&output, &["already"]) {
                continue;
            }
            return json!({"ok": false, "code": "failed", "detail": detail});
        }
        json!({"ok": true, "output": output})
    }

    fn home(&self) -> String {
        self.sys.home.clone()
    }

    pub fn status(&self, dir: Option<&str>) -> Value {
        let cli = self.cli();
        let cwd = dir.map(str::to_string).unwrap_or_else(|| self.home());
        let win = self.sys.win;
        let (plugins, markets) = match &cli {
            Some(cli) => std::thread::scope(|s| {
                let p = s.spawn(|| {
                    let r = (self.run)(cli, &["plugin".into(), "list".into(), "--json".into()], &cwd, LIST_TIMEOUT);
                    if r.code == Some(0) {
                        parse::parse_plugin_list(&r.stdout, &|p| dir.is_some_and(|d| claude::same_dir(p, d, win)))
                    } else {
                        Vec::new()
                    }
                });
                let m = self.marketplaces(cli, &cwd);
                (p.join().unwrap_or_default(), m)
            }),
            None => (Vec::new(), Vec::new()),
        };
        let user_dir = claude::join(&crate::util::path_str(&self.sys.claude_root()), &["skills"]);
        let project_dir = dir.map(|d| claude::join(d, &[".claude", "skills"]));
        let configured = status::read_configured(&self.sys.claude_json(), dir, win);
        json!({
            "cli": cli.is_some(),
            "dir": dir,
            "prereqs": status::probe_prereqs(&self.env, &self.sys),
            "plugins": plugins,
            "marketplaces": markets,
            "configured": status::configured_json(&configured),
            "skills": {"user": status::list_skills(&user_dir), "project": project_dir.as_deref().map(status::list_skills).unwrap_or_default()},
            "skillDirs": {"user": user_dir, "project": project_dir},
        })
    }

    pub fn mcp(&self, dir: Option<&str>, fresh: bool) -> Value {
        let key = dir.unwrap_or("").to_string();
        if let Some(hit) = self.mcp.lock().unwrap().get(&key) {
            if !fresh && crate::util::now_ms() - hit["at"].as_f64().unwrap_or(0.0) < MCP_TTL {
                return hit.clone();
            }
        }
        let Some(cli) = self.cli() else { return Value::Null };
        let cwd = dir.map(str::to_string).unwrap_or_else(|| self.home());
        let res = (self.run)(&cli, &["mcp".into(), "list".into()], &cwd, MCP_LIST_TIMEOUT);
        if res.code != Some(0) && claude::js_trim(&res.stdout).is_empty() {
            return Value::Null;
        }
        let configured = status::read_configured(&self.sys.claude_json(), dir, self.sys.win);
        let servers: Vec<Value> = parse::parse_mcp_list(&res.stdout)
            .into_iter()
            .map(|mut row| {
                let name = row["name"].as_str().unwrap_or("").to_string();
                let scopes = configured.iter().rev().find(|(n, _)| *n == name).map(|(_, s)| json!(s)).unwrap_or_else(|| json!([]));
                row.as_object_mut().map(|m| m.insert("scopes".into(), scopes));
                row
            })
            .collect();
        let out = json!({"at": crate::util::now_ms() as u64, "servers": servers});
        self.mcp.lock().unwrap().insert(key, out.clone());
        out
    }

    fn mutate(&self, ctx: &Ctx, kind: &str, scope: &Value, raw_dir: &Value, build: &dyn Fn(&str, &str) -> Option<Vec<CliStep>>) -> Result<Value, DeskError> {
        let Some(_busy) = self.take() else { return Ok(fail("busy")) };
        let dir = match check_dir(raw_dir) {
            DirArg::Bad => return Ok(fail("badInput")),
            DirArg::None => None,
            DirArg::Dir(d) => Some(d),
        };
        let project = !scope.is_null() && scope != "user";
        if project && dir.is_none() {
            return Ok(fail("badInput"));
        }
        let Some(cli) = self.cli() else { return Ok(fail("noCli")) };
        let cwd = if project { dir.clone().unwrap_or_default() } else { self.home() };
        let Some(steps) = build(&cli, &cwd) else { return Ok(fail("badInput")) };
        let plan = json!({
            "kind": kind,
            "commands": steps.iter().map(commands::show_command).collect::<Vec<_>>(),
            "scope": scope,
            "dir": if scope == "user" { None } else { dir.clone() },
        });
        if ctx.host("extensions.confirm", plan)? != Value::Bool(true) {
            return Ok(fail("cancelled"));
        }
        let mut res = self.run_steps(&cli, &steps, &cwd, MUTATE_TIMEOUT);
        self.mcp.lock().unwrap().clear();
        if let Some(m) = res.as_object_mut() {
            m.insert("cwd".into(), Value::String(cwd));
            m.insert("cli".into(), Value::String(cli));
        }
        Ok(res)
    }

    pub fn install(&self, ctx: &Ctx, req: &Value) -> Result<Value, DeskError> {
        let scope = req.get("scope").cloned().unwrap_or(Value::Null);
        let win = self.sys.win;
        let res = self.mutate(ctx, "install", &scope, req.get("dir").unwrap_or(&Value::Null), &|cli, cwd| commands::build_install(req, win, &self.marketplaces(cli, cwd)))?;
        Ok(strip(res))
    }

    pub fn remove(&self, ctx: &Ctx, req: &Value) -> Result<Value, DeskError> {
        let scope = req.get("scope").cloned().unwrap_or(Value::Null);
        Ok(strip(self.mutate(ctx, "remove", &scope, req.get("dir").unwrap_or(&Value::Null), &|_, _| commands::build_remove(req))?))
    }

    pub fn source(&self, ctx: &Ctx, req: &Value) -> Result<Value, DeskError> {
        let market = req.get("type").and_then(Value::as_str) == Some("marketplace");
        let scope = if market { Value::Null } else { req.get("scope").cloned().unwrap_or(Value::Null) };
        let dir = if market { Value::Null } else { req.get("dir").cloned().unwrap_or(Value::Null) };
        let res = self.mutate(ctx, "source", &scope, &dir, &|_, _| commands::build_source(req))?;
        let (Some(cli), Some(cwd)) = (res.get("cli").and_then(Value::as_str), res.get("cwd").and_then(Value::as_str)) else { return Ok(strip(res)) };
        if res["ok"] != true || !market {
            return Ok(strip(res));
        }
        let marketplace = parse::marketplace_from_output(res.get("output").and_then(Value::as_str).unwrap_or(""), req.get("repo").and_then(Value::as_str).unwrap_or(""));
        let avail = (self.run)(cli, &["plugin".into(), "list".into(), "--available".into(), "--json".into()], cwd, LIST_TIMEOUT);
        let plugins = if avail.code == Some(0) { parse::parse_available(&avail.stdout, &marketplace, 300) } else { Vec::new() };
        Ok(json!({"ok": true, "marketplace": marketplace, "plugins": plugins}))
    }

    pub fn login(&self, name: &Value, raw_dir: &Value) -> Value {
        let ok_name = name.as_str().filter(|n| {
            let mut cs = n.chars();
            matches!(cs.next(), Some(c) if c.is_ascii_alphanumeric()) && n.len() <= 100 && cs.all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
        });
        let Some(name) = ok_name else { return fail("badInput") };
        if self.busy.load(Ordering::SeqCst) {
            return fail("busy");
        }
        let dir = match check_dir(raw_dir) {
            DirArg::Bad => return fail("badInput"),
            DirArg::None => None,
            DirArg::Dir(d) => Some(d),
        };
        let Some(_busy) = self.take() else { return fail("busy") };
        let Some(cli) = self.cli() else { return fail("noCli") };
        let cwd = dir.unwrap_or_else(|| self.home());
        let step = CliStep { args: vec!["mcp".into(), "login".into(), name.into()], secrets: Vec::new() };
        let res = self.run_steps(&cli, &[step], &cwd, LOGIN_TIMEOUT);
        self.mcp.lock().unwrap().clear();
        strip(res)
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    register_with(desk, ExtService::new(Sys::from_env(), proc::env_vars(), None));
}

pub fn register_with(desk: &mut Desk, svc: ExtService) {
    let svc = Arc::new(svc);
    let dir_of = |v: &Value| match check_dir(v) {
        DirArg::Bad => Err(()),
        DirArg::None => Ok(None),
        DirArg::Dir(d) => Ok(Some(d)),
    };
    let s = svc.clone();
    desk.on(MODULE, "ccm:ext-status", move |_, args| Ok(dir_of(arg(args, 0)).map(|d| s.status(d.as_deref())).unwrap_or(Value::Null)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:ext-mcp", move |_, args| Ok(dir_of(arg(args, 0)).map(|d| s.mcp(d.as_deref(), arg(args, 1) == &Value::Bool(true))).unwrap_or(Value::Null)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:ext-install", move |ctx, args| s.install(ctx, arg(args, 0)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:ext-remove", move |ctx, args| s.remove(ctx, arg(args, 0)));
    let s = svc.clone();
    desk.on(MODULE, "ccm:ext-source", move |ctx, args| s.source(ctx, arg(args, 0)));
    let s = svc;
    desk.on(MODULE, "ccm:ext-login", move |_, args| Ok(s.login(arg(args, 0), arg(args, 1))));
}
