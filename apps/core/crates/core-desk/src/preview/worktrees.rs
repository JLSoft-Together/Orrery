use super::changes::{common_dir, MAX_DIFF_BYTES};
use super::exec::{exec_git, ExecOpts};
use super::paths::{real_or_null, resolve_str, safe_repo_path};
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_BETWEEN: usize = 1000;
const MAX_FILES: usize = 2000;

fn detail(s: &str) -> String {
    let lines: Vec<&str> = s.trim().split('\n').map(|l| l.trim_end_matches('\r')).take(4).collect();
    crate::js::slice(&lines.join("\n"), 400)
}

fn git(dir: &str, args: &[&str], o: &ExecOpts) -> super::exec::ExecResult {
    exec_git(Path::new(dir), args, o)
}

pub fn is_branch_name(name: &str) -> bool {
    if name.is_empty() || crate::js::len(name) > 200 || name.starts_with('-') || name.starts_with('/') || name.ends_with('/') || name.ends_with('.') {
        return false;
    }
    if name.contains("..") || name.contains("@{") || name.contains("//") || name == "@" {
        return false;
    }
    if name.chars().any(|c| (c as u32) <= 0x20 || c == '\x7f' || "~^:?*[\\".contains(c)) {
        return false;
    }
    name.split('/').all(|p| !p.is_empty() && !p.starts_with('.') && !p.ends_with(".lock"))
}

pub struct Main {
    pub path: String,
    pub branch: String,
}

pub fn main_checkout(dir: &str) -> Option<Main> {
    let res = git(dir, &["worktree", "list", "--porcelain"], &ExecOpts::timeout(15_000));
    if !res.ok() {
        return None;
    }
    let text = res.stdout.replace("\r\n", "\n");
    let block = Regex::new(r"\n\n+").expect("regex").split(&text).find(|b| b.split('\n').any(|l| l.starts_with("worktree ")))?.to_string();
    let lines: Vec<&str> = block.split('\n').filter(|l| !l.is_empty()).collect();
    let wt = lines.iter().find(|l| l.starts_with("worktree "))?;
    let branch = lines.iter().find(|l| l.starts_with("branch ")).map(|l| l[7..].to_string())?;
    let branch = branch.strip_prefix("refs/heads/").map(String::from).unwrap_or(branch);
    let real = real_or_null(&resolve_str(&wt[9..]))?;
    if branch.is_empty() {
        return None;
    }
    Some(Main { path: real, branch })
}

