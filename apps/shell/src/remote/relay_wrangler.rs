use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use core_desk::deepseek::runner::{default_runner, RunOpts};
use core_desk::preview::format::Resolved;
use regex::Regex;
use serde_json::{json, Map, Value};

use super::super::cf_api::{self, Account, Res, SCRIPT};
use super::super::Remote;
use super::{bundle_meta, Failure, VIA_WRANGLER, WORKER_JS};
use crate::state::lock;

pub const WRANGLER: &str = "wrangler@4.160.1";
pub const WHOAMI_WAIT: Duration = Duration::from_secs(180);
pub const LOGIN_WAIT: Duration = Duration::from_secs(300);
pub const DEPLOY_WAIT: Duration = Duration::from_secs(300);
pub const DELETE_WAIT: Duration = Duration::from_secs(120);
const TAIL_LINES: usize = 6;
const TAIL_CHARS: usize = 600;
const NAME_CHARS: usize = 80;
const OFFLINE: [&str; 7] = ["fetch failed", "ENOTFOUND", "ECONNREFUSED", "ECONNRESET", "EAI_AGAIN", "ETIMEDOUT", "getaddrinfo"];
const LOGGED_OUT: [&str; 3] = ["not authenticated", "wrangler login", "\"loggedIn\": false"];
const NO_SUBDOMAIN: &str = "register a workers.dev subdomain";

#[derive(Debug, Default, Clone)]
pub struct Run {
    pub code: Option<i32>,
    pub out: String,
    pub err: String,
    pub timed_out: bool,
    pub cancelled: bool,
    pub missing: bool,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.code == Some(0) && !self.timed_out && !self.cancelled
    }

    fn text(&self) -> String {
        format!("{}\n{}", self.out, self.err)
    }
}

pub struct Call<'a> {
    pub args: &'a [&'a str],
    pub cwd: &'a Path,
    pub account: Option<&'a str>,
    pub timeout: Duration,
}

pub trait Wrangler: Send + Sync {
    fn run(&self, call: &Call, cancel: &Arc<AtomicBool>) -> Run;
}

pub struct Npx;

fn path_key(env: &BTreeMap<String, String>, win: bool) -> String {
    env.keys().find(|k| if win { k.eq_ignore_ascii_case("PATH") } else { k.as_str() == "PATH" }).cloned().unwrap_or_else(|| "PATH".into())
}

fn extra_dirs(env: &BTreeMap<String, String>, win: bool) -> Vec<PathBuf> {
    let get = |k: &str| env.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).map(|(_, v)| PathBuf::from(v)).filter(|p| p.is_absolute());
    let mut out = Vec::new();
    if win {
        out.extend(get("ProgramFiles").map(|p| p.join("nodejs")));
        out.extend(get("APPDATA").map(|p| p.join("npm")));
        out.extend(get("LOCALAPPDATA").map(|p| p.join("Volta").join("bin")));
    } else {
        out.push(PathBuf::from("/opt/homebrew/bin"));
        out.push(PathBuf::from("/usr/local/bin"));
        if let Some(h) = get("HOME") {
            out.push(h.join(".volta").join("bin"));
            out.push(h.join(".local").join("bin"));
        }
    }
    out
}

pub fn find_npx(env: &BTreeMap<String, String>, win: bool) -> Option<PathBuf> {
    core_desk::extensions::status::find_on_path(if win { "npx.cmd" } else { "npx" }, env, &extra_dirs(env, win), win)
}

