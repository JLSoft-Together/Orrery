use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::remote::dashboard::{error, Phone};
use crate::state::lock;

pub const MAX_CLIP: usize = 64 * 1024;
pub const CAP: &str = "clip";
const QUEUE: usize = 4;

pub trait ClipSink: Send + Sync {
    fn write(&self, text: &str) -> bool;
    fn read(&self) -> Option<String>;
}

pub struct SystemClip;

impl ClipSink for SystemClip {
    fn write(&self, text: &str) -> bool {
        arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_owned())).is_ok()
    }

    fn read(&self) -> Option<String> {
        arboard::Clipboard::new().and_then(|mut c| c.get_text()).ok()
    }
}

pub struct ClipHub {
    tx: broadcast::Sender<Arc<str>>,
    sink: Mutex<Arc<dyn ClipSink>>,
}

impl Default for ClipHub {
    fn default() -> ClipHub {
        ClipHub { tx: broadcast::channel(QUEUE).0, sink: Mutex::new(Arc::new(SystemClip)) }
    }
}

pub fn mask_changes(text: &str) -> bool {
    crate::remote::push_content::mask_text(text) != text
}

impl ClipHub {
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<str>> {
        self.tx.subscribe()
    }

    pub fn listeners(&self) -> usize {
        self.tx.receiver_count()
    }

    pub fn set_sink(&self, sink: Arc<dyn ClipSink>) {
        *lock(&self.sink) = sink;
    }

    fn sink(&self) -> Arc<dyn ClipSink> {
        lock(&self.sink).clone()
    }

    pub fn receive(&self, text: &str) -> Result<(), &'static str> {
        if text.is_empty() {
            return Err("invalid");
        }
        if text.len() > MAX_CLIP {
            return Err("limit");
        }
        if mask_changes(text) {
            return Err("secret");
        }
        if self.sink().write(text) {
            Ok(())
        } else {
            Err("internal")
        }
    }

    pub fn send(&self) -> Result<usize, &'static str> {
        if self.listeners() == 0 {
            return Err("noSession");
        }
        let text = self.sink().read().filter(|t| !t.is_empty()).ok_or("empty")?;
        if text.len() > MAX_CLIP {
            return Err("limit");
        }
        if mask_changes(&text) {
            return Err("secret");
        }
        self.tx.send(Arc::from(text)).map_err(|_| "noSession")
    }
}

pub async fn next_clip(rx: &mut Option<broadcast::Receiver<Arc<str>>>) -> Option<Arc<str>> {
    let Some(r) = rx else { return std::future::pending().await };
    loop {
        match r.recv().await {
            Ok(text) => return Some(text),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return std::future::pending().await,
        }
    }
}

impl Phone {
    pub(in crate::remote) fn clip_attach(&mut self) {
        if self.x.clip.is_none() && self.caps.iter().any(|c| c == CAP) {
            self.x.clip = Some(self.ctx.board.clip.subscribe());
        }
    }

    pub(in crate::remote) async fn clip_push(&mut self, id: &str, v: &Value) -> bool {
        if !self.ctx.host.control() {
            return self.sec.json(&error(Value::from(id), "denied")).await;
        }
        let reply = match v.get("text").and_then(Value::as_str).map(|t| self.ctx.board.clip.receive(t)) {
            Some(Ok(())) => json!({ "re": id, "ok": true }),
            Some(Err(code)) => error(Value::from(id), code),
            None => error(Value::from(id), "invalid"),
        };
        self.sec.json(&reply).await
    }

    pub(in crate::remote) async fn on_clip(&mut self, text: Arc<str>) -> bool {
        self.sec.json(&json!({ "t": "clip", "text": &*text })).await
    }
}

impl crate::remote::Remote {
    pub fn clip_send(&self) -> Value {
        let Some(ctx) = lock(&self.ctx).clone() else { return json!({ "ok": false, "error": "off" }) };
        match ctx.board.clip.send() {
            Ok(sent) => json!({ "ok": true, "sent": sent }),
            Err(error) => json!({ "ok": false, "error": error }),
        }
    }
}

#[cfg(test)]
#[path = "clip_tests.rs"]
mod tests;
