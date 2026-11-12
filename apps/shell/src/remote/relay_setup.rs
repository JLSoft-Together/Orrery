use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::cf_api::{self, Account, Bundle, Res};
use super::cf_http::{Http, Req, UreqHttp, API_BASE};
use super::config::Config;
use super::keys::Read;
use super::Remote;
use crate::state::lock;

pub const TOKEN_SECRET: &str = "orrery-cf-token";
const MIN_TOKEN: usize = 20;
const MAX_TOKEN: usize = 256;
const CANCEL_WAIT: Duration = Duration::from_secs(90);
const CANCEL_POLL: Duration = Duration::from_millis(50);
const WORKER_JS: &str = include_str!("../../resources/relay/worker.js");
const WORKER_META: &str = include_str!("../../resources/relay/worker.json");

pub type HttpFactory = Box<dyn Fn(&str, &str) -> Box<dyn Http> + Send + Sync>;
pub type ConfirmFn = Box<dyn Fn() -> bool + Send + Sync>;

pub struct Env {
    pub factory: Option<HttpFactory>,
    pub health_wait: Duration,
    pub health_gap: Duration,
}

impl Default for Env {
    fn default() -> Self {
        Env { factory: None, health_wait: Duration::from_secs(60), health_gap: Duration::from_secs(3) }
    }
}

#[derive(Default)]
pub struct Setup {
    pub busy: Mutex<bool>,
    pub removing: AtomicBool,
    pub cancel: AtomicBool,
    pub error: Mutex<Option<&'static str>>,
    pub warning: Mutex<Option<&'static str>>,
    pub env: Mutex<Env>,
    pub confirm: Mutex<Option<ConfirmFn>>,
}

pub fn bundle_meta() -> Value {
    serde_json::from_str(WORKER_META).unwrap_or(Value::Null)
}

pub fn bundle_hash() -> String {
    bundle_meta()["hash"].as_str().unwrap_or_default().to_owned()
}

pub fn clean_token(raw: &str) -> Option<&str> {
    let t = raw.trim();
    ((MIN_TOKEN..=MAX_TOKEN).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')).then_some(t)
}

pub struct BusyGuard<'a>(&'a Mutex<bool>);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        *lock(self.0) = false;
    }
}

impl Remote {
    pub fn set_delete_confirm(&self, f: ConfirmFn) {
        *lock(&self.setup.confirm) = Some(f);
    }

    fn http(&self, base: &str, token: &str) -> Box<dyn Http> {
        match lock(&self.setup.env).factory.as_ref() {
            Some(f) => f(base, token),
            None => Box::new(UreqHttp::new(base, token)),
        }
    }

    pub(super) fn edit_config<R>(&self, f: impl FnOnce(&mut Config) -> R) -> R {
        match lock(&self.ctx).clone() {
            Some(c) => {
                let mut cfg = lock(&c.config);
                let out = f(&mut cfg);
                cfg.save(&self.dir);
                out
            }
            None => {
                let mut cfg = Config::load(&self.dir);
                let out = f(&mut cfg);
                cfg.save(&self.dir);
                out
            }
        }
    }

    fn read_config(&self) -> Config {
        match lock(&self.ctx).clone() {
            Some(c) => lock(&c.config).clone(),
            None => Config::load(&self.dir),
        }
    }

    fn saved_token(&self) -> Option<Zeroizing<String>> {
        match self.secrets.read(TOKEN_SECRET) {
            Read::Found(t) if clean_token(&t).is_some() => Some(Zeroizing::new(t.trim().to_owned())),
            _ => None,
        }
    }

