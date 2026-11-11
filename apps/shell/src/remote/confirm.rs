use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

pub const CONFIRM_TTL: Duration = Duration::from_secs(60);
pub const MAX_CONFIRMS: usize = 4;
const TEXT_MAX: usize = 400;

struct Ask {
    cmd: Value,
    until: Instant,
}

#[derive(Default)]
pub struct Confirms {
    map: HashMap<String, Ask>,
}

fn clip(v: &Value) -> Value {
    match v.as_str() {
        Some(s) => Value::from(s.chars().take(TEXT_MAX).collect::<String>()),
        None => Value::from(""),
    }
}

pub fn ask_event(cid: &str, re: &str, needs: &Value) -> Value {
    json!({
        "t": "confirm.ask",
        "cid": cid,
        "re": re,
        "title": clip(&needs["title"]),
        "detail": clip(&needs["detail"]),
        "ok": clip(&needs["ok"]),
        "cancel": clip(&needs["cancel"]),
    })
}

pub fn command_of(v: &Value) -> Value {
    let mut cmd: Map<String, Value> = v.as_object().cloned().unwrap_or_default();
    cmd.remove("id");
    cmd.remove("confirmed");
    Value::Object(cmd)
}

impl Confirms {
    pub fn add(&mut self, cmd: Value, now: Instant) -> Option<String> {
        self.map.retain(|_, a| a.until > now);
        if self.map.len() >= MAX_CONFIRMS {
            let oldest = self.map.iter().min_by_key(|(_, a)| a.until).map(|(k, _)| k.clone());
            if let Some(k) = oldest {
                self.map.remove(&k);
            }
        }
        let cid = crate::config::try_random_hex(12).ok()?;
        self.map.insert(cid.clone(), Ask { cmd, until: now + CONFIRM_TTL });
        Some(cid)
    }

    pub fn take(&mut self, cid: &str, now: Instant) -> Option<Value> {
        let ask = self.map.remove(cid)?;
        (ask.until > now).then_some(ask.cmd)
    }
}
