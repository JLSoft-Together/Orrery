pub mod format;
pub mod service;

use crate::{arg, js, util, Ctx, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};
use service::BackupService;
use std::sync::Arc;

pub const MODULE: &str = "backup";

fn local(ctx: &Ctx) -> Value {
    ctx.host("backup.readLocal", Value::Null).ok().filter(Value::is_object).unwrap_or_else(|| json!({}))
}

fn github(ctx: &Ctx) -> Value {
    ctx.host("backup.github", Value::Null).unwrap_or_else(|_| json!([]))
}

fn path_of(v: Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

fn export(svc: &BackupService, env: &DeskEnv, ctx: &Ctx) -> Result<Value, DeskError> {
    let local = local(ctx);
    let backup = svc.snapshot(&local, &github(ctx));
    let default_path = env.documents_dir.join(format::export_name(util::local_date()));
    let Some(file) = path_of(ctx.host("backup.saveDialog", json!({"defaultPath": util::path_str(&default_path)}))?) else {
        return Ok(json!({"ok": false, "error": "cancelled"}));
    };
    if std::fs::write(&file, js::stringify_indent(&backup.to_value(), " ")).is_err() {
        return Ok(json!({"ok": false, "error": "write"}));
    }
    Ok(json!({"ok": true, "file": file, "sections": format::sections_of(&backup, None)}))
}

fn pick(svc: &BackupService, ctx: &Ctx) -> Result<Value, DeskError> {
    let Some(file) = path_of(ctx.host("backup.openDialog", Value::Null)?) else {
        return Ok(json!({"ok": false, "error": "cancelled"}));
    };
    let Ok(meta) = std::fs::metadata(&file) else {
        return Ok(json!({"ok": false, "error": "read"}));
    };
    if meta.len() > format::MAX_BACKUP_BYTES as u64 {
        return Ok(json!({"ok": false, "error": "too-large"}));
    }
    match std::fs::read(&file) {
        Ok(bytes) => Ok(svc.preview(&String::from_utf8_lossy(&bytes), &file)),
        Err(_) => Ok(json!({"ok": false, "error": "read"})),
    }
}

fn apply(svc: &BackupService, ctx: &Ctx, args: &[Value]) -> Result<Value, DeskError> {
    let picked = format::clean_section_ids(arg(args, 1));
    let (Some(token), Some(picked)) = (arg(args, 0).as_str(), picked) else {
        return Ok(json!({"ok": false, "error": "invalid"}));
    };
    if picked.is_empty() {
        return Ok(json!({"ok": false, "error": "invalid"}));
    }
    if svc.staged_sections(token).is_none() {
        return Ok(json!({"ok": false, "error": "expired"}));
    }
    if ctx.host("backup.confirmApply", json!({"sections": picked}))? != Value::Bool(true) {
        return Ok(json!({"ok": false, "error": "cancelled"}));
    }
    let current = local(ctx);
    Ok(svc.stage(token, &picked, &current, &github(ctx)))
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let svc = Arc::new(BackupService::new(&env.data_dir, &env.app_version, service::platform_name()));
    let (s, e) = (svc.clone(), env.clone());
    desk.on(MODULE, "ccm:backup-export", move |ctx, _| export(&s, &e, ctx));
    let s = svc.clone();
    desk.on(MODULE, "ccm:backup-pick", move |ctx, _| pick(&s, ctx));
    let s = svc.clone();
    desk.on(MODULE, "ccm:backup-apply", move |ctx, args| apply(&s, ctx, args));
    desk.on(MODULE, "ccm:backup-relaunch", |ctx, _| {
        ctx.host("backup.relaunch", Value::Null)?;
        Ok(Value::Bool(true))
    });
    let s = svc.clone();
    desk.on(MODULE, "desk:backup-apply-pending", move |_, _| Ok(json!(s.apply_pending())));
    let s = svc;
    desk.on(MODULE, "desk:backup-write-auto", move |ctx, args| {
        let kind = match arg(args, 0).as_str() {
            Some(k @ ("pre-update" | "pre-import")) => k,
            _ => return Ok(Value::Null),
        };
        let tag = arg(args, 1).as_str().unwrap_or_default();
        Ok(s.write_auto(kind, tag, arg(args, 2), &github(ctx)).map(|p| json!(util::path_str(&p))).unwrap_or(Value::Null))
    });
}
