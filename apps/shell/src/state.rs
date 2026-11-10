use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::appearance;
use crate::pty::PtyHost;
use crate::serve::Serve;
use crate::strings::{strings, Strings};

pub const MAIN: &str = "main";

pub struct Shell {
    pub port: u16,
    pub key: String,
    pub origin: String,
    pub data_dir: PathBuf,
    pub webview_dir: PathBuf,
    pub init_script: String,
    pub pty: PtyHost,
    pub serve: Mutex<Option<Serve>>,
    pub look: Mutex<appearance::Store>,
    pub replies: Mutex<HashMap<String, mpsc::Sender<Value>>>,
    pub link_times: Mutex<Vec<Instant>>,
    pub quitting: AtomicBool,
    pub stopped: AtomicBool,
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shell {
    pub fn t(&self) -> &'static Strings {
        strings(lock(&self.look).locale())
    }

    pub fn is_app_url(&self, url: &tauri::Url) -> bool {
        url.origin().ascii_serialization() == self.origin
    }

    pub fn reply(&self, id: &str, value: Value) -> bool {
        match lock(&self.replies).remove(id) {
            Some(tx) => tx.send(value).is_ok(),
            None => false,
        }
    }

    pub fn ask(&self, app: &AppHandle, id: &str, event: &str, payload: Value, timeout: Duration) -> Option<Value> {
        app.get_webview_window(MAIN)?;
        let (tx, rx) = mpsc::channel();
        lock(&self.replies).insert(id.to_owned(), tx);
        if app.emit_to(MAIN, event, payload).is_err() {
            lock(&self.replies).remove(id);
            return None;
        }
        let got = rx.recv_timeout(timeout).ok();
        lock(&self.replies).remove(id);
        got
    }

    pub fn take_link_slot(&self, now: Instant) -> bool {
        take_slot(&mut lock(&self.link_times), now, 10, Duration::from_secs(60))
    }

    pub fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        self.pty.stop();
        if let Some(mut serve) = lock(&self.serve).take() {
            serve.stop();
        }
    }
}

pub fn take_slot(times: &mut Vec<Instant>, now: Instant, max: usize, window: Duration) -> bool {
    times.retain(|t| now.duration_since(*t) < window);
    if times.len() >= max {
        return false;
    }
    times.push(now);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_slots_allow_ten_a_minute() {
        let mut times = Vec::new();
        let start = Instant::now();
        for _ in 0..10 {
            assert!(take_slot(&mut times, start, 10, Duration::from_secs(60)));
        }
        assert!(!take_slot(&mut times, start + Duration::from_secs(59), 10, Duration::from_secs(60)));
        assert!(take_slot(&mut times, start + Duration::from_secs(60), 10, Duration::from_secs(60)));
    }
}
