use super::status::{read_status, resolve_dir, StatusRead, FILE};
use crate::Emitter;
use notify::{RecursiveMode, Watcher};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

pub const EVENT: &str = "ccm:agent-status";
const KEYS_MAX: usize = 64;
const DIR_MAX: usize = 1024;

#[derive(Clone, Copy)]
pub struct Timing {
    pub debounce: Duration,
    pub recheck: Duration,
    pub hold: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { debounce: Duration::from_millis(300), recheck: Duration::from_secs(30), hold: Duration::from_secs(5) }
    }
}

enum Cmd {
    Set(String, Option<String>),
    Touch(String, u64),
    Broken(String, u64),
    Debounce(String, u64, u64),
    Recheck(String, u64),
    Hold(String, u64, u64),
    Stop,
}

struct Entry {
    dir: String,
    id: u64,
    watcher: Option<notify::RecommendedWatcher>,
    watching: Option<String>,
    debounce: u64,
    recheck: bool,
    hold: Option<u64>,
    seq: u64,
    last: Option<Value>,
}

pub struct AgentStatusWatch {
    tx: Sender<Cmd>,
}

impl Drop for AgentStatusWatch {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

fn same(a: &Option<Value>, b: &Option<Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => ["phase", "progress", "note", "updatedAt"].iter().all(|k| x.get(k) == y.get(k)),
        _ => false,
    }
}

fn later(tx: &Sender<Cmd>, wait: Duration, cmd: Cmd) {
    let tx = tx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(wait);
        let _ = tx.send(cmd);
    });
}

struct Worker {
    entries: HashMap<String, Entry>,
    next: u64,
    tx: Sender<Cmd>,
    emitter: Emitter,
    timing: Timing,
}

impl Worker {
    fn emit(&self, key: &str, status: &Option<Value>) {
        self.emitter.emit(EVENT, json!({"key": key, "status": status}));
    }

    fn set(&mut self, key: String, dir: Option<String>) {
        let Some(dir) = dir.filter(|d| crate::js::len(d) <= DIR_MAX && crate::claude::is_absolute(d)) else {
            self.entries.remove(&key);
            return;
        };
        if let Some(e) = self.entries.get(&key) {
            if e.dir == dir {
                self.emit(&key, &e.last);
                return;
            }
        }
        self.entries.remove(&key);
        if self.entries.len() >= KEYS_MAX {
            return;
        }
        self.next += 1;
        let entry = Entry { dir, id: self.next, watcher: None, watching: None, debounce: 0, recheck: false, hold: None, seq: 0, last: None };
        self.entries.insert(key.clone(), entry);
        self.probe(&key);
    }

    fn entry(&mut self, key: &str, id: u64) -> Option<&mut Entry> {
        self.entries.get_mut(key).filter(|e| e.id == id)
    }

    fn schedule(&mut self, key: &str, id: u64) {
        let wait = self.timing.debounce;
        let tx = self.tx.clone();
        let Some(e) = self.entry(key, id) else { return };
        e.debounce += 1;
        later(&tx, wait, Cmd::Debounce(key.to_string(), id, e.debounce));
    }

    fn arm_recheck(&mut self, key: &str) {
        let wait = self.timing.recheck;
        let tx = self.tx.clone();
        let Some(e) = self.entries.get_mut(key) else { return };
        if e.recheck {
            return;
        }
        e.recheck = true;
        later(&tx, wait, Cmd::Recheck(key.to_string(), e.id));
    }

    fn attach(&mut self, key: &str, orrery: &str) {
        let tx = self.tx.clone();
        let Some(e) = self.entries.get_mut(key) else { return };
        e.watcher = None;
        e.watching = None;
        let (k, id) = (key.to_string(), e.id);
        let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let cmd = match &res {
                Err(_) => Some(Cmd::Broken(k.clone(), id)),
                Ok(ev) if ev.paths.is_empty() || ev.paths.iter().any(|p| p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase() == FILE)) => Some(Cmd::Touch(k.clone(), id)),
                Ok(_) => None,
            };
            if let Some(cmd) = cmd {
                let _ = tx.send(cmd);
            }
        });
        match made.and_then(|mut w| w.watch(std::path::Path::new(orrery), RecursiveMode::NonRecursive).map(|_| w)) {
            Ok(w) => {
                e.watcher = Some(w);
                e.watching = Some(orrery.to_string());
            }
            Err(_) => self.arm_recheck(key),
        }
    }

    fn probe(&mut self, key: &str) {
        let Some(dir) = self.entries.get(key).map(|e| e.dir.clone()) else { return };
        let Some(orrery) = resolve_dir(&dir) else {
            if let Some(e) = self.entries.get_mut(key) {
                e.watcher = None;
                e.watching = None;
            }
            self.apply(key, StatusRead::Missing);
            self.arm_recheck(key);
            return;
        };
        if self.entries.get(key).and_then(|e| e.watching.as_deref()) != Some(orrery.as_str()) {
            self.attach(key, &orrery);
        }
        self.apply(key, read_status(&orrery));
    }

    fn apply(&mut self, key: &str, read: StatusRead) {
        let wait = self.timing.hold;
        let tx = self.tx.clone();
        let Some(e) = self.entries.get_mut(key) else { return };
        let status = match read {
            StatusRead::Invalid => {
                if e.last.is_some() && e.hold.is_none() {
                    e.seq += 1;
                    e.hold = Some(e.seq);
                    later(&tx, wait, Cmd::Hold(key.to_string(), e.id, e.seq));
                }
                return;
            }
            StatusRead::Missing => None,
            StatusRead::Ok(v) => Some(v),
        };
        e.hold = None;
        self.publish(key, status);
    }

    fn publish(&mut self, key: &str, status: Option<Value>) {
        let Some(e) = self.entries.get_mut(key) else { return };
        if same(&e.last, &status) {
            return;
        }
        e.last = status.clone();
        self.emit(key, &status);
    }

    fn run(mut self, rx: std::sync::mpsc::Receiver<Cmd>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                Cmd::Stop => return,
                Cmd::Set(k, d) => self.set(k, d),
                Cmd::Touch(k, id) => self.schedule(&k, id),
                Cmd::Broken(k, id) => {
                    if let Some(e) = self.entry(&k, id) {
                        e.watcher = None;
                        e.watching = None;
                        self.schedule(&k, id);
                    }
                }
                Cmd::Debounce(k, id, n) => {
                    if self.entry(&k, id).is_some_and(|e| e.debounce == n) {
                        self.probe(&k);
                    }
                }
                Cmd::Recheck(k, id) => {
                    if let Some(e) = self.entry(&k, id) {
                        e.recheck = false;
                        self.probe(&k);
                    }
                }
                Cmd::Hold(k, id, n) => {
                    if let Some(e) = self.entry(&k, id).filter(|e| e.hold == Some(n)) {
                        e.hold = None;
                        self.publish(&k, None);
                    }
                }
            }
        }
    }
}

impl AgentStatusWatch {
    pub fn new(emitter: Emitter, timing: Timing) -> Self {
        let (tx, rx) = channel();
        let worker = Worker { entries: HashMap::new(), next: 0, tx: tx.clone(), emitter, timing };
        std::thread::spawn(move || worker.run(rx));
        AgentStatusWatch { tx }
    }

    pub fn set(&self, key: &str, dir: Option<String>) {
        let _ = self.tx.send(Cmd::Set(key.to_string(), dir));
    }
}
