use std::time::Instant;

use serde_json::{json, Value};

use super::config::now_secs;
use super::ctx::Ctx;
use super::dashboard::{error, Phone};
use super::devices::{clean_mute, WebSub};
use super::fcm::Outcome;
use super::push::{locale_of, Job};
use super::push_content::{mask_text, notice, refine, scene_of, Ask, Scene};
use super::webpush::{encrypt, unb64, valid_public, valid_subject, AUTH_LEN};
use super::webpush_send::{topic, valid_endpoint, Delivery};
use crate::state::lock;

pub const DEFAULT_SUBJECT: &str = "mailto:orrery-remote@localhost";

pub fn parse_sub(v: &Value) -> Option<WebSub> {
    let endpoint = v.get("endpoint")?.as_str()?;
    let p256dh = v.get("p256dh")?.as_str()?;
    let auth = v.get("auth")?.as_str()?;
    if !valid_endpoint(endpoint) || p256dh.len() > 100 || auth.len() > 40 {
        return None;
    }
    let key = unb64(p256dh)?;
    let secret = unb64(auth)?;
    if secret.len() != AUTH_LEN || !valid_public(&key) {
        return None;
    }
    Some(WebSub { endpoint: endpoint.to_owned(), p256dh: p256dh.trim_end_matches('=').to_owned(), auth: auth.trim_end_matches('=').to_owned() })
}

pub fn subject_for(configured: &str, relay: &str) -> String {
    if valid_subject(configured) {
        return configured.to_owned();
    }
    let relay = relay.trim_end_matches('/');
    if relay.starts_with("https://") && valid_subject(relay) {
        return relay.to_owned();
    }
    DEFAULT_SUBJECT.to_owned()
}

pub fn web_payload(job: &Job, title: &str, body: &str) -> Value {
    let tag = if job.pane.is_empty() { format!("{}:{}", job.machine, job.kind) } else { format!("{}:{}", job.machine, job.pane) };
    let mut out = json!({ "title": title, "body": body, "m": job.machine, "k": job.kind, "tag": tag });
    if !job.pane.is_empty() {
        out["p"] = Value::from(job.pane.clone());
    }
    out
}

impl Ctx {
    pub fn web_ready(&self) -> bool {
        self.webpush.vapid().is_some()
    }

    pub fn note_episode(&self, episode: &Value) -> bool {
        lock(&self.facts).note(episode, Instant::now())
    }

    pub fn settled(&self, job: &Job) -> Job {
        if job.terminal.is_empty() {
            return job.clone();
        }
        let facts = lock(&self.facts);
        Job { kind: refine(job.kind, facts.get(&job.terminal, Instant::now())), ..job.clone() }
    }

    pub fn notice(&self, job: &Job) -> (String, String) {
        let scene = if job.content && !job.pane.is_empty() { self.board.summary().map(|s| scene_of(&s, &job.pane)).unwrap_or_default() } else { Scene::default() };
        let facts = lock(&self.facts);
        let fact = if job.content && !job.terminal.is_empty() { facts.get(&job.terminal, Instant::now()) } else { None };
        let worked = job.worked.or_else(|| fact.filter(|f| f.reason == "done").and_then(|f| f.worked));
        notice(&Ask { kind: job.kind, locale: job.locale, content: job.content, scene: &scene, fact, worked }, &mask_text)
    }

    pub fn vapid_subject(&self) -> String {
        let cfg = lock(&self.config);
        subject_for(&cfg.webpush_subject, &cfg.relay.url)
    }

    pub fn deliver_web(&self, job: &Job, sub: &WebSub) -> Outcome {
        let (title, body) = self.notice(job);
        let plain = web_payload(job, &title, &body).to_string();
        let (Some(key), Some(auth)) = (unb64(&sub.p256dh), unb64(&sub.auth)) else { return Outcome::Fail("rejected") };
        let Some(sealed) = encrypt(&key, &auth, plain.as_bytes()) else { return Outcome::Fail("rejected") };
        let topic = topic(&job.machine, if job.pane.is_empty() { job.kind } else { &job.pane }, &sub.auth);
        let subject = self.vapid_subject();
        let outcome = self.webpush.send(&Delivery { endpoint: &sub.endpoint, subject: &subject, topic: &topic, urgency: "high", body: &sealed }, now_secs(), &|| self.still_wanted(job));
        if outcome == Outcome::Dead && lock(&self.devices).clear_web_endpoint(&sub.endpoint) {
            self.changed();
        }
        outcome
    }

    pub fn webpush_config_event(&self) -> Value {
        match self.webpush.public_key() {
            Some(key) => json!({ "t": "webpush.config", "key": key }),
            None => json!({ "t": "webpush.config" }),
        }
    }
}

impl Phone {
    pub(super) async fn push_register_web(&mut self, id: &str, v: &Value) -> bool {
        let Some(sub) = parse_sub(v) else { return self.sec.json(&error(Value::from(id), "invalid")).await };
        if !self.ctx.web_ready() {
            return self.sec.json(&error(Value::from(id), "internal")).await;
        }
        let locale = locale_of(v.get("locale").and_then(Value::as_str));
        let mute = clean_mute(v.get("mute").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str));
        let content = v.get("content").and_then(Value::as_bool).unwrap_or(true);
        let stored = lock(&self.ctx.devices).set_web_push(&self.device, sub, locale, mute, content);
        if stored {
            self.ctx.changed();
        }
        let reply = if stored { json!({ "re": id, "ok": true }) } else { error(Value::from(id), "denied") };
        self.sec.json(&reply).await
    }
}

impl super::Remote {
    pub fn note_episode(&self, episode: &Value) {
        if let Some(c) = lock(&self.ctx).clone() {
            c.note_episode(episode);
        }
    }
}