pub fn known_branch(dir: &str, branch: &Value) -> Option<String> {
    let b = branch.as_str().filter(|b| is_branch_name(b))?;
    let r = format!("refs/heads/{b}");
    if git(dir, &["show-ref", "--verify", "--quiet", &r], &ExecOpts::max(1024)).ok() {
        Some(b.to_string())
    } else {
        None
    }
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

fn err_d(e: &str, d: &str) -> Value {
    json!({"ok": false, "error": e, "detail": detail(d)})
}

fn first_num(re: &str, s: &str) -> f64 {
    Regex::new(re).expect("regex").captures(s).and_then(|c| c[1].parse().ok()).unwrap_or(0.0)
}

pub fn compare_branch(dir: &str, input: &Value) -> Value {
    let Some(main) = main_checkout(dir) else { return err("not-repo") };
    let Some(branch) = known_branch(dir, input).filter(|b| *b != main.branch) else { return err("unknown") };
    let range = format!("refs/heads/{}...refs/heads/{branch}", main.branch);
    let counts = git(dir, &["rev-list", "--left-right", "--count", &range], &ExecOpts::max(4096));
    if !counts.ok() {
        return err_d("failed", &counts.stderr);
    }
    let nums: Vec<f64> = counts.stdout.split_whitespace().map(|s| s.parse().unwrap_or(f64::NAN)).collect();
    let behind = nums.first().copied().unwrap_or(0.0);
    let ahead = nums.get(1).copied().unwrap_or(0.0);
    let stat = git(dir, &["diff", "--shortstat", &range], &ExecOpts::max(4096).with_timeout(20_000));
    let num = |f: f64| if f.is_nan() { Value::Null } else { json!(f as i64) };
    json!({
        "ok": true, "base": main.branch, "branch": branch, "ahead": num(ahead), "behind": num(behind),
        "files": first_num(r"([0-9]+) files? changed", &stat.stdout) as i64,
        "added": first_num(r"([0-9]+) insertions?", &stat.stdout) as i64,
        "removed": first_num(r"([0-9]+) deletions?", &stat.stdout) as i64,
        "merged": ahead == 0.0,
    })
}

pub fn merge_commands(base: &str, branch: &str, mode: &str) -> Vec<String> {
    if mode == "ff" {
        vec![format!("git switch {base}"), format!("git merge --ff-only {branch}")]
    } else {
        vec![format!("git switch {base}"), format!("git merge --squash {branch}"), format!("git commit -m \"Squash {branch}\"")]
    }
}

pub fn merge_branch(dir: &str, input: &Value, mode: &str, confirm: &dyn Fn(&[String], &str, &str) -> bool, keep: bool) -> Value {
    let Some(main) = main_checkout(dir) else { return err("not-repo") };
    let Some(branch) = known_branch(dir, input).filter(|b| *b != main.branch) else { return err("unknown") };
    let dirty = git(&main.path, &["status", "--porcelain", "--untracked-files=no"], &ExecOpts::timeout(20_000));
    if !dirty.ok() {
        return err_d("failed", &dirty.stderr);
    }
    if !dirty.stdout.trim().is_empty() {
        return err("dirty-base");
    }
    if !confirm(&merge_commands(&main.branch, &branch, mode), &main.path, &main.branch) {
        return err("cancelled");
    }
    let target = format!("refs/heads/{branch}");
    if mode == "ff" {
        let res = git(&main.path, &["merge", "--ff-only", "--no-edit", &target], &ExecOpts::timeout(120_000));
        if !res.ok() {
            let low = res.stderr.to_lowercase();
            let kind = if low.contains("not possible to fast-forward") || low.contains("fatal: not possible") { "not-ff" } else { "failed" };
            return err_d(kind, &res.stderr);
        }
    } else {
        let res = git(&main.path, &["merge", "--squash", &target], &ExecOpts::timeout(120_000));
        if !res.ok() {
            let both = format!("{}{}", res.stdout, res.stderr);
            let conflict = both.to_lowercase().contains("conflict");
            if conflict && keep {
                return json!({"ok": false, "error": "conflict", "detail": detail(&both), "kept": main.path});
            }
            git(&main.path, &["reset", "--merge"], &ExecOpts::timeout(30_000));
            return err_d(if conflict { "conflict" } else { "failed" }, &both);
        }
        if git(&main.path, &["diff", "--cached", "--quiet"], &ExecOpts::timeout(15_000)).ok() {
            return err("empty");
        }
        let mut o = ExecOpts::timeout(180_000).with_input(&format!("Squash {branch}\n"));
        o.kill_tree = true;
        let c = git(&main.path, &["commit", "-q", "-F", "-"], &o);
        if !c.ok() {
            return err_d("failed", if c.stderr.is_empty() { &c.stdout } else { &c.stderr });
        }
    }
    let head = git(&main.path, &["rev-parse", "HEAD"], &ExecOpts::max(4096));
    json!({"ok": true, "mode": mode, "hash": head.stdout.trim()})
}

pub fn diff_between(dir: &str, a: &Value, b: &Value) -> Value {
    let (a, b) = std::thread::scope(|s| {
        let x = s.spawn(|| known_branch(dir, a));
        let y = s.spawn(|| known_branch(dir, b));
        (x.join().ok().flatten(), y.join().ok().flatten())
    });
    let (Some(a), Some(b)) = (a, b) else { return err("unknown") };
    if a == b {
        return err("unknown");
    }
    let (ra, rb) = (format!("refs/heads/{a}"), format!("refs/heads/{b}"));
    let (names, nums) = std::thread::scope(|s| {
        let x = s.spawn(|| git(dir, &["diff", "--no-renames", "--name-status", "-z", &ra, &rb], &ExecOpts::timeout(30_000)));
        let y = s.spawn(|| git(dir, &["diff", "--no-renames", "--numstat", "-z", &ra, &rb], &ExecOpts::timeout(30_000)));
        (x.join().unwrap_or_default(), y.join().unwrap_or_default())
    });
    if !names.ok() {
        return err_d("failed", &names.stderr);
    }
    let re = Regex::new(r"(?s)^(-|[0-9]+)\t(-|[0-9]+)\t(.+)$").expect("regex");
    let mut counts: std::collections::HashMap<String, (f64, f64, bool)> = std::collections::HashMap::new();
    for rec in nums.stdout.split('\0') {
        if let Some(m) = re.captures(rec).filter(|m| !m[3].contains('\n')) {
            let bin = &m[1] == "-";
            counts.insert(m[3].to_string(), (if bin { 0.0 } else { m[1].parse().unwrap_or(0.0) }, if &m[2] == "-" { 0.0 } else { m[2].parse().unwrap_or(0.0) }, bin));
        }
    }
    let parts: Vec<&str> = names.stdout.split('\0').collect();
    let mut files = Vec::new();
    let mut i = 0;
    while i + 1 < parts.len() {
        let status: String = parts[i].chars().take(1).collect();
        let p = parts[i + 1];
        i += 2;
        if p.is_empty() || !"AMDT".contains(status.as_str()) {
            continue;
        }
        let (added, removed, binary) = counts.get(p).copied().unwrap_or((0.0, 0.0, false));
        files.push(json!({"path": p, "status": status, "added": added as i64, "removed": removed as i64, "binary": binary}));
    }
    let truncated = files.len() > MAX_BETWEEN;
    files.truncate(MAX_BETWEEN);
    json!({"ok": true, "a": a, "b": b, "files": files, "truncated": truncated})
}

pub fn file_between(dir: &str, root: &str, a: &Value, b: &Value, rel: &Value) -> Value {
    let a = known_branch(dir, a);
    let b = known_branch(dir, b);
    let safe = safe_repo_path(root, rel);
    let (Some(a), Some(b)) = (a, b) else { return err("unknown") };
    let Some(safe) = safe else { return err("denied") };
    let (ra, rb) = (format!("refs/heads/{a}"), format!("refs/heads/{b}"));
    let res = git(dir, &["diff", "--no-ext-diff", "--no-textconv", "--no-renames", &ra, &rb, "--", &safe.rel], &ExecOpts::timeout(30_000).with_max(MAX_DIFF_BYTES));
    if res.truncated {
        return json!({"ok": true, "text": "", "tooLarge": true});
    }
    if !res.ok() {
        return err_d("failed", &res.stderr);
    }
    json!({"ok": true, "text": res.stdout, "tooLarge": false})
}

pub fn take_file(root: &str, from: &Value, rel: &Value, confirm: &dyn Fn(&[String], &str, &str) -> bool) -> Value {
    let from = known_branch(root, from);
    let safe = safe_repo_path(root, rel);
    let (Some(from), Some(safe)) = (from, safe) else { return err("unknown") };
    let spec = format!("refs/heads/{from}:{}", safe.rel);
    if !git(root, &["cat-file", "-e", &spec], &ExecOpts::max(1024)).ok() {
        return err("missing");
    }
    let dirty = git(root, &["status", "--porcelain", "--", &safe.rel], &ExecOpts::timeout(20_000));
    if !dirty.ok() {
        return err_d("failed", &dirty.stderr);
    }
    if !dirty.stdout.trim().is_empty() {
        return err("dirty");
    }
    let cmds = vec![format!("cd {root}"), format!("git checkout {from} -- {}", safe.rel)];
    if !confirm(&cmds, &from, &safe.rel) {
        return err("cancelled");
    }
    let r = format!("refs/heads/{from}");
    let res = git(root, &["checkout", &r, "--", &safe.rel], &ExecOpts::timeout(30_000));
    if res.ok() {
        json!({"ok": true})
    } else {
        err_d("failed", &res.stderr)
    }
}

fn zlines(s: &str) -> Vec<String> {
    s.split('\0').filter(|l| !l.is_empty()).map(String::from).collect()
}

pub fn changed_vs_base(root: &str) -> Value {
    let (main, head, common) = std::thread::scope(|s| {
        let a = s.spawn(|| main_checkout(root));
        let b = s.spawn(|| git(root, &["symbolic-ref", "--short", "-q", "HEAD"], &ExecOpts::max(4096)));
        let c = s.spawn(|| common_dir(root));
        (a.join().ok().flatten(), b.join().unwrap_or_default(), c.join().ok().flatten())
    });
    let branch = if head.ok() { Some(head.stdout.trim().to_string()).filter(|s| !s.is_empty()) } else { None };
    if main.is_none() && head.code.is_none() {
        return err("not-repo");
    }
    let mut from = "HEAD".to_string();
    if let Some(m) = main.as_ref().filter(|m| branch.as_deref() != Some(m.branch.as_str())) {
        let mb = git(root, &["merge-base", &format!("refs/heads/{}", m.branch), "HEAD"], &ExecOpts::max(4096).with_timeout(15_000));
        if mb.ok() && !mb.stdout.trim().is_empty() {
            from = mb.stdout.trim().to_string();
        }
    }
    let res = git(root, &["diff", "--no-renames", "--name-only", "-z", &from], &ExecOpts::timeout(30_000).with_max(512 * 1024));
    if !res.ok() && !res.truncated {
        return err(if res.code.is_none() { "failed" } else { "not-repo" });
    }
    let files = zlines(&res.stdout);
    let truncated = res.truncated || files.len() > MAX_FILES;
    json!({
        "ok": true, "root": resolve_str(root), "common": common, "branch": branch, "base": main.map(|m| m.branch),
        "files": files.into_iter().take(MAX_FILES).collect::<Vec<_>>(), "truncated": truncated,
    })
}

pub fn dry_merge(dir: &str, a: &Value, b: &Value, signal: Arc<AtomicBool>) -> Value {
    let (Some(a), Some(b)) = (known_branch(dir, a), known_branch(dir, b)) else { return err("unknown") };
    if a == b {
        return err("unknown");
    }
    let mut o = ExecOpts::timeout(60_000).with_max(512 * 1024);
    o.signal = Some(signal.clone());
    let (ra, rb) = (format!("refs/heads/{a}"), format!("refs/heads/{b}"));
    let res = git(dir, &["merge-tree", "--write-tree", "--name-only", "--no-messages", "-z", &ra, &rb], &o);
    if signal.load(Ordering::SeqCst) {
        return err("cancelled");
    }
    if res.code == Some(0) {
        return json!({"ok": true, "clean": true, "conflicts": []});
    }
    if res.code == Some(1) {
        let mut seen: Vec<String> = Vec::new();
        for l in zlines(&res.stdout).into_iter().skip(1) {
            if !seen.contains(&l) {
                seen.push(l);
            }
        }
        return json!({"ok": true, "clean": false, "conflicts": seen});
    }
    let low = res.stderr.to_lowercase();
    if low.contains("usage: git merge-tree") || low.contains("unknown option") || low.contains("--write-tree") {
        return err("unsupported");
    }
    let lines: Vec<&str> = res.stderr.trim().split('\n').map(|l| l.trim_end_matches('\r')).take(3).collect();
    json!({"ok": false, "error": "failed", "detail": crate::js::slice(&lines.join("\n"), 300)})
}
