use crate::run::collect::{collect, CollectOpts};
use crate::run::parse::read_adb_frames;
use crate::run::proc::{kill_pid, spawn, SpawnSpec};
use crate::run::types::Env;
use std::io::Read;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdbWatchTiming {
    pub first_list_ms: u64,
    pub poll_ms: u64,
    pub restart_window_ms: u64,
    pub max_restarts: usize,
    pub poll_timeout_ms: u64,
}

impl Default for AdbWatchTiming {
    fn default() -> Self {
        AdbWatchTiming { first_list_ms: 3_000, poll_ms: 3_000, restart_window_ms: 5 * 60_000, max_restarts: 3, poll_timeout_ms: 10_000 }
    }
}

pub type AdbLookup = Arc<dyn Fn() -> Option<String> + Send + Sync>;
pub type Parser<T> = Arc<dyn Fn(&str) -> Vec<T> + Send + Sync>;
pub type Listener<T> = Arc<dyn Fn(Vec<T>) + Send + Sync>;

pub struct AdbWatchOpts<T> {
    pub cwd: String,
    pub env: Env,
    pub adb: AdbLookup,
    pub parse: Parser<T>,
    pub timing: AdbWatchTiming,
    pub kill: Option<fn(u32)>,
}

struct State<T> {
    items: Vec<T>,
    started: bool,
    got_first: bool,
    tracker: Option<u32>,
    restarts: Vec<Instant>,
    polling: bool,
    poll_loop: bool,
    stopped: bool,
    listeners: Vec<Listener<T>>,
}

struct Inner<T> {
    opts: AdbWatchOpts<T>,
    state: Mutex<State<T>>,
    cv: Condvar,
}

pub struct AdbWatch<T> {
    inner: Arc<Inner<T>>,
}

impl<T: Clone + PartialEq + Send + 'static> AdbWatch<T> {
    pub fn new(opts: AdbWatchOpts<T>) -> Self {
        let state = State { items: Vec::new(), started: false, got_first: false, tracker: None, restarts: Vec::new(), polling: false, poll_loop: false, stopped: false, listeners: Vec::new() };
        AdbWatch { inner: Arc::new(Inner { opts, state: Mutex::new(state), cv: Condvar::new() }) }
    }

    pub fn list(&self) -> Vec<T> {
        start(&self.inner);
        let deadline = Instant::now() + Duration::from_millis(self.inner.opts.timing.first_list_ms);
        let mut st = lock(&self.inner);
        while !st.got_first && !st.stopped {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            st = self.inner.cv.wait_timeout(st, deadline - now).map(|r| r.0).unwrap_or_else(|e| e.into_inner().0);
        }
        st.items.clone()
    }

    pub fn on_change(&self, cb: Listener<T>) {
        {
            let mut st = lock(&self.inner);
            if st.stopped {
                return;
            }
            st.listeners.push(cb);
        }
        start(&self.inner);
        schedule_poll(&self.inner, false);
    }

    pub fn stop(&self, sync: bool) {
        let tracker = {
            let mut st = lock(&self.inner);
            st.stopped = true;
            st.listeners.clear();
            st.tracker.take()
        };
        self.inner.cv.notify_all();
        if let Some(pid) = tracker {
            kill_with(&self.inner, pid, sync);
        }
    }
}

fn lock<T>(inner: &Inner<T>) -> MutexGuard<'_, State<T>> {
    inner.state.lock().unwrap_or_else(|e| e.into_inner())
}

fn kill_with<T>(inner: &Inner<T>, pid: u32, sync: bool) {
    match inner.opts.kill {
        Some(k) => k(pid),
        None => kill_pid(pid, sync),
    }
}

fn publish<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>, next: Vec<T>) {
    let listeners = {
        let mut st = lock(inner);
        let changed = st.items != next;
        st.items = next;
        st.got_first = true;
        inner.cv.notify_all();
        if !changed {
            return;
        }
        st.listeners.clone()
    };
    let items = lock(inner).items.clone();
    for cb in listeners {
        cb(items.clone());
    }
}

fn start<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>) {
    {
        let mut st = lock(inner);
        if st.started {
            return;
        }
        st.started = true;
    }
    let inner = inner.clone();
    std::thread::spawn(move || {
        let adb = (inner.opts.adb)();
        if lock(&inner).stopped {
            return;
        }
        match adb {
            None => publish(&inner, Vec::new()),
            Some(adb) => run_tracker(&inner, adb),
        }
    });
}

