use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::watch;

use super::config::FirebaseConfig;
use super::ctx::Ctx;
use super::dashboard::{error, Phone};
use super::devices::{clean_mute, Device, WebSub};
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Job {
    pub device: String,
    pub token: String,
    pub machine: String,
    pub pane: String,
    pub kind: &'static str,
    pub locale: &'static str,
    pub web: Option<WebSub>,
    pub content: bool,
    pub terminal: String,
    pub worked: Option<u64>,
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

    pub fn push_targets(&self, keep: impl Fn(&Device) -> bool) -> Vec<Job> {
        let (fcm, web) = (self.push_ready(), self.web_ready());
        let machine = self.machine_id();
        lock(&self.devices)
            .list()
            .iter()
            .filter(|d| !d.push_off && keep(d))
            .filter_map(|d| {
                let sub = d.web_push.clone().filter(|_| web);
                if sub.is_none() && (d.push_token.is_empty() || !fcm) {
                    return None;
                }
                Some(Job { device: d.device_id.clone(), token: d.push_token.clone(), machine: machine.clone(), locale: locale_of(Some(&d.push_locale)), web: sub, content: d.push_content, ..Job::default() })
            })
            .collect()
    }

    #[cfg(test)]
    pub fn push_attention(&self, pane: &str, kind: &str, now: Instant) -> usize {
        self.push_pane(pane, kind, "", None, now)
    }

    pub fn push_pane(&self, pane: &str, kind: &str, terminal: &str, worked: Option<u64>, now: Instant) -> usize {
        if !self.running.load(Ordering::SeqCst) || !valid_pane_key(pane) {
            return 0;
        }
        let kind = kind_of(kind);
        let targets = self.push_targets(|d| d.wants(kind));
        let mut queued = 0;
        {
            let mut st = lock(&self.push);
            for job in targets {
                if self.is_live(&job.device) || !st.admit(&job.device, pane, now) {
                    continue;
                }
                st.push(Job { pane: pane.to_owned(), kind, terminal: terminal.to_owned(), worked, ..job });
                queued += 1;
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn push_security(&self, except: Option<&str>) -> usize {
        if !self.running.load(Ordering::SeqCst) {
            return 0;
        }
        let targets = self.push_targets(|d| Some(d.device_id.as_str()) != except);
        let queued = targets.len();
        {
            let mut st = lock(&self.push);
            for job in targets {
                st.push(Job { kind: "security", content: false, ..job });
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

    pub fn still_wanted(&self, job: &Job) -> bool {
        lock(&self.devices).find(&job.device).is_some_and(|d| {
            !d.push_off
                && match &job.web {
                    Some(sub) => d.web_push.as_ref() == Some(sub),
                    None => !job.token.is_empty() && d.push_token == job.token,
                }
        })
    }

    pub fn deliver(&self, job: &Job) -> Outcome {
        let job = &self.settled(job);
        if !self.still_wanted(job) {
            return Outcome::Fail("no_device");
        }
        if let Some(sub) = &job.web {
            return self.deliver_web(job, sub);
        }
        let project = lock(&self.config).firebase.project_id.clone();
        if project.is_empty() {
            return Outcome::Fail("not_configured");
        }
        let outcome = self.fcm.send_while(&project, &message(job), &|| self.still_wanted(job));
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
        let settle = *lock(&ctx.settle);
        if !settle.is_zero() {
            tokio::select! {
                _ = tokio::time::sleep(settle) => {}
                _ = stop.changed() => break,
            }
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
        if v.get("kind").and_then(Value::as_str) == Some("webpush") {
            return self.push_register_web(id, v).await;
        }
        let token = v.get("token").and_then(Value::as_str).unwrap_or("");
        if !valid_token(token) {
            return self.sec.json(&error(Value::from(id), "invalid")).await;
        }
        let locale = locale_of(v.get("locale").and_then(Value::as_str));
        let mute = clean_mute(v.get("mute").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str));
        let content = v.get("content").and_then(Value::as_bool).unwrap_or(true);
        let stored = {
            let mut devices = lock(&self.ctx.devices);
            let ok = devices.set_push_mute(&self.device, token, locale, mute);
            if ok {
                devices.set_push_content(&self.device, content);
            }
            ok
        };
        if stored {
            self.ctx.changed();
        }
        let reply = if stored { json!({ "re": id, "ok": true }) } else { error(Value::from(id), "denied") };
        self.sec.json(&reply).await
    }

    pub(super) async fn push_unregister(&mut self, id: &str) -> bool {
        lock(&self.ctx.devices).clear_push(&self.device);
        self.ctx.changed();
        self.sec.json(&json!({ "re": id, "ok": true })).await
    }
}
