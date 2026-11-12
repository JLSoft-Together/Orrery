use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{mpsc, watch};

use super::ctx::Ctx;
use super::pane_link::pane_of;

const WORKING: &str = "working";
const TAP_RETRY: Duration = Duration::from_secs(2);
const TERMINALS_MAX: usize = 4096;

#[derive(Default)]
pub struct Watch {
    status: HashMap<String, String>,
    started: HashMap<String, Instant>,
    worked: HashMap<String, u64>,
}

fn events(text: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(text).ok().and_then(|v| v.get("events").and_then(Value::as_array).cloned()).unwrap_or_default()
}

pub fn transition(from: &str, to: &str) -> Option<&'static str> {
    match (from, to) {
        (WORKING, "waiting") => Some("attention"),
        (WORKING, "idle") => Some("done"),
        (WORKING | "waiting", "error") => Some("error"),
        _ => None,
    }
}

impl Watch {
    pub fn apply(&mut self, text: &str) -> Vec<(String, &'static str)> {
        let mut out = Vec::new();
        for event in events(text) {
            let payload = &event["payload"];
            match event["type"].as_str() {
                Some("snapshot") => {
                    self.status.clear();
                    self.started.clear();
                    self.worked.clear();
                    for t in payload["terminals"].as_array().into_iter().flatten() {
                        self.set(t["id"].as_str(), t["status"].as_str());
                    }
                }
                Some("terminal.created") => self.set(payload["id"].as_str(), payload["status"].as_str()),
                Some("terminal.updated") => {
                    let (Some(id), Some(to)) = (payload["id"].as_str(), payload["status"].as_str()) else { continue };
                    let from = self.status.get(id).cloned().unwrap_or_default();
                    if let Some(kind) = transition(&from, to) {
                        if kind == "done" {
                            if let Some(at) = self.started.get(id) {
                                self.worked.insert(id.to_owned(), at.elapsed().as_millis() as u64);
                            }
                        }
                        out.push((id.to_owned(), kind));
                    }
                    self.set(Some(id), Some(to));
                }
                Some("terminal.removed") => {
                    if let Some(id) = payload["id"].as_str() {
                        self.status.remove(id);
                        self.started.remove(id);
                        self.worked.remove(id);
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn take_worked(&mut self, id: &str) -> Option<u64> {
        self.worked.remove(id)
    }

    fn set(&mut self, id: Option<&str>, status: Option<&str>) {
        let (Some(id), Some(status)) = (id, status) else { return };
        if self.status.len() >= TERMINALS_MAX && !self.status.contains_key(id) {
            return;
        }
        let was = self.status.insert(id.to_owned(), status.to_owned());
        if status == WORKING && was.as_deref() != Some(WORKING) {
            self.started.insert(id.to_owned(), Instant::now());
        } else if status != WORKING {
            self.started.remove(id);
        }
    }
}

impl Ctx {
    #[cfg(test)]
    pub fn push_terminal(&self, terminal: &str, kind: &str, now: Instant) -> usize {
        self.push_terminal_after(terminal, kind, None, now)
    }

    pub fn push_terminal_after(&self, terminal: &str, kind: &str, worked: Option<u64>, now: Instant) -> usize {
        let Some(pane) = self.board.summary().and_then(|s| pane_of(&s, terminal)) else { return 0 };
        self.push_pane(&pane, kind, terminal, worked, now)
    }
}

pub async fn run(ctx: Arc<Ctx>, mut stop: watch::Receiver<bool>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Arc<str>>();
    let mut watcher = Watch::default();
    let guard = loop {
        if let Some(tap) = ctx.host.tap() {
            let sink = tx.clone();
            let (snapshot, guard) = tap.attach(move |text| {
                let _ = sink.send(text);
            });
            watcher.apply(&snapshot);
            break guard;
        }
        tokio::select! {
            _ = tokio::time::sleep(TAP_RETRY) => {}
            _ = stop.changed() => return,
        }
    };
    loop {
        tokio::select! {
            text = rx.recv() => {
                let Some(text) = text else { break };
                for (terminal, kind) in watcher.apply(&text) {
                    let worked = watcher.take_worked(&terminal);
                    ctx.push_terminal_after(&terminal, kind, worked, Instant::now());
                }
            }
            _ = stop.changed() => break,
        }
    }
    drop(guard);
}
