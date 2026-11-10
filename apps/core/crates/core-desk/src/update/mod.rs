pub mod core;
pub mod download;
pub mod github;
pub mod net;
pub mod pick;
pub mod semver;

use crate::{arg, Desk, DeskEnv, DeskError};
use core::{Cfg, Core};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

pub const MODULE: &str = "update";

struct Lazy {
    cell: OnceLock<Arc<Core>>,
    data_dir: PathBuf,
    current: String,
    emitter: crate::Emitter,
    host: crate::HostLink,
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    }
}

impl Lazy {
    fn core(&self) -> Result<Arc<Core>, DeskError> {
        if let Some(c) = self.cell.get() {
            return Ok(c.clone());
        }
        let env = self.host.call("update.env", Value::Null).unwrap_or(Value::Null);
        let text = |k: &str, d: &str| env.get(k).and_then(Value::as_str).unwrap_or(d).to_string();
        let test_feed = std::env::var("CCM_UPDATE_FEED").ok().filter(|v| !v.is_empty());
        let cfg = Cfg {
            current: self.current.clone(),
            kind: text("kind", "unsupported"),
            asset_kind: text("assetKind", "unsupported"),
            can_download: env.get("canDownload") == Some(&Value::Bool(true)),
            arch: text("arch", arch()),
            allow_local: test_feed.is_some(),
            feed: test_feed.unwrap_or_else(|| github::DEFAULT_FEED.to_string()),
            data_dir: self.data_dir.clone(),
        };
        Ok(self.cell.get_or_init(|| Core::new(cfg, self.emitter.clone(), self.host.clone())).clone())
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let lazy = Arc::new(Lazy { cell: OnceLock::new(), data_dir: env.data_dir.clone(), current: env.app_version.clone(), emitter: desk.emitter(), host: desk.host_link() });
    let l = lazy.clone();
    desk.on_close(Arc::new(move || {
        if let Some(c) = l.cell.get() {
            c.dispose();
        }
    }));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-state", move |_, _| Ok(l.core()?.state()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-check", move |_, _| Ok(l.core()?.check()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-download", move |_, _| Ok(l.core()?.download()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-cancel", move |_, _| Ok(l.core()?.cancel()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-install", move |_, _| Ok(l.core()?.install()));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-skip", move |_, args| Ok(l.core()?.skip(arg(args, 0))));
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-auto", move |_, args| {
        let core = l.core()?;
        Ok(match arg(args, 0) {
            Value::Bool(on) => core.set_auto_check(*on),
            _ => core.state(),
        })
    });
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-page", move |_, _| {
        l.core()?.open_page();
        Ok(Value::Null)
    });
    let l = lazy.clone();
    desk.on(MODULE, "ccm:update-reveal", move |_, _| {
        l.core()?.reveal_file();
        Ok(Value::Null)
    });
    desk.on(MODULE, "desk:update-start", move |_, _| {
        lazy.core()?.start();
        Ok(json!(true))
    });
}

#[cfg(test)]
mod tests;
