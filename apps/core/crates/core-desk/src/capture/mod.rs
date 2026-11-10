pub mod files;
pub mod folder;

use crate::{arg, Ctx, Desk, DeskEnv};
use files::{clean_dir, parse_patch, parse_shot_request, CAPTURE_FILE};
use folder::{Folder, Status};
use serde_json::{json, Map, Value};
use std::sync::Arc;

pub const MODULE: &str = "capture";

pub fn folder_for(env: &DeskEnv) -> Folder {
    Folder::new(env.data_dir.join(CAPTURE_FILE))
}

fn told(ctx: &Ctx, s: Status) -> Status {
    if let Some(dir) = &s.missing {
        let _ = ctx.host("capture.missing", json!({"dir": dir}));
    }
    s
}

fn held(ctx: &Ctx) -> Vec<String> {
    let mut list: Vec<String> = match ctx.host("capture.held", Value::Null) {
        Ok(Value::Array(a)) => a.into_iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    };
    list.extend(crate::devices::recorder::held_files());
    list
}

fn set(ctx: &Ctx, f: &Folder, patch: &Map<String, Value>) -> Value {
    f.set(patch);
    let s = told(ctx, f.status());
    if let Some(active) = &s.active {
        f.sweep(active, &held(ctx));
    }
    s.value
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let folder = Arc::new(folder_for(env));
    let f = folder.clone();
    desk.on(MODULE, "ccm:capture-get", move |ctx, _| Ok(told(ctx, f.status()).value));
    let f = folder.clone();
    desk.on(MODULE, "ccm:capture-pick", move |ctx, _| {
        let near = f.dir();
        let picked = ctx.host("capture.pick", json!({"near": near})).unwrap_or(Value::Null);
        Ok(match clean_dir(&picked) {
            Some(dir) => set(ctx, &f, &Map::from_iter([("dir".to_string(), json!(dir))])),
            None => told(ctx, f.status()).value,
        })
    });
    let f = folder.clone();
    desk.on(MODULE, "ccm:capture-set", move |ctx, args| {
        Ok(match parse_patch(arg(args, 0)) {
            Some(patch) => set(ctx, &f, &patch),
            None => Value::Null,
        })
    });
    let f = folder.clone();
    desk.on(MODULE, "ccm:capture-shot", move |ctx, args| {
        let Some(req) = parse_shot_request(arg(args, 0)) else { return Ok(json!({"ok": false, "error": "invalid"})) };
        let Some(dir) = told(ctx, f.status()).active else { return Ok(json!({"ok": false, "error": "no-dir"})) };
        Ok(match f.write_shot(&dir, &req.png, &req.device, req.frame) {
            Some(file) if Folder::shot_ok(&dir, &file) => json!({"ok": true, "path": file}),
            _ => json!({"ok": false, "error": "failed"}),
        })
    });
    let f = folder.clone();
    desk.on(MODULE, "ccm:capture-open", move |ctx, _| {
        let Some(dir) = told(ctx, f.status()).active else { return Ok(json!(false)) };
        Ok(json!(matches!(ctx.host("capture.open", json!({"dir": dir})), Ok(Value::Bool(true)))))
    });
    desk.on(MODULE, "desk:capture-sweep", move |ctx, _| {
        let Some(active) = told(ctx, folder.status()).active else { return Ok(json!(0)) };
        Ok(json!(folder.sweep(&active, &held(ctx))))
    });
}
