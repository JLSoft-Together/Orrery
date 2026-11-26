use core_remote_proto::pair::{PairTicket, MAX_LAN};
use serde_json::{json, Value};

use super::config::now_secs;
use super::ctx::Ctx;
use super::pair::{self, PairMeta};
use super::Remote;
use crate::state::lock;

impl Ctx {
    pub fn start_pair(&self, meta: PairMeta) -> Result<Value, String> {
        let ticket = if meta.wants_relay() && self.relay_state() == "online" { Some(PairTicket::generate().map_err(|_| "rng")?) } else { None };
        let relay = ticket.is_some();
        let via = meta.via.name();
        let extra = meta.extra.clone();
        let (code, exp) = self.pairing.start_with(now_secs(), ticket.as_ref(), meta).ok_or("rng")?;
        let Some(qr) = pair::qr(self, code, exp, ticket, &extra) else {
            self.pairing.cancel();
            return Err("qr".into());
        };
        self.changed();
        let lan = (extra.len() + self.lan().len()).min(MAX_LAN);
        Ok(json!({ "qr": qr, "exp": exp, "relay": relay, "lan": lan, "via": via }))
    }
}

impl Remote {
    pub fn pair_start(&self) -> Result<Value, String> {
        self.pair_start_with(&Value::Null)
    }

    pub fn pair_start_with(&self, arg: &Value) -> Result<Value, String> {
        self.running_ctx()?.start_pair(PairMeta::from_web(arg))
    }

    pub fn pair_cancel(&self) -> Value {
        if let Some(c) = lock(&self.ctx).clone() {
            if c.pairing.cancel() {
                c.changed();
            }
        }
        json!(true)
    }
}
