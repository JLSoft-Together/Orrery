use serde_json::{json, Value};

use super::config::now_secs;
use super::Remote;

const MAX_TEXT: usize = 64;

fn text(arg: &Value, key: &str) -> Option<String> {
    arg.get(key).and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= MAX_TEXT).map(str::to_owned)
}

impl Remote {
    pub fn guard_call(&self, arg: &Value) -> Value {
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return json!({ "ok": false, "error": e }),
        };
        let now = now_secs();
        let done = match arg.get("op").and_then(Value::as_str).unwrap_or("get") {
            "get" => Ok(false),
            "unblock" => match text(arg, "key") {
                Some(k) => Ok(ctx.guard.unblock(Some(&k))),
                None => Err("invalid"),
            },
            "unblock-all" => Ok(ctx.guard.unblock(None)),
            "block" => {
                let ttl = match arg.get("ttl") {
                    None | Some(Value::Null) => Ok(None),
                    Some(v) => v.as_u64().map(Some).ok_or("invalid"),
                };
                match (text(arg, "cidr"), ttl) {
                    (Some(c), Ok(ttl)) => ctx.guard.block(&c, ttl, now).map(|()| true),
                    _ => Err("invalid"),
                }
            }
            "trust" => {
                let list: Option<Vec<String>> = arg.get("list").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect());
                match list {
                    Some(l) if l.iter().all(|s| s.len() <= MAX_TEXT) => ctx.guard.set_trust(&l).map(|()| true),
                    _ => Err("invalid"),
                }
            }
            _ => Err("invalid"),
        };
        if matches!(done, Ok(true)) {
            ctx.changed();
        }
        match done {
            Ok(_) => json!({ "ok": true, "state": ctx.guard.json(now) }),
            Err(e) => json!({ "ok": false, "error": e, "state": ctx.guard.json(now) }),
        }
    }
}
