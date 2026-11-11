use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use core_desk::git::run_git;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

const GIT_TIMEOUT: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(120);

pub type Emit = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;

struct Track {
    gen: u64,
    branch: Option<String>,
    git_dir: Option<PathBuf>,
    pending: u64,
    watcher: Option<RecommendedWatcher>,
}

type Map = Arc<Mutex<HashMap<String, Track>>>;

pub struct Branches {
    map: Map,
    next: AtomicU64,
    emit: Emit,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn branch_of_head(content: &str) -> Option<String> {
    let text = content.trim();
    if let Some(rest) = text.strip_prefix("ref:") {
        return rest.trim_start().strip_prefix("refs/heads/").filter(|b| !b.is_empty()).map(str::to_owned);
    }
    ((7..=64).contains(&text.len()) && is_hex(text)).then(|| text[..7].to_owned())
}

fn read_branch(git_dir: &Path) -> Option<String> {
    std::fs::read_to_string(git_dir.join("HEAD")).ok().as_deref().and_then(branch_of_head)
}

fn update(map: &Map, emit: &Emit, id: &str, gen: u64) {
    let git_dir = match lock(map).get(id) {
        Some(t) if t.gen == gen => t.git_dir.clone(),
        _ => return,
    };
    let branch = git_dir.as_deref().and_then(read_branch);
    let mut m = lock(map);
    let Some(t) = m.get_mut(id).filter(|t| t.gen == gen) else { return };
    if t.branch == branch {
        return;
    }
    t.branch = branch.clone();
    drop(m);
    emit(id, branch.as_deref());
}

fn on_head(map: &Map, emit: &Emit, id: &str, gen: u64) {
    let seq = {
        let mut m = lock(map);
        let Some(t) = m.get_mut(id).filter(|t| t.gen == gen) else { return };
        t.pending += 1;
        t.pending
    };
    let (map, emit, id) = (map.clone(), emit.clone(), id.to_owned());
    let _ = std::thread::Builder::new().name("pty-branch-debounce".into()).spawn(move || {
        std::thread::sleep(DEBOUNCE);
        if lock(&map).get(&id).is_some_and(|t| t.gen == gen && t.pending == seq) {
            update(&map, &emit, &id, gen);
        }
    });
}

impl Branches {
    pub fn new(emit: Emit) -> Branches {
        Branches { map: Arc::new(Mutex::new(HashMap::new())), next: AtomicU64::new(1), emit }
    }

    pub fn track(&self, id: &str, dir: &str) {
        let gen = self.next.fetch_add(1, Ordering::SeqCst);
        {
            let mut m = lock(&self.map);
            let branch = m.remove(id).and_then(|t| t.branch);
            m.insert(id.to_owned(), Track { gen, branch, git_dir: None, pending: 0, watcher: None });
        }
        let (map, emit, id, dir) = (self.map.clone(), self.emit.clone(), id.to_owned(), PathBuf::from(dir));
        let _ = std::thread::Builder::new().name("pty-branch".into()).spawn(move || {
            let res = run_git(&dir, &["rev-parse", "--absolute-git-dir"], GIT_TIMEOUT);
            let git_dir = res.ok.then(|| res.stdout.trim().to_owned()).filter(|s| !s.is_empty()).map(PathBuf::from);
            {
                let mut m = lock(&map);
                let Some(t) = m.get_mut(&id).filter(|t| t.gen == gen) else { return };
                t.git_dir = git_dir.clone();
            }
            update(&map, &emit, &id, gen);
            let Some(git_dir) = git_dir else { return };
            let (wmap, wemit, wid) = (map.clone(), emit.clone(), id.clone());
            let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let Ok(ev) = res else { return };
                if ev.paths.iter().any(|p| p.file_name().is_some_and(|n| n == "HEAD")) {
                    on_head(&wmap, &wemit, &wid, gen);
                }
            });
            let Ok(mut watcher) = made else { return };
            if watcher.watch(&git_dir, RecursiveMode::NonRecursive).is_err() {
                return;
            }
            if let Some(t) = lock(&map).get_mut(&id).filter(|t| t.gen == gen) {
                t.watcher = Some(watcher);
            }
        });
    }

    pub fn remove(&self, id: &str) {
        lock(&self.map).remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::mpsc;

    #[test]
    fn head_content_maps_to_a_branch_or_a_short_hash() {
        assert_eq!(branch_of_head("ref: refs/heads/main\n").as_deref(), Some("main"));
        assert_eq!(branch_of_head("ref:refs/heads/feat/a").as_deref(), Some("feat/a"));
        assert_eq!(branch_of_head("0123456789abcdef0123456789abcdef01234567\n").as_deref(), Some("0123456"));
        assert_eq!(branch_of_head("ref: refs/tags/v1"), None);
        assert_eq!(branch_of_head("ref: refs/heads/"), None);
        assert_eq!(branch_of_head("xyz"), None);
        assert_eq!(branch_of_head(""), None);
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(args).env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t").output().unwrap().status.success();
        assert!(ok, "{args:?}");
    }

    #[test]
    fn a_pane_gets_its_branch_follows_checkout_and_cd_out_of_git() {
        let base = std::env::temp_dir().join(format!("orrery-pty-branch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        let plain = base.join("plain");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&plain).unwrap();
        git(&repo, &["init", "-q", "-b", "trunk"]);
        std::fs::write(repo.join("a.txt"), "a").unwrap();
        git(&repo, &["add", "a.txt"]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        let (tx, rx) = mpsc::channel::<(String, Option<String>)>();
        let tx = Mutex::new(tx);
        let b = Branches::new(Arc::new(move |id: &str, br: Option<&str>| {
            let _ = lock(&tx).send((id.to_owned(), br.map(str::to_owned)));
        }));
        let wait = || rx.recv_timeout(Duration::from_secs(10)).unwrap();
        b.track("p1", &repo.to_string_lossy());
        assert_eq!(wait(), ("p1".into(), Some("trunk".into())));
        std::thread::sleep(Duration::from_millis(300));
        git(&repo, &["checkout", "-q", "-b", "side"]);
        assert_eq!(wait(), ("p1".into(), Some("side".into())));
        b.track("p1", &plain.to_string_lossy());
        assert_eq!(wait(), ("p1".into(), None));
        b.remove("p1");
        let _ = std::fs::remove_dir_all(&base);
    }
}
