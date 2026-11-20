use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::config::now_secs;
use super::install::{is_apk, suggestion, url_path, valid_url, MAX_URL};
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

    fn install_state(&self) -> Value {
        let cfg = self.read_config();
        let apk = Some(PathBuf::from(&cfg.apk_path)).filter(|p| !cfg.apk_path.is_empty() && is_apk(p));
        let suggested = suggestion().filter(|s| apk.as_deref() != Some(s.as_path()));
        json!({
            "ok": true,
            "apk": apk.as_deref().map(file_json),
            "missing": !cfg.apk_path.is_empty() && apk.is_none(),
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
        let path = PathBuf::from(lock(&ctx.config).apk_path.clone());
        if !is_apk(&path) {
            return json!({ "ok": false, "error": "apk" });
        }
        let Some((token, exp)) = ctx.install.issue(path, now_secs()) else { return json!({ "ok": false, "error": "rng" }) };
        let lan = ctx.lan();
        let port = lan.first().map_or(0, std::net::SocketAddr::port);
        let addresses: Vec<String> = lan.iter().map(|a| a.ip().to_string()).collect();
        json!({ "ok": true, "path": url_path(&token), "exp": exp, "port": port, "addresses": addresses })
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
