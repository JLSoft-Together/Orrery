pub mod awake;
mod config;
mod confirm;
mod ctx;
mod dashboard;
mod devices;
pub mod host;
mod keys;
mod lan;
mod limits;
mod link;
mod mdns;
mod pair;
pub mod pane_link;
mod patch;
mod phone_pty;
mod phone_ws;
pub mod pty_bridge;
mod rebind;
pub mod scope;
mod relay;
mod session;
mod status;
mod text;
mod viewer;
mod viewer_events;
mod viewer_map;
mod workspace;

#[cfg(test)]
mod test_util;
#[cfg(test)]
mod pair_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod relay_tests;
#[cfg(test)]
mod unit_tests;
#[cfg(test)]
mod ws_tests;
#[cfg(test)]
mod viewer_tests;
#[cfg(test)]
mod viewer_guard_tests;
#[cfg(test)]
mod pty_tests;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tokio::sync::watch;

pub use ctx::{Asked, Control, Host, PtyEndpoint, Welcome};
pub use viewer_events::VEvent;

use config::{now_secs, port_override, Config};
use ctx::Ctx;
use devices::Devices;
use keys::Keys;
use crate::state::lock;
use keys::KeyStore;

pub const CHANGED: &str = "ccm:remote-changed";
pub const MAX_SUMMARY: usize = 1 << 20;
pub const CHANNELS: [&str; 8] = [
    "ccm:remote-publish",
    "ccm:remote-status",
    "ccm:remote-enable",
    "ccm:remote-pair-start",
    "ccm:remote-pair-cancel",
    "ccm:remote-revoke",
    "ccm:remote-revoke-all",
    "ccm:remote-keep-awake",
];
const SWEEP_EVERY: Duration = Duration::from_secs(3600);
const DRAIN: Duration = Duration::from_millis(400);

#[derive(Default, Clone)]
pub struct Options {
    pub addrs: Option<Vec<Ipv4Addr>>,
    pub port: Option<u16>,
    pub mdns: bool,
    pub name: Option<String>,
}

struct Running {
    rt: Runtime,
    stop: watch::Sender<bool>,
}

pub struct Remote {
    host: Arc<dyn Host>,
    secrets: Box<dyn KeyStore>,
    dir: PathBuf,
    opts: Options,
    ctx: Mutex<Option<Arc<Ctx>>>,
    running: Mutex<Option<Running>>,
    error: Mutex<Option<&'static str>>,
}

async fn sweep(ctx: Arc<Ctx>, mut stop: watch::Receiver<bool>) {
    loop {
        ctx.sweep(now_secs());
        tokio::select! {
            _ = tokio::time::sleep(SWEEP_EVERY) => {}
            _ = stop.changed() => break,
        }
    }
}

impl Remote {
    pub fn new(host: Arc<dyn Host>, secrets: Box<dyn KeyStore>, dir: PathBuf, opts: Options) -> Remote {
        Remote { host, secrets, dir, opts, ctx: Mutex::new(None), running: Mutex::new(None), error: Mutex::new(None) }
    }

