use std::path::Path;
use std::sync::Arc;

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

pub fn wire(app: &AppHandle) {
    let secrets = crate::vault::Keyring { service: crate::legacy::keyring_service() };
    app.manage(crate::vault::Vault::new(Box::new(secrets), crate::legacy::source()));
    crate::preview_ipc::manage(app);
    let state = app.state::<DeskState>();
    let sink = app.clone();
    state.desk.set_sink(Arc::new(move |event, payload| emit_app(&sink, event, payload)));
    let host = app.clone();
    state.desk.set_host(Arc::new(move |method, params| hosts::call(&host, None, method, &params).map_err(DeskError::Host)));
    crate::mcp::start(app);
    crate::lifecycle::boot(app);
    crate::episodes::start(app);
    crate::lifecycle::migrate_bridges(app);
    let boot = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = boot.state::<DeskState>().desk.dispatch("desk:update-start", Value::Null) {
            eprintln!("[shell] update start: {e}");
        }
    });
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
    allow: Option<String>,
}

pub fn allow_answer(allow: Option<&str>, method: &str, params: &Value) -> Option<Value> {
    let project = allow.filter(|p| std::path::Path::new(p).is_absolute())?;
    if method != "preview.allowed" || params.get("path").and_then(Value::as_str) != Some(project) {
        return None;
    }
    Some(serde_json::json!({ "real": project, "roots": [] }))
}

impl DeskIo for Io {
    fn emit(&self, event: &str, payload: Value) {
        emit_app(&self.app, event, payload);
    }

    fn host(&self, _req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        if let Some(v) = allow_answer(self.allow.as_deref(), method, &params) {
            return Ok(v);
        }
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

pub fn call_inside(app: &AppHandle, channel: &str, args: Value) -> Result<Value, String> {
    inside(app, channel, args, None)
}

pub fn call_inside_allowing(app: &AppHandle, channel: &str, args: Value, project: &str) -> Result<Value, String> {
    inside(app, channel, args, Some(project.to_owned()))
}

fn inside(app: &AppHandle, channel: &str, args: Value, allow: Option<String>) -> Result<Value, String> {
    let io = Io { app: app.clone(), caller: None, allow };
    let state = app.state::<DeskState>();
    state.desk.dispatch_with(&Ctx::new(&io, 0), channel, args).map_err(error_text)
}

pub async fn dispatch(app: AppHandle, webview: Webview, channel: String, args: Vec<Value>) -> Result<Value, String> {
    if !is_page_channel(&channel) {
        return Err(format!("not ported: {channel}"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let io = Io { app: app.clone(), caller: Some(webview), allow: None };
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
    fn the_scoped_allow_answers_only_preview_allowed_for_that_project() {
        let p = serde_json::json!({ "path": "/work/app", "fresh": false });
        assert_eq!(allow_answer(Some("/work/app"), "preview.allowed", &p), Some(serde_json::json!({ "real": "/work/app", "roots": [] })));
        assert_eq!(allow_answer(None, "preview.allowed", &p), None);
        assert_eq!(allow_answer(Some("/work/other"), "preview.allowed", &p), None);
        assert_eq!(allow_answer(Some("/work/app"), "preview.checkDir", &serde_json::json!({ "dir": "/work/app" })), None);
        assert_eq!(allow_answer(Some("/work/app"), "preview.allowed", &serde_json::json!({ "path": "/work/app/sub" })), None);
        assert_eq!(allow_answer(Some("work/app"), "preview.allowed", &serde_json::json!({ "path": "work/app" })), None);
        assert_eq!(allow_answer(Some(""), "preview.allowed", &serde_json::json!({ "path": "" })), None);
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
