use super::commit::head_unpushed;
use super::exec::{exec_git, ExecOpts, ExecResult};
use super::log::resolve_commit;
use serde_json::{json, Value};
use std::path::Path;

const MAX_STASHES: usize = 200;
const MAX_NAME: usize = 200;
const STASH_FORMAT: &str = "--format=%H%x00%ct%x00%P%x00%gs";

fn stash_opts(timeout_ms: u64) -> ExecOpts {
    let mut o = ExecOpts::timeout(timeout_ms);
    o.env.push(("GIT_LITERAL_PATHSPECS".to_string(), "0".to_string()));
    o
}

fn err(kind: &str) -> Value {
    json!({"ok": false, "error": kind})
}

fn failed(res: &ExecResult) -> Value {
    if res.timed_out {
        return err("timeout");
    }
    let text = if res.stderr.trim().is_empty() { &res.stdout } else { &res.stderr };
    json!({"ok": false, "error": "failed", "detail": crate::js::slice(text.trim(), 600)})
}

#[derive(Debug, Clone, PartialEq)]
pub struct StashRow {
    pub hash: String,
    pub index: usize,
    pub time: i64,
    pub untracked: bool,
    pub branch: String,
    pub message: String,
}

pub fn split_subject(gs: &str) -> (String, String) {
    let rest = gs.strip_prefix("WIP on ").or_else(|| gs.strip_prefix("On ")).unwrap_or(gs);
    match rest.split_once(": ") {
        Some((branch, msg)) => (branch.to_string(), msg.to_string()),
        None => (String::new(), rest.to_string()),
    }
}

pub fn parse_list(out: &str) -> Vec<StashRow> {
    out.split('\n')
        .filter(|l| !l.is_empty())
        .enumerate()
        .filter_map(|(index, line)| {
            let mut it = line.splitn(4, '\0');
            let hash = it.next()?.to_string();
            let time = it.next()?.parse().unwrap_or(0);
            let parents = it.next()?;
            let (branch, message) = split_subject(it.next().unwrap_or(""));
            Some(StashRow { hash, index, time, untracked: parents.split(' ').filter(|p| !p.is_empty()).count() >= 3, branch, message })
        })
        .collect()
}

fn list_rows(root: &str) -> Result<Vec<StashRow>, ExecResult> {
    let max = format!("--max-count={MAX_STASHES}");
    let res = exec_git(Path::new(root), &["stash", "list", &max, STASH_FORMAT], &ExecOpts::max(2 * 1024 * 1024).with_timeout(15_000));
    if res.ok() {
        Ok(parse_list(&res.stdout))
    } else {
        Err(res)
    }
}

pub fn list(root: &str) -> Value {
    match list_rows(root) {
        Ok(rows) => {
            let items: Vec<Value> = rows
                .iter()
                .map(|r| json!({"hash": r.hash, "index": r.index, "time": r.time, "untracked": r.untracked, "branch": r.branch, "message": r.message}))
                .collect();
            json!({"ok": true, "stashes": items})
        }
        Err(res) => failed(&res),
    }
}

fn has_head(root: &str) -> bool {
    exec_git(Path::new(root), &["rev-parse", "--verify", "-q", "HEAD"], &ExecOpts::max(4096)).ok()
}

fn clean_name(v: Option<&Value>) -> Option<String> {
    let raw = match v {
        None | Some(Value::Null) => return Some(String::new()),
        Some(Value::String(s)) => s,
        Some(_) => return None,
    };
    if raw.contains('\0') {
        return None;
    }
    let flat: String = raw.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    Some(crate::js::slice(flat.split_whitespace().collect::<Vec<_>>().join(" ").as_str(), MAX_NAME))
}

pub fn push(root: &str, name: Option<&Value>, untracked: bool) -> Value {
    let Some(name) = clean_name(name) else { return err("denied") };
    if !has_head(root) {
        return err("no-commit");
    }
    let r = Path::new(root);
    let scope = if untracked { "-uall" } else { "-uno" };
    let status = exec_git(r, &["status", "--porcelain=v1", scope], &ExecOpts::max(1024 * 1024).with_timeout(20_000));
    if !status.ok() {
        return failed(&status);
    }
    if status.stdout.trim().is_empty() {
        return err("nothing");
    }
    let mut args = vec!["stash", "push"];
    if untracked {
        args.push("--include-untracked");
    }
    if !name.is_empty() {
        args.push("-m");
        args.push(&name);
    }
    let res = exec_git(r, &args, &stash_opts(120_000));
    if !res.ok() {
        return failed(&res);
    }
    if res.stdout.contains("No local changes") {
        return err("nothing");
    }
    let top = exec_git(r, &["rev-parse", "--verify", "-q", "refs/stash"], &ExecOpts::max(4096));
    json!({"ok": true, "hash": top.stdout.trim()})
}

