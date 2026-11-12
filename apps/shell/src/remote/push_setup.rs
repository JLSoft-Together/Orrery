use serde_json::{json, Value};

use super::config::{Config, FirebaseConfig};
use super::fcm::Outcome;
use super::fcm_files::{self, ServiceAccount, MAX_FILE};
use super::keys::Read;
use super::push::{locale_of, Job};
use super::Remote;
use crate::state::lock;

pub const SECRET: &str = "orrery-fcm-sa";
const CLEARED: &str = "-";

impl Remote {
    pub(super) fn saved_account(&self) -> Option<ServiceAccount> {
        if !Config::load(&self.dir).firebase.is_set() {
            return None;
        }
        if let Some(a) = fcm_files::load_account(self.secrets.as_ref(), &self.dir) {
            return Some(a);
        }
        match self.secrets.read(SECRET) {
            Read::Found(v) if v.as_str() != CLEARED => fcm_files::restore(&v),
            _ => None,
        }
    }

    fn firebase_config(&self) -> FirebaseConfig {
        match lock(&self.ctx).clone() {
            Some(c) => lock(&c.config).firebase.clone(),
            None => Config::load(&self.dir).firebase,
        }
    }

    pub fn push_status(&self) -> Value {
        let cfg = self.firebase_config();
        let ctx = lock(&self.ctx).clone();
        let tokens = match &ctx {
            Some(c) => lock(&c.devices).list().iter().filter(|d| !d.push_token.is_empty()).count(),
            None => super::devices::Devices::load(&self.dir).list().iter().filter(|d| !d.push_token.is_empty()).count(),
        };
        let configured = cfg.is_set() && ctx.as_ref().map_or_else(|| self.saved_account().is_some(), |c| c.fcm.has_account());
        let mut out = json!({ "configured": configured, "tokens": tokens });
        if configured {
            out["projectId"] = Value::from(cfg.project_id);
        }
        if let Some(e) = ctx.and_then(|c| c.fcm.last_error()) {
            out["error"] = Value::from(e);
        }
        out
    }

    pub fn push_setup(&self, arg: &Value) -> Value {
        let (Some(gs), Some(sa)) = (arg["googleServices"].as_str(), arg["serviceAccount"].as_str()) else { return fail("bad_json") };
        if gs.len() > MAX_FILE || sa.len() > MAX_FILE {
            return fail("bad_json");
        }
        let parsed = match fcm_files::parse(gs, sa) {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        if !fcm_files::save_account(self.secrets.as_ref(), &self.dir, &parsed.account) {
            return fail("keyring");
        }
        if matches!(self.secrets.read(SECRET), Read::Found(_)) {
            let _ = self.secrets.delete(SECRET);
        }
        let project = parsed.firebase.project_id.clone();
        let ctx = self.ctx().ok();
        let moved = self.edit_config(|cfg| {
            let moved = !cfg.firebase.project_id.is_empty() && cfg.firebase.project_id != project;
            cfg.firebase = parsed.firebase.clone();
            moved
        });
        if let Some(c) = &ctx {
            c.fcm.set_account(Some(parsed.account));
            if moved {
                lock(&c.devices).clear_all_push();
            }
            c.push_config_changed();
            c.changed();
        } else if moved {
            let mut devices = super::devices::Devices::load(&self.dir);
            devices.clear_all_push();
        }
        json!({ "ok": true, "projectId": project })
    }

    pub fn push_test(&self, arg: &Value) -> Value {
        let Ok(ctx) = self.ctx() else { return fail("keyring") };
        if !ctx.push_ready() {
            return fail("not_configured");
        }
        let only = arg.get("deviceId").and_then(Value::as_str);
        let machine = ctx.machine_id();
        let jobs: Vec<Job> = lock(&ctx.devices)
            .list()
            .iter()
            .filter(|d| !d.push_token.is_empty() && only.is_none_or(|id| id == d.device_id))
            .map(|d| Job { device: d.device_id.clone(), token: d.push_token.clone(), machine: machine.clone(), pane: String::new(), kind: "test", locale: locale_of(Some(&d.push_locale)) })
            .collect();
        if jobs.is_empty() {
            return fail("no_device");
        }
        let (mut sent, mut error) = (0, None);
        for job in &jobs {
            match ctx.deliver(job) {
                Outcome::Sent => sent += 1,
                Outcome::Dead => error = error.or(Some("unregistered")),
                Outcome::Fail(e) => error = Some(e),
            }
        }
        match (sent, error) {
            (0, Some(e)) => fail(e),
            (n, _) => json!({ "ok": true, "sent": n }),
        }
    }

    pub fn push_remove(&self) -> Value {
        fcm_files::remove_account(&self.dir);
        if matches!(self.secrets.read(SECRET), Read::Found(_)) {
            let _ = self.secrets.delete(SECRET);
        }
        self.edit_config(|cfg| cfg.firebase = FirebaseConfig::default());
        match self.ctx().ok() {
            Some(c) => {
                c.fcm.set_account(None);
                lock(&c.devices).clear_all_push();
                c.push_config_changed();
                c.changed();
            }
            None => super::devices::Devices::load(&self.dir).clear_all_push(),
        }
        json!({ "ok": true })
    }
}

fn fail(code: &str) -> Value {
    json!({ "ok": false, "error": code })
}
