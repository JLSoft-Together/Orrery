use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use super::super::ctx::Ctx;
use super::super::devices::Device;
use super::super::pane_link::{pane_of, valid_pane_key};
use super::super::push_content::{clean, label_of, mask_text, scene_of, LABEL_MAX};
use super::{kind_of, Job, PushState, HOUR, PANES_MAX, PANE_GAP};
use crate::state::{lock, take_slot};

pub const PER_HOUR_INFO: usize = 20;
pub const RUNNING_AFTER: Duration = Duration::from_secs(120);
pub const PAUSE_MAX_MS: u64 = 24 * 3600 * 1000;
pub const MENU_ITEMS_MAX: usize = 3;
pub const WAIT_LABELS_MAX: usize = 2;
pub const WAITING_MAX: usize = 999;
const ONGOING_TTL: Duration = Duration::from_secs(12 * 3600);
const ONGOING_MAX: usize = 256;
const SIG_MAX: usize = 16;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extra {
    pub waiting: Option<usize>,
    pub labels: Vec<String>,
    pub menu: Option<Value>,
    pub apk: Option<Value>,
}

pub fn is_info(kind: &str) -> bool {
    matches!(kind, "done" | "running" | "open" | "clear" | "apk")
}

pub fn is_group(kind: &str) -> bool {
    matches!(kind, "attention" | "question" | "permission" | "done" | "error" | "running")
}

pub fn carries_count(kind: &str) -> bool {
    is_group(kind) || matches!(kind, "open" | "clear" | "apk")
}

fn ends_run(kind: &str) -> bool {
    matches!(kind, "attention" | "question" | "permission" | "done" | "error")
}

pub fn wants_kind(d: &Device, kind: &str) -> bool {
    match kind {
        "running" => d.wants("done"),
        "open" | "clear" | "apk" => true,
        other => d.wants(other),
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub fn extra_data(job: &Job) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(w) = job.extra.waiting {
        out.insert("w".into(), Value::from(w.to_string()));
    }
    if let Some(t0) = job.started.filter(|_| job.kind == "running") {
        out.insert("t0".into(), Value::from(t0.to_string()));
    }
    out
}

pub fn menu_of(summary: &Value, pane: &str, mask: &dyn Fn(&str) -> String) -> Option<Value> {
    let menu = summary["tabs"].as_array()?.iter().filter_map(|t| t["panes"].as_array()).flatten().find(|p| p["key"].as_str() == Some(pane))?.get("menu")?;
    let sig = menu["sig"].as_str().filter(|s| (1..=SIG_MAX).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric()))?;
    let raw = menu["items"].as_array()?;
    if raw.iter().any(|i| i.get("checked").is_some_and(Value::is_boolean)) {
        return None;
    }
    let items: Vec<Value> = raw
        .iter()
        .filter(|i| i["other"].as_bool() != Some(true) && i["submit"].as_bool() != Some(true))
        .filter_map(|i| {
            let index = i["index"].as_u64()?;
            let label = clean(i["label"].as_str()?, mask, LABEL_MAX);
            (!label.is_empty()).then(|| json!({ "index": index, "label": label }))
        })
        .take(MENU_ITEMS_MAX)
        .collect();
    (!items.is_empty()).then(|| json!({ "sig": sig, "items": items }))
}

pub fn waiting_labels(summary: &Value, terminals: &[String], mask: &dyn Fn(&str) -> String) -> Vec<String> {
    terminals
        .iter()
        .filter_map(|t| pane_of(summary, t))
        .map(|pane| label_of(&scene_of(summary, &pane), mask))
        .filter(|l| !l.is_empty())
        .take(WAIT_LABELS_MAX)
        .collect()
}

impl PushState {
    pub fn admit_info(&mut self, device: &str, pane: &str, gap: bool, now: Instant) -> bool {
        let key = (device.to_owned(), pane.to_owned());
        if gap && self.panes.get(&key).is_some_and(|t| now.duration_since(*t) < PANE_GAP) {
            return false;
        }
        if !take_slot(self.hours_info.entry(device.to_owned()).or_default(), now, PER_HOUR_INFO, HOUR) {
            return false;
        }
        if gap {
            if self.panes.len() >= PANES_MAX {
                self.panes.retain(|_, t| now.duration_since(*t) < PANE_GAP);
            }
            self.panes.insert(key, now);
        }
        true
    }

    pub fn admit_kind(&mut self, device: &str, pane: &str, kind: &str, now: Instant, now_ms: u64) -> bool {
        if ends_run(kind) && self.take_ongoing(device, pane, now) {
            self.panes.insert((device.to_owned(), pane.to_owned()), now);
            return true;
        }
        if is_group(kind) && self.paused(device, now_ms) {
            return false;
        }
        let ok = if is_info(kind) { self.admit_info(device, pane, kind == "done", now) } else { self.admit(device, pane, now) };
        if ok && kind == "running" {
            self.mark_ongoing(device, pane, now);
        }
        ok
    }

    fn mark_ongoing(&mut self, device: &str, pane: &str, now: Instant) {
        if self.ongoing.len() >= ONGOING_MAX {
            self.ongoing.retain(|_, at| now.duration_since(*at) < ONGOING_TTL);
        }
        if self.ongoing.len() < ONGOING_MAX {
            self.ongoing.insert((device.to_owned(), pane.to_owned()), now);
        }
    }

    fn take_ongoing(&mut self, device: &str, pane: &str, now: Instant) -> bool {
        self.ongoing.remove(&(device.to_owned(), pane.to_owned())).is_some_and(|at| now.duration_since(at) < ONGOING_TTL)
    }