fn index_of(root: &str, hash: &Value) -> Result<usize, Value> {
    let want = hash.as_str().unwrap_or("");
    if want.len() < 7 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("denied"));
    }
    let want = want.to_ascii_lowercase();
    match list_rows(root) {
        Ok(rows) => rows.iter().find(|r| r.hash.starts_with(&want)).map(|r| r.index).ok_or_else(|| err("gone")),
        Err(res) => Err(failed(&res)),
    }
}

fn reapply(root: &str, hash: &Value, verb: &str) -> Value {
    let index = match index_of(root, hash) {
        Ok(i) => i,
        Err(e) => return e,
    };
    let spec = format!("stash@{{{index}}}");
    let res = exec_git(Path::new(root), &["stash", verb, "--quiet", &spec], &stash_opts(120_000));
    if res.ok() {
        return json!({"ok": true});
    }
    let all = format!("{}\n{}", res.stdout, res.stderr);
    if all.contains("CONFLICT") || all.contains("Merge conflict") {
        return err("conflict");
    }
    if all.contains("would be overwritten") || all.contains("Please commit your changes or stash them") || all.contains("already exists, no checkout") {
        return err("dirty");
    }
    failed(&res)
}

pub fn apply(root: &str, hash: &Value) -> Value {
    reapply(root, hash, "apply")
}

pub fn pop(root: &str, hash: &Value) -> Value {
    reapply(root, hash, "pop")
}

pub fn drop_entry(root: &str, hash: &Value) -> Value {
    let index = match index_of(root, hash) {
        Ok(i) => i,
        Err(e) => return e,
    };
    let spec = format!("stash@{{{index}}}");
    let res = exec_git(Path::new(root), &["stash", "drop", "--quiet", &spec], &ExecOpts::timeout(30_000));
    if res.ok() {
        json!({"ok": true})
    } else {
        failed(&res)
    }
}

pub fn drop_label(root: &str, hash: &Value) -> Option<String> {
    let index = index_of(root, hash).ok()?;
    list_rows(root).ok()?.into_iter().find(|r| r.index == index).map(|r| format!("stash@{{{}}}: {}", r.index, r.message))
}

pub fn undo_commit(root: &str, hash: &Value) -> Value {
    let full = match resolve_commit(root, hash) {
        Ok(f) => f,
        Err(e) => return err(e),
    };
    let r = Path::new(root);
    let head = exec_git(r, &["rev-parse", "--verify", "-q", "HEAD"], &ExecOpts::max(4096));
    if !head.ok() {
        return err("no-commit");
    }
    if head.stdout.trim() != full {
        return err("stale");
    }
    let parents = exec_git(r, &["rev-list", "--parents", "-n", "1", "HEAD"], &ExecOpts::max(16_384));
    if !parents.ok() {
        return failed(&parents);
    }
    match parents.stdout.split_whitespace().count() {
        0 | 1 => return err("root-commit"),
        2 => {}
        _ => return err("merge"),
    }
    if !head_unpushed(root) {
        return err("pushed");
    }
    let msg = exec_git(r, &["log", "-1", "--format=%B", "HEAD"], &ExecOpts::max(64 * 1024).with_timeout(15_000));
    if !msg.ok() {
        return failed(&msg);
    }
    let subject = exec_git(r, &["log", "-1", "--format=%s", "HEAD"], &ExecOpts::max(16_384));
    let res = exec_git(r, &["reset", "--soft", "HEAD~1"], &ExecOpts::timeout(30_000));
    if !res.ok() {
        return failed(&res);
    }
    json!({"ok": true, "hash": full, "message": msg.stdout.trim_end(), "subject": subject.stdout.trim()})
}

#[cfg(test)]
#[path = "stash_tests.rs"]
mod tests;
