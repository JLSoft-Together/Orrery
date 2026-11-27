use std::path::Path;

use serde_json::{json, Value};

use super::config::now_secs;
use super::install::{is_apk, page_path, pick_source, save_name, suggestion, url_path, valid_url, ApkSource, TicketKind, MAX_URL};
use super::Remote;
use crate::state::lock;

fn file_json(path: &Path) -> Value {
    let size = std::fs::metadata(path).map_or(0, |m| m.len());
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    json!({ "path": path.to_string_lossy(), "name": name, "size": size })
}

impl Remote {
    pub(crate) fn set_apk(&self, path: &Path) -> bool {
        if !is_apk(path) {
            return false;
        }
        let text = path.to_string_lossy().into_owned();
        self.edit_config(|c| c.apk_path = text);
        if let Some(c) = lock(&self.ctx).clone() {
            c.install.revoke();
        }
        true
    }

    pub(crate) fn apk_source(&self, custom: &str) -> Option<ApkSource> {
        pick_source(custom, self.host.bundled_apk(), suggestion())
    }

    fn install_state(&self) -> Value {
        let cfg = self.read_config();
        let found = self.apk_source(&cfg.apk_path);
        let custom_ok = !cfg.apk_path.is_empty() && is_apk(Path::new(&cfg.apk_path));
        let current = found.as_ref().map(|f| f.path.as_path());
        let suggested = suggestion().filter(|s| current != Some(s.as_path()));
        json!({
            "ok": true,
            "apk": current.map(file_json),
            "source": found.as_ref().map(|f| f.source.name()),
            "version": found.as_ref().map_or("", |f| f.version.as_str()),
            "missing": !cfg.apk_path.is_empty() && !custom_ok,
            "suggested": suggested.as_deref().map(file_json),
            "url": cfg.apk_url,
            "running": lock(&self.running).is_some(),
        })
    }

    fn install_start(&self) -> Value {
        let ctx = match self.running_ctx() {
            Ok(c) => c,
            Err(e) => return json!({ "ok": false, "error": e }),
        };
        let custom = lock(&ctx.config).apk_path.clone();
        let Some(found) = self.apk_source(&custom) else { return json!({ "ok": false, "error": "apk" }) };
        let Some((token, exp)) = ctx.install.issue_with(TicketKind::ApkPage, found.path, found.version, now_secs()) else { return json!({ "ok": false, "error": "rng" }) };
        let lan = ctx.lan();
        let port = lan.first().map_or(0, std::net::SocketAddr::port);
        let addresses: Vec<String> = lan.iter().map(|a| a.ip().to_string()).collect();
        json!({ "ok": true, "path": url_path(&token), "page": page_path(&token), "exp": exp, "port": port, "addresses": addresses })
    }

    fn install_arm(&self, on: bool) -> Value {
        match self.running_ctx() {
            Ok(ctx) => {
                ctx.install.arm(on, now_secs());
                json!({ "ok": true, "armed": on })
            }
            Err(e) if !on => json!({ "ok": true, "armed": false, "error": e }),
            Err(e) => json!({ "ok": false, "error": e }),
        }
    }

    fn install_save(&self) -> Value {
        let custom = self.read_config().apk_path;
        let Some(found) = self.apk_source(&custom) else { return json!({ "ok": false, "error": "apk" }) };
        match self.host.save_apk(&found.path, &save_name(&found.version)) {
            Ok(Some(path)) => json!({ "ok": true, "path": path.to_string_lossy() }),
            Ok(None) => json!({ "ok": false, "error": "cancel" }),
            Err(_) => json!({ "ok": false, "error": "save" }),
        }
    }

    pub fn install_call(&self, arg: &Value) -> Value {
        match arg.get("op").and_then(Value::as_str).unwrap_or("get") {
            "get" => self.install_state(),
            "pick" => {
                let Some(path) = self.host.pick_apk() else { return self.install_state() };
                if !self.set_apk(&path) {
                    return json!({ "ok": false, "error": "apk" });
                }
                self.install_state()
            }
            "suggested" => match suggestion() {
                Some(p) if self.set_apk(&p) => self.install_state(),
                _ => json!({ "ok": false, "error": "apk" }),
            },
            "clear" => {
                self.edit_config(|c| c.apk_path.clear());
                if let Some(c) = lock(&self.ctx).clone() {
                    c.install.revoke();
                }
                self.install_state()
            }
            "url" => {
                let url = arg.get("url").and_then(Value::as_str).map(str::trim).unwrap_or("");
                if url.len() > MAX_URL || (!url.is_empty() && !valid_url(url)) {
                    return json!({ "ok": false, "error": "url" });
                }
                let url = url.to_owned();
                self.edit_config(|c| c.apk_url = url);
                self.install_state()
            }
            "start" => self.install_start(),
            "save" => self.install_save(),
            "arm" => self.install_arm(true),
            "disarm" => self.install_arm(false),
            "stop" => {
                if let Some(c) = lock(&self.ctx).clone() {
                    c.install.revoke();
                }
                json!({ "ok": true })
            }
            _ => json!({ "ok": false, "error": "invalid" }),
        }
    }
}
