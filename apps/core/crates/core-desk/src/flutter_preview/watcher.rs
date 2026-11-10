use notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub const SAVE_DEBOUNCE_MS: u64 = 400;

pub type Unwatch = Box<dyn FnOnce() + Send>;

pub fn watch_dir(dir: &Path, recursive: bool, only_file: Option<&'static str>, debounce: Duration, on_change: Arc<dyn Fn(Vec<String>) + Send + Sync>) -> Unwatch {
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        for p in ev.paths {
            if only_file.is_none_or(|f| p.file_name().is_some_and(|n| n == f)) {
                let _ = tx.send(p);
            }
        }
    });
    let mut watcher = match made {
        Ok(w) => w,
        Err(_) => return Box::new(|| {}),
    };
    let mode = if recursive { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
    if watcher.watch(dir, mode).is_err() {
        return Box::new(|| {});
    }
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut changed = vec![first.to_string_lossy().to_string()];
            loop {
                match rx.recv_timeout(debounce) {
                    Ok(p) => {
                        let s = p.to_string_lossy().to_string();
                        if !changed.contains(&s) {
                            changed.push(s);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            on_change(changed);
        }
    });
    Box::new(move || drop(watcher))
}
