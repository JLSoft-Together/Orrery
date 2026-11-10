use super::rules::{build_dir_kind, needs_reinstall, BUILD_DIR_NAMES, BUILD_SCAN_DEPTH};
use crate::fs_tree::{Cancelled, Signal};
use crate::git::GitResult;
use crate::util;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub rel: String,
    pub name: String,
    pub kind: &'static str,
}

const SKIP: [&str; 7] = [".git", ".hg", ".svn", ".idea", ".vscode", ".cxx", ".externalNativeBuild"];
const MAX_DIRS: usize = 20_000;
const CHUNK: usize = 100;

pub fn home_dir() -> String {
    std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default()
}

pub fn refused_root(root: &str, home: &str) -> bool {
    let r = util::resolve(root);
    let norm = |p: &Path| {
        let s = util::path_str(p);
        if cfg!(windows) {
            s.to_lowercase().trim_end_matches(['\\', '/']).to_string()
        } else {
            s.trim_end_matches('/').to_string()
        }
    };
    r.parent().is_none() || norm(&r) == norm(&util::resolve(home))
}

pub fn find_build_dirs(root: &Path, git: bool, signal: Option<&Signal>) -> Result<Vec<Candidate>, Cancelled> {
    let mut out = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(String::new(), 0)]);
    let mut seen = 0;
    while seen < MAX_DIRS {
        let Some((rel, depth)) = queue.pop_front() else { break };
        if signal.is_some_and(|s| s.load(Ordering::SeqCst)) {
            return Err(Cancelled);
        }
        seen += 1;
        let dir = if rel.is_empty() { root.to_path_buf() } else { root.join(&rel) };
        let names = crate::fs_tree::read_names(&dir);
        let siblings: HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();
        for name in names {
            let Ok(meta) = fs::symlink_metadata(dir.join(&name)) else { continue };
            if meta.file_type().is_symlink() || !meta.is_dir() {
                continue;
            }
            let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let kind = if BUILD_DIR_NAMES.contains(&name.as_str()) { build_dir_kind(&name, &siblings, git) } else { None };
            if let Some(kind) = kind {
                out.push(Candidate { rel: child, name, kind });
                continue;
            }
            if name == "node_modules" || SKIP.contains(&name.as_str()) || depth + 1 >= BUILD_SCAN_DEPTH {
                continue;
            }
            queue.push_back((child, depth + 1));
        }
    }
    Ok(out)
}

pub fn is_git_work_tree(root: &str, git: &dyn Fn(&str, &[String]) -> GitResult) -> bool {
    let r = git(root, &["rev-parse".into(), "--is-inside-work-tree".into()]);
    r.ok && r.stdout.trim() == "true"
}

pub fn git_ignored_only(root: &str, cands: &[Candidate], git: &dyn Fn(&str, &[String]) -> GitResult) -> Vec<Candidate> {
    let mut ignored: HashSet<String> = HashSet::new();
    for part in cands.chunks(CHUNK) {
        let mut args: Vec<String> = vec!["-c".into(), "core.quotePath=false".into(), "check-ignore".into(), "--".into()];
        args.extend(part.iter().map(|c| format!("{}/", c.rel)));
        let r = git(root, &args);
        if !r.ok && !r.stderr.trim().is_empty() {
            return Vec::new();
        }
        for p in r.stdout.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)) {
            if !p.is_empty() {
                ignored.insert(p.trim_end_matches('/').to_string());
            }
        }
    }
    let kept: Vec<Candidate> = cands.iter().filter(|c| ignored.contains(&c.rel)).cloned().collect();
    let mut tracked: HashSet<String> = HashSet::new();
    for part in kept.chunks(CHUNK) {
        let mut args: Vec<String> = vec!["ls-files".into(), "-z".into(), "--".into()];
        args.extend(part.iter().map(|c| c.rel.clone()));
        let r = git(root, &args);
        if !r.ok {
            return Vec::new();
        }
        for f in r.stdout.split('\0') {
            if let Some(owner) = part.iter().find(|c| f == c.rel || f.starts_with(&format!("{}/", c.rel))) {
                tracked.insert(owner.rel.clone());
            }
        }
    }
    kept.into_iter().filter(|c| !tracked.contains(&c.rel)).collect()
}

pub fn reinstall_of(c: &Candidate) -> bool {
    needs_reinstall(&c.name)
}
