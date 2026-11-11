use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::state::{lock, take_slot};

pub const VIEWER_PER_MINUTE: usize = 120;
pub const WRITE_PER_MINUTE: usize = 20;
pub const COALESCE: Duration = Duration::from_millis(100);
pub const QUEUE: usize = 64;
pub const MAX_PATHS: usize = 200;
pub const MAX_RUN_EVENT: usize = 256 * 1024;
const MINUTE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub enum VEvent {
    Fs { root: String, paths: Vec<String> },
    Git { root: String },
    Run(Arc<Value>),
}

struct Sub {
    tx: mpsc::Sender<VEvent>,
    run: bool,
}

#[derive(Default)]
pub struct Hub {
    subs: Mutex<HashMap<String, Sub>>,
    rates: Mutex<HashMap<String, Vec<Instant>>>,
    writes: Mutex<HashMap<String, Vec<Instant>>>,
    busy: Mutex<std::collections::HashSet<String>>,
}

fn take(map: &Mutex<HashMap<String, Vec<Instant>>>, device: &str, now: Instant, max: usize) -> bool {
    let mut map = lock(map);
    map.retain(|_, times| times.iter().any(|t| now.duration_since(*t) < MINUTE));
    take_slot(map.entry(device.to_owned()).or_default(), now, max, MINUTE)
}

impl Hub {
    pub fn register(&self, owner: &str, tx: mpsc::Sender<VEvent>) {
        lock(&self.subs).insert(owner.to_owned(), Sub { tx, run: false });
    }

    pub fn remove(&self, owner: &str) {
        lock(&self.subs).remove(owner);
    }

    pub fn want_run(&self, owner: &str) {
        if let Some(s) = lock(&self.subs).get_mut(owner) {
            s.run = true;
        }
    }

    pub fn push(&self, owner: &str, event: VEvent) -> bool {
        lock(&self.subs).get(owner).is_some_and(|s| s.tx.try_send(event).is_ok())
    }

    pub fn push_run(&self, payload: &Value) {
        let subs = lock(&self.subs);
        if !subs.values().any(|s| s.run) || serde_json::to_vec(payload).map_or(true, |b| b.len() > MAX_RUN_EVENT) {
            return;
        }
        let shared = Arc::new(payload.clone());
        for s in subs.values().filter(|s| s.run) {
            let _ = s.tx.try_send(VEvent::Run(shared.clone()));
        }
    }

    pub fn is_open(&self, owner: &str) -> bool {
        lock(&self.subs).contains_key(owner)
    }

    pub fn take_busy(&self, key: &str) -> bool {
        lock(&self.busy).insert(key.to_owned())
    }

    pub fn free_busy(&self, key: &str) {
        lock(&self.busy).remove(key);
    }

    pub fn take_rate(&self, device: &str, now: Instant) -> bool {
        take(&self.rates, device, now, VIEWER_PER_MINUTE)
    }

    pub fn take_write(&self, device: &str, now: Instant) -> bool {
        take(&self.writes, device, now, WRITE_PER_MINUTE)
    }
}

pub fn merge(batch: Vec<VEvent>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut fs: HashMap<String, usize> = HashMap::new();
    let mut git: HashMap<String, usize> = HashMap::new();
    for ev in batch {
        match ev {
            VEvent::Fs { root, paths } => {
                let at = *fs.entry(root.clone()).or_insert_with(|| {
                    out.push(json!({ "t": "fs.changed", "root": root, "paths": [] }));
                    out.len() - 1
                });
                let list = out[at]["paths"].as_array_mut().expect("paths");
                for p in paths {
                    if list.len() < MAX_PATHS && !list.iter().any(|x| x.as_str() == Some(p.as_str())) {
                        list.push(Value::from(p));
                    }
                }
            }
            VEvent::Git { root } => {
                git.entry(root.clone()).or_insert_with(|| {
                    out.push(json!({ "t": "git.changed", "root": root }));
                    out.len() - 1
                });
            }
            VEvent::Run(payload) => out.push(json!({ "t": "run.event", "event": payload.as_ref() })),
        }
    }
    out
}
