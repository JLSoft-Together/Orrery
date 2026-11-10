use super::exec::{exec_git, ExecOpts, ExecResult};
use super::log::{parse_log, ref_ok, LOG_FORMAT};
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

pub type Auth<'a> = Option<&'a dyn Fn(&str) -> Option<Vec<(String, String)>>>;

pub const PUSH_PREVIEW_MAX: usize = 100;

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex"))
}

fn local() -> ExecOpts {
    ExecOpts::timeout(20_000)
}

fn net(auth: Auth, remote: Option<&str>) -> ExecOpts {
    let mut o = ExecOpts::timeout(120_000);
    o.kill_tree = true;
    if let (Some(a), Some(r)) = (auth, remote) {
        if let Some(env) = a(r) {
            o.env = env;
        }
    }
    o
}

fn git(root: &str, args: &[&str], o: &ExecOpts) -> ExecResult {
    exec_git(Path::new(root), args, o)
}

pub fn check_branch_name(name: &Value) -> bool {
    static CHAR: OnceLock<Regex> = OnceLock::new();
    static DOT: OnceLock<Regex> = OnceLock::new();
    static LOCK: OnceLock<Regex> = OnceLock::new();
    let Some(n) = name.as_str() else { return false };
    if n.is_empty() || crate::js::len(n) > 200 || n.chars().any(js_space) || n.starts_with('-') {
        return false;
    }
    if n.contains("..") || n.contains("@{") || n == "@" {
        return false;
    }
    if re(&CHAR, r"[\x00-\x1f\x7f~^:?*\[\\]").is_match(n) {
        return false;
    }
    if n.starts_with('/') || n.ends_with('/') || n.contains("//") || n.ends_with('.') || re(&DOT, r"(^|/)\.").is_match(n) {
        return false;
    }
    !re(&LOCK, r"\.lock(/|$)").is_match(n)
}

pub fn js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

pub fn sanitize_git_error(stderr: &str) -> String {
    static URL: OnceLock<Regex> = OnceLock::new();
    let line = stderr.split('\n').map(|l| l.trim_end_matches('\r')).find(|l| !l.trim().is_empty()).unwrap_or("");
    let clean = re(&URL, r"(?i)([a-z][a-z0-9+.-]*://)[^/@\s]*@").replace_all(line, "$1");
    crate::js::slice(&clean, 300)
}

pub fn failed(res: &ExecResult, fallback: &str) -> Value {
    static AUTH: OnceLock<Regex> = OnceLock::new();
    static NONFF: OnceLock<Regex> = OnceLock::new();
    static CONFLICT: OnceLock<Regex> = OnceLock::new();
    if res.timed_out {
        return json!({"ok": false, "error": "timeout"});
    }
    let s = &res.stderr;
    if res.code.is_none() {
        return json!({"ok": false, "error": "failed", "detail": sanitize_git_error(s)});
    }
    let low = s.to_lowercase();
    let error = if low.contains("not fully merged") {
        "not-merged"
    } else if low.contains("already exists") {
        "exists"
    } else if re(&AUTH, r"(?i)Authentication failed|could not read Username|Permission denied|Repository not found|\b403\b.*(?:Forbidden|error)|returned error: 403").is_match(s) {
        "auth"
    } else if re(&NONFF, r"(?i)non-fast-forward|Not possible to fast-forward|rejected").is_match(s) {
        "non-ff"
    } else if re(&CONFLICT, r"(?i)would be overwritten|conflict").is_match(s) {
        "conflict"
    } else {
        fallback
    };
    json!({"ok": false, "error": error, "detail": sanitize_git_error(s)})
}

fn done(res: &ExecResult) -> Value {
    if res.ok() {
        json!({"ok": true})
    } else {
        failed(res, "failed")
    }
}

