use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::confirm::{ask_event, command_of, ASK_COMMAND};
use super::ctx::Ctx;
use super::dashboard::{error, Phone};
use super::phone_ws::{Done, MAX_IN_FLIGHT};
use super::viewer_events::{merge, VEvent, COALESCE, QUEUE};
use super::viewer_map::{build_args, lookup, outcome, pre_needs, pre_of, watch_key, Spec, Watch, SLOW};

pub const MAX_WATCHES: usize = 8;
pub const MAX_REPLY: usize = 900_000;

pub async fn flush_due(at: Option<tokio::time::Instant>) {
    match at {
        Some(t) => tokio::time::sleep_until(t).await,
        None => std::future::pending().await,
    }
}

struct Busy {
    ctx: Arc<Ctx>,
    key: String,
}

impl Busy {
    fn take(ctx: &Arc<Ctx>, key: String) -> Option<Busy> {
        ctx.viewer.take_busy(&key).then(|| Busy { ctx: ctx.clone(), key })
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.ctx.viewer.free_busy(&self.key);
    }
}

struct Guard {
    ctx: Arc<Ctx>,
    owner: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.ctx.viewer.remove(&self.owner);
        self.ctx.host.viewer_release(&self.owner);
    }
}

pub struct ViewerState {
    pub(super) owner: String,
    tx: mpsc::Sender<VEvent>,
    pub(super) rx: mpsc::Receiver<VEvent>,
    watched: HashSet<String>,
    pending: HashSet<String>,
    guard: Option<Guard>,
    batch: Vec<VEvent>,
    pub(super) flush_at: Option<tokio::time::Instant>,
}

impl ViewerState {
    pub fn new(device: &str, nonce: &str) -> ViewerState {
        let (tx, rx) = mpsc::channel(QUEUE);
        ViewerState { owner: format!("remote:{device}:{nonce}"), tx, rx, watched: HashSet::new(), pending: HashSet::new(), guard: None, batch: Vec::new(), flush_at: None }
    }
}

