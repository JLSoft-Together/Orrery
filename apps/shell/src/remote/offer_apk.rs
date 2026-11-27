use std::io::Read;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

use ring::digest::{Context, SHA256};
use serde_json::{json, Value};

use super::super::config::now_secs;
use super::super::ctx::Ctx;
use super::super::install::{is_apk, url_path, TicketKind};
use super::super::lan::is_private_v4;
use super::super::push_content::{clean, mask_text};
use super::super::viewer_events::run_log::valid_id;
use super::{now_ms, Extra, Job};
use crate::state::lock;

const OWNER: &str = "offer-apk";
const SESSIONS: &str = "ccm:run-sessions";
const URLS_MAX: usize = 3;
const NAME_MAX: usize = 80;
const PATH_MAX: usize = 4096;
const BUILD: &str = "build";
const OUTPUTS: &str = "outputs";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub path: PathBuf,
    pub size: u64,
    pub app: String,
    pub variant: String,
}

pub fn find_session<'a>(list: &'a Value, id: &str) -> Option<&'a Value> {
    list.as_array()?.iter().find(|s| s["id"].as_str() == Some(id))
}

pub fn in_build(root: &Path, apk: &Path) -> bool {
    let Ok(rel) = apk.strip_prefix(root) else { return false };
    let parts: Vec<&std::ffi::OsStr> = rel.components().filter_map(|c| if let Component::Normal(p) = c { Some(p) } else { None }).collect();
    parts.windows(2).any(|w| w[0] == BUILD && w[1] == OUTPUTS)
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= PATH_MAX && !s.contains('\0'))
}

pub fn vet(session: &Value) -> Result<Built, &'static str> {
    let apk = session.get("apk").filter(|a| a.is_object()).ok_or("noApk")?;
    let (raw, cwd) = (text(apk, "path").ok_or("noApk")?, text(session, "cwd").ok_or("noApk")?);
    let path = std::fs::canonicalize(raw).map_err(|_| "gone")?;
    let root = std::fs::canonicalize(cwd).map_err(|_| "outside")?;
    if !in_build(&root, &path) {
        return Err("outside");
    }
    if !is_apk(&path) {
        return Err(if path.is_file() { "notApk" } else { "gone" });
    }
    let size = std::fs::metadata(&path).map_err(|_| "gone")?.len();
    let name = |key: &str| clean(apk.get(key).and_then(Value::as_str).unwrap_or(""), &mask_text, NAME_MAX);
    Ok(Built { path, size, app: name("applicationId"), variant: name("variant") })
}

pub fn sha256_hex(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut ctx = Context::new(&SHA256);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Some(ctx.finish().as_ref().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn urls_of(lan: &[std::net::SocketAddr], token_path: &str) -> Vec<String> {
    lan.iter().filter(|a| matches!(a.ip(), IpAddr::V4(v4) if is_private_v4(v4))).take(URLS_MAX).map(|a| format!("http://{}:{}{token_path}", a.ip(), a.port())).collect()
}

pub fn offer_value(built: &Built, sha: &str, urls: &[String], exp: u64) -> Value {
    json!({ "url": urls.first(), "urls": urls, "size": built.size, "sha256": sha, "app": built.app, "variant": built.variant, "exp": exp })
}

impl Ctx {
    pub fn offer_targets(&self) -> Vec<Job> {
        self.push_targets(|d| d.push_content).into_iter().filter(|j| j.web.is_some() || j.seal_key.is_some()).collect()
    }

    fn session_list(&self) -> Option<Value> {
        let env = self.host.viewer(OWNER, SESSIONS, json!([]), &[]);
        env.get("raw").filter(|r| r.is_array()).cloned()
    }

    pub fn queue_apk(&self, targets: Vec<Job>, apk: &Value, now: Instant) -> usize {
        let at = now_ms();
        let mut queued = 0;
        {
            let mut st = lock(&self.push);
            for job in targets {
                if !st.admit_kind(&job.device, "", "apk", now, at) {
                    continue;
                }
                st.push(Job { kind: "apk", content: true, extra: Extra { apk: Some(apk.clone()), ..Extra::default() }, ..job });
                queued += 1;
            }
        }
        if queued > 0 {
            self.push_wake.notify_one();
        }
        queued
    }

    pub fn offer_apk(&self, session_id: &str, now: Instant) -> Result<usize, &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("off");
        }
        if !valid_id(session_id) {
            return Err("invalid");
        }
        let targets = self.offer_targets();
        if targets.is_empty() {
            return Err("none");
        }
        let lan = self.lan();
        if urls_of(&lan, "").is_empty() {
            return Err("lan");
        }
        let list = self.session_list().ok_or("offline")?;
        let session = find_session(&list, session_id).ok_or("notFound")?;
        let built = vet(session)?;
        let sha = sha256_hex(&built.path).ok_or("gone")?;
        let (token, exp) = self.install.issue_with(TicketKind::ApkOffer, built.path.clone(), built.variant.clone(), now_secs()).ok_or("rng")?;
        let urls = urls_of(&lan, &url_path(&token));
        match self.queue_apk(targets, &offer_value(&built, &sha, &urls, exp), now) {
            0 => Err("limit"),
            n => Ok(n),
        }
    }
}

impl super::super::Remote {
    pub fn offer_apk(&self, arg: &Value) -> Value {
        let id = arg.as_str().or_else(|| arg["sessionId"].as_str()).unwrap_or("");
        let Some(ctx) = lock(&self.ctx).clone() else { return json!({ "ok": false, "error": "off" }) };
        match ctx.offer_apk(id, Instant::now()) {
            Ok(sent) => json!({ "ok": true, "sent": sent }),
            Err(error) => json!({ "ok": false, "error": error }),
        }
    }
}

#[cfg(test)]
#[path = "offer_apk_tests.rs"]
mod tests;
