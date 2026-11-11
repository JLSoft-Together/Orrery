use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core_desk::preview::exec::{exec_git, ExecOpts};
use core_desk::preview::paths::{real_or_null, root_key};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::state::lock;

pub type OnChange = Arc<dyn Fn(&str, Vec<String>) + Send + Sync>;

const IGNORED: [&str; 9] = ["node_modules", ".git", "build", ".gradle", ".dart_tool", "dist", ".vitest", "out", ".idea"];
const GIT_DIR_FILES: [&str; 6] = ["index", "HEAD", "MERGE_HEAD", "packed-refs", "FETCH_HEAD", "ORIG_HEAD"];
pub const ROOT_DEBOUNCE: Duration = Duration::from_millis(150);
pub const GIT_DEBOUNCE: Duration = Duration::from_millis(200);

pub fn ignored(rel: &Path) -> bool {
    rel.components().any(|c| IGNORED.contains(&c.as_os_str().to_string_lossy().as_ref()))
}

#[derive(Default)]
struct Pending {
    paths: Vec<String>,
    last: Option<Instant>,
    running: bool,
}

struct Entry {
    id: u64,
    refs: usize,
    _watcher: Option<RecommendedWatcher>,
}

type Entries = Arc<Mutex<HashMap<String, Entry>>>;

fn schedule(pending: &Arc<Mutex<Pending>>, path: Option<String>, debounce: Duration, alive: impl Fn() -> bool + Send + 'static, fire: impl Fn(Vec<String>) + Send + 'static) {
    let mut p = lock(pending);
    if let Some(path) = path {
        if !p.paths.contains(&path) {
            p.paths.push(path);
        }
    }
    p.last = Some(Instant::now());
    if p.running {
        return;
    }
    p.running = true;
    drop(p);
    let pending = pending.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(debounce);
        let mut p = lock(&pending);
        if p.last.is_some_and(|t| t.elapsed() < debounce) {
            continue;
        }
        p.running = false;
        let changed = std::mem::take(&mut p.paths);
        drop(p);
        if alive() {
            fire(changed);
        }
        break;
    });
}

fn alive_check(entries: &Entries, key: &str, id: u64) -> impl Fn() -> bool + Send + 'static {
    let (entries, key) = (Arc::downgrade(entries), key.to_owned());
    move || entries.upgrade().is_some_and(|e| lock(&e).get(&key).is_some_and(|x| x.id == id))
}

fn take_ref(entries: &Entries, key: &str) -> bool {
    match lock(entries).get_mut(key) {
        Some(e) => {
            e.refs += 1;
            true
        }
        None => false,
    }
}

fn drop_ref(entries: &Entries, key: &str) {
    let mut map = lock(entries);
    if let Some(e) = map.get_mut(key) {
        e.refs = e.refs.saturating_sub(1);
        if e.refs == 0 {
            map.remove(key);
        }
    }
}

pub struct RootWatcher {
    entries: Entries,
    listener: OnChange,
    debounce: Duration,
    next: AtomicU64,
}

impl RootWatcher {
    pub fn new(listener: OnChange, debounce: Duration) -> RootWatcher {
        RootWatcher { entries: Arc::default(), listener, debounce, next: AtomicU64::new(1) }
    }

    pub fn has(&self, root: &str) -> bool {
        lock(&self.entries).contains_key(&root_key(root))
    }

    pub fn add(&self, root: &str) -> bool {
        let key = root_key(root);
        if take_ref(&self.entries, &key) {
            return true;
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let pending = Arc::new(Mutex::new(Pending::default()));
        let (base, owned_root, listener, debounce) = (PathBuf::from(root), root.to_owned(), self.listener.clone(), self.debounce);
        let alive = Arc::new(alive_check(&self.entries, &key, id));
        let real_base = real_or_null(root).map(PathBuf::from);
        let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            for p in ev.paths.iter() {
                let rel = p.strip_prefix(&base).ok().or_else(|| real_base.as_ref().and_then(|b| p.strip_prefix(b).ok()));
                if rel.is_some_and(ignored) {
                    continue;
                }
                let (alive, listener, owned) = (alive.clone(), listener.clone(), owned_root.clone());
                schedule(&pending, Some(p.to_string_lossy().into_owned()), debounce, move || alive(), move |changed| listener(&owned, changed));
            }
        });
        let Ok(mut watcher) = made else { return false };
        if watcher.watch(Path::new(root), RecursiveMode::Recursive).is_err() {
            return false;
        }
        lock(&self.entries).insert(key, Entry { id, refs: 1, _watcher: Some(watcher) });
        true
    }

    pub fn remove(&self, root: &str) {
        drop_ref(&self.entries, &root_key(root));
    }
}

pub struct GitDirWatcher {
    entries: Entries,
    listener: OnChange,
    debounce: Duration,
    next: AtomicU64,
}

impl GitDirWatcher {
    pub fn new(listener: OnChange, debounce: Duration) -> GitDirWatcher {
        GitDirWatcher { entries: Arc::default(), listener, debounce, next: AtomicU64::new(1) }
    }

