pub mod analyze;
pub mod axml;
pub mod history;
pub mod zip;

use crate::js;
use crate::preview::paths::{dirname, is_absolute, is_within, real_or_null, root_key};
use crate::util::{mtime_ms, now_ms};
use crate::{arg, Ctx, Desk, DeskEnv};
use analyze::{analyze_apk, compare_snapshots, is_apk_name, side_of, Snapshot, DIFF_FILES};
use history::History;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use zip::ApkError;

pub const MODULE: &str = "apk";
const CACHE_MAX: usize = 4;
const PICKED_MAX: usize = 50;

fn failure(code: &str) -> Value {
    json!({"ok": false, "error": code})
}

struct State {
    history: History,
    picked: Mutex<VecDeque<String>>,
    cache: Mutex<Vec<(String, Snapshot)>>,
}

fn host_path(ctx: &Ctx, method: &str, key: &str, v: &Value) -> Option<String> {
    ctx.host(method, json!({ key: v })).ok().and_then(|r| r.as_str().map(String::from))
}

fn gate(st: &State, ctx: &Ctx, p: &Value) -> Result<String, Value> {
    let Some(s) = p.as_str().filter(|s| js::len(s) <= 4096 && is_absolute(s) && is_apk_name(s)) else { return Err(failure("invalid")) };
    let Some(real) = real_or_null(s) else { return Err(failure("missing")) };
    if !is_apk_name(&real) {
        return Err(failure("invalid"));
    }
    if st.picked.lock().unwrap().contains(&root_key(&real)) {
        return Ok(real);
    }
    if host_path(ctx, "gate.file", "path", &json!(real)).is_some() {
        Ok(real)
    } else {
        Err(failure("denied"))
    }
}

fn snapshot(st: &State, file: &str) -> Result<Snapshot, ApkError> {
    let meta = std::fs::metadata(file).ok().filter(|m| m.is_file()).ok_or(ApkError::ReadFailed)?;
    let key = format!("{}|{}|{}", root_key(file), meta.len(), js::number(mtime_ms(&meta)));
    {
        let mut cache = st.cache.lock().unwrap();
        if let Some(i) = cache.iter().position(|(k, _)| *k == key) {
            let hit = cache.remove(i);
            let snap = hit.1.clone();
            cache.push(hit);
            return Ok(snap);
        }
    }
    let snap = analyze_apk(file, now_ms())?;
    let mut cache = st.cache.lock().unwrap();
    cache.retain(|(k, _)| *k != key);
    cache.push((key, snap.clone()));
    while cache.len() > CACHE_MAX {
        cache.remove(0);
    }
    Ok(snap)
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let st = Arc::new(State { history: History::new(&env.data_dir), picked: Mutex::new(VecDeque::new()), cache: Mutex::new(Vec::new()) });

    let s = st.clone();
    desk.on(MODULE, "ccm:apk-pick", move |ctx, args| {
        let near = arg(args, 0);
        let dir = near.as_str().filter(|n| is_absolute(n)).and_then(|_| host_path(ctx, "gate.dir", "dir", near));
        let picked = ctx.host("apk.pick", json!({"defaultPath": dir}))?;
        let Some(file) = picked.as_str().filter(|f| is_apk_name(f)) else { return Ok(Value::Null) };
        let Some(real) = real_or_null(file) else { return Ok(Value::Null) };
        let mut list = s.picked.lock().unwrap();
        if list.len() >= PICKED_MAX {
            list.pop_front();
        }
        list.retain(|k| *k != root_key(&real));
        list.push_back(root_key(&real));
        Ok(json!(real))
    });

    let s = st.clone();
    desk.on(MODULE, "ccm:apk-analyze", move |ctx, args| {
        let Some(req) = arg(args, 0).as_object() else { return Ok(failure("denied")) };
        let file = match gate(&s, ctx, req.get("path").unwrap_or(&Value::Null)) {
            Ok(f) => f,
            Err(v) => return Ok(v),
        };
        let snap = match snapshot(&s, &file) {
            Ok(x) => x,
            Err(e) => return Ok(failure(e.code())),
        };
        let root = req.get("root").unwrap_or(&Value::Null);
        let dir = if root.is_null() { None } else { host_path(ctx, "gate.dir", "dir", root) };
        let project = match dir {
            Some(d) if is_within(&d, &file) => d,
            _ => dirname(&file),
        };
        let prev = s.history.record(&project, &snap);
        Ok(json!({"ok": true, "report": snap.report, "previous": prev.as_ref().map(side_of), "diff": prev.as_ref().map(|p| compare_snapshots(p, &snap, DIFF_FILES))}))
    });

    let s = st;
    desk.on(MODULE, "ccm:apk-compare", move |ctx, args| {
        let Some(req) = arg(args, 0).as_object() else { return Ok(failure("denied")) };
        let a = gate(&s, ctx, req.get("path").unwrap_or(&Value::Null));
        let b = gate(&s, ctx, req.get("other").unwrap_or(&Value::Null));
        let (file, other) = match (a, b) {
            (Err(v), _) | (Ok(_), Err(v)) => return Ok(v),
            (Ok(a), Ok(b)) => (a, b),
        };
        let cur = match snapshot(&s, &file) {
            Ok(x) => x,
            Err(e) => return Ok(failure(e.code())),
        };
        let old = match snapshot(&s, &other) {
            Ok(x) => x,
            Err(e) => return Ok(failure(e.code())),
        };
        Ok(json!({"ok": true, "other": old.report, "diff": compare_snapshots(&old, &cur, DIFF_FILES)}))
    });
}