fn run_tracker<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>, mut adb: String) {
    loop {
        track_once(inner, &adb);
        if lock(inner).stopped {
            return;
        }
        let now = Instant::now();
        let retry = {
            let mut st = lock(inner);
            let window = Duration::from_millis(inner.opts.timing.restart_window_ms);
            st.restarts.retain(|at| now.duration_since(*at) < window);
            if st.restarts.len() < inner.opts.timing.max_restarts {
                st.restarts.push(now);
                true
            } else {
                st.polling = true;
                false
            }
        };
        if !retry {
            schedule_poll(inner, true);
            return;
        }
        match (inner.opts.adb)() {
            Some(next) if !lock(inner).stopped => adb = next,
            _ => return,
        }
    }
}

fn track_once<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>, adb: &str) {
    let spec = SpawnSpec { file: adb.into(), args: vec!["track-devices".into(), "-l".into()], cwd: inner.opts.cwd.clone(), ..Default::default() };
    let Ok(mut child) = spawn(&spec, &inner.opts.env) else { return };
    let pid = child.id();
    {
        let mut st = lock(inner);
        if st.stopped {
            drop(st);
            kill_with(inner, pid, false);
            let _ = child.wait();
            return;
        }
        st.tracker = Some(pid);
    }
    if let Some(mut err) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut sink = [0u8; 4096];
            while matches!(err.read(&mut sink), Ok(n) if n > 0) {}
        });
    }
    if let Some(mut out) = child.stdout.take() {
        let mut buf = String::new();
        let mut pending: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        while let Ok(n) = out.read(&mut chunk) {
            if n == 0 {
                break;
            }
            pending.extend_from_slice(&chunk[..n]);
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                Err(_) => pending.len(),
            };
            let text: Vec<u8> = pending.drain(..valid).collect();
            buf.push_str(&String::from_utf8_lossy(&text));
            let (frames, rest) = read_adb_frames(&buf);
            buf = if rest.len() > 4 + 0xffff { String::new() } else { rest };
            if let Some(last) = frames.last() {
                let parsed = (inner.opts.parse)(last);
                publish(inner, parsed);
            }
        }
    }
    let _ = child.wait();
    let mut st = lock(inner);
    if st.tracker == Some(pid) {
        st.tracker = None;
    }
}

fn poll_once<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>) {
    let Some(adb) = (inner.opts.adb)() else {
        publish(inner, Vec::new());
        return;
    };
    let opts = CollectOpts { timeout_ms: Some(inner.opts.timing.poll_timeout_ms), kill: Some(inner.opts.kill.unwrap_or(kill_tree_async)), ..Default::default() };
    let r = collect(&adb, &["devices".into(), "-l".into()], &inner.opts.cwd, &inner.opts.env, &opts);
    if r.code == Some(0) {
        let parsed = (inner.opts.parse)(&r.out);
        publish(inner, parsed);
    }
}

fn kill_tree_async(pid: u32) {
    kill_pid(pid, false);
}

fn schedule_poll<T: Clone + PartialEq + Send + 'static>(inner: &Arc<Inner<T>>, immediate: bool) {
    {
        let mut st = lock(inner);
        if st.poll_loop || st.stopped || !st.polling || (!immediate && st.listeners.is_empty()) {
            return;
        }
        st.poll_loop = true;
    }
    let inner = inner.clone();
    std::thread::spawn(move || {
        if immediate {
            poll_once(&inner);
        }
        loop {
            {
                let st = lock(&inner);
                if st.stopped || !st.polling || st.listeners.is_empty() {
                    drop(st);
                    lock(&inner).poll_loop = false;
                    return;
                }
                let wait = Duration::from_millis(inner.opts.timing.poll_ms);
                let (st, _) = inner.cv.wait_timeout_while(st, wait, |s| !s.stopped).unwrap_or_else(|e| e.into_inner());
                if st.stopped {
                    drop(st);
                    lock(&inner).poll_loop = false;
                    return;
                }
            }
            poll_once(&inner);
        }
    });
}

#[cfg(all(test, unix))]
#[path = "adb_watch_tests.rs"]
mod tests;
