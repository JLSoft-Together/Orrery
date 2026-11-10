#![allow(dead_code)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use core_store::model::TerminalSession;
use core_store::persist::localtime::{parse_iso, Zone};
use core_toast::{HostMessage, PhoneEpisode, TelegramEpisode, ToastContent, ToastOptions, ToastSnapshot, Toaster};
use serde_json::{json, Value};

pub fn t0() -> i64 {
    parse_iso("2026-10-08T10:00:00", Zone::Local).unwrap()
}

pub fn iso(ms: i64) -> String {
    core_store::time::iso_of_ms(ms)
}

pub fn term(id: &str, status: &str, extra: Value) -> TerminalSession {
    let mut v = json!({ "id": id, "title": format!("title-{id}"), "status": status });
    if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra) {
        o.extend(e);
    }
    serde_json::from_value(v).unwrap()
}

pub fn tag(key: &str) -> String {
    core_toast::util::toast_tag(key)
}

pub fn w(id: &str) -> String {
    tag(&format!("waiting:{id}"))
}

pub struct Rig {
    pub t: Toaster,
    pub now: Arc<AtomicI64>,
    pub calls: Vec<String>,
    pub shown: Vec<ToastContent>,
    pub phone: Vec<PhoneEpisode>,
    pub telegram: Vec<TelegramEpisode>,
}

impl Rig {
    pub fn new(supported: bool) -> Self {
        Self::with(ToastOptions { supported, port: 4317, file: None, link: None }, t0())
    }

    pub fn with(options: ToastOptions, start: i64) -> Self {
        let now = Arc::new(AtomicI64::new(start));
        let c = now.clone();
        let t = Toaster::new(options, Arc::new(move || c.load(Ordering::SeqCst)));
        Self { t, now, calls: Vec::new(), shown: Vec::new(), phone: Vec::new(), telegram: Vec::new() }
    }

    pub fn flush(&mut self) {
        for m in self.t.drain() {
            match m {
                HostMessage::Show(c) => {
                    self.calls.push(format!("show:{}", c.tag));
                    self.shown.push(c);
                }
                HostMessage::Hide(tag) => self.calls.push(format!("hide:{tag}")),
                HostMessage::Phone(p) => self.phone.push(p),
                HostMessage::Telegram(t) => self.telegram.push(t),
            }
        }
    }

    pub fn see(&mut self, terminals: &[TerminalSession]) {
        self.t.observe(&ToastSnapshot { terminals: terminals.iter().collect(), ..Default::default() });
        self.flush();
    }

    pub fn advance(&self, ms: i64) {
        self.now.fetch_add(ms, Ordering::SeqCst);
    }

    pub fn now_ms(&self) -> i64 {
        self.now.load(Ordering::SeqCst)
    }

    pub fn titles(&self) -> Vec<&str> {
        self.shown.iter().map(|c| c.title.as_str()).collect()
    }

    pub fn shows(&self) -> usize {
        self.calls.iter().filter(|c| c.starts_with("show")).count()
    }
}
