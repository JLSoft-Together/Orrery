use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use core_store::extra::GitDiffStat;
use core_store::time::{iso_of_ms, now_ms};

use crate::job_stop::run_capped;

#[derive(Debug, Clone, PartialEq)]
pub enum GitRun {
    Stat(GitDiffStat),
    Failed,
    NotARepo,
    MissingDir,
}

pub type Runner = Arc<dyn Fn(&str, i64) -> GitRun + Send + Sync>;
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

fn number_before(out: &str, words: &[&str]) -> u64 {
    for w in words {
        if let Some(i) = out.find(w) {
            let head = out[..i].trim_end();
            let digits: String = head.chars().rev().take_while(char::is_ascii_digit).collect::<Vec<_>>().into_iter().rev().collect();
            if !digits.is_empty() && head.len() < out[..i].len() {
                return digits.parse().unwrap_or(0);
            }
        }
    }
    0
}

pub fn parse_shortstat(out: &str) -> (u64, u64, u64) {
    (
        number_before(out, &["files changed", "file changed"]),
        number_before(out, &["insertions(+)", "insertion(+)"]),
        number_before(out, &["deletions(-)", "deletion(-)"]),
    )
}

pub fn git_path() -> Option<PathBuf> {
    if !cfg!(windows) {
        return Some(PathBuf::from("git"));
    }
    let path = std::env::var_os("PATH").or_else(|| std::env::var_os("Path"))?;
    std::env::split_paths(&path).map(|d| d.join("git.exe")).find(|p| p.is_file())
}

enum Git {
    Out(String),
    Failed,
    NotARepo,
    MissingDir,
}

fn git(bin: &std::path::Path, cwd: &str, args: &[&str]) -> Git {
    let mut all = vec!["-C", cwd];
    all.extend_from_slice(args);
    let Some(r) = run_capped(bin, &all, &[("GIT_OPTIONAL_LOCKS", "0"), ("GIT_TERMINAL_PROMPT", "0")], Duration::from_secs(5)) else { return Git::Failed };
    if r.code == Some(0) {
        return Git::Out(r.stdout);
    }
    let err = r.stderr.to_lowercase();
    if r.code == Some(128) && err.contains("not a git repository") {
        return Git::NotARepo;
    }
    if r.code == Some(128) && err.contains("cannot change to") {
        return Git::MissingDir;
    }
    Git::Failed
}

pub fn run_diff_stat(cwd: &str, now: i64) -> GitRun {
    let Some(bin) = git_path() else { return GitRun::Failed };
    let short = match git(&bin, cwd, &["diff", "--shortstat", "--histogram", "HEAD"]) {
        Git::Out(s) => s,
        Git::Failed => return GitRun::Failed,
        Git::NotARepo => return GitRun::NotARepo,
        Git::MissingDir => return GitRun::MissingDir,
    };
    let untracked = match git(&bin, cwd, &["ls-files", "--others", "--exclude-standard", "--directory", "--no-empty-directory"]) {
        Git::Out(s) => s.split('\n').filter(|l| !l.trim().is_empty()).count() as u64,
        _ => 0,
    };
    let (files, insertions, deletions) = parse_shortstat(&short);
    GitRun::Stat(GitDiffStat { files, insertions, deletions, untracked, at: iso_of_ms(now) })
}

pub fn same_diff(a: &GitDiffStat, b: &GitDiffStat) -> bool {
    a.files == b.files && a.insertions == b.insertions && a.deletions == b.deletions && a.untracked == b.untracked
}

struct Slot {
    waiters: Vec<String>,
    next: Option<Vec<String>>,
}

pub struct DiffStatReader {
    run: Runner,
    clock: Clock,
    not_repo_ttl: i64,
    missing_ttl: i64,
    slots: HashMap<String, (String, Slot)>,
    not_repo: HashMap<String, (i64, i64)>,
    tx: Sender<(String, GitRun)>,
    rx: Receiver<(String, GitRun)>,
    ready: Vec<(String, Option<GitDiffStat>)>,
}

fn key_of(cwd: &str) -> String {
    if cfg!(windows) {
        cwd.to_lowercase()
    } else {
        cwd.to_owned()
    }
}

impl DiffStatReader {
    pub fn new(run: Runner, clock: Clock) -> Self {
        let (tx, rx) = channel();
        DiffStatReader { run, clock, not_repo_ttl: 300_000, missing_ttl: 30_000, slots: HashMap::new(), not_repo: HashMap::new(), tx, rx, ready: Vec::new() }
    }

    pub fn system() -> Self {
        Self::new(Arc::new(run_diff_stat), Arc::new(now_ms))
    }

    pub fn with_ttls(mut self, not_repo: i64, missing: i64) -> Self {
        self.not_repo_ttl = not_repo;
        self.missing_ttl = missing;
        self
    }

    pub fn busy(&self) -> bool {
        !self.slots.is_empty() || !self.ready.is_empty()
    }