fn current_branch(root: &str) -> Option<String> {
    let res = git(root, &["symbolic-ref", "-q", "--short", "HEAD"], &local());
    if res.ok() {
        Some(res.stdout.trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn invalid() -> Value {
    json!({"ok": false, "error": "invalid"})
}

fn denied() -> Value {
    json!({"ok": false, "error": "denied"})
}

pub fn create_branch(root: &str, name: &Value, base: &Value, checkout: &Value) -> Value {
    static HEX: OnceLock<Regex> = OnceLock::new();
    if !check_branch_name(name) {
        return invalid();
    }
    let name = name.as_str().unwrap_or("");
    if !git(root, &["check-ref-format", "--branch", name], &local()).ok() {
        return invalid();
    }
    let base_ok = ref_ok(base) || base.as_str().map(|b| re(&HEX, r"^[0-9a-f]{4,64}$").is_match(b)).unwrap_or(false);
    if !base_ok {
        return invalid();
    }
    let spec = format!("{}^{{commit}}", base.as_str().unwrap_or(""));
    let verify = git(root, &["rev-parse", "--verify", "-q", &spec], &local());
    if !verify.ok() {
        return invalid();
    }
    let start = verify.stdout.trim().to_string();
    let res = if checkout == &json!(true) { git(root, &["switch", "-c", name, &start], &local()) } else { git(root, &["branch", "--", name, &start], &local()) };
    done(&res)
}

pub struct DeletePlan {
    pub args: Vec<String>,
    pub remote: bool,
    pub label: String,
}

pub fn delete_plan(root: &str, r: &Value, force: bool) -> Result<DeletePlan, &'static str> {
    let Some(s) = r.as_str().filter(|_| ref_ok(r)) else { return Err("denied") };
    if s == "HEAD" || s.starts_with("refs/tags/") {
        return Err("denied");
    }
    if let Some(name) = s.strip_prefix("refs/heads/") {
        if !check_branch_name(&json!(name)) {
            return Err("denied");
        }
        if current_branch(root).as_deref() == Some(name) {
            return Err("current");
        }
        let args = vec!["branch".into(), if force { "-D" } else { "-d" }.into(), "--".into(), name.to_string()];
        return Ok(DeletePlan { args, remote: false, label: name.to_string() });
    }
    let short = s.strip_prefix("refs/remotes/").unwrap_or("");
    let out = git(root, &["remote"], &local()).stdout;
    let mut remotes: Vec<&str> = out.split('\n').map(|l| l.trim_end_matches('\r')).filter(|r| !r.is_empty() && short.starts_with(&format!("{r}/"))).collect();
    remotes.sort_by_key(|r| std::cmp::Reverse(crate::js::len(r)));
    let Some(remote) = remotes.first() else { return Err("denied") };
    let branch = &short[remote.len() + 1..];
    if branch.is_empty() || branch == "HEAD" || !check_branch_name(&json!(branch)) {
        return Err("denied");
    }
    Ok(DeletePlan { args: vec!["push".into(), remote.to_string(), "--delete".into(), format!("refs/heads/{branch}")], remote: true, label: short.to_string() })
}

pub fn run_delete(root: &str, args: &[String], auth: Auth) -> Value {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let o = if refs.first() == Some(&"push") { net(auth, refs.get(1).copied()) } else { local() };
    done(&git(root, &refs, &o))
}

fn list_remotes(root: &str) -> Vec<String> {
    let res = git(root, &["remote"], &local());
    if !res.ok() {
        return Vec::new();
    }
    res.stdout.split('\n').map(|l| l.trim_end_matches('\r')).filter(|r| !r.is_empty() && !r.starts_with('-')).map(String::from).collect()
}

pub fn fetch_all(root: &str, auth: Auth) -> Value {
    static OPT: OnceLock<Regex> = OnceLock::new();
    let remotes = if auth.is_some() { list_remotes(root) } else { Vec::new() };
    let pick = if remotes.iter().any(|r| r == "origin") { Some("origin") } else { remotes.first().map(String::as_str) };
    let o = net(auth, pick);
    let mut res = git(root, &["fetch", "--all", "--no-write-fetch-head"], &o);
    if !res.ok() && !res.timed_out && re(&OPT, r"(?i)unknown option.*no-write-fetch-head").is_match(&res.stderr) {
        res = git(root, &["fetch", "--all"], &o);
    }
    done(&res)
}

pub fn update_branch(root: &str, r: &Value, auth: Auth) -> Value {
    let Some(s) = r.as_str().filter(|s| ref_ok(r) && s.starts_with("refs/heads/")) else { return denied() };
    let local_name = &s["refs/heads/".len()..];
    let up = git(root, &["for-each-ref", "--format=%(upstream:remotename)%1f%(upstream:remoteref)", s], &local());
    let line = up.stdout.trim().to_string();
    let mut it = line.split('\x1f');
    let remote = it.next().unwrap_or("");
    let remote_ref = it.next().unwrap_or("");
    if remote.is_empty() || !remote_ref.starts_with("refs/heads/") {
        return json!({"ok": false, "error": "no-upstream"});
    }
    let o = net(auth, Some(remote));
    let res = if current_branch(root).as_deref() == Some(local_name) {
        git(root, &["pull", "--ff-only", "--no-rebase"], &o)
    } else {
        git(root, &["fetch", remote, &format!("{remote_ref}:{s}")], &o)
    };
    done(&res)
}

pub fn remote_url(root: &str, remote: &Value) -> Option<String> {
    let r = remote.as_str().filter(|r| !r.is_empty() && !r.starts_with('-'))?;
    let res = git(root, &["remote", "get-url", r], &local());
    if res.ok() {
        Some(res.stdout.trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn local_branch_of(root: &str, r: &Value) -> Option<String> {
    let s = r.as_str().filter(|s| ref_ok(r) && s.starts_with("refs/heads/"))?;
    let name = &s["refs/heads/".len()..];
    if !check_branch_name(&json!(name)) {
        return None;
    }
    let spec = format!("{s}^{{commit}}");
    if git(root, &["rev-parse", "--verify", "-q", &spec], &local()).ok() {
        Some(name.to_string())
    } else {
        None
    }
}

fn valid_branch_name(root: &str, name: &Value) -> bool {
    check_branch_name(name) && git(root, &["check-ref-format", "--branch", name.as_str().unwrap_or("")], &local()).ok()
}

pub fn push_preview(root: &str, r: &Value, remote: &Value, branch: &Value) -> Value {
    let Some(local_name) = local_branch_of(root, r) else { return denied() };
    let s = r.as_str().unwrap_or("");
    let remotes = list_remotes(root);
    if remotes.is_empty() {
        return json!({"ok": false, "error": "no-remote"});
    }
    let up = git(root, &["for-each-ref", "--format=%(upstream:remotename)%1f%(upstream:remoteref)%1f%(push:remotename)", s], &local());
    let line = up.stdout.trim().to_string();
    let mut it = line.split('\x1f');
    let (up_remote, up_ref, push_remote) = (it.next().unwrap_or(""), it.next().unwrap_or(""), it.next().unwrap_or(""));
    let up_branch = up_ref.strip_prefix("refs/heads/").unwrap_or("");
    let has = |n: &str| remotes.iter().any(|r| r == n);
    let has_upstream = has(up_remote) && !up_branch.is_empty();
    let target: Value = if remote.is_null() {
        json!(if has_upstream {
            up_remote.to_string()
        } else if has(push_remote) {
            push_remote.to_string()
        } else if has("origin") {
            "origin".to_string()
        } else {
            remotes[0].clone()
        })
    } else {
        remote.clone()
    };
    let Some(target) = target.as_str().filter(|t| has(t)).map(String::from) else { return invalid() };
    let to: Value = if branch.is_null() { json!(if has_upstream && target == up_remote { up_branch.to_string() } else { local_name.clone() }) } else { branch.clone() };
    if !valid_branch_name(root, &to) {
        return invalid();
    }
    let to = to.as_str().unwrap_or("").to_string();
    let tracking = format!("refs/remotes/{target}/{to}");
    let exists = git(root, &["rev-parse", "--verify", "-q", &format!("{tracking}^{{commit}}")], &local()).ok();
    let not = if exists {
        tracking.clone()
    } else if target.contains(['?', '*', '[']) {
        "--remotes".to_string()
    } else {
        format!("--remotes={target}")
    };
    let fmt = format!("--format={LOG_FORMAT}");
    let n = (PUSH_PREVIEW_MAX + 1).to_string();
    let log = git(root, &["log", "-z", &fmt, "-n", &n, s, "--not", &not, "--"], &local());
    if !log.ok() {
        return json!({"ok": false, "error": if log.timed_out { "timeout" } else { "failed" }, "detail": sanitize_git_error(&log.stderr)});
    }
    let commits = parse_log(&log.stdout);
    let more = commits.len() > PUSH_PREVIEW_MAX;
    json!({
        "ok": true, "local": local_name, "remotes": remotes, "remote": target, "branch": to,
        "upstream": has_upstream && target == up_remote && to == up_branch, "hasUpstream": has_upstream, "exists": exists,
        "commits": commits.into_iter().take(PUSH_PREVIEW_MAX).collect::<Vec<_>>(), "more": more,
    })
}

pub fn push_rejection(stdout: &str) -> Option<Value> {
    static PAREN: OnceLock<Regex> = OnceLock::new();
    let line = stdout.split('\n').map(|l| l.trim_end_matches('\r')).find(|l| l.starts_with("!\t"))?;
    let summary = line.split('\t').nth(2).unwrap_or("");
    if summary.to_lowercase().contains("remote rejected") {
        let inner = re(&PAREN, r"\((.*)\)").captures(summary).map(|c| c[1].to_string()).unwrap_or_else(|| summary.to_string());
        return Some(json!({"ok": false, "error": "remote-rejected", "detail": sanitize_git_error(&inner)}));
    }
    Some(json!({"ok": false, "error": "non-ff", "detail": sanitize_git_error(summary)}))
}

pub fn push_branch(root: &str, r: &Value, remote: &Value, branch: &Value, set_upstream: &Value, auth: Auth) -> Value {
    let Some(local_name) = local_branch_of(root, r) else { return denied() };
    let remotes = list_remotes(root);
    if remotes.is_empty() {
        return json!({"ok": false, "error": "no-remote"});
    }
    let Some(remote) = remote.as_str().filter(|x| remotes.iter().any(|r| r == x)) else { return invalid() };
    if !valid_branch_name(root, branch) {
        return invalid();
    }
    let b = branch.as_str().unwrap_or("");
    let spec = format!("refs/heads/{local_name}:refs/heads/{b}");
    let mut args = vec!["push", "--porcelain"];
    if set_upstream == &json!(true) {
        args.push("--set-upstream");
    }
    args.push(remote);
    args.push(&spec);
    let res = git(root, &args, &net(auth, Some(remote)));
    if res.ok() {
        return json!({"ok": true});
    }
    if !res.timed_out {
        if let Some(v) = push_rejection(&res.stdout) {
            return v;
        }
    }
    failed(&res, "failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_errors() {
        assert!(check_branch_name(&json!("feat/x-1")));
        for bad in ["", "-x", "a..b", "a b", "a~", "/a", "a/", "a//b", "a.", "a/.b", "x.lock", "@"] {
            assert!(!check_branch_name(&json!(bad)), "{bad}");
        }
        assert_eq!(sanitize_git_error("\nfatal: https://u:tok@github.com/x failed\nmore"), "fatal: https://github.com/x failed");
        let r = push_rejection("To x\n!\trefs/heads/a:refs/heads/a\t[remote rejected] (hook declined)\nDone").unwrap();
        assert_eq!(r["error"], "remote-rejected");
        assert_eq!(r["detail"], "hook declined");
    }
}
