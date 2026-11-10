pub mod manager;
pub mod model;
pub mod payload;

use crate::claude::{self, check_dir, diff, mask::mask_hook_text, proc, DirArg, Sys};
use crate::{arg, js, util, Ctx, Desk, DeskEnv, DeskError};
use manager::{read_json, FileState, Source};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const MODULE: &str = "claudeHooks";
pub const HOOK_TEST_TIMEOUT_MS: u64 = 10_000;
pub const HOOK_TEST_OUTPUT_MAX: usize = 4096;
const CAPTURE_MAX: usize = 64 * 1024;

pub struct HooksService {
    pub sys: Sys,
    pub data_dir: PathBuf,
    pub env: Vec<(String, String)>,
    busy: AtomicBool,
    testing: AtomicBool,
    approved: Mutex<HashSet<String>>,
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

fn is_id(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| s.len() == 24 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

struct Flag<'a>(&'a AtomicBool);

impl Drop for Flag<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn take(flag: &AtomicBool) -> Option<Flag<'_>> {
    flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| Flag(flag))
}

impl HooksService {
    pub fn new(sys: Sys, data_dir: PathBuf, env: Vec<(String, String)>) -> Self {
        HooksService { sys, data_dir, env, busy: AtomicBool::new(false), testing: AtomicBool::new(false), approved: Mutex::new(HashSet::new()) }
    }

