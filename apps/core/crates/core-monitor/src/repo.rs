use std::collections::{HashMap, HashSet};

use core_store::persist::paths::{cwd_string, is_absolute, resolve, Style};

pub type RootProbe = Box<dyn Fn(&str) -> bool + Send>;

pub fn style() -> Style {
    if cfg!(windows) {
        Style::Win
    } else {
        Style::Posix
    }
}

fn dirname(p: &str, st: Style) -> String {
    let sep = |c: char| c == '/' || (st == Style::Win && c == '\\');
    let root_len = root_len(p, st);
    let rest = &p[root_len..];
    match rest.trim_end_matches(sep).rfind(sep) {
        Some(i) => {
            let head = rest[..i].trim_end_matches(sep);
            if head.is_empty() {
                p[..root_len].to_owned()
            } else {
                format!("{}{}", &p[..root_len], head)
            }
        }
        None if root_len > 0 => p[..root_len].to_owned(),
        None => ".".to_owned(),
    }
}

fn root_len(p: &str, st: Style) -> usize {
    let b = p.as_bytes();
    if st == Style::Win && b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return if b.len() >= 3 && (b[2] == b'\\' || b[2] == b'/') { 3 } else { 2 };
    }
    usize::from(b.first().is_some_and(|c| *c == b'/' || (st == Style::Win && *c == b'\\')))
}

fn has_git(dir: &str) -> bool {
    std::path::Path::new(dir).join(".git").symlink_metadata().is_ok()
}

pub struct RepoRoots {
    probe: RootProbe,
    stops: HashSet<String>,
    probes: HashMap<String, bool>,
    roots: HashMap<String, String>,
}

impl RepoRoots {
    pub fn new(probe: RootProbe, stop_at: &[String]) -> Self {
        let cwd = cwd_string();
        let stops = stop_at.iter().map(|d| resolve(d, style(), &cwd).to_lowercase()).collect();
        RepoRoots { probe, stops, probes: HashMap::new(), roots: HashMap::new() }
    }

    pub fn system() -> Self {
        let home = std::env::var("USERPROFILE").ok().or_else(|| std::env::var("HOME").ok()).unwrap_or_default();
        Self::new(Box::new(has_git), &[home])
    }

    pub fn known(&self, dir: &str) -> bool {
        self.roots.contains_key(dir)
    }

    pub fn resolve(&mut self, dir: &str) -> String {
        if let Some(r) = self.roots.get(dir).filter(|r| !r.is_empty()) {
            return r.clone();
        }
        let st = style();
        let mut root = dir.to_owned();
        if is_absolute(dir, st) {
            let start = resolve(dir, st, &cwd_string());
            let mut current = start.clone();
            loop {
                let parent = dirname(&current, st);
                if current != start && (self.stops.contains(&current.to_lowercase()) || parent == current) {
                    break;
                }
                if self.probe_once(&current) {
                    root = if current == start { dir.to_owned() } else { current.clone() };
                    break;
                }
                if parent == current {
                    break;
                }
                current = parent;
            }
        }
        self.roots.insert(dir.to_owned(), root.clone());
        root
    }

    fn probe_once(&mut self, dir: &str) -> bool {
        let key = dir.to_lowercase();
        if let Some(v) = self.probes.get(&key) {
            return *v;
        }
        let v = (self.probe)(dir);
        self.probes.insert(key, v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_dir_to_the_nearest_repo_root_below_home() {
        let (home, repo, deep, other) = if cfg!(windows) { ("C:\\Users\\me", "C:\\Users\\me\\work\\app", "C:\\Users\\me\\work\\app\\src\\ui", "D:\\loose\\dir") } else { ("/home/me", "/home/me/work/app", "/home/me/work/app/src/ui", "/loose/dir") };
        let (r, h) = (repo.to_owned(), home.to_owned());
        let mut roots = RepoRoots::new(Box::new(move |d: &str| d == r || d == h), &[home.to_owned()]);
        assert_eq!(roots.resolve(deep), repo);
        assert_eq!(roots.resolve(repo), repo);
        assert_eq!(roots.resolve(other), other);
        assert_eq!(roots.resolve("relative"), "relative");
        assert!(roots.known(deep));
        assert_eq!(dirname(if cfg!(windows) { "C:\\" } else { "/" }, style()), if cfg!(windows) { "C:\\" } else { "/" });
    }
}