    pub fn request(&mut self, waiter: &str, cwd: &str) {
        let key = key_of(cwd);
        if let Some((_, slot)) = self.slots.get_mut(&key) {
            slot.next.get_or_insert_with(Vec::new).push(waiter.to_owned());
            return;
        }
        self.launch(key, cwd.to_owned(), vec![waiter.to_owned()]);
    }

    fn launch(&mut self, key: String, cwd: String, waiters: Vec<String>) {
        let now = (self.clock)();
        if let Some((at, ttl)) = self.not_repo.get(&key) {
            if now - at < *ttl {
                self.ready.extend(waiters.into_iter().map(|w| (w, None)));
                return;
            }
        }
        let (run, tx, k, c) = (self.run.clone(), self.tx.clone(), key.clone(), cwd.clone());
        std::thread::spawn(move || {
            let r = run(&c, now);
            let _ = tx.send((k, r));
        });
        self.slots.insert(key, (cwd, Slot { waiters, next: None }));
    }

    fn finish(&mut self, key: String, r: GitRun) {
        let Some((cwd, slot)) = self.slots.remove(&key) else { return };
        let value = match r {
            GitRun::NotARepo | GitRun::MissingDir => {
                let ttl = if r == GitRun::NotARepo { self.not_repo_ttl } else { self.missing_ttl };
                self.not_repo.insert(key.clone(), ((self.clock)(), ttl));
                None
            }
            GitRun::Stat(s) => {
                self.not_repo.remove(&key);
                Some(s)
            }
            GitRun::Failed => {
                self.not_repo.remove(&key);
                None
            }
        };
        self.ready.extend(slot.waiters.into_iter().map(|w| (w, value.clone())));
        if let Some(next) = slot.next {
            self.launch(key, cwd, next);
        }
    }

    pub fn poll(&mut self) -> Vec<(String, Option<GitDiffStat>)> {
        while let Ok((key, r)) = self.rx.try_recv() {
            self.finish(key, r);
        }
        std::mem::take(&mut self.ready)
    }

    pub fn wait(&mut self, timeout: Duration) -> Vec<(String, Option<GitDiffStat>)> {
        if self.ready.is_empty() && !self.slots.is_empty() {
            match self.rx.recv_timeout(timeout) {
                Ok((key, r)) => self.finish(key, r),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
            }
        }
        self.poll()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    #[test]
    fn parses_shortstat_lines() {
        assert_eq!(parse_shortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)\n"), (3, 10, 2));
        assert_eq!(parse_shortstat(" 1 file changed, 1 insertion(+)\n"), (1, 1, 0));
        assert_eq!(parse_shortstat(" 1 file changed, 4 deletions(-)\n"), (1, 0, 4));
        assert_eq!(parse_shortstat(""), (0, 0, 0));
    }

    fn reader(result: GitRun, calls: Arc<AtomicUsize>, clock: Arc<AtomicI64>) -> DiffStatReader {
        let c = clock.clone();
        DiffStatReader::new(
            Arc::new(move |_: &str, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(20));
                result.clone()
            }),
            Arc::new(move || c.load(Ordering::SeqCst)),
        )
    }

    fn drain(r: &mut DiffStatReader) -> Vec<(String, Option<GitDiffStat>)> {
        let mut out = Vec::new();
        while r.busy() {
            out.extend(r.wait(Duration::from_secs(5)));
        }
        out
    }

    fn stat() -> GitRun {
        GitRun::Stat(GitDiffStat { files: 1, insertions: 2, deletions: 0, untracked: 0, at: "x".into() })
    }

    #[test]
    fn runs_git_at_most_twice_for_simultaneous_callers_on_the_same_cwd() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut r = reader(stat(), calls.clone(), Arc::new(AtomicI64::new(0)));
        for w in ["a", "b", "c", "d"] {
            r.request(w, "D:\\Repo");
        }
        r.request("e", "d:\\repo");
        let out = drain(&mut r);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(out.len(), 5);
        assert!(out.iter().all(|(_, s)| s.is_some()));
        r.request("f", "D:\\Repo");
        drain(&mut r);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn caches_a_non_repo_and_a_missing_dir_for_their_ttl_but_never_a_failure() {
        let clock = Arc::new(AtomicI64::new(0));
        for (result, ttl) in [(GitRun::NotARepo, 300_000), (GitRun::MissingDir, 30_000)] {
            let calls = Arc::new(AtomicUsize::new(0));
            let mut r = reader(result, calls.clone(), clock.clone());
            clock.store(0, Ordering::SeqCst);
            r.request("a", "/x");
            drain(&mut r);
            clock.store(ttl - 1, Ordering::SeqCst);
            r.request("a", "/x");
            assert_eq!(drain(&mut r), [("a".to_owned(), None)]);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            clock.store(ttl, Ordering::SeqCst);
            r.request("a", "/x");
            drain(&mut r);
            assert_eq!(calls.load(Ordering::SeqCst), 2);
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let mut r = reader(GitRun::Failed, calls.clone(), clock);
        r.request("a", "/x");
        drain(&mut r);
        r.request("a", "/x");
        drain(&mut r);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
