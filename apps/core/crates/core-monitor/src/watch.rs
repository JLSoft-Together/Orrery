use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

#[derive(Debug, Clone, PartialEq)]
pub enum WatchEvent {
    Registry,
    Project(String),
    SessionsLost,
    ProjectsLost,
}

pub type Wake = std::sync::Arc<dyn Fn() + Send + Sync>;

pub struct Watcher {
    tx: Sender<WatchEvent>,
    rx: Receiver<WatchEvent>,
    sessions: Option<RecommendedWatcher>,
    projects: Option<RecommendedWatcher>,
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
}

impl Default for Watcher {
    fn default() -> Self {
        Self::new()
    }
}

fn relative(root: &Path, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(root).ok()?;
    let s = rel.to_string_lossy().into_owned();
    (!s.is_empty()).then_some(s)
}

impl Watcher {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Watcher { tx, rx, sessions: None, projects: None, wake: std::sync::Arc::new(|| {}) }
    }

    pub fn with_wake(wake: Wake) -> Self {
        let mut w = Self::new();
        w.wake = wake;
        w
    }

    pub fn watch_sessions(&mut self, dir: &Path) -> notify::Result<()> {
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let e = match res {
                Ok(ev) if matches!(ev.kind, EventKind::Access(_)) => return,
                Ok(_) => WatchEvent::Registry,
                Err(_) => WatchEvent::SessionsLost,
            };
            let _ = tx.send(e);
            wake();
        })?;
        w.watch(dir, RecursiveMode::NonRecursive)?;
        self.sessions = Some(w);
        Ok(())
    }

    pub fn watch_projects(&mut self, dir: &Path) -> notify::Result<()> {
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        let root: PathBuf = dir.to_path_buf();
        let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(ev) if matches!(ev.kind, EventKind::Access(_)) => return,
                Ok(ev) => {
                    for p in &ev.paths {
                        if let Some(rel) = relative(&root, p) {
                            let _ = tx.send(WatchEvent::Project(rel));
                        }
                    }
                }
                Err(_) => {
                    let _ = tx.send(WatchEvent::ProjectsLost);
                }
            }
            wake();
        })?;
        w.watch(dir, RecursiveMode::Recursive)?;
        self.projects = Some(w);
        Ok(())
    }

    pub fn drain(&mut self) -> Vec<WatchEvent> {
        let out: Vec<WatchEvent> = self.rx.try_iter().collect();
        if out.contains(&WatchEvent::SessionsLost) {
            self.sessions = None;
        }
        if out.contains(&WatchEvent::ProjectsLost) {
            self.projects = None;
        }
        out
    }
}
