use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use core_desk::git::{git_path, run_git, GitResult};
use core_desk::preview::worktrees::is_branch_name;
use serde_json::{json, Map, Value};

use crate::presets::resolve_cwd;
use crate::wtcopy::copy_setup_files;

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
const MUTATE_TIMEOUT: Duration = Duration::from_secs(120);

static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq)]
pub struct PaneDir {
    pub cwd: String,
    pub live_cwd: Option<String>,
    pub exited: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub path: String,
    pub branch: Option<String>,
    pub head: String,
    pub main: bool,
    pub locked: bool,
    pub prunable: bool,
}

impl Entry {
    fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("path".into(), json!(self.path));
        if let Some(b) = &self.branch {
            o.insert("branch".into(), json!(b));
        }
        o.insert("head".into(), json!(self.head));
        o.insert("main".into(), json!(self.main));
        o.insert("locked".into(), json!(self.locked));
        o.insert("prunable".into(), json!(self.prunable));
        Value::Object(o)
    }
}

fn norm(p: &str) -> String {
    if cfg!(windows) {
        p.replace('/', "\\")
    } else {
        p.to_owned()
    }
}

pub fn parse_worktrees(porcelain: &str) -> Vec<Entry> {
    let text = porcelain.replace("\r\n", "\n");
    let mut out: Vec<Entry> = Vec::new();
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block.split('\n').filter(|l| !l.is_empty()).collect();
        let Some(wt) = lines.iter().find_map(|l| l.strip_prefix("worktree ")) else { continue };
        let flag = |name: &str| lines.iter().any(|l| *l == name || l.starts_with(&format!("{name} ")));
        out.push(Entry {
            path: norm(wt),
            branch: lines.iter().find_map(|l| l.strip_prefix("branch ")).map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_owned()),
            head: lines.iter().find_map(|l| l.strip_prefix("HEAD ")).unwrap_or("").to_owned(),
            main: out.is_empty(),
            locked: flag("locked"),
            prunable: flag("prunable"),
        });
    }
    out
}

pub fn worktree_dest(main: &str, branch: &str) -> PathBuf {
    let repo = Path::new(main);
    let name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    repo.parent().unwrap_or(repo).join(format!("{name}-wt")).join(branch.replace('/', "-"))
}

fn clean(p: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in Path::new(p).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => {
                let s = other.as_os_str().to_string_lossy().into_owned();
                out.push(if cfg!(windows) { s.to_lowercase() } else { s });
            }
        }
    }
    out
}

pub fn is_inside(child: &str, parent: &str) -> bool {
    let (c, p) = (clean(child), clean(parent));
    c.len() >= p.len() && c[..p.len()] == p[..]
}

fn error(code: &str, detail: &str) -> Value {
    let first: String = detail.trim().lines().next().unwrap_or("").chars().take(200).collect();
    if first.is_empty() {
        json!({ "t": "wt.error", "code": code })
    } else {
        json!({ "t": "wt.error", "code": code, "detail": first })
    }
}

fn git(cwd: &Path, args: &[&str], timeout: Duration) -> GitResult {
    run_git(cwd, args, timeout)
}

fn list_at(cwd: &Path) -> Option<Vec<Entry>> {
    let res = git(cwd, &["worktree", "list", "--porcelain"], LIST_TIMEOUT);
    res.ok.then(|| parse_worktrees(&res.stdout))
}

fn add(cwd: &Path, list: &[Entry], msg: &Value) -> Value {
    let branch = msg.get("branch").and_then(Value::as_str).map_or("", str::trim);
    if !is_branch_name(branch) {
        return error("branch", "");
    }
    if list.iter().any(|w| w.branch.as_deref() == Some(branch)) {
        return error("exists", branch);
    }
    let main = &list[0];
    let dest = worktree_dest(&main.path, branch);
    let dest_text = dest.to_string_lossy().into_owned();
    if dest.is_dir() {
        return error("exists", &dest_text);
    }
    let known = git(cwd, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")], LIST_TIMEOUT).ok;
    let args: Vec<&str> = if known { vec!["worktree", "add", "--", &dest_text, branch] } else { vec!["worktree", "add", "-b", branch, "--", &dest_text] };
    let res = git(cwd, &args, MUTATE_TIMEOUT);
    if !res.ok {
        return error("git", &res.stderr);
    }
    let copied = copy_setup_files(Path::new(&main.path), &dest, msg.get("copy").unwrap_or(&Value::Null));
    let mut reply = json!({ "t": "wt.added", "path": dest_text, "branch": branch });
    if !copied.is_empty() {
        reply["copied"] = json!(copied);
    }
    reply
}

fn remove(cwd: &Path, list: &[Entry], msg: &Value, panes: &[PaneDir]) -> Value {
    let asked = msg.get("path").and_then(Value::as_str).map(clean);
    let Some(target) = list.iter().find(|w| asked.as_ref().is_some_and(|a| *a == clean(&w.path))) else { return error("unknown", "") };
    if target.main {
        return error("main", "");
    }
    let held = panes.iter().any(|p| !p.exited && (is_inside(&p.cwd, &target.path) || p.live_cwd.as_deref().is_some_and(|l| is_inside(l, &target.path))));
    if held {
        return error("busy", "");
    }
    if !target.prunable {
        let status = git(Path::new(&target.path), &["status", "--porcelain"], LIST_TIMEOUT);
        if !status.ok {
            return error("git", &status.stderr);
        }
        if !status.stdout.trim().is_empty() {
            return error("dirty", "");
        }
    }
    let args: Vec<&str> = if target.prunable { vec!["worktree", "prune"] } else { vec!["worktree", "remove", "--", &target.path] };
    let res = git(cwd, &args, MUTATE_TIMEOUT);
    if !res.ok {
        let lower = res.stderr.to_lowercase();
        let dirty = ["modified", "untracked", "dirty"].iter().any(|w| lower.contains(w));
        return error(if dirty { "dirty" } else { "git" }, &res.stderr);
    }
    json!({ "t": "wt.removed", "path": target.path })
}

pub fn answer(msg: &Value, panes: &[PaneDir], home: &Path) -> Value {
    if git_path().is_none() {
        return error("noGit", "");
    }
    let asked = msg.get("cwd").and_then(Value::as_str).filter(|c| !c.is_empty());
    let Some(cwd) = asked.and_then(|c| resolve_cwd(Some(c), home)) else { return error("notRepo", "") };
    let kind = msg.get("t").and_then(Value::as_str).unwrap_or("");
    let list = list_at(&cwd).filter(|l| !l.is_empty());
    let Some(list) = list else {
        return if kind == "wt.list" { json!({ "t": "wt.list", "repo": null, "worktrees": [] }) } else { error("notRepo", "") };
    };
    if kind == "wt.list" {
        return json!({ "t": "wt.list", "repo": list[0].path, "worktrees": list.iter().map(Entry::to_json).collect::<Vec<_>>() });
    }
    if kind != "wt.add" && kind != "wt.remove" {
        return error("git", "");
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return error("busy", "");
    }
    let reply = if kind == "wt.add" { add(&cwd, &list, msg) } else { remove(&cwd, &list, msg, panes) };
    BUSY.store(false, Ordering::SeqCst);
    reply
}

#[cfg(test)]
mod tests;
