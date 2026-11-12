use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core_remote_proto::relay::issue_token;
use core_remote_proto::{MIN_PROTO, PROTO};
use core_serve::StoreTap;
use serde_json::{json, Value};
use tokio::sync::{mpsc, watch, Notify};

use super::config::Config;
use super::devices::{Devices, EXPIRE_SECS};
use super::keys::Keys;
use super::limits::{Gate, MAX_SESSIONS};
use super::pair::Pairing;
use super::fcm::Fcm;
use super::push::{config_event, PushState};
use super::status::Shape;
use super::viewer_events::Hub;
use super::workspace::Board;
use crate::state::lock;

const UNKNOWN_NOTICE_GAP: Duration = Duration::from_secs(600);
pub const MAX_PAIR_TRIES: usize = 5;
pub const MAX_RELAY_PAIR_TRIES: usize = 20;
pub const RENEW_BEFORE: u64 = 7 * 86_400;
const PAIR_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    Revoked,
    Full,
}

pub struct Welcome {
    pub version: String,
    pub theme: Value,
    pub locale: String,
}

pub enum Asked {
    Reply(Value),
    Offline,
}

pub struct PtyEndpoint {
    pub url: String,
    pub protocol: String,
}

pub trait Host: Send + Sync + 'static {
    fn confirm_pair(&self, name: &str) -> bool;
    fn notify(&self, title: &str, body: &str);
    fn tap(&self) -> Option<StoreTap>;
    fn welcome(&self) -> Welcome;
    fn changed(&self, status: Value);
    fn awake(&self, on: bool);
    fn ask(&self, action: &str, args: Value) -> Asked;
    fn pty(&self) -> Option<PtyEndpoint>;
    fn viewer(&self, owner: &str, channel: &str, args: Value, accept: &[String]) -> Value;
    fn viewer_release(&self, owner: &str);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Revoked,
    Off,
    Restarting,
}

struct Live {
    device: String,
    via: &'static str,
    ctl: mpsc::Sender<Control>,
}

pub struct Ctx {
    pub host: Arc<dyn Host>,
    pub dir: PathBuf,
    pub keys: Keys,
    pub name: String,
    pub config: Mutex<Config>,
    pub devices: Mutex<Devices>,
    pub pairing: Pairing,
    pub gate: Arc<Gate>,
    pub relay_wake: Notify,
    pub running: AtomicBool,
    live: Mutex<HashMap<u64, Live>>,
    next: AtomicU64,
    relay_state: Mutex<&'static str>,
    lan: Mutex<Vec<SocketAddr>>,
    unknown_at: Mutex<Option<Instant>>,
    pair_tries: Mutex<HashMap<String, Vec<Instant>>>,
    pub board: Board,
    pub viewer: Hub,
    pub push: Mutex<PushState>,
    pub push_wake: Notify,
    pub push_cfg: watch::Sender<Value>,
    pub fcm: Fcm,
}

impl Ctx {
    pub fn new(host: Arc<dyn Host>, dir: PathBuf, keys: Keys, name: String) -> Ctx {
        let config = Config::load(&dir);
        let devices = Devices::load(&dir);
        let push_event = config_event(&config.firebase);
        Ctx {
            host,
            dir,
            keys,
            name,
            config: Mutex::new(config),
            devices: Mutex::new(devices),
            pairing: Pairing::default(),
            gate: Arc::new(Gate::default()),
            relay_wake: Notify::new(),
            running: AtomicBool::new(false),
            live: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            relay_state: Mutex::new("off"),
            lan: Mutex::new(Vec::new()),
            unknown_at: Mutex::new(None),
            pair_tries: Mutex::new(HashMap::new()),
            board: Board::default(),
            viewer: Hub::default(),
            push: Mutex::new(PushState::default()),
            push_wake: Notify::new(),
            push_cfg: watch::channel(push_event).0,
            fcm: Fcm::default(),
        }
    }

    pub fn is_live(&self, device: &str) -> bool {
        lock(&self.live).values().any(|l| l.device == device)
    }

    pub fn machine_id(&self) -> String {
        lock(&self.config).machine_id.clone()
    }

