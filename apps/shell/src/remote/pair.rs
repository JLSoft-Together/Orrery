use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_remote_proto::pair::{PairCode, PairResponder, QrPayload};
use core_remote_proto::Error;
use serde_json::{json, Value};
use tokio::time::{timeout, timeout_at, Instant};

use super::config::now_secs;
use super::ctx::Ctx;
use super::devices::Device;
use super::link::{Link, Out, Secure, CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_NORMAL, CLOSE_PROTO};
use crate::state::lock;

pub const PAIR_TTL: u64 = 300;
pub const MAX_NAME: usize = 40;
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(120);
const PAIR_MSG1_LEN: usize = 2 + 32 + 16;

struct Slot {
    code: PairCode,
    exp: u64,
    serial: u64,
}

#[derive(Default)]
pub struct Pairing {
    slot: Mutex<Option<Slot>>,
    serial: AtomicU64,
}

pub struct Ticket {
    code: PairCode,
    serial: u64,
}

impl Pairing {
    pub fn start(&self, now: u64) -> Option<(PairCode, u64)> {
        let code = PairCode::generate().ok()?;
        let exp = now + PAIR_TTL;
        let serial = self.serial.fetch_add(1, Ordering::SeqCst) + 1;
        *lock(&self.slot) = Some(Slot { code: code.clone(), exp, serial });
        Some((code, exp))
    }

    pub fn cancel(&self) -> bool {
        lock(&self.slot).take().is_some()
    }

    pub fn active(&self, now: u64) -> Option<u64> {
        lock(&self.slot).as_ref().filter(|s| now < s.exp).map(|s| s.exp)
    }

    pub fn ticket(&self, now: u64) -> Option<Ticket> {
        let mut slot = lock(&self.slot);
        if slot.as_ref().is_some_and(|s| now >= s.exp) {
            *slot = None;
        }
        slot.as_ref().map(|s| Ticket { code: s.code.clone(), serial: s.serial })
    }

    pub fn consume(&self, ticket: &Ticket) -> bool {
        let mut slot = lock(&self.slot);
        if slot.as_ref().is_some_and(|s| s.serial == ticket.serial) {
            *slot = None;
            return true;
        }
        false
    }
}

pub fn is_hidden(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

pub fn clean_name(raw: &str) -> String {
    let kept: String = raw.chars().filter(|c| !c.is_control() && !is_hidden(*c)).collect();
    let name: String = kept.trim().chars().take(MAX_NAME).collect();
    if name.trim().is_empty() {
        "Phone".to_owned()
    } else {
        name.trim().to_owned()
    }
}

pub fn is_pair_msg1(msg: &[u8]) -> bool {
    msg.len() == PAIR_MSG1_LEN
}

pub fn qr(ctx: &Ctx, code: PairCode, exp: u64) -> Option<String> {
    let cfg = lock(&ctx.config).clone();
    let lan = ctx.lan();
    let first = lan.first();
    let payload = QrPayload {
        machine: cfg.machine_id,
        name: ctx.name.clone(),
        host: first.map(|a| a.ip().to_string()),
        port: first.map(|a| a.port()),
        relay: (!cfg.relay.url.is_empty()).then_some(cfg.relay.url),
        mkey: *ctx.keys.noise.public(),
        code,
        exp,
    };
    payload.encode().ok()
}

fn merge(base: &mut Value, extra: Option<Value>) {
    if let (Some(obj), Some(Value::Object(more))) = (base.as_object_mut(), extra) {
        obj.extend(more);
    }
}

pub async fn run(ctx: Arc<Ctx>, mut link: Link, msg1: Vec<u8>, deadline: Instant) {
    let Some(ticket) = ctx.pairing.ticket(now_secs()) else {
        link.out.close(CLOSE_DEVICE, "pairing").await;
        return;
    };
    let machine = ctx.machine_id();
    let (responder, msg2) = match PairResponder::start(&ctx.keys.noise, &ticket.code, &machine, &msg1) {
        Ok(v) => v,
        Err(Error::ProtoTooOld { .. }) => return link.out.close(CLOSE_PROTO, "old").await,
        Err(Error::ProtoTooNew { .. }) => return link.out.close(CLOSE_PROTO, "new").await,
        Err(_) => return link.out.close(CLOSE_HANDSHAKE, "handshake").await,
    };
    if !link.out.send(Out::Msg(msg2)).await {
        return;
    }
    let Ok(Some(msg3)) = timeout_at(deadline, link.rx.recv()).await else {
        return link.out.close(CLOSE_HANDSHAKE, "timeout").await;
    };
    let paired = match responder.finish(&msg3) {
        Ok(p) => p,
        Err(_) => return link.out.close(CLOSE_HANDSHAKE, "handshake").await,
    };
    if !ctx.pairing.consume(&ticket) {
        return link.out.close(CLOSE_DEVICE, "pairing").await;
    }
    ctx.changed();
    let mut sec = Secure { transport: paired.transport, out: link.out.clone() };
    if lock(&ctx.devices).is_full(&paired.device_id) {
        sec.json(&json!({ "t": "pair.denied", "error": "limit" })).await;
        return sec.close(CLOSE_NORMAL, "denied").await;
    }
    let host = ctx.host.clone();
    let name = clean_name(&paired.name);
    let shown = name.clone();
    let asked = timeout(CONFIRM_TIMEOUT, tokio::task::spawn_blocking(move || host.confirm_pair(&shown))).await;
    if !matches!(asked, Ok(Ok(true))) {
        sec.json(&json!({ "t": "pair.denied" })).await;
        return sec.close(CLOSE_NORMAL, "denied").await;
    }
    let now = now_secs();
    let added = lock(&ctx.devices).add(Device::new(&paired.device_id, &name, &paired.device_static, now));
    if !added {
        sec.json(&json!({ "t": "pair.denied", "error": "internal" })).await;
        return sec.close(CLOSE_NORMAL, "denied").await;
    }
    let mut reply = json!({ "t": "pair.ok", "machine": machine, "name": ctx.name });
    merge(&mut reply, ctx.relay_token(&paired.device_id, now));
    sec.json(&reply).await;
    sec.close(CLOSE_NORMAL, "paired").await;
    ctx.changed();
}