    pub fn set_pause(&mut self, device: &str, until_ms: Option<u64>, now_ms: u64) {
        self.pauses.retain(|_, u| *u > now_ms);
        match until_ms.filter(|u| *u > now_ms && *u <= now_ms + PAUSE_MAX_MS) {
            Some(until) => {
                self.pauses.insert(device.to_owned(), until);
            }
            None => {
                self.pauses.remove(device);
            }
        }
    }

    pub fn paused(&self, device: &str, now_ms: u64) -> bool {
        self.pauses.get(device).is_some_and(|u| *u > now_ms)
    }

    pub fn set_waiting(&mut self, ids: Vec<String>) -> bool {
        let was = !self.waiting.is_empty();
        self.waiting = ids;
        was && self.waiting.is_empty()
    }

    pub fn told(&self, device: &str) -> usize {
        self.told.get(device).copied().unwrap_or(0)
    }

    pub fn forget_device(&mut self, device: &str) {
        self.pauses.remove(device);
        self.told.remove(device);
        self.ongoing.retain(|(d, _), _| d != device);
    }
}

impl Ctx {
    pub fn set_waiting(&self, ids: Vec<String>) -> bool {
        lock(&self.push).set_waiting(ids)
    }

    pub fn note_pause(&self, device: &str, raw: Option<&Value>) {
        lock(&self.push).set_pause(device, raw.and_then(Value::as_u64), now_ms());
    }

    pub fn queue_pane(&self, pane: &str, kind: &str, terminal: &str, worked: Option<u64>, started: Option<u64>, now: Instant) -> usize {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) || !valid_pane_key(pane) {
            return 0;
        }
        let kind = kind_of(kind);
        let ongoing: Vec<String> = lock(&self.push).ongoing.keys().filter(|(_, p)| p == pane).map(|(d, _)| d.clone()).collect();
        let targets = self.push_targets(|d| wants_kind(d, kind) || (ends_run(kind) && ongoing.contains(&d.device_id)));
        let now_ms = now_ms();
        let mut queued = 0;
        {
            let mut st = lock(&self.push);
            for job in targets {
                if (kind != "open" && self.is_live(&job.device)) || !st.admit_kind(&job.device, pane, kind, now, now_ms) {
                    continue;
                }
                st.push(Job { pane: pane.to_owned(), kind, terminal: terminal.to_owned(), worked, started, ..job });
                queued += 1;
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn push_running(&self, terminal: &str, started_ms: u64, now: Instant) -> usize {
        let Some(pane) = self.board.summary().and_then(|s| pane_of(&s, terminal)) else { return 0 };
        self.queue_pane(&pane, "running", terminal, None, Some(started_ms), now)
    }

    pub fn push_open(&self, pane: &str, now: Instant) -> Result<usize, &'static str> {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("off");
        }
        if !valid_pane_key(pane) {
            return Err("invalid");
        }
        let known = self.board.summary().is_some_and(|s| s["tabs"].as_array().is_some_and(|tabs| tabs.iter().filter_map(|t| t["panes"].as_array()).flatten().any(|p| p["key"].as_str() == Some(pane))));
        if !known {
            return Err("notFound");
        }
        if self.push_targets(|_| true).is_empty() {
            return Err("none");
        }
        match self.queue_pane(pane, "open", "", None, None, now) {
            0 => Err("limit"),
            n => Ok(n),
        }
    }

    pub fn push_clear(&self, now: Instant) -> usize {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return 0;
        }
        let targets = self.push_targets(|_| true);
        let mut queued = 0;
        {
            let mut st = lock(&self.push);
            for job in targets.into_iter().filter(|j| j.web.is_none() && !j.token.is_empty()) {
                if st.told(&job.device) == 0 || self.is_live(&job.device) || !st.admit_info(&job.device, "", false, now) {
                    continue;
                }
                st.push(Job { kind: "clear", content: false, seal_key: None, pane: String::new(), ..job });
                queued += 1;
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn enrich(&self, job: &Job) -> Result<Job, &'static str> {
        let mut job = job.clone();
        if !carries_count(job.kind) {
            return Ok(job);
        }
        let (ids, told, paused) = {
            let st = lock(&self.push);
            (st.waiting.clone(), st.told(&job.device), st.paused(&job.device, now_ms()))
        };
        if job.kind == "clear" && !ids.is_empty() {
            return Err("superseded");
        }
        if job.kind == "clear" && told == 0 {
            return Err("nothing");
        }
        if is_group(job.kind) && paused {
            return Err("paused");
        }
        job.extra.waiting = Some(ids.len().min(WAITING_MAX));
        if job.content && job.kind != "clear" {
            if let Some(summary) = self.board.summary() {
                job.extra.labels = waiting_labels(&summary, &ids, &mask_text);
                if matches!(job.kind, "attention" | "question" | "permission") && !job.pane.is_empty() {
                    job.extra.menu = menu_of(&summary, &job.pane, &mask_text);
                }
            }
        }
        Ok(job)
    }

    pub fn note_delivered(&self, job: &Job) {
        if let Some(w) = job.extra.waiting {
            lock(&self.push).told.insert(job.device.clone(), w);
        }
    }
}

impl super::super::Remote {
    pub fn send_pane(&self, arg: &Value) -> Value {
        let key = arg.as_str().or_else(|| arg["paneKey"].as_str()).unwrap_or("");
        let Some(ctx) = lock(&self.ctx).clone() else { return json!({ "ok": false, "error": "off" }) };
        match ctx.push_open(key, Instant::now()) {
            Ok(sent) => json!({ "ok": true, "sent": sent }),
            Err(error) => json!({ "ok": false, "error": error }),
        }
    }
}
