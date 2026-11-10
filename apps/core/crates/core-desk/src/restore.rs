use crate::backup::format::scrub_json;
use crate::claude::is_absolute;
use crate::{arg, js, util, Ctx, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const MODULE: &str = "restore";
pub const RUN_STATE_FILE: &str = "run-state.json";
pub const MAX_PROMPTS_FILE: usize = 2 * 1024 * 1024;
const MAX_DIRS: usize = 200;

pub fn read_previous_run(file: &Path) -> bool {
    std::fs::read(file).ok().and_then(|b| serde_json::from_str::<Value>(&String::from_utf8_lossy(&b)).ok()).is_some_and(|v| v.get("running") == Some(&Value::Bool(true)))
}

pub fn mark(file: &Path, running: bool, now: f64) {
    let _ = std::fs::write(file, js::stringify(&json!({"running": running, "at": util::iso_string(now)})));
}

pub fn dirs_exist(raw: &Value) -> Value {
    let Some(list) = raw.as_array().filter(|a| a.len() <= MAX_DIRS) else { return Value::Null };
    Value::Array(
        list.iter()
            .map(|p| {
                let Some(s) = p.as_str() else { return Value::Bool(false) };
                let b = s.as_bytes();
                let unc = b.len() >= 2 && matches!(b[0], b'/' | b'\\') && matches!(b[1], b'/' | b'\\');
                Value::Bool(core_transcript::jsstr::units(s) <= 1024 && is_absolute(s) && !unc && Path::new(s).is_dir())
            })
            .collect(),
    )
}

pub fn prompts_file_text(raw: &Value) -> Option<(String, usize)> {
    let s = raw.as_str().filter(|s| core_transcript::jsstr::units(s) <= MAX_PROMPTS_FILE)?;
    let data: Value = serde_json::from_str(s).ok()?;
    let count = data.as_object()?.get("prompts")?.as_array()?.len();
    Some((js::stringify_indent(&scrub_json(&data, 0), " "), count))
}

pub struct RestoreState {
    previous: Mutex<Option<bool>>,
    asked: AtomicBool,
}

fn path_of(v: Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

fn export(ctx: &Ctx, raw: &Value) -> Result<Value, DeskError> {
    let Some((text, count)) = prompts_file_text(raw) else { return Ok(json!({"ok": false, "error": "invalid"})) };
    let Some(file) = path_of(ctx.host("restore.saveDialog", Value::Null)?) else { return Ok(json!({"ok": false, "error": "cancelled"})) };
    if std::fs::write(&file, text).is_err() {
        return Ok(json!({"ok": false, "error": "write"}));
    }
    Ok(json!({"ok": true, "file": file, "count": count}))
}

fn import(ctx: &Ctx) -> Result<Value, DeskError> {
    let Some(file) = path_of(ctx.host("restore.openDialog", Value::Null)?) else { return Ok(json!({"ok": false, "error": "cancelled"})) };
    let Ok(meta) = std::fs::metadata(&file) else { return Ok(json!({"ok": false, "error": "read"})) };
    if meta.len() > MAX_PROMPTS_FILE as u64 {
        return Ok(json!({"ok": false, "error": "too-large"}));
    }
    let Ok(bytes) = std::fs::read(&file) else { return Ok(json!({"ok": false, "error": "read"})) };
    let text = String::from_utf8_lossy(&bytes);
    match serde_json::from_str::<Value>(text.strip_prefix('\u{feff}').unwrap_or(&text)) {
        Ok(data) => Ok(json!({"ok": true, "data": data})),
        Err(_) => Ok(json!({"ok": false, "error": "invalid"})),
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let state = Arc::new(RestoreState { previous: Mutex::new(None), asked: AtomicBool::new(false) });
    let file = env.data_dir.join(RUN_STATE_FILE);
    let s = state.clone();
    let f = file.clone();
    desk.on(MODULE, "desk:run-boot", move |_, _| {
        let prev = read_previous_run(&f);
        *s.previous.lock().unwrap() = Some(prev);
        mark(&f, true, util::now_ms());
        Ok(json!({"unclean": prev}))
    });
    desk.on(MODULE, "desk:run-clean", move |_, _| {
        mark(&file, false, util::now_ms());
        Ok(Value::Bool(true))
    });
    let s = state;
    desk.on(MODULE, "ccm:run-state", move |ctx, _| {
        let first = !s.asked.swap(true, Ordering::SeqCst);
        let known = *s.previous.lock().unwrap();
        let unclean = match known {
            Some(v) => v,
            None => {
                let v = ctx.host("restore.previous", Value::Null).ok().and_then(|v| v.get("unclean").and_then(Value::as_bool)).unwrap_or(false);
                *s.previous.lock().unwrap() = Some(v);
                v
            }
        };
        Ok(json!({"first": first, "unclean": first && unclean}))
    });
    desk.on(MODULE, "ccm:dirs-exist", |_, args| Ok(dirs_exist(arg(args, 0))));
    desk.on(MODULE, "ccm:prompts-export", |ctx, args| export(ctx, arg(args, 0)));
    desk.on(MODULE, "ccm:prompts-import", |ctx, _| import(ctx));
}
