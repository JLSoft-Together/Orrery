pub mod install;
pub mod settings;

use crate::claude::Sys;
use crate::{arg, js, Ctx, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};
use settings::{read_settings, Read};
use std::path::PathBuf;

fn command(ctx: &Ctx, method: &str) -> Result<Option<String>, DeskError> {
    Ok(ctx.host(method, Value::Null)?.as_str().filter(|s| !s.is_empty()).map(str::to_string))
}

fn statusline_enable(ctx: &Ctx, file: &std::path::Path) -> Result<Value, DeskError> {
    let Some(bridge) = command(ctx, "statusline.command")? else { return Ok(json!("failed")) };
    let read = read_settings(file);
    let (state, current) = settings::status_state(&read);
    if state == "ours" || state == "unreadable" {
        return Ok(json!(state));
    }
    let s = match &read {
        Read::Ok(s) | Read::Missing(s) => s.clone(),
        Read::Unreadable => return Ok(json!("unreadable")),
    };
    let planned = js::stringify_indent(&json!({"statusLine": settings::planned_status_line(&s, state, current.as_deref(), &bridge)}), "  ");
    let req = json!({"file": crate::util::path_str(file), "planned": planned, "other": state == "other"});
    if ctx.host("statusline.confirm", req)? != Value::Bool(true) {
        return Ok(json!("cancelled"));
    }
    Ok(json!(settings::write_status_line(file, &bridge)))
}

fn hooks_set(ctx: &Ctx, file: &std::path::Path, on: &Value) -> Result<Value, DeskError> {
    let Value::Bool(on) = on else { return Ok(json!("failed")) };
    let Some(bridge) = command(ctx, "hooks.command")? else { return Ok(json!("failed")) };
    let read = read_settings(file);
    let state = settings::hooks_state(&read);
    let s = match &read {
        Read::Ok(s) | Read::Missing(s) => s.clone(),
        Read::Unreadable => return Ok(json!("unreadable")),
    };
    if (*on && state == "ours") || (!*on && state == "none") {
        return Ok(json!("ok"));
    }
    let cmd = on.then_some(bridge.as_str());
    let planned = js::stringify_indent(&json!({"hooks": settings::planned_hooks(&s, cmd)}), "  ");
    let req = json!({"file": crate::util::path_str(file), "planned": planned, "on": on});
    if ctx.host("hooks.confirm", req)? != Value::Bool(true) {
        return Ok(json!("cancelled"));
    }
    Ok(json!(settings::write_hooks(file, cmd)))
}

fn bridge_source(args: &[Value]) -> PathBuf {
    if let Some(s) = arg(args, 0).get("source").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        return PathBuf::from(s);
    }
    if let Ok(s) = std::env::var("CCM_BRIDGE_BIN") {
        if !s.trim().is_empty() {
            return PathBuf::from(s.trim());
        }
    }
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join(install::BRIDGE_EXE))).unwrap_or_default()
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let file = || Sys::from_env().settings_file();
    desk.on("statusline", "ccm:statusline-state", move |_, _| Ok(json!(settings::status_state(&read_settings(&file())).0)));
    desk.on("statusline", "ccm:statusline-enable", move |ctx, _| statusline_enable(ctx, &file()));
    desk.on("hooks", "ccm:hooks-state", move |_, _| Ok(json!(settings::hooks_state(&read_settings(&file())))));
    desk.on("hooks", "ccm:hooks-set", move |ctx, args| hooks_set(ctx, &file(), arg(args, 0)));
    let data = env.data_dir.clone();
    desk.on("bridge", "desk:bridge-install", move |_, args| Ok(install::install_json(&bridge_source(args), &data)));
    let data = env.data_dir.clone();
    desk.on("bridge", "desk:bridge-migrate", move |_, args| {
        let r = arg(args, 0);
        let s = r.get("statusline").and_then(Value::as_str).filter(|s| !s.is_empty());
        let h = r.get("hook").and_then(Value::as_str).filter(|s| !s.is_empty());
        Ok(install::migrate(&file(), &data, s, h))
    });
}
