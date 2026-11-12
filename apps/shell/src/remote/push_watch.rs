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
                    for t in payload["terminals"].as_array().into_iter().flatten() {
                        self.set(t["id"].as_str(), t["status"].as_str());
                    }
                }
                Some("terminal.created") => self.set(payload["id"].as_str(), payload["status"].as_str()),
                Some("terminal.updated") => {
                    let (Some(id), Some(to)) = (payload["id"].as_str(), payload["status"].as_str()) else { continue };
                    let from = self.status.get(id).cloned().unwrap_or_default();
                    if let Some(kind) = transition(&from, to) {
                        out.push((id.to_owned(), kind));
                    }
                    self.set(Some(id), Some(to));
                }
                Some("terminal.removed") => {
                    if let Some(id) = payload["id"].as_str() {
                        self.status.remove(id);
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn set(&mut self, id: Option<&str>, status: Option<&str>) {
        let (Some(id), Some(status)) = (id, status) else { return };
        if self.status.len() < TERMINALS_MAX || self.status.contains_key(id) {
            self.status.insert(id.to_owned(), status.to_owned());
        }
    }
}

impl Ctx {
    pub fn push_terminal(&self, terminal: &str, kind: &str, now: Instant) -> usize {
        let Some(pane) = self.board.summary().and_then(|s| pane_of(&s, terminal)) else { return 0 };
        self.push_attention(&pane, kind, now)
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
                    ctx.push_terminal(&terminal, kind, Instant::now());
                }
            }
            _ = stop.changed() => break,
        }
    }
    drop(guard);
}