impl Wrangler for Npx {
    fn run(&self, call: &Call, cancel: &Arc<AtomicBool>) -> Run {
        let win = cfg!(windows);
        let mut env: BTreeMap<String, String> = core_desk::claude::proc::env_vars().into_iter().collect();
        let Some(npx) = find_npx(&env, win) else { return Run { missing: true, ..Run::default() } };
        let key = path_key(&env, win);
        let dir = npx.parent().map(core_desk::util::path_str).unwrap_or_default();
        let old = env.get(&key).cloned().unwrap_or_default();
        env.insert(key, if old.is_empty() { dir } else { format!("{dir}{}{old}", if win { ';' } else { ':' }) });
        env.insert("WRANGLER_SEND_METRICS".into(), "false".into());
        env.insert("NO_COLOR".into(), "1".into());
        env.insert("FORCE_COLOR".into(), "0".into());
        env.remove("CLOUDFLARE_ACCOUNT_ID");
        if let Some(a) = call.account {
            env.insert("CLOUDFLARE_ACCOUNT_ID".into(), a.to_owned());
        }
        let mut args = vec!["--yes".to_owned(), format!("--package={WRANGLER}"), "wrangler".to_owned()];
        args.extend(call.args.iter().map(|a| (*a).to_owned()));
        let resolved = Resolved { file: core_desk::util::path_str(&npx), args, verbatim: false };
        let cwd = core_desk::util::path_str(call.cwd);
        let opts = RunOpts { cwd: &cwd, timeout: call.timeout, stdin: None, on_line: None, cancel: Some(cancel.clone()) };
        let r = default_runner(env)(&resolved, &[], &opts);
        Run { code: r.code, out: r.stdout, err: r.stderr, timed_out: r.timed_out, cancelled: r.cancelled, missing: r.spawn_error }
    }
}

pub struct WorkDir(pub PathBuf);