    pub fn admit_live(&self, device: &str, key: &[u8; 32], via: &'static str, ctl: mpsc::Sender<Control>, now: u64) -> Result<u64, Admit> {
        let devices = lock(&self.devices);
        if !devices.authorize(device, key, now) {
            return Err(Admit::Revoked);
        }
        let mut live = lock(&self.live);
        if live.len() >= MAX_SESSIONS {
            return Err(Admit::Full);
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        live.insert(id, Live { device: device.to_owned(), via, ctl });
        Ok(id)
    }

    pub fn take_pair_try(&self, key: String, max: usize, now: Instant) -> bool {
        let mut map = lock(&self.pair_tries);
        map.retain(|_, times| times.iter().any(|t| now.duration_since(*t) < PAIR_WINDOW));
        let times = map.entry(key).or_default();
        crate::state::take_slot(times, now, max, PAIR_WINDOW)
    }

    pub fn drop_live(&self, id: u64) {
        lock(&self.live).remove(&id);
    }

    pub fn live_count(&self) -> usize {
        lock(&self.live).len()
    }

    pub fn kick(&self, devices: &[String], ctl: Control) {
        for live in lock(&self.live).values().filter(|l| devices.contains(&l.device)) {
            let _ = live.ctl.try_send(ctl);
        }
    }

    pub fn kick_all(&self, ctl: Control) {
        for live in lock(&self.live).values() {
            let _ = live.ctl.try_send(ctl);
        }
    }

    pub fn set_lan(&self, addrs: Vec<SocketAddr>) {
        *lock(&self.lan) = addrs;
    }

    pub fn lan(&self) -> Vec<SocketAddr> {
        lock(&self.lan).clone()
    }

    pub fn set_relay_state(&self, state: &'static str) {
        let changed = {
            let mut cur = lock(&self.relay_state);
            let changed = *cur != state;
            *cur = state;
            changed
        };
        if changed {
            self.changed();
        }
    }

    pub fn relay_state(&self) -> &'static str {
        *lock(&self.relay_state)
    }

    pub fn changed(&self) {
        self.host.changed(self.status());
    }

    pub fn status(&self) -> Value {
        let running = self.running.load(Ordering::SeqCst);
        let cfg = lock(&self.config).clone();
        let online: Vec<(String, &'static str)> = lock(&self.live).values().map(|l| (l.device.clone(), l.via)).collect();
        let lan = self.lan();
        let shape = Shape {
            running,
            port: lan.first().map_or(cfg.port, SocketAddr::port),
            addresses: lan.iter().map(|a| a.ip().to_string()).collect(),
            relay: self.relay_state(),
            pairing: self.pairing.active(super::config::now_secs()),
            name: &self.name,
        };
        let mut out = shape.json(&cfg, lock(&self.devices).list(), &online);
        out["error"] = if running && lan.is_empty() { Value::from("lan") } else { Value::Null };
        out
    }

    pub fn publish(&self, summary: Value) {
        self.board.publish(summary);
    }

    pub fn summary(&self) -> Option<Value> {
        self.board.summary()
    }

    pub fn notify_unknown(&self, now: Instant) {
        {
            let mut at = lock(&self.unknown_at);
            if at.is_some_and(|t| now.duration_since(t) < UNKNOWN_NOTICE_GAP) {
                return;
            }
            *at = Some(now);
        }
        let t = super::text::remote_text(&self.host.welcome().locale);
        self.host.notify(t.unknown_title, t.unknown_body);
    }

    pub fn relay_token(&self, device: &str, now: u64) -> Option<Value> {
        let (url, token, exp) = {
            let cfg = lock(&self.config);
            if !cfg.relay.registered || cfg.relay.url.is_empty() {
                return None;
            }
            let exp = now + EXPIRE_SECS;
            let token = issue_token(&self.keys.relay, &cfg.machine_id, device, exp).ok()?;
            (cfg.relay.url.clone(), token, exp)
        };
        lock(&self.devices).set_token_exp(device, exp);
        Some(json!({ "relay": url, "relayToken": token, "relayExp": exp }))
    }

    pub fn renew_token(&self, device: &str, now: u64) -> Option<Value> {
        let exp = lock(&self.devices).find(device)?.token_exp;
        if exp > now + RENEW_BEFORE {
            return None;
        }
        let fresh = self.relay_token(device, now)?;
        Some(json!({ "t": "relay.token", "token": fresh["relayToken"], "exp": fresh["relayExp"], "relay": fresh["relay"] }))
    }

    pub fn welcome(&self, re: &str) -> Value {
        let w = self.host.welcome();
        let mut out = json!({
            "re": re,
            "ok": true,
            "machine": self.machine_id(),
            "name": self.name,
            "version": w.version,
            "proto": PROTO,
            "minProto": MIN_PROTO,
            "theme": w.theme,
            "locale": w.locale,
        });
        let cfg = lock(&self.config);
        if cfg.relay.registered && !cfg.relay.url.is_empty() {
            out["relay"] = Value::from(cfg.relay.url.clone());
        }
        out
    }

    pub fn revoke(&self, ids: &[String]) -> usize {
        let gone = {
            let mut devices = lock(&self.devices);
            let gone = devices.remove(ids);
            self.kick(&gone, Control::Revoked);
            gone
        };
        if gone.is_empty() {
            return 0;
        }
        {
            let mut cfg = lock(&self.config);
            if cfg.relay.registered {
                for id in &gone {
                    if !cfg.relay.pending_revoke.contains(id) {
                        cfg.relay.pending_revoke.push(id.clone());
                    }
                }
                cfg.save(&self.dir);
            }
        }
        self.relay_wake.notify_one();
        self.changed();
        gone.len()
    }

    pub fn sweep(&self, now: u64) -> usize {
        let expired = lock(&self.devices).expired(now);
        self.revoke(&expired)
    }
}
