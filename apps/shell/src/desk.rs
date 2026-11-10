use std::path::Path;

use core_desk::{Ctx, Desk, DeskEnv, DeskError, DeskIo};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Webview};

use crate::hosts;
use crate::state::MAIN;
use crate::window::COMPACT;

pub struct DeskState {
    pub desk: Desk,
}

pub fn env_for(data_dir: &Path, version: &str) -> DeskEnv {
    let mut env = DeskEnv::from_env();
    env.data_dir = data_dir.to_path_buf();
    if std::env::var("CCM_APP_VERSION").map_or(true, |v| v.trim().is_empty()) {
        version.clone_into(&mut env.app_version);
    }
    env
}

pub fn boot(env: &DeskEnv) -> DeskState {
    let desk = Desk::standard(env);
    if let Err(e) = desk.dispatch("desk:backup-apply-pending", Value::Null) {
        eprintln!("[shell] backup apply pending: {e}");
    }
    DeskState { desk }
}

pub fn emit_app(app: &AppHandle, event: &str, payload: Value) {
    for label in [MAIN, COMPACT] {
        if app.get_webview_window(label).is_some() {
            let _ = app.emit_to(label, event, payload.clone());
        }
    }
}

struct Io {
    app: AppHandle,
    caller: Option<Webview>,
}

impl DeskIo for Io {
    fn emit(&self, event: &str, payload: Value) {
        emit_app(&self.app, event, payload);
    }

    fn host(&self, _req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        hosts::call(&self.app, self.caller.as_ref(), method, &params).map_err(DeskError::Host)
    }
}

pub fn is_page_channel(channel: &str) -> bool {
    channel.starts_with("ccm:")
}

pub fn error_text(e: DeskError) -> String {
    match e {
        DeskError::UnknownChannel(c) => format!("not ported: {c}"),
        other => other.to_string(),
    }
}

pub async fn dispatch(app: AppHandle, webview: Webview, channel: String, args: Vec<Value>) -> Result<Value, String> {
    if !is_page_channel(&channel) {
        return Err(format!("not ported: {channel}"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let io = Io { app: app.clone(), caller: Some(webview) };
        let state = app.state::<DeskState>();
        state.desk.dispatch_with(&Ctx::new(&io, 0), &channel, Value::Array(args)).map_err(error_text)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_page_channels_reach_the_desk() {
        assert!(is_page_channel("ccm:backup-export"));
        assert!(!is_page_channel("desk:backup-apply-pending"));
        assert!(!is_page_channel("shell:window"));
    }

    #[test]
    fn unknown_channels_read_as_not_ported() {
        assert_eq!(error_text(DeskError::UnknownChannel("ccm:x".into())), "not ported: ccm:x");
        assert_eq!(error_text(DeskError::Failed("boom".into())), "boom");
    }

    #[test]
    fn the_standard_desk_serves_the_ported_channels() {
        let dir = std::env::temp_dir().join(format!("orrery-shell-desk-{}", std::process::id()));
        let state = boot(&env_for(&dir, "9.9.9"));
        let channels = state.desk.channels();
        for c in ["ccm:backup-export", "ccm:backup-pick", "ccm:backup-apply", "ccm:backup-relaunch", "ccm:build-scan", "ccm:build-clean", "ccm:build-cancel", "ccm:libs-check", "ccm:libs-vulns"] {
            assert!(channels.iter().any(|x| x == c), "{c}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_desk_channels_exist_in_the_generated_shim() {
        let shim = include_str!("shim.js");
        let dir = std::env::temp_dir().join(format!("orrery-shell-desk-shim-{}", std::process::id()));
        for c in boot(&env_for(&dir, "1.0.0")).desk.channels().iter().filter(|c| is_page_channel(c)) {
            assert!(shim.contains(&format!("\"{c}\"")), "{c}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