impl WorkDir {
    pub fn new() -> Res<WorkDir> {
        let mut raw = [0u8; 8];
        getrandom::fill(&mut raw).map_err(|_| "bundle")?;
        let dir = std::env::temp_dir().join(format!("orrery-relay-{}", raw.iter().map(|b| format!("{b:02x}")).collect::<String>()));
        std::fs::create_dir_all(&dir).map_err(|_| "bundle")?;
        Ok(WorkDir(dir))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn safe_file(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && !name.starts_with('.') && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

pub fn project(meta: &Value) -> Res<Value> {
    let main = meta["mainModule"].as_str().filter(|m| safe_file(m)).ok_or("bundle")?;
    let date = meta["compatibilityDate"].as_str().filter(|d| !d.is_empty()).ok_or("bundle")?;
    let list = meta["migrations"].as_array().filter(|l| !l.is_empty()).ok_or("bundle")?;
    let migrations: Vec<Value> = list
        .iter()
        .map(|m| match m["tag"].as_str() {
            Some(tag) => Ok(json!({ "tag": tag, "new_sqlite_classes": m["new_sqlite_classes"] })),
            None => Err("bundle"),
        })
        .collect::<Res<_>>()?;
    let mut objects = Vec::new();
    let mut limits = Vec::new();
    let mut vars = Map::new();
    for b in meta["bindings"].as_array().ok_or("bundle")? {
        let name = b["name"].as_str().filter(|n| !n.is_empty()).ok_or("bundle")?;
        match b["type"].as_str() {
            Some("durable_object_namespace") => objects.push(json!({ "name": name, "class_name": b["class_name"].as_str().ok_or("bundle")? })),
            Some("ratelimit") => limits.push(json!({ "name": name, "namespace_id": b["namespace_id"], "simple": b["simple"] })),
            Some("plain_text") => {
                vars.insert(name.to_owned(), Value::from(b["text"].as_str().ok_or("bundle")?));
            }
            _ => return Err("bundle"),
        }
    }
    Ok(json!({
        "name": SCRIPT,
        "main": main,
        "compatibility_date": date,
        "no_bundle": true,
        "workers_dev": true,
        "preview_urls": false,
        "keep_vars": false,
        "observability": { "enabled": false },
        "durable_objects": { "bindings": objects },
        "migrations": migrations,
        "ratelimits": limits,
        "vars": vars,
    }))
}

pub fn write_project(dir: &Path, meta: &Value, script: &str) -> Res<()> {
    let config = project(meta)?;
    let main = config["main"].as_str().ok_or("bundle")?;
    std::fs::write(dir.join(main), script).map_err(|_| "bundle")?;
    std::fs::write(dir.join("wrangler.json"), serde_json::to_vec_pretty(&config).map_err(|_| "bundle")?).map_err(|_| "bundle")?;
    Ok(())
}

pub enum Who {
    In(Vec<Account>),
    Out,
}

fn clean_name(raw: &str, id: &str) -> String {
    let name: String = raw.chars().filter(|c| !c.is_control()).take(NAME_CHARS).collect();
    if name.trim().is_empty() {
        id.to_owned()
    } else {
        name
    }
}

pub fn parse_whoami(out: &str) -> Option<Who> {
    let start = out.find('{')?;
    let end = out.rfind('}')?;
    let v: Value = serde_json::from_str(out.get(start..=end)?).ok()?;
    match v["loggedIn"].as_bool()? {
        false => Some(Who::Out),
        true => {
            let mut seen = Vec::new();
            let mut list = Vec::new();
            for a in v["accounts"].as_array().map(Vec::as_slice).unwrap_or_default() {
                let Some(id) = a["id"].as_str().filter(|i| cf_api::valid_account(i)) else { continue };
                if seen.contains(&id) {
                    continue;
                }
                seen.push(id);
                list.push(Account { id: id.to_owned(), name: clean_name(a["name"].as_str().unwrap_or(id), id) });
            }
            Some(Who::In(list))
        }
    }
}

fn url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https://orrery-relay\.([a-z0-9-]{1,63})\.workers\.dev").expect("url regex"))
}

pub fn deployed_url(text: &str) -> Option<String> {
    url_re().captures_iter(text).find_map(|c| cf_api::worker_url(c.get(1)?.as_str()).ok())
}

fn mask_res() -> &'static [Regex; 3] {
    static RE: OnceLock<[Regex; 3]> = OnceLock::new();
    RE.get_or_init(|| {
        [
            Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").expect("ansi regex"),
            Regex::new(r"[^\s@<>()]+@[^\s@<>()]+\.[A-Za-z]{2,}").expect("mail regex"),
            Regex::new(r"[A-Za-z0-9_\-]{24,}").expect("secret regex"),
        ]
    })
}

pub fn tail(text: &str) -> String {
    let [ansi, mail, secret] = mask_res();
    let plain = ansi.replace_all(text, "");
    let plain = mail.replace_all(&plain, "***");
    let plain = secret.replace_all(&plain, "***");
    let lines: Vec<String> = plain
        .lines()
        .map(|l| l.chars().filter(|c| !c.is_control() && !matches!(*c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')).collect::<String>())
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect();
    let joined = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    let count = joined.chars().count();
    joined.chars().skip(count.saturating_sub(TAIL_CHARS)).collect()
}

fn fail(code: &'static str, run: &Run) -> Failure {
    let detail = tail(if run.err.trim().is_empty() { &run.out } else { &run.err });
    Failure { code, extra: if detail.is_empty() { Value::Null } else { json!({ "detail": detail }) } }
}

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| text.contains(n))
}

pub fn onboarding(account: &str) -> String {
    format!("https://dash.cloudflare.com/{account}/workers/onboarding")
}

impl Remote {
    fn wrangler(&self) -> Arc<dyn Wrangler> {
        lock(&self.setup.env).wrangler.clone().unwrap_or_else(|| Arc::new(Npx))
    }

    fn wrangler_call(&self, w: &dyn Wrangler, args: &[&str], cwd: &Path, account: Option<&str>, timeout: Duration, cancel: &Arc<AtomicBool>) -> Result<Run, Failure> {
        let run = w.run(&Call { args, cwd, account, timeout }, cancel);
        if run.missing {
            return Err("no_node".into());
        }
        if run.cancelled || cancel.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        Ok(run)
    }

    pub fn relay_quick(&self, arg: &Value, progress: &dyn Fn(&str)) -> Value {
        let picked = arg.as_str().filter(|a| cf_api::valid_account(a)).map(str::to_owned);
        self.guarded(|| self.run_quick(picked, progress))
    }

    fn whoami(&self, w: &dyn Wrangler, cwd: &Path) -> Result<Option<Vec<Account>>, Failure> {
        let cancel = self.setup.cancel.clone();
        let run = self.wrangler_call(w, &["whoami", "--json"], cwd, None, WHOAMI_WAIT, &cancel)?;
        if run.timed_out {
            return Err("timeout".into());
        }
        let text = run.text();
        match parse_whoami(&run.out) {
            Some(Who::In(list)) => Ok(Some(list)),
            Some(Who::Out) => Ok(None),
            None if !run.ok() && has_any(&text, &LOGGED_OUT) => Ok(None),
            None if has_any(&text, &OFFLINE) => Err("offline".into()),
            None => Err(fail("wrangler_failed", &run)),
        }
    }

    fn run_quick(&self, picked: Option<String>, progress: &dyn Fn(&str)) -> Result<String, Failure> {
        let w = self.wrangler();
        let w = w.as_ref();
        let work = WorkDir::new()?;
        let cancel = self.setup.cancel.clone();
        progress("login");
        let mut who = self.whoami(w, &work.0)?;
        if who.is_none() {
            let run = self.wrangler_call(w, &["login"], &work.0, None, LOGIN_WAIT, &cancel)?;
            if run.timed_out {
                return Err("login_timeout".into());
            }
            if !run.ok() {
                return Err(if has_any(&run.text(), &OFFLINE) { "offline".into() } else { fail("login_cancelled", &run) });
            }
            self.cancelled()?;
            who = self.whoami(w, &work.0)?;
        }
        let accounts = who.ok_or("login_cancelled")?;
        let saved = self.read_config().relay;
        let wanted = picked.or_else(|| (saved.via == VIA_WRANGLER && !saved.account_id.is_empty()).then_some(saved.account_id));
        let account = match (wanted.and_then(|id| accounts.iter().find(|a| a.id == id)), accounts.as_slice()) {
            (Some(a), _) => a.clone(),
            (None, []) => return Err("no_account".into()),
            (None, [one]) => one.clone(),
            (None, many) => {
                return Err(Failure { code: "pick_account", extra: json!({ "accounts": many.iter().map(|a| json!({ "id": a.id, "name": a.name })).collect::<Vec<_>>() }) });
            }
        };
        self.cancelled()?;
        progress("deploy");
        let meta = bundle_meta();
        if meta.is_null() {
            return Err("bundle".into());
        }
        write_project(&work.0, &meta, WORKER_JS)?;
        let run = self.wrangler_call(w, &["deploy", "--no-bundle"], &work.0, Some(&account.id), DEPLOY_WAIT, &cancel)?;
        if run.timed_out {
            return Err("timeout".into());
        }
        let text = run.text();
        if !run.ok() {
            if text.contains(NO_SUBDOMAIN) {
                return Err(Failure { code: "subdomain_missing", extra: json!({ "link": onboarding(&account.id) }) });
            }
            if has_any(&text, &OFFLINE) {
                return Err("offline".into());
            }
            return Err(fail("deploy_failed", &run));
        }
        let url = deployed_url(&text).ok_or_else(|| fail("deploy_failed", &run))?;
        drop(work);
        self.cancelled()?;
        self.finish_deploy(url, &account, VIA_WRANGLER, progress).map_err(Failure::from)
    }

    pub(super) fn wrangler_delete(&self, account: &str) -> Res<()> {
        let work = WorkDir::new()?;
        let w = self.wrangler();
        let cancel = Arc::new(AtomicBool::new(false));
        let run = self.wrangler_call(w.as_ref(), &["delete", "--name", SCRIPT, "--force"], &work.0, Some(account), DELETE_WAIT, &cancel).map_err(|f| f.code)?;
        if run.ok() || run.text().contains("does not exist") || run.text().contains("10007") {
            Ok(())
        } else if run.timed_out {
            Err("timeout")
        } else if has_any(&run.text(), &OFFLINE) {
            Err("offline")
        } else {
            Err("rejected")
        }
    }
}
