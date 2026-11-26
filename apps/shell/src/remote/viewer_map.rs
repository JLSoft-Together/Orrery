use serde_json::{json, Map, Value};

use super::ctx::{Asked, Host};
use super::viewer_events::run_log;

pub const MAX_PATH: usize = 4096;
pub const FAST: u64 = 15;
pub const SLOW: u64 = 60;
pub const CHECK_RUN: &str = "ccm:checklist-run";
pub const RUN_LOG: &str = "ccm:remote-run-log";
pub const RUN_WATCH: &str = "ccm:remote-run-watch";
const MAX_ID: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arg {
    None,
    Obj,
    Str(&'static str),
    StrFlag(&'static str, &'static str),
    Write,
    Check(&'static str),
    RunLog,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pre {
    Discard,
    Push,
    Overwrite,
    CheckRun,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Watch {
    No,
    FsAdd,
    FsDrop,
    GitAdd,
    GitDrop,
    RunAdd,
    RunDrop,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub t: &'static str,
    pub channel: &'static str,
    pub arg: Arg,
    pub secs: u64,
    pub write: bool,
    pub run: bool,
    pub pre: Option<Pre>,
    pub watch: Watch,
}

const fn s(t: &'static str, channel: &'static str, arg: Arg) -> Spec {
    Spec { t, channel, arg, secs: FAST, write: false, run: false, pre: None, watch: Watch::No }
}

impl Spec {
    const fn slow(self) -> Spec {
        Spec { secs: SLOW, ..self }
    }
    const fn write(self) -> Spec {
        Spec { write: true, ..self }
    }
    const fn run(self) -> Spec {
        Spec { run: true, ..self }
    }
    const fn pre(self, p: Pre) -> Spec {
        Spec { pre: Some(p), ..self }
    }
    const fn watch(self, w: Watch) -> Spec {
        Spec { watch: w, ..self }
    }
}

pub const SPECS: [Spec; 37] = [
    s("fs.list", "ccm:list-dir", Arg::Obj),
    s("fs.files", "ccm:list-files", Arg::Str("root")),
    s("fs.read", "ccm:read-file", Arg::Str("path")),
    s("fs.write", "ccm:file-write", Arg::Write).write(),
    s("fs.watch", "ccm:watch", Arg::Str("root")).watch(Watch::FsAdd),
    s("fs.unwatch", "ccm:unwatch", Arg::Str("root")).watch(Watch::FsDrop),
    s("git.status", "ccm:changes-status", Arg::Str("root")),
    s("git.diff", "ccm:changes-diff", Arg::Obj),
    s("git.file", "ccm:changes-file", Arg::Str("path")),
    s("git.watch", "ccm:changes-watch", Arg::Str("root")).watch(Watch::GitAdd),
    s("git.unwatch", "ccm:changes-unwatch", Arg::Str("root")).watch(Watch::GitDrop),
    s("git.stage", "ccm:git-stage", Arg::Obj),
    s("git.unstage", "ccm:git-unstage", Arg::Obj),
    s("git.commit", "ccm:git-commit", Arg::Obj),
    s("git.discard", "ccm:changes-discard", Arg::Obj).pre(Pre::Discard),
    s("log.refs", "ccm:log-refs", Arg::Str("root")),
    s("log.page", "ccm:log-page", Arg::Obj),
    s("log.commit", "ccm:log-commit", Arg::Obj),
    s("log.diff", "ccm:log-diff", Arg::Obj),
    s("branch.create", "ccm:git-branch-create", Arg::Obj),
    s("branch.delete", "ccm:git-branch-delete", Arg::Obj).slow(),
    s("branch.update", "ccm:git-branch-update", Arg::Obj).slow(),
    s("branch.fetch", "ccm:git-fetch", Arg::Str("root")).slow(),
    s("branch.push-preview", "ccm:git-push-preview", Arg::Obj),
    s("branch.push", "ccm:git-push", Arg::Obj).slow().pre(Pre::Push),
    s("run.configs", "ccm:run-configs", Arg::Str("root")),
    s("run.start", "ccm:run-start", Arg::Obj).run().slow(),
    s("run.stop", "ccm:run-stop", Arg::Str("sessionId")),
    s("run.reload", "ccm:run-reload", Arg::StrFlag("sessionId", "hot")),
    s("run.sessions", "ccm:run-sessions", Arg::None).run(),
    s("run.devices", "ccm:run-devices", Arg::None),
    s("run.log", RUN_LOG, Arg::RunLog),
    s("run.watch", RUN_WATCH, Arg::None).run().watch(Watch::RunAdd),
    s("run.unwatch", RUN_WATCH, Arg::None).watch(Watch::RunDrop),
    s("checklist.get", "ccm:checklist", Arg::Check("list")),
    s("checklist.toggle", "ccm:checklist", Arg::Check("toggle")).write(),
    s("checklist.run", CHECK_RUN, Arg::Check("list")).pre(Pre::CheckRun),
];

pub fn lookup(t: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|x| x.t == t)
}

pub fn local_path_ok(s: &str) -> bool {
    !s.starts_with("\\\\") && !s.starts_with("//") && !s.contains('\0')
}

const PATH_FIELDS: [&str; 5] = ["root", "dir", "path", "file", "origPath"];

fn text<'a>(cmd: &'a Value, key: &str) -> Option<&'a str> {
    cmd.get(key).and_then(Value::as_str).filter(|v| !v.is_empty() && v.len() <= MAX_PATH && (!PATH_FIELDS.contains(&key) || local_path_ok(v)))
}

pub fn paths_ok(cmd: &Value) -> bool {
    PATH_FIELDS.iter().all(|k| match cmd.get(*k) {
        Some(Value::String(s)) => local_path_ok(s),
        Some(Value::Null) | None => true,
        Some(_) => false,
    })
}

pub fn pre_of(spec: &Spec, cmd: &Value) -> Option<Pre> {
    if spec.pre.is_some() {
        return spec.pre;
    }
    let based = cmd.get("expectedMtime").or_else(|| cmd.get("base")).is_some_and(Value::is_number);
    (spec.t == "fs.write" && !based).then_some(Pre::Overwrite)
}

pub fn clean(cmd: &Value) -> Map<String, Value> {
    let mut obj = cmd.as_object().cloned().unwrap_or_default();
    for k in ["id", "t", "confirmed", "_accept"] {
        obj.remove(k);
    }
    obj
}

fn check_args(op: &str, cmd: &Value) -> Option<Value> {
    let mut req = json!({ "op": op, "dir": text(cmd, "dir")? });
    match cmd.get("wt") {
        None | Some(Value::Null) => {}
        Some(Value::String(w)) if !w.is_empty() && w.len() <= MAX_PATH && local_path_ok(w) => req["wt"] = json!(w),
        Some(_) => return None,
    }
    if op == "toggle" {
        let id = cmd.get("id").and_then(Value::as_str).filter(|i| !i.is_empty() && i.len() <= MAX_ID)?;
        req["id"] = json!(id);
        req["done"] = json!(cmd.get("done").and_then(Value::as_bool)?);
    }
    Some(json!([req]))
}

fn run_log_args(cmd: &Value) -> Option<Value> {
    let session = cmd.get("session").and_then(Value::as_str).filter(|s| run_log::valid_id(s))?;
    let mut req = json!({ "session": session });
    match cmd.get("since") {
        None | Some(Value::Null) => {}
        Some(v) => req["since"] = json!(v.as_u64()?),
    }
    Some(json!([req]))
}

pub fn check_run(host: &dyn Host, owner: &str, accept: &[String], args: Value) -> Value {
    let probe = host.viewer(owner, "ccm:checklist", args.clone(), accept);
    let allowed = probe.get("raw").is_some_and(|r| r.get("ok") == Some(&Value::Bool(true)));
    if !allowed {
        return probe;
    }
    let req = args.get(0).cloned().unwrap_or(Value::Null);
    let target = json!({ "dir": req["dir"], "wt": req.get("wt").cloned().unwrap_or(Value::Null) });
    match host.ask("checklist.run", target) {
        Asked::Reply(v) => json!({ "raw": v }),
        Asked::Offline => json!({ "error": "offline" }),
    }
}

pub fn build_args(spec: &Spec, cmd: &Value) -> Option<Value> {
    Some(match spec.arg {
        Arg::None => json!([]),
        Arg::Obj => paths_ok(cmd).then(|| json!([Value::Object(clean(cmd))]))?,
        Arg::Str(f) => json!([text(cmd, f)?]),
        Arg::StrFlag(f, g) => json!([text(cmd, f)?, cmd.get(g) == Some(&Value::Bool(true))]),
        Arg::Check(op) => check_args(op, cmd)?,
        Arg::RunLog => run_log_args(cmd)?,
        Arg::Write => {
            let path = text(cmd, "path")?;
            let body = cmd.get("text").and_then(Value::as_str)?;
            let mut req = json!({ "path": path, "text": body });
            if let Some(m) = cmd.get("expectedMtime").or_else(|| cmd.get("base")).filter(|m| m.is_number()) {
                req["expectedMtime"] = m.clone();
            }
            json!([req])
        }
    })
}

pub fn watch_key(spec: &Spec, cmd: &Value) -> Option<String> {
    let kind = match spec.watch {
        Watch::No | Watch::RunAdd | Watch::RunDrop => return None,
        Watch::FsAdd | Watch::FsDrop => "fs",
        Watch::GitAdd | Watch::GitDrop => "git",
    };
    let root = text(cmd, "root")?.trim_end_matches(['/', '\\']);
    let root = if cfg!(windows) { root.to_lowercase().replace('/', "\\") } else { root.to_owned() };
    Some(format!("{kind}:{root}"))
}

pub fn outcome(env: &Value) -> Value {
    if let Some(n) = env.get("needsConfirm").filter(|n| n.is_object()) {
        return json!({ "needsConfirm": n });
    }
    if let Some(e) = env.get("error").and_then(Value::as_str) {
        return json!({ "ok": false, "error": e });
    }
    let Some(raw) = env.get("raw") else { return json!({ "ok": false, "error": "internal" }) };
    let denied = json!({ "ok": false, "error": "denied" });
    match raw {
        Value::Null | Value::Bool(false) => denied,
        Value::Object(o) if o.get("kind").and_then(Value::as_str) == Some("denied") => denied,
        Value::Object(o) if o.get("ok") == Some(&Value::Bool(false)) => match o.get("error").and_then(Value::as_str) {
            Some("denied") => denied,
            Some("cancelled") => json!({ "ok": true, "cancelled": true }),
            _ => json!({ "ok": true, "data": raw }),
        },
        _ => json!({ "ok": true, "data": raw }),
    }
}

fn clip(s: &str) -> String {
    s.chars().take(200).collect()
}

pub fn pre_needs(locale: &str, pre: Pre, cmd: &Value) -> Value {
    let vi = locale != "en";
    let cancel = if vi { "Huỷ" } else { "Cancel" };
    let field = |k: &str| clip(cmd.get(k).and_then(Value::as_str).unwrap_or(""));
    match pre {
        Pre::Discard => {
            let what = if cmd.get("hunk").is_some() { field("path") + " (hunk)" } else { field("path") };
            let (title, ok) = if vi { ("Bỏ thay đổi?", "Bỏ thay đổi") } else { ("Discard changes?", "Discard") };
            json!({ "title": title, "detail": what, "ok": ok, "cancel": cancel })
        }
        Pre::Push => {
            let (remote, source, branch) = (field("remote"), field("ref"), field("branch"));
            let dest = if branch.is_empty() { source.rsplit('/').next().unwrap_or("").to_owned() } else { branch };
            let target = if remote.is_empty() { dest.clone() } else { format!("{remote}/{dest}") };
            let mut lines = vec![format!("{source} → {target}")];
            if cmd.get("setUpstream") == Some(&Value::Bool(true)) {
                lines.push(if vi { "Đặt upstream".into() } else { "Set upstream".into() });
            }
            if cmd.get("force") == Some(&Value::Bool(true)) {
                lines.push("Force push".into());
            }
            if matches!(dest.as_str(), "main" | "master") {
                lines.push(if vi { format!("Cảnh báo: đây là nhánh mặc định {dest} của remote") } else { format!("Warning: this is the remote default branch {dest}") });
            }
            let (title, ok) = if vi { ("Push nhánh lên remote?", "Push") } else { ("Push branch to remote?", "Push") };
            json!({ "title": title, "detail": lines.join("\n"), "ok": ok, "cancel": cancel })
        }
        Pre::CheckRun => {
            let (title, ok) = if vi { ("Chạy kiểm trên máy?", "Chạy kiểm") } else { ("Run the check on the computer?", "Run check") };
            let target = if cmd.get("wt").is_some_and(Value::is_string) { field("wt") } else { field("dir") };
            json!({ "title": title, "detail": target, "ok": ok, "cancel": cancel })
        }
        Pre::Overwrite => {
            let (title, ok) = if vi { ("Ghi đè file?", "Ghi đè") } else { ("Overwrite file?", "Overwrite") };
            let note = if vi { "Không kiểm tra thay đổi mới hơn trên máy." } else { "Newer changes on the computer are not checked." };
            json!({ "title": title, "detail": format!("{}\n{note}", field("path")), "ok": ok, "cancel": cancel })
        }
    }
}