    fn cx(&self) -> manager::Ctx<'_> {
        manager::Ctx { sys: &self.sys, data_dir: &self.data_dir }
    }

    pub fn list(&self, dir: Option<&str>) -> Value {
        self.cx().list(dir)
    }

    pub fn toggle(&self, ctx: &Ctx, id: &Value, dir: Option<&str>, on: &Value) -> Result<Value, DeskError> {
        let (Some(id), Value::Bool(on)) = (is_id(id), on) else { return Ok(err("invalid")) };
        let on = *on;
        let Some(_busy) = take(&self.busy) else { return Ok(err("busy")) };
        let cx = self.cx();
        let found = cx.collect(dir).found;
        let Some(hit) = found.into_iter().find(|f| f.id() == id && f.enabled() != on) else { return Ok(err("notFound")) };
        if hit.row["orrery"] == Value::Bool(true) {
            return Ok(err("orrery"));
        }
        if hit.row["source"] == Source::Plugin.name() {
            return Ok(err("readOnly"));
        }
        if !cx.file_allowed(&hit.file, dir) {
            return Ok(err("denied"));
        }
        let file = Path::new(&hit.file);
        let read = read_json(file);
        let Some(data) = read.data.as_ref().filter(|_| read.state != FileState::Unreadable && (on || read.state == FileState::Ok)) else {
            return Ok(err("unreadable"));
        };
        let next = if on { model::restore_hook_entry(data, &hit.flat) } else { model::remove_hook_entry(data, &hit.flat) };
        if !on && next.is_none() {
            return Ok(err("notFound"));
        }
        let stash = cx.read_stash();
        let rest: Vec<Value> = stash.iter().filter(|e| e["key"].as_str() != Some(id)).cloned().collect();
        let Some(next) = next else {
            return Ok(if cx.write_stash(&rest) { json!({"ok": true}) } else { err("failed") });
        };
        let text = diff::format_settings(&Value::Object(next), &read.text);
        let diff = cx.toggle_diff(&read.text, &text);
        let req = json!({"file": hit.file, "action": if on { "enable" } else { "disable" }, "event": hit.flat.event, "diff": diff});
        if ctx.host("claudeHooks.confirmWrite", req)? != Value::Bool(true) {
            return Ok(err("cancelled"));
        }
        let again = read_json(file);
        if again.text != read.text || again.state != read.state || again.mtime != read.mtime {
            return Ok(err("changed"));
        }
        if on {
            if !claude::write_atomic(file, &text, read.state == FileState::Ok) {
                return Ok(err("failed"));
            }
            cx.write_stash(&rest);
            return Ok(json!({"ok": true}));
        }
        let entry = json!({"key": id, "file": hit.file, "event": hit.flat.event, "matcher": hit.flat.matcher, "hook": hit.flat.hook, "at": util::iso_string(util::now_ms())});
        let mut with = rest.clone();
        with.push(entry);
        if !cx.write_stash(&with) {
            return Ok(err("failed"));
        }
        if claude::write_atomic(file, &text, true) {
            return Ok(json!({"ok": true}));
        }
        cx.write_stash(&stash);
        Ok(err("failed"))
    }

    pub fn test(&self, ctx: &Ctx, id: &Value, dir: Option<&str>) -> Result<Value, DeskError> {
        let Some(id) = is_id(id) else { return Ok(err("invalid")) };
        let Some(_testing) = take(&self.testing) else { return Ok(err("busy")) };
        let found = self.cx().collect(dir).found;
        let Some(hit) = found.into_iter().find(|f| f.id() == id) else { return Ok(err("notFound")) };
        let Some(command) = hit.flat.hook.get("command").and_then(Value::as_str).filter(|_| hit.row["testable"] == Value::Bool(true)) else {
            return Ok(err("notTestable"));
        };
        let (home, platform) = (self.sys.home.as_str(), self.sys.platform());
        let cwd = dir.unwrap_or(home).to_string();
        let approved = self.approved.lock().unwrap().contains(id);
        if !approved {
            let req = json!({"event": hit.flat.event, "text": hit.row["text"], "cwd": mask_hook_text(&cwd, home, platform)});
            if ctx.host("claudeHooks.confirmTest", req)? != Value::Bool(true) {
                return Ok(err("cancelled"));
            }
            self.approved.lock().unwrap().insert(id.to_string());
        }
        let root = hit.plugin_root.as_deref();
        let command = payload::expand_plugin_root(command, root);
        let mut hook = hit.flat.hook.clone();
        if let Some(args) = hook.get("args").and_then(Value::as_array).cloned() {
            let mapped: Vec<Value> = args.into_iter().map(|a| a.as_str().map(|s| Value::String(payload::expand_plugin_root(s, root))).unwrap_or(a)).collect();
            hook.insert("args".into(), Value::Array(mapped));
        }
        let env_get = |k: &str| self.env.iter().find(|(n, _)| if self.sys.win { n.eq_ignore_ascii_case(k) } else { n == k }).map(|(_, v)| v.clone());
        let spec = payload::shell_for(&hook, &command, &env_get, self.sys.win, &|p| Path::new(p).exists());
        let session = format!("orrery-test-{}", util::uuid());
        let body = format!("{}\n", js::stringify(&payload::fake_payload(&hit.flat.event, hit.flat.matcher.as_deref(), &cwd, &session)));
        let env = payload::test_env(&self.env, &cwd, root);
        let out = proc::run(proc::RunOpts {
            file: &spec.file,
            args: &spec.args,
            cwd: &cwd,
            env: &env,
            stdin: Some(body.as_bytes()),
            timeout: Duration::from_millis(HOOK_TEST_TIMEOUT_MS),
            tree: true,
            cap: CAPTURE_MAX * 4,
        });
        let Some(out) = out else { return Ok(err("spawn")) };
        let clip = |s: &str| model::clip_output(&mask_hook_text(claude::tail(s, CAPTURE_MAX), home, platform), HOOK_TEST_OUTPUT_MAX);
        let (o, oc) = clip(&out.stdout);
        let (e, ec) = clip(&out.stderr);
        Ok(json!({"ok": true, "code": out.code, "timedOut": out.timed_out, "ms": out.ms as u64, "stdout": o, "stderr": e, "truncated": oc || ec}))
    }
}

fn with_dir(v: &Value, f: impl FnOnce(Option<&str>) -> Result<Value, DeskError>) -> Result<Value, DeskError> {
    match check_dir(v) {
        DirArg::Bad => Ok(err("invalid")),
        DirArg::None => f(None),
        DirArg::Dir(d) => f(Some(&d)),
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let svc = Arc::new(HooksService::new(Sys::from_env(), env.data_dir.clone(), proc::env_vars()));
    let s = svc.clone();
    desk.on(MODULE, "ccm:hooks-list", move |_, args| with_dir(arg(args, 0), |d| Ok(s.list(d))));
    let s = svc.clone();
    desk.on(MODULE, "ccm:hooks-toggle", move |ctx, args| {
        let r = arg(args, 0);
        with_dir(r.get("dir").unwrap_or(&Value::Null), |d| s.toggle(ctx, r.get("id").unwrap_or(&Value::Null), d, r.get("on").unwrap_or(&Value::Null)))
    });
    let s = svc;
    desk.on(MODULE, "ccm:hooks-test", move |ctx, args| {
        let r = arg(args, 0);
        with_dir(r.get("dir").unwrap_or(&Value::Null), |d| s.test(ctx, r.get("id").unwrap_or(&Value::Null), d))
    });
}
