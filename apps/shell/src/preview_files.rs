use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use core_desk::preview::exec::{exec_git, ExecOpts};
use core_desk::preview::paths::{is_within, root_key};
use serde_json::{json, Value};

use crate::state::lock;

pub const MAX_LIST: usize = 50_000;
const TTL: Duration = Duration::from_secs(30);
const WALK_IGNORE: [&str; 13] = ["node_modules", ".git", "dist", ".next", ".nuxt", ".svelte-kit", ".turbo", ".cache", ".venv", "__pycache__", ".gradle", ".idea", "coverage"];

pub fn parse_ls_files(out: &str, max: usize) -> (Vec<String>, bool) {
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for p in out.split('\0').filter(|p| !p.is_empty()) {
        if seen.contains(p) {
            continue;
        }
        if files.len() >= max {
            return (files, true);
        }
        seen.insert(p);
        files.push(p.to_owned());
    }
    (files, false)
}

pub fn walk_files(root: &Path, max: usize) -> (Vec<String>, bool) {
    let mut files = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::from([String::new()]);
    while let Some(rel) = queue.pop_front() {
        let Ok(read) = std::fs::read_dir(root.join(&rel)) else { continue };
        let mut entries: Vec<(String, std::fs::FileType)> = read.flatten().filter_map(|e| Some((e.file_name().to_string_lossy().into_owned(), e.file_type().ok()?))).collect();
        entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then_with(|| a.0.cmp(&b.0)));
        for (name, kind) in entries {
            let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if kind.is_dir() {
                if !WALK_IGNORE.contains(&name.as_str()) {
                    queue.push_back(child);
                }
            } else if kind.is_file() {
                if files.len() >= max {
                    return (files, true);
                }
                files.push(child);
            }
        }
    }
    (files, false)
}

pub fn list_files(root: &str, max: usize) -> Value {
    let res = exec_git(Path::new(root), &["ls-files", "-co", "--exclude-standard", "-z"], &ExecOpts::max(max * 260).with_timeout(20_000));
    let (files, truncated, source) = if res.ok() {
        let (f, t) = parse_ls_files(&res.stdout, max);
        (f, t, "git")
    } else {
        let (f, t) = walk_files(Path::new(root), max);
        (f, t, "walk")
    };
    json!({"root": root, "files": files, "truncated": truncated, "source": source})
}

#[derive(Default)]
pub struct FileListCache {
    entries: Mutex<HashMap<String, (Instant, Value)>>,
}

impl FileListCache {
    pub fn get_with(&self, root: &str, list: impl FnOnce(&str) -> Value) -> Value {
        let key = root_key(root);
        if let Some((_, v)) = lock(&self.entries).get(&key).filter(|(at, _)| at.elapsed() < TTL) {
            return v.clone();
        }
        let v = list(root);
        lock(&self.entries).insert(key, (Instant::now(), v.clone()));
        v
    }

    pub fn get(&self, root: &str) -> Value {
        self.get_with(root, |r| list_files(r, MAX_LIST))
    }

    pub fn invalidate(&self, changed: &str) {
        if changed.is_empty() {
            return;
        }
        lock(&self.entries).retain(|key, _| !(is_within(key, changed) || is_within(changed, key)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ls_files_output_is_deduped_and_capped() {
        assert_eq!(parse_ls_files("a\0b\0a\0\0", 10), (vec!["a".to_owned(), "b".to_owned()], false));
        assert_eq!(parse_ls_files("a\0b\0c\0", 2), (vec!["a".to_owned(), "b".to_owned()], true));
    }

    #[test]
    fn the_walk_skips_build_folders_and_the_cache_drops_overlapping_roots() {
        let base = std::env::temp_dir().join(format!("orrery-files-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::create_dir_all(base.join("node_modules/x")).unwrap();
        std::fs::write(base.join("src/b.rs"), "").unwrap();
        std::fs::write(base.join("A.md"), "").unwrap();
        std::fs::write(base.join("node_modules/x/i.js"), "").unwrap();
        assert_eq!(walk_files(&base, 10), (vec!["A.md".to_owned(), "src/b.rs".to_owned()], false));
        let v = list_files(&base.to_string_lossy(), 10);
        assert_eq!(v["files"], json!(["A.md", "src/b.rs"]));
        let cache = FileListCache::default();
        let root = base.to_string_lossy().into_owned();
        let calls = std::cell::Cell::new(0);
        let get = || cache.get_with(&root, |_| {
            calls.set(calls.get() + 1);
            json!(calls.get())
        });
        assert_eq!(get(), json!(1));
        assert_eq!(get(), json!(1));
        cache.invalidate(&base.join("src").to_string_lossy());
        assert_eq!(get(), json!(2));
        cache.invalidate("/somewhere/else");
        assert_eq!(get(), json!(2));
        let _ = std::fs::remove_dir_all(base);
    }
}