    fn ctx(&self) -> Result<Arc<Ctx>, &'static str> {
        let mut slot = lock(&self.ctx);
        if let Some(c) = slot.as_ref() {
            return Ok(c.clone());
        }
        let keys = Keys::load_or_create(self.secrets.as_ref()).ok_or("keyring")?;
        let name = self.opts.name.clone().unwrap_or_else(config::machine_name);
        let c = Arc::new(Ctx::new(self.host.clone(), self.dir.clone(), keys, name));
        *slot = Some(c.clone());
        Ok(c)
    }

    pub fn wants_on(&self) -> bool {
        std::fs::read_to_string(Config::path(&self.dir)).ok().and_then(|t| serde_json::from_str::<Config>(&t).ok()).is_some_and(|c| c.enabled)
    }

    pub fn status(&self) -> Value {
        let mut out = match lock(&self.ctx).clone() {
            Some(c) => c.status(),
            None => {
                let cfg = Config::load(&self.dir);
                let name = self.opts.name.clone().unwrap_or_else(config::machine_name);
                let shape = status::Shape { running: false, port: cfg.port, addresses: Vec::new(), relay: "off", pairing: None, name: &name };
                shape.json(&cfg, Devices::load(&self.dir).list(), &[])
            }
        };
        if let Some(e) = *lock(&self.error) {
            out["error"] = Value::from(e);
        } else if out.get("error").is_none() {
            out["error"] = Value::Null;
        }
        out
    }

    #[cfg(test)]
    pub fn bound(&self) -> Vec<SocketAddr> {
        lock(&self.ctx).as_ref().map(|c| c.lan()).unwrap_or_default()
    }

    fn set_error(&self, e: Option<&'static str>) {
        *lock(&self.error) = e;
    }

    pub fn enable(&self, on: bool) -> Value {
        if on {
            self.start();
        } else {
            self.stop(Control::Off);
            match lock(&self.ctx).clone() {
                Some(c) => {
                    let mut cfg = lock(&c.config);
                    cfg.enabled = false;
                    cfg.save(&self.dir);
                }
                None => {
                    let mut cfg = Config::load(&self.dir);
                    cfg.enabled = false;
                    cfg.save(&self.dir);
                }
            }
        }
        let status = self.status();
        self.host.changed(status.clone());
        status
    }

    fn start(&self) {
        let mut running = lock(&self.running);
        if running.is_some() {
            return;
        }
        let ctx = match self.ctx() {
            Ok(c) => c,
            Err(e) => return self.set_error(Some(e)),
        };
        let Ok(rt) = tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("remote").enable_all().build() else {
            return self.set_error(Some("runtime"));
        };
        let cfg = lock(&ctx.config).clone();
        let port = self.opts.port.or_else(port_override).unwrap_or(cfg.port);
        let addrs = self.opts.addrs.clone().unwrap_or_else(lan::lan_addrs);
        let listeners = lan::bind(&addrs, port);
        let bound: Vec<SocketAddr> = listeners.iter().filter_map(|l| l.local_addr().ok()).collect();
        self.set_error(None);
        ctx.set_lan(bound);
        let (stop, stop_rx) = watch::channel(false);
        let plan = rebind::Plan { port, dynamic: self.opts.addrs.is_none(), mdns: self.opts.mdns };
        rt.spawn(rebind::supervise(ctx.clone(), listeners, plan, stop_rx.clone()));
        if !cfg.relay.url.is_empty() {
            rt.spawn(relay::run(ctx.clone(), stop_rx.clone()));
        }
        rt.spawn(sweep(ctx.clone(), stop_rx));
        if cfg.keep_awake {
            self.host.awake(true);
        }
        ctx.running.store(true, Ordering::SeqCst);
        {
            let mut c = lock(&ctx.config);
            c.enabled = true;
            c.save(&self.dir);
        }
        *running = Some(Running { rt, stop });
    }

    fn stop(&self, why: Control) {
        let Some(run) = lock(&self.running).take() else { return };
        let ctx = lock(&self.ctx).clone();
        if let Some(c) = &ctx {
            c.kick_all(why);
            let deadline = std::time::Instant::now() + DRAIN;
            while c.live_count() > 0 && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            c.pairing.cancel();
        }
        let _ = run.stop.send(true);
        run.rt.shutdown_timeout(Duration::from_millis(500));
        if let Some(c) = &ctx {
            c.running.store(false, Ordering::SeqCst);
            c.set_lan(Vec::new());
            c.set_relay_state("off");
            if lock(&c.config).keep_awake {
                self.host.awake(false);
            }
        }
    }

    pub fn shutdown(&self) {
        self.stop(Control::Restarting);
    }

    fn running_ctx(&self) -> Result<Arc<Ctx>, String> {
        if lock(&self.running).is_none() {
            return Err("off".into());
        }
        self.ctx().map_err(str::to_owned)
    }

    pub fn pair_start(&self) -> Result<Value, String> {
        let ctx = self.running_ctx()?;
        let (code, exp) = ctx.pairing.start(now_secs()).ok_or("rng")?;
        let qr = pair::qr(&ctx, code, exp).ok_or("qr")?;
        ctx.changed();
        Ok(json!({ "qr": qr, "exp": exp }))
    }

    pub fn pair_cancel(&self) -> Value {
        if let Some(c) = lock(&self.ctx).clone() {
            if c.pairing.cancel() {
                c.changed();
            }
        }
        json!(true)
    }

    pub fn revoke(&self, ids: &[String]) -> Result<Value, String> {
        let ctx = self.ctx().map_err(str::to_owned)?;
        ctx.revoke(ids);
        Ok(json!(true))
    }

    pub fn revoke_all(&self) -> Result<Value, String> {
        let ctx = self.ctx().map_err(str::to_owned)?;
        let ids = lock(&ctx.devices).ids();
        ctx.revoke(&ids);
        Ok(json!(true))
    }

    pub fn publish(&self, summary: &Value) -> bool {
        if !summary.is_object() || serde_json::to_vec(summary).map_or(true, |b| b.len() > MAX_SUMMARY) {
            return false;
        }
        match lock(&self.ctx).clone() {
            Some(c) => {
                c.publish(summary.clone());
                true
            }
            None => false,
        }
    }

    pub fn viewer_event(&self, owner: &str, event: VEvent) {
        if let Some(c) = lock(&self.ctx).clone() {
            c.viewer.push(owner, event);
        }
    }

    pub fn ask_device(&self, owner: &str, needs: &Value) -> bool {
        let ctx = lock(&self.ctx).clone();
        ctx.is_some_and(|c| c.viewer.ask(owner, needs, confirm::CONFIRM_TTL))
    }

    pub fn run_event(&self, payload: &Value) {
        if let Some(c) = lock(&self.ctx).clone() {
            c.viewer.push_run(payload);
        }
    }

    pub fn keep_awake(&self, on: bool) -> Value {
        if let Some(c) = lock(&self.ctx).clone() {
            let changed = {
                let mut cfg = lock(&c.config);
                let changed = cfg.keep_awake != on;
                cfg.keep_awake = on;
                cfg.save(&self.dir);
                changed
            };
            if changed && lock(&self.running).is_some() {
                self.host.awake(on);
            }
        } else {
            let mut cfg = Config::load(&self.dir);
            cfg.keep_awake = on;
            cfg.save(&self.dir);
        }
        let status = self.status();
        self.host.changed(status.clone());
        status
    }

    pub fn call(&self, channel: &str, args: &[Value]) -> Result<Value, String> {
        let arg = args.first().unwrap_or(&Value::Null);
        match channel {
            "ccm:remote-status" => Ok(self.status()),
            "ccm:remote-enable" => Ok(self.enable(arg.as_bool().unwrap_or(false))),
            "ccm:remote-pair-start" => Ok(self.pair_start().unwrap_or(Value::Null)),
            "ccm:remote-pair-cancel" => Ok(self.pair_cancel()),
            "ccm:remote-revoke" => {
                let Some(id) = arg.as_str() else { return Ok(json!(false)) };
                Ok(self.revoke(&[id.to_owned()]).unwrap_or(json!(false)))
            }
            "ccm:remote-revoke-all" => Ok(self.revoke_all().unwrap_or(json!(false))),
            "ccm:remote-publish" => Ok(json!(self.publish(arg))),
            "ccm:remote-keep-awake" => Ok(self.keep_awake(arg.as_bool().unwrap_or(true))),
            _ => Err("unknown channel".into()),
        }
    }
}

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}
