use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::ctx::{Control, Ctx};
use super::Remote;
use crate::state::lock;

pub const MAX_GRACE_SECS: u64 = 30;
pub const MAX_ID: usize = 128;

struct Held {
    serial: u64,
    until: Instant,
}

#[derive(Default)]
pub struct Hold {
    held: Mutex<HashMap<String, Held>>,
    serial: AtomicU64,
}

impl Hold {
    pub fn hold(&self, id: &str, grace: Duration, now: Instant) -> u64 {
        let serial = self.serial.fetch_add(1, Ordering::SeqCst) + 1;
        lock(&self.held).insert(id.to_owned(), Held { serial, until: now + grace });
        serial
    }

    pub fn is_held(&self, id: &str) -> bool {
        lock(&self.held).contains_key(id)
    }

    pub fn undo(&self, id: &str, now: Instant) -> bool {
        let mut held = lock(&self.held);
        let ok = held.get(id).is_some_and(|h| now < h.until);
        if ok {
            held.remove(id);
        }
        ok
    }

    pub fn due(&self, id: &str, serial: u64) -> bool {
        let mut held = lock(&self.held);
        let ok = held.get(id).is_some_and(|h| h.serial == serial);
        if ok {
            held.remove(id);
        }
        ok
    }

    pub fn forget(&self, id: &str) {
        lock(&self.held).remove(id);
    }

    pub fn clear(&self, ids: &[String]) {
        let mut held = lock(&self.held);
        for id in ids {
            held.remove(id);
        }
    }

    pub fn drain(&self) -> Vec<String> {
        lock(&self.held).drain().map(|(id, _)| id).collect()
    }

    pub fn ids(&self) -> Vec<String> {
        lock(&self.held).keys().cloned().collect()
    }
}

fn target(arg: &Value) -> Option<&str> {
    let id = match arg {
        Value::String(s) => Some(s.as_str()),
        Value::Object(_) => arg.get("id").and_then(Value::as_str),
        _ => None,
    };
    id.filter(|s| !s.is_empty() && s.len() <= MAX_ID)
}

impl Ctx {
    pub fn revoke_later(self: &Arc<Self>, id: &str, grace: Duration) -> bool {
        if lock(&self.devices).find(id).is_none() {
            return false;
        }
        let serial = self.hold.hold(id, grace, Instant::now());
        self.kick(&[id.to_owned()], Control::Held);
        self.changed();
        let ctx = self.clone();
        let key = id.to_owned();
        let spawned = std::thread::Builder::new().name("remote-revoke".into()).spawn(move || {
            std::thread::sleep(grace);
            if ctx.hold.due(&key, serial) {
                ctx.revoke(&[key]);
            }
        });
        if spawned.is_err() {
            self.revoke(&[id.to_owned()]);
        }
        true
    }

    pub fn undo_revoke(&self, id: &str) -> bool {
        let ok = self.hold.undo(id, Instant::now());
        if ok {
            self.changed();
        }
        ok
    }

    pub fn flush_revokes(&self) -> usize {
        let ids = self.hold.drain();
        if ids.is_empty() {
            return 0;
        }
        self.revoke(&ids)
    }
}

impl Remote {
    pub fn revoke(&self, ids: &[String]) -> Result<Value, String> {
        let ctx = self.ctx().map_err(str::to_owned)?;
        ctx.revoke(ids);
        Ok(json!(true))
    }

    pub fn revoke_call(&self, arg: &Value) -> Value {
        let Some(id) = target(arg) else { return json!(false) };
        let grace = arg.get("grace").and_then(Value::as_u64).unwrap_or(0).min(MAX_GRACE_SECS);
        if grace == 0 {
            return self.revoke(&[id.to_owned()]).unwrap_or(json!(false));
        }
        let Ok(ctx) = self.ctx() else { return json!(false) };
        json!(ctx.revoke_later(id, Duration::from_secs(grace)))
    }

    pub fn revoke_undo(&self, arg: &Value) -> Value {
        let Some(id) = target(arg) else { return json!(false) };
        json!(lock(&self.ctx).clone().is_some_and(|c| c.undo_revoke(id)))
    }

    pub fn revoke_all(&self) -> Result<Value, String> {
        let ctx = self.ctx().map_err(str::to_owned)?;
        let ids = lock(&ctx.devices).ids();
        ctx.revoke(&ids);
        Ok(json!(true))
    }

    pub fn rename_device(&self, arg: &Value) -> Result<Value, String> {
        let id = arg.get("deviceId").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= MAX_ID).ok_or("device")?;
        let name = arg.get("name").and_then(Value::as_str).ok_or("name")?;
        let ctx = self.ctx().map_err(str::to_owned)?;
        let shown = lock(&ctx.devices).rename(id, name);
        if shown.is_none() {
            return Ok(json!(false));
        }
        ctx.changed();
        Ok(json!(true))
    }

    pub(super) fn flush_revokes(&self) {
        if let Some(c) = lock(&self.ctx).clone() {
            c.flush_revokes();
        }
    }
}
