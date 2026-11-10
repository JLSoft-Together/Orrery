use super::changes::hunk_patch_for;
use super::exec::{exec_git, ExecOpts};
use super::paths::safe_repo_path;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const MAX_PATHS: usize = 500;
const MAX_MESSAGE: usize = 20_000;

fn detail(s: &str) -> String {
    crate::js::slice(s.trim(), 600)
}

fn clean_paths(root: &str, input: &Value) -> Option<Vec<String>> {
    let list: Vec<Value> = match input {
        Value::Array(a) => a.clone(),
        other => vec![other.clone()],
    };
    if list.is_empty() || list.len() > MAX_PATHS {
        return None;
    }
    list.iter().map(|p| safe_repo_path(root, p).map(|s| s.rel)).collect()
}

fn result(res: &super::exec::ExecResult) -> Value {
    if res.ok() {
        json!({"ok": true})
    } else {
        json!({"ok": false, "error": "failed", "detail": detail(&res.stderr)})
    }
}

pub fn stage_paths(root: &str, paths: &Value) -> Value {
    let Some(rels) = clean_paths(root, paths) else { return json!({"ok": false, "error": "denied"}) };
    let mut args = vec!["add", "-A", "--"];
    args.extend(rels.iter().map(String::as_str));
    result(&exec_git(Path::new(root), &args, &ExecOpts::timeout(30_000)))
}

pub fn unstage_paths(root: &str, paths: &Value) -> Value {
    let Some(rels) = clean_paths(root, paths) else { return json!({"ok": false, "error": "denied"}) };
    let r = Path::new(root);
    let head = exec_git(r, &["rev-parse", "--verify", "-q", "HEAD"], &ExecOpts::max(4096));
    let mut args: Vec<&str> = if head.ok() { vec!["restore", "--staged", "--"] } else { vec!["rm", "--cached", "-q", "-r", "--ignore-unmatch", "--"] };
    args.extend(rels.iter().map(String::as_str));
    result(&exec_git(r, &args, &ExecOpts::timeout(30_000)))
}

pub fn stage_hunk(root: &str, rel: &Value, patch: &Value) -> Value {
    let Some(safe) = safe_repo_path(root, rel) else { return json!({"ok": false, "error": "denied"}) };
    let Some(text) = hunk_patch_for(&safe.rel, patch) else { return json!({"ok": false, "error": "denied"}) };
    let r = Path::new(root);
    let check = exec_git(r, &["apply", "--cached", "--whitespace=nowarn", "--check", "-"], &ExecOpts::default().with_input(&text));
    if !check.ok() {
        return json!({"ok": false, "error": "stale", "detail": detail(&check.stderr)});
    }
    result(&exec_git(r, &["apply", "--cached", "--whitespace=nowarn", "-"], &ExecOpts::default().with_input(&text)))
}

pub fn head_unpushed(root: &str) -> bool {
    let r = Path::new(root);
    if !exec_git(r, &["rev-parse", "--verify", "-q", "HEAD"], &ExecOpts::max(4096)).ok() {
        return false;
    }
    let res = exec_git(r, &["branch", "-r", "--contains", "HEAD", "--format=%(refname)"], &ExecOpts::max(64 * 1024).with_timeout(15_000));
    res.ok() && res.stdout.trim().is_empty()
}

pub struct CleanCommit {
    pub message: String,
    pub amend: bool,
}

pub fn clean_commit(input: &Value) -> Option<CleanCommit> {
    let m = input.get("message").and_then(Value::as_str)?;
    if crate::js::len(m) > MAX_MESSAGE || m.contains('\0') {
        return None;
    }
    Some(CleanCommit { message: m.replace("\r\n", "\n"), amend: input.get("amend") == Some(&json!(true)) })
}

pub fn commit(root: &str, c: &CleanCommit, signal: Arc<AtomicBool>) -> Value {
    if c.message.trim().is_empty() {
        return json!({"ok": false, "error": "empty-message"});
    }
    if c.amend && !head_unpushed(root) {
        return json!({"ok": false, "error": "pushed"});
    }
    let r = Path::new(root);
    if !c.amend && exec_git(r, &["diff", "--cached", "--quiet"], &ExecOpts::timeout(15_000)).ok() {
        return json!({"ok": false, "error": "nothing"});
    }
    let mut args = vec!["commit", "-q", "--cleanup=strip", "-F", "-"];
    if c.amend {
        args.push("--amend");
    }
    let mut o = ExecOpts::timeout(180_000).with_input(&c.message);
    o.kill_tree = true;
    o.signal = Some(signal.clone());
    let res = exec_git(r, &args, &o);
    if signal.load(std::sync::atomic::Ordering::SeqCst) {
        return json!({"ok": false, "error": "cancelled"});
    }
    if res.timed_out {
        return json!({"ok": false, "error": "timeout"});
    }
    if !res.ok() {
        let err = detail(if res.stderr.is_empty() { &res.stdout } else { &res.stderr });
        let kind = if err.to_lowercase().contains("hook") { "hook" } else { "failed" };
        return json!({"ok": false, "error": kind, "detail": err});
    }
    let head = exec_git(r, &["log", "-1", "--format=%H%x00%s"], &ExecOpts::max(16_384));
    let line = head.stdout.trim().to_string();
    let mut it = line.split('\0');
    json!({"ok": true, "hash": it.next().unwrap_or(""), "subject": it.next().unwrap_or("")})
}
