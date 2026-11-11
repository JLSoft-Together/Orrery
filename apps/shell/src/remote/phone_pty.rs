use serde_json::Value;

use super::dashboard::{error, Phone};
use super::pty_bridge::{spawn, Attach, FromHub, Slot, Spec, ToHub};

pub const MAX_SESSION_ID: usize = 64;
pub const MAX_DIM: u64 = 1000;
pub const CLAIM_FLAG: u8 = 0b0000_0010;
const PTY_IN: u8 = 2;

fn dim(v: &Value, key: &str, fallback: u16) -> u16 {
    v.get(key).and_then(Value::as_u64).filter(|n| (1..=MAX_DIM).contains(n)).map_or(fallback, |n| n as u16)
}

pub fn stream_of(v: &Value) -> Option<u32> {
    v.get("stream").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0)
}

pub fn strip_claim(plain: &mut [u8]) -> bool {
    if plain.len() >= 2 && plain[0] == PTY_IN && plain[1] & CLAIM_FLAG != 0 {
        plain[1] &= !CLAIM_FLAG;
        return true;
    }
    false
}

impl Phone {
    pub(super) async fn pty_attach(&mut self, id: &str, v: &Value) -> bool {
        let Some(stream) = stream_of(v) else { return self.sec.json(&error(Value::from(id), "invalid")).await };
        let Some(session) = v.get("sessionId").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= MAX_SESSION_ID) else {
            return self.sec.json(&error(Value::from(id), "invalid")).await;
        };
        if self.x.attaches.contains_key(&stream) {
            return self.sec.json(&error(Value::from(id), "invalid")).await;
        }
        if !self.ctx.board.has_session(session) {
            return self.sec.json(&error(Value::from(id), "notFound")).await;
        }
        let Some(slot) = Slot::take(&self.ctx, &self.device) else { return self.sec.json(&error(Value::from(id), "limit")).await };
        self.x.next_gen += 1;
        let gen = self.x.next_gen;
        let spec = Spec { re: id.to_owned(), stream, session: session.to_owned(), cols: dim(v, "cols", 80), rows: dim(v, "rows", 24), gen, backlog: self.x.backlog.clone() };
        let tx = spawn(self.ctx.host.clone(), spec, slot, self.x.hub_tx.clone());
        self.x.attaches.insert(stream, Attach { tx, gen });
        true
    }

    pub(super) async fn pty_detach(&mut self, id: &str, v: &Value) -> bool {
        let gone = stream_of(v).and_then(|s| self.x.attaches.remove(&s)).is_some();
        let reply = if gone { serde_json::json!({ "re": id, "ok": true }) } else { error(Value::from(id), "notFound") };
        self.sec.json(&reply).await
    }

    pub(super) async fn pty_resize(&mut self, id: &str, v: &Value) -> bool {
        let Some(a) = stream_of(v).and_then(|s| self.x.attaches.get(&s)) else { return self.sec.json(&error(Value::from(id), "notFound")).await };
        let sent = a.tx.try_send(ToHub::Resize(dim(v, "cols", 80), dim(v, "rows", 24))).is_ok();
        let reply = if sent { serde_json::json!({ "re": id, "ok": true }) } else { error(Value::from(id), "busy") };
        self.sec.json(&reply).await
    }

    pub(super) async fn pty_input(&mut self, stream: u32, bytes: Vec<u8>, claim: bool) -> bool {
        let Some(a) = self.x.attaches.get(&stream) else { return true };
        if a.tx.try_send(ToHub::Input(bytes, claim)).is_ok() {
            return true;
        }
        self.x.attaches.remove(&stream);
        self.sec.json(&serde_json::json!({ "t": "pty.overflow", "stream": stream })).await
    }

    pub(super) async fn on_hub(&mut self, msg: FromHub) -> bool {
        let live = |x: &super::phone_ws::Extra, stream: u32, gen: u64| x.attaches.get(&stream).is_some_and(|a| a.gen == gen);
        match msg {
            FromHub::Json(stream, gen, v) => !live(&self.x, stream, gen) || self.sec.json(&v).await,
            FromHub::Out(stream, gen, bytes) => {
                let len = bytes.len();
                let mut ok = true;
                if live(&self.x, stream, gen) {
                    for part in bytes.chunks(core_remote_proto::frame::MAX_MESSAGE) {
                        if !self.sec.frame(core_remote_proto::frame::Kind::PtyOut, stream, part).await {
                            ok = false;
                            break;
                        }
                    }
                }
                self.x.backlog.fetch_sub(len, std::sync::atomic::Ordering::SeqCst);
                ok
            }
            FromHub::Ended(stream, gen, stalled) => {
                if !live(&self.x, stream, gen) {
                    return true;
                }
                self.x.attaches.remove(&stream);
                !stalled || self.sec.json(&serde_json::json!({ "t": "pty.overflow", "stream": stream })).await
            }
        }
    }
}
