use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::watch;

use super::config::FirebaseConfig;
use super::ctx::Ctx;
use super::dashboard::{error, Phone};
use super::devices::clean_mute;
use super::fcm::Outcome;
use super::pane_link::valid_pane_key;
use crate::state::{lock, take_slot};

pub const PANE_GAP: Duration = Duration::from_secs(30);
pub const PER_HOUR: usize = 30;
pub const HOUR: Duration = Duration::from_secs(3600);
pub const QUEUE_MAX: usize = 32;
const PANES_MAX: usize = 256;
const TOKEN_MIN: usize = 16;
const TOKEN_MAX: usize = 512;

pub fn valid_token(token: &str) -> bool {
    (TOKEN_MIN..=TOKEN_MAX).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'.' | b'-'))
}

pub fn kind_of(raw: &str) -> &'static str {
    match raw {
        "done" => "done",
        "question" => "question",
        "permission" => "permission",
        "error" => "error",
        "security" => "security",
        _ => "attention",
    }
}

pub fn locale_of(raw: Option<&str>) -> &'static str {
    if raw == Some("vi") {
        "vi"
    } else {
        "en"
    }
}

pub fn title_body(kind: &str, locale: &str) -> (&'static str, &'static str) {
    let vi = locale == "vi";
    match (kind_of(kind), vi) {
        ("done", false) => ("Agent finished", "A session finished its turn. Tap to open it."),
        ("done", true) => ("Agent đã xong lượt", "Một phiên vừa xong lượt. Chạm để mở."),
        ("question", false) => ("Agent has a question", "A session is waiting for your answer. Tap to open it."),
        ("question", true) => ("Agent đang hỏi", "Một phiên đang chờ câu trả lời. Chạm để mở."),
        ("permission", false) => ("Permission needed", "A session needs your approval. Tap to open it."),
        ("permission", true) => ("Cần cấp quyền", "Một phiên đang chờ bạn cho phép. Chạm để mở."),
        ("error", false) => ("Session error", "A session stopped with an error. Tap to open it."),
        ("error", true) => ("Phiên bị lỗi", "Một phiên vừa dừng vì lỗi. Chạm để mở."),
        ("security", false) => ("Orrery security alert", "A new device, a blocked IP or a sign-in from a new place on your computer. Check Settings → Remote on the computer."),
        ("security", true) => ("Cảnh báo bảo mật Orrery", "Có thiết bị mới, IP bị chặn hoặc nơi nối lạ trên máy tính. Xem Cài đặt → Điều khiển từ xa trên máy tính."),
        (_, false) => ("Session needs attention", "A session needs you. Tap to open it."),
        (_, true) => ("Phiên cần chú ý", "Một phiên cần bạn. Chạm để mở."),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub device: String,
    pub token: String,
    pub machine: String,
    pub pane: String,
    pub kind: &'static str,
    pub locale: &'static str,
}

pub fn message(job: &Job) -> Value {
    let (title, body) = if job.kind == "test" { test_text(job.locale) } else { title_body(job.kind, job.locale) };
    let mut data = json!({ "m": job.machine, "k": job.kind });
    if !job.pane.is_empty() {
        data["p"] = Value::from(job.pane.clone());
    }
    json!({
        "message": {
            "token": job.token,
            "notification": { "title": title, "body": body },
            "data": data,
            "android": { "priority": "high", "notification": { "channel_id": "attention" } },
        }
    })
}

pub fn test_text(locale: &str) -> (&'static str, &'static str) {
    if locale == "vi" {
        ("Orrery", "Thông báo thử từ máy tính. Push đã hoạt động.")
    } else {
        ("Orrery", "Test notification from your computer. Push works.")
    }
}

#[derive(Default)]
pub struct PushState {
    panes: HashMap<(String, String), Instant>,
    hours: HashMap<String, Vec<Instant>>,
    queue: VecDeque<Job>,
}

impl PushState {
    pub fn admit(&mut self, device: &str, pane: &str, now: Instant) -> bool {
        let key = (device.to_owned(), pane.to_owned());
        if self.panes.get(&key).is_some_and(|t| now.duration_since(*t) < PANE_GAP) {
            return false;
        }
        if !take_slot(self.hours.entry(device.to_owned()).or_default(), now, PER_HOUR, HOUR) {
            return false;
        }
        if self.panes.len() >= PANES_MAX {
            self.panes.retain(|_, t| now.duration_since(*t) < PANE_GAP);
        }
        self.panes.insert(key, now);
        true
    }

    pub fn push(&mut self, job: Job) {
        if self.queue.len() >= QUEUE_MAX {
            self.queue.pop_front();
        }
        self.queue.push_back(job);
    }

    pub fn drain(&mut self) -> Vec<Job> {
        self.queue.drain(..).collect()
    }
}

impl Ctx {
    pub fn push_ready(&self) -> bool {
        self.fcm.has_account() && lock(&self.config).firebase.is_set()
    }

    pub fn push_attention(&self, pane: &str, kind: &str, now: Instant) -> usize {
        if !self.running.load(Ordering::SeqCst) || !valid_pane_key(pane) || !self.push_ready() {
            return 0;
        }
        let machine = self.machine_id();
        let kind = kind_of(kind);
        let targets: Vec<(String, String, String)> = lock(&self.devices)
            .list()
            .iter()
            .filter(|d| !d.push_token.is_empty() && d.wants(kind))
            .map(|d| (d.device_id.clone(), d.push_token.clone(), d.push_locale.clone()))
            .collect();
        let mut queued = 0;
        {
            let mut st = lock(&self.push);
            for (device, token, locale) in targets {
                if self.is_live(&device) || !st.admit(&device, pane, now) {
                    continue;
                }
                st.push(Job { device, token, machine: machine.clone(), pane: pane.to_owned(), kind, locale: locale_of(Some(&locale)) });
                queued += 1;
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn push_security(&self, except: Option<&str>) -> usize {
        if !self.running.load(Ordering::SeqCst) || !self.push_ready() {
            return 0;
        }
        let machine = self.machine_id();
        let targets: Vec<(String, String, String)> = lock(&self.devices)
            .list()
            .iter()
            .filter(|d| !d.push_token.is_empty() && Some(d.device_id.as_str()) != except)
            .map(|d| (d.device_id.clone(), d.push_token.clone(), d.push_locale.clone()))
            .collect();
        let queued = targets.len();
        {
            let mut st = lock(&self.push);
            for (device, token, locale) in targets {
                st.push(Job { device, token, machine: machine.clone(), pane: String::new(), kind: "security", locale: locale_of(Some(&locale)) });
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn take_push_jobs(&self) -> Vec<Job> {
        lock(&self.push).drain()
    }

    pub fn deliver(&self, job: &Job) -> Outcome {
        let project = lock(&self.config).firebase.project_id.clone();
        if project.is_empty() {
            return Outcome::Fail("not_configured");
        }
        let outcome = self.fcm.send(&project, &message(job));
        if outcome == Outcome::Dead {
            lock(&self.devices).clear_push_token(&job.token);
            self.changed();
        }
        outcome
    }

    pub fn push_config_event(&self) -> Value {
        config_event(&lock(&self.config).firebase)
    }

    pub fn push_config_changed(&self) {
        let event = self.push_config_event();
        self.push_cfg.send_replace(event);
    }
}

pub fn config_event(cfg: &FirebaseConfig) -> Value {
    if cfg.is_set() {
        json!({ "t": "push.config", "apiKey": cfg.api_key, "appId": cfg.app_id, "senderId": cfg.sender_id, "projectId": cfg.project_id })
    } else {
        json!({ "t": "push.config" })
    }
}

pub async fn run(ctx: Arc<Ctx>, mut stop: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = ctx.push_wake.notified() => {}
            _ = stop.changed() => break,
        }
        for job in ctx.take_push_jobs() {
            if (job.kind != "security" && ctx.is_live(&job.device)) || *stop.borrow() {
                continue;
            }
            let c = ctx.clone();
            let _ = tokio::task::spawn_blocking(move || c.deliver(&job)).await;
        }
    }
}

impl Phone {
    pub(super) async fn push_register(&mut self, id: &str, v: &Value) -> bool {
        let token = v.get("token").and_then(Value::as_str).unwrap_or("");
        if !valid_token(token) {
            return self.sec.json(&error(Value::from(id), "invalid")).await;
        }
        let locale = locale_of(v.get("locale").and_then(Value::as_str));
        let mute = clean_mute(v.get("mute").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str));
        let stored = lock(&self.ctx.devices).set_push_mute(&self.device, token, locale, mute);
        let reply = if stored { json!({ "re": id, "ok": true }) } else { error(Value::from(id), "denied") };
        self.sec.json(&reply).await
    }

    pub(super) async fn push_unregister(&mut self, id: &str) -> bool {
        lock(&self.ctx.devices).clear_push(&self.device);
        self.sec.json(&json!({ "re": id, "ok": true })).await
    }
}