    pub fn add(&self, root: &str) {
        let key = root_key(root);
        if take_ref(&self.entries, &key) {
            return;
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        lock(&self.entries).insert(key.clone(), Entry { id, refs: 1, _watcher: None });
        let res = exec_git(Path::new(root), &["rev-parse", "--absolute-git-dir"], &ExecOpts::max(16_384).with_timeout(10_000));
        if !res.ok() {
            return;
        }
        let git_dir = PathBuf::from(res.stdout.trim());
        let refs_dir = git_dir.join("refs");
        let real_refs = real_or_null(&refs_dir.to_string_lossy()).map(PathBuf::from);
        let pending = Arc::new(Mutex::new(Pending::default()));
        let (owned_root, listener, debounce) = (root.to_owned(), self.listener.clone(), self.debounce);
        let alive = Arc::new(alive_check(&self.entries, &key, id));
        let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            let hit = ev.paths.iter().any(|p| {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let in_refs = p.starts_with(&refs_dir) || real_refs.as_ref().is_some_and(|r| p.starts_with(r));
                if in_refs {
                    !name.ends_with(".lock")
                } else {
                    GIT_DIR_FILES.contains(&name.as_str())
                }
            });
            if hit {
                let (alive, listener, owned) = (alive.clone(), listener.clone(), owned_root.clone());
                schedule(&pending, None, debounce, move || alive(), move |_| listener(&owned, Vec::new()));
            }
        });
        let Ok(mut watcher) = made else { return };
        let _ = watcher.watch(&git_dir, RecursiveMode::NonRecursive);
        let _ = watcher.watch(&git_dir.join("refs"), RecursiveMode::Recursive);
        if let Some(e) = lock(&self.entries).get_mut(&key).filter(|e| e.id == id) {
            e._watcher = Some(watcher);
        }
    }

    pub fn remove(&self, root: &str) {
        drop_ref(&self.entries, &root_key(root));
    }
}

#[derive(Default)]
pub struct Subs {
    map: Mutex<HashMap<String, HashMap<String, (String, usize)>>>,
}

impl Subs {
    pub fn hold(&self, owner: &str, root: &str) {
        let mut map = lock(&self.map);
        let held = map.entry(owner.to_owned()).or_default().entry(root_key(root)).or_insert_with(|| (root.to_owned(), 0));
        held.1 += 1;
    }

    pub fn release_one(&self, owner: &str, root: &str) -> Option<String> {
        let mut map = lock(&self.map);
        let roots = map.get_mut(owner)?;
        let key = root_key(root);
        let held = roots.get_mut(&key)?;
        held.1 -= 1;
        let out = held.0.clone();
        if held.1 == 0 {
            roots.remove(&key);
        }
        Some(out)
    }

    pub fn release_all(&self, owner: &str) -> Vec<String> {
        let Some(roots) = lock(&self.map).remove(owner) else { return Vec::new() };
        roots.into_values().flat_map(|(root, refs)| std::iter::repeat_n(root, refs)).collect()
    }

    pub fn owners_of(&self, root: &str) -> Vec<String> {
        let key = root_key(root);
        lock(&self.map).iter().filter(|(_, roots)| roots.contains_key(&key)).map(|(o, _)| o.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn build_and_vcs_folders_are_ignored() {
        assert!(ignored(Path::new("node_modules/x/y.js")));
        assert!(ignored(Path::new("app/build/out.txt")));
        assert!(!ignored(Path::new("src/main.rs")));
        assert!(!ignored(Path::new("")));
    }

    #[test]
    fn subscriptions_count_refs_per_owner() {
        let subs = Subs::default();
        subs.hold("main", "/a");
        subs.hold("main", "/a");
        subs.hold("compact", "/a");
        let mut owners = subs.owners_of("/a");
        owners.sort();
        assert_eq!(owners, vec!["compact".to_owned(), "main".to_owned()]);
        assert_eq!(subs.release_one("main", "/a"), Some("/a".to_owned()));
        assert_eq!(subs.owners_of("/a").len(), 2);
        assert_eq!(subs.release_all("main"), vec!["/a".to_owned()]);
        assert_eq!(subs.owners_of("/a"), vec!["compact".to_owned()]);
        assert_eq!(subs.release_one("main", "/a"), None);
    }

    #[test]
    fn a_watched_root_reports_changes_until_its_last_ref_goes() {
        let base = std::env::temp_dir().join(format!("orrery-watch-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
        std::fs::create_dir_all(base.join("node_modules")).unwrap();
        let root = real_or_null(&base.to_string_lossy()).unwrap();
        let (tx, rx) = mpsc::channel::<(String, Vec<String>)>();
        let tx = Mutex::new(tx);
        let w = RootWatcher::new(Arc::new(move |r: &str, c: Vec<String>| {
            let _ = lock(&tx).send((r.to_owned(), c));
        }), Duration::from_millis(50));
        assert!(w.add(&root));
        assert!(w.add(&root));
        assert!(!w.add(&format!("{root}/missing")));
        std::thread::sleep(Duration::from_millis(300));
        while rx.try_recv().is_ok() {}
        std::fs::write(Path::new(&root).join("a.txt"), "a").unwrap();
        let (got, changed) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(got, root);
        assert!(changed.iter().any(|c| c.ends_with("a.txt")), "{changed:?}");
        w.remove(&root);
        assert!(w.has(&root));
        w.remove(&root);
        assert!(!w.has(&root));
        let _ = std::fs::remove_dir_all(base);
    }
}
