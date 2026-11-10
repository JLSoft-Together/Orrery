use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::watch::Wake;

#[derive(Debug, Clone, PartialEq)]
pub enum FsEvent {
    Changed(&'static str, PathBuf),
    Lost(&'static str),
}

pub struct FsWatch {
    tx: Sender<FsEvent>,
    rx: Receiver<FsEvent>,
    handles: HashMap<&'static str, RecommendedWatcher>,
    wake: Wake,
}

impl FsWatch {
    pub fn new(wake: Option<Wake>) -> Self {
        let (tx, rx) = channel();
        FsWatch { tx, rx, handles: HashMap::new(), wake: wake.unwrap_or_else(|| std::sync::Arc::new(|| {})) }
    }

    pub fn is_watching(&self, tag: &'static str) -> bool {
        self.handles.contains_key(tag)
    }

    pub fn watch(&mut self, tag: &'static str, dir: &Path, recursive: bool) -> notify::Result<()> {
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        let mut w = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(ev) if matches!(ev.kind, EventKind::Access(_)) => return,
                Ok(ev) => {
                    for p in ev.paths {
                        let _ = tx.send(FsEvent::Changed(tag, p));
                    }
                }
                Err(_) => {
                    let _ = tx.send(FsEvent::Lost(tag));
                }
            }
            wake();
        })?;
        w.watch(dir, if recursive { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive })?;
        self.handles.insert(tag, w);
        Ok(())
    }

    pub fn unwatch(&mut self, tag: &'static str) {
        self.handles.remove(tag);
    }

    pub fn drain(&mut self) -> Vec<FsEvent> {
        let out: Vec<FsEvent> = self.rx.try_iter().collect();
        for e in &out {
            if let FsEvent::Lost(tag) = e {
                self.handles.remove(tag);
            }
        }
        out
    }
}