    fn acquire(&self) -> Option<BusyGuard<'_>> {
        let mut busy = lock(&self.setup.busy);
        if *busy {
            return None;
        }
        *busy = true;
        Some(BusyGuard(&self.setup.busy))
    }

    fn cancelled(&self) -> Res<()> {
        if self.setup.cancel.load(Ordering::SeqCst) {
            Err("cancelled")
        } else {
            Ok(())
        }
    }

    fn restart_if_running(&self) {
        if lock(&self.running).is_some() {
            self.stop(super::Control::Restarting);
            if self.read_config().enabled {
                self.start();
            }
        }
    }

    pub fn relay_status(&self) -> Value {
        let cfg = self.read_config().relay;
        let error = *lock(&self.setup.error);
        let warning = *lock(&self.setup.warning);
        let managed = !cfg.account_id.is_empty();
        let state = if *lock(&self.setup.busy) {
            "deploying"
        } else if cfg.url.is_empty() {
            if error.is_some() {
                "error"
            } else {
                "none"
            }
        } else {
            "ready"
        };
        let mut out = json!({
            "state": state,
            "updateAvailable": managed && !cfg.url.is_empty() && cfg.deployed_hash != bundle_hash(),
            "hasToken": self.saved_token().is_some(),
        });
        if !cfg.url.is_empty() {
            out["url"] = Value::from(cfg.url);
        }
        if managed {
            out["account"] = Value::from(if cfg.account_name.is_empty() { cfg.account_id } else { cfg.account_name });
        }
        if let Some(e) = error {
            out["error"] = Value::from(e);
        }
        if let Some(w) = warning {
            out["warning"] = Value::from(w);
        }
        out
    }

    pub fn relay_token(&self, raw: &Value) -> Value {
        let Some(token) = raw.as_str().and_then(clean_token) else { return json!({ "ok": false, "error": "token_invalid" }) };
        let Some(_guard) = self.acquire() else { return json!({ "ok": false, "error": "busy" }) };
        match self.check_token(token) {
            Ok(accounts) => {
                *lock(&self.setup.error) = None;
                json!({ "ok": true, "accounts": accounts.iter().map(|a| json!({ "id": a.id, "name": a.name })).collect::<Vec<_>>() })
            }
            Err(e) => json!({ "ok": false, "error": e }),
        }
    }

    fn check_token(&self, token: &str) -> Res<Vec<Account>> {
        let http = self.http(API_BASE, token);
        cf_api::verify(http.as_ref())?;
        let accounts = cf_api::accounts(http.as_ref())?;
        if accounts.is_empty() {
            return Err("no_account");
        }
        if !self.secrets.write(TOKEN_SECRET, token) {
            return Err("keyring");
        }
        Ok(accounts)
    }

    pub fn relay_deploy(&self, arg: &Value, progress: &dyn Fn(&str)) -> Value {
        let Some(account) = arg.as_str().filter(|a| cf_api::valid_account(a)) else { return json!({ "ok": false, "error": "no_account" }) };
        let Some(guard) = self.acquire() else { return json!({ "ok": false, "error": "busy" }) };
        self.setup.cancel.store(false, Ordering::SeqCst);
        *lock(&self.setup.error) = None;
        *lock(&self.setup.warning) = None;
        self.host.changed(self.status());
        let out = match self.run_deploy(account, progress) {
            Ok(url) => {
                let mut out = json!({ "ok": true, "url": url });
                if let Some(w) = *lock(&self.setup.warning) {
                    out["warning"] = Value::from(w);
                }
                out
            }
            Err(e) => {
                if e != "cancelled" {
                    *lock(&self.setup.error) = Some(e);
                }
                json!({ "ok": false, "error": e })
            }
        };
        drop(guard);
        self.host.changed(self.status());
        out
    }

    fn run_deploy(&self, account_id: &str, progress: &dyn Fn(&str)) -> Res<String> {
        let token = self.saved_token().ok_or("no_token")?;
        let http = self.http(API_BASE, &token);
        let http = http.as_ref();
        progress("verify");
        cf_api::verify(http)?;
        self.cancelled()?;
        let account = cf_api::accounts(http)?.into_iter().find(|a| a.id == account_id).ok_or("no_account")?;
        self.cancelled()?;
        progress("subdomain");
        let sub = match cf_api::subdomain(http, account_id)? {
            Some(s) => s,
            None => cf_api::create_subdomain(http, account_id)?,
        };
        let url = cf_api::worker_url(&sub)?;
        self.cancelled()?;
        progress("upload");
        let meta = bundle_meta();
        if meta.is_null() {
            return Err("bundle");
        }
        let current = cf_api::migration_tag(http, account_id)?;
        self.cancelled()?;
        cf_api::upload(http, account_id, &Bundle { script: WORKER_JS, meta: &meta }, current.as_deref())?;
        self.cancelled()?;
        progress("enable");
        cf_api::enable_workers_dev(http, account_id)?;
        self.cancelled()?;
        progress("health");
        let healthy = self.wait_health(&url)?;
        self.cancelled()?;
        *lock(&self.setup.warning) = (!healthy).then_some("health");
        let hash = bundle_hash();
        let changed = self.edit_config(|cfg| {
            let changed = cfg.relay.url != url;
            if changed {
                cfg.relay.url = url.clone();
                cfg.relay.registered = false;
            }
            cfg.relay.account_id = account.id.clone();
            cfg.relay.account_name = account.name.clone();
            cfg.relay.deployed_hash = hash;
            changed
        });
        if changed {
            self.restart_if_running();
        }
        progress("done");
        Ok(url)
    }

    fn wait_health(&self, url: &str) -> Res<bool> {
        let (wait, gap) = {
            let env = lock(&self.setup.env);
            (env.health_wait, env.health_gap)
        };
        let http = self.http(url, "");
        let deadline = Instant::now() + wait;
        loop {
            self.cancelled()?;
            if http.call(Req::get("/health")).is_some_and(|r| r.status == 200 && r.body.trim() == "ok") {
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(gap);
        }
    }

    fn acquire_for_remove(&self) -> Option<BusyGuard<'_>> {
        if let Some(g) = self.acquire() {
            return Some(g);
        }
        if self.setup.removing.load(Ordering::SeqCst) {
            return None;
        }
        self.setup.cancel.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + CANCEL_WAIT;
        while Instant::now() < deadline {
            std::thread::sleep(CANCEL_POLL);
            if let Some(g) = self.acquire() {
                return Some(g);
            }
        }
        None
    }

    pub fn relay_remove(&self, arg: &Value) -> Value {
        let delete_worker = arg.get("deleteWorker").and_then(Value::as_bool).unwrap_or(false);
        let Some(guard) = self.acquire_for_remove() else { return json!({ "ok": false, "error": "busy" }) };
        self.setup.removing.store(true, Ordering::SeqCst);
        let out = self.run_remove(delete_worker);
        self.setup.removing.store(false, Ordering::SeqCst);
        self.setup.cancel.store(false, Ordering::SeqCst);
        drop(guard);
        self.host.changed(self.status());
        out
    }

    fn run_remove(&self, delete_worker: bool) -> Value {
        let cfg = self.read_config().relay;
        let mut worker_deleted = false;
        if delete_worker && !cfg.account_id.is_empty() {
            let confirmed = lock(&self.setup.confirm).as_ref().is_some_and(|f| f());
            if !confirmed {
                return json!({ "ok": false, "error": "cancelled" });
            }
            let Some(token) = self.saved_token() else { return json!({ "ok": false, "error": "no_token" }) };
            let http = self.http(API_BASE, &token);
            if let Err(e) = cf_api::delete_script(http.as_ref(), &cfg.account_id) {
                *lock(&self.setup.error) = Some(e);
                return json!({ "ok": false, "error": e });
            }
            worker_deleted = true;
        }
        *lock(&self.setup.error) = None;
        *lock(&self.setup.warning) = None;
        let had_url = self.edit_config(|c| {
            let had = !c.relay.url.is_empty();
            c.relay.url.clear();
            c.relay.registered = false;
            c.relay.pending_revoke.clear();
            c.relay.account_id.clear();
            c.relay.account_name.clear();
            c.relay.deployed_hash.clear();
            had
        });
        if had_url {
            self.restart_if_running();
        }
        if !self.secrets.delete(TOKEN_SECRET) {
            return json!({ "ok": false, "error": "keyring", "workerDeleted": worker_deleted });
        }
        json!({ "ok": true, "workerDeleted": worker_deleted })
    }
}