pub fn accepted(cmd: &Value) -> Vec<String> {
    cmd.get("_accept").and_then(Value::as_array).map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

pub fn with_accept(mut cmd: Value, needs: &Value) -> Value {
    let Some(key) = needs.get("key").and_then(Value::as_str) else { return cmd };
    let mut keys = accepted(&cmd);
    keys.push(key.to_owned());
    cmd["_accept"] = json!(keys);
    cmd
}

pub fn fits(reply: &Value) -> bool {
    serde_json::to_vec(reply).is_ok_and(|b| b.len() <= MAX_REPLY)
}

impl Phone {
    pub(super) async fn viewer_command(&mut self, id: &str, spec: &'static Spec, v: &Value) -> bool {
        let mut cmd = command_of(v);
        if let Some(o) = cmd.as_object_mut() {
            o.remove("_accept");
        }
        let now = Instant::now();
        if !self.ctx.viewer.take_rate(&self.device, now) || (spec.write && !self.ctx.viewer.take_write(&self.device, now)) {
            return self.sec.json(&error(Value::from(id), "rate")).await;
        }
        self.viewer_call(id, spec, cmd, false).await
    }

    pub(super) async fn viewer_call(&mut self, id: &str, spec: &'static Spec, cmd: Value, confirmed: bool) -> bool {
        if self.x.in_flight >= MAX_IN_FLIGHT {
            return self.sec.json(&error(Value::from(id), "busy")).await;
        }
        let Some(args) = build_args(spec, &cmd) else { return self.sec.json(&error(Value::from(id), "invalid")).await };
        let key = watch_key(spec, &cmd);
        let mut reserved = None;
        match (spec.watch, &key) {
            (Watch::FsAdd | Watch::GitAdd, Some(k)) if self.x.v.watched.contains(k) => return self.sec.json(&json!({ "re": id, "ok": true, "data": true })).await,
            (Watch::FsAdd | Watch::GitAdd, Some(k)) => {
                if self.x.v.watched.len() + self.x.v.pending.len() >= MAX_WATCHES || self.x.v.pending.contains(k) {
                    return self.sec.json(&error(Value::from(id), "limit")).await;
                }
                self.x.v.pending.insert(k.clone());
                reserved = Some(k.clone());
            }
            (Watch::No, _) => {}
            (_, None) => return self.sec.json(&error(Value::from(id), "invalid")).await,
            _ => {}
        }
        self.x.in_flight += 1;
        let (tx, action) = (self.x.done_tx.clone(), spec.t.to_owned());
        let id = id.to_owned();
        if let (Some(pre), false) = (pre_of(spec, &cmd), confirmed) {
            let needs = pre_needs(&self.ctx.host.welcome().locale, pre, &cmd);
            let _ = tx.send(Done::new(id, action, cmd, false, Ok(json!({ "needsConfirm": needs })))).await;
            return true;
        }
        let busy = if spec.secs == SLOW {
            let root = cmd.get("root").and_then(Value::as_str).unwrap_or("");
            match Busy::take(&self.ctx, format!("{}|{root}", self.device)) {
                Some(b) => Some(b),
                None => {
                    self.x.in_flight -= 1;
                    if let Some(k) = reserved {
                        self.x.v.pending.remove(&k);
                    }
                    return self.sec.json(&error(Value::from(id), "busy")).await;
                }
            }
        } else {
            None
        };
        self.viewer_guard();
        if spec.run {
            self.ctx.viewer.want_run(&self.x.v.owner);
        }
        let ctx = self.ctx.clone();
        let (owner, channel, accept) = (self.x.v.owner.clone(), spec.channel, accepted(&cmd));
        let adds = matches!(spec.watch, Watch::FsAdd | Watch::GitAdd);
        let wait = Duration::from_secs(spec.secs);
        tokio::spawn(async move {
            let work = tokio::task::spawn_blocking(move || {
                let _busy = busy;
                let env = ctx.host.viewer(&owner, channel, args, &accept);
                if adds && !ctx.viewer.is_open(&owner) {
                    ctx.host.viewer_release(&owner);
                }
                env
            });
            let result = match tokio::time::timeout(wait, work).await {
                Ok(Ok(env)) => Ok(outcome(&env)),
                Ok(Err(_)) => Err("offline"),
                Err(_) => Err("timeout"),
            };
            let _ = tx.send(Done::new(id, action, cmd, confirmed, result)).await;
        });
        true
    }

    pub(super) fn viewer_settle(&mut self, action: &str, cmd: &Value) {
        if let Some(k) = lookup(action).and_then(|s| watch_key(s, cmd)) {
            self.x.v.pending.remove(&k);
        }
    }

    fn viewer_guard(&mut self) {
        if self.x.v.guard.is_none() {
            self.ctx.viewer.register(&self.x.v.owner, self.x.v.tx.clone());
            self.x.v.guard = Some(Guard { ctx: self.ctx.clone(), owner: self.x.v.owner.clone() });
        }
    }

    pub(super) fn viewer_after(&mut self, action: &str, cmd: &Value, v: &Value) {
        let Some(spec) = lookup(action) else { return };
        if v.get("ok") != Some(&Value::Bool(true)) || v.get("data") == Some(&Value::Bool(false)) {
            return;
        }
        let Some(key) = watch_key(spec, cmd) else { return };
        match spec.watch {
            Watch::FsAdd | Watch::GitAdd => drop(self.x.v.watched.insert(key)),
            Watch::FsDrop | Watch::GitDrop => drop(self.x.v.watched.remove(&key)),
            Watch::No => {}
        }
    }

    pub(super) fn on_vevent(&mut self, event: VEvent) {
        let v = &mut self.x.v;
        v.batch.push(event);
        let now = tokio::time::Instant::now();
        if v.batch.len() >= QUEUE {
            v.flush_at = Some(now);
        } else if matches!(v.batch.last(), Some(VEvent::Ask { .. })) {
            v.flush_at = Some(now);
        } else {
            v.flush_at.get_or_insert(now + COALESCE);
        }
    }

    async fn on_ask(&mut self, aid: &str, needs: &Value) -> bool {
        let cmd = json!({ "t": ASK_COMMAND, "aid": aid });
        let Some(cid) = self.x.confirms.add(cmd, Instant::now()) else {
            self.ctx.viewer.answer(aid, false);
            return true;
        };
        self.sec.json(&ask_event(&cid, ASK_COMMAND, needs)).await
    }

    pub(super) async fn flush_vevents(&mut self) -> bool {
        self.x.v.flush_at = None;
        let (asks, batch): (Vec<VEvent>, Vec<VEvent>) = std::mem::take(&mut self.x.v.batch).into_iter().partition(|e| matches!(e, VEvent::Ask { .. }));
        for ask in asks {
            let VEvent::Ask { aid, needs } = ask else { continue };
            if !self.on_ask(&aid, &needs).await {
                return false;
            }
        }
        for msg in merge(batch) {
            if !self.sec.json(&msg).await {
                return false;
            }
        }
        true
    }
}
