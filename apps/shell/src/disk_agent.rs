use std::collections::HashSet;

use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};

use crate::disk_clean_text::{agent_box, ask_buttons};
use crate::disk_guard::{self as guard, Plat};
use crate::disk_map::{clean_search, to_children, to_search, DiskMap, DiskState};
use crate::disk_text::{delete_box, Ask};

pub const SEARCH_MAX: i64 = 200;
pub const USAGE_MAX: i64 = 100;
pub const DELETE_MAX: usize = 200;
pub const REASON_MAX: usize = 500;
const DAY_MS: f64 = 86_400_000.0;

pub fn display_text(v: &str, max: usize) -> String {
    let strip = |c: char| {
        let u = c as u32;
        u <= 0x1f || (0x7f..=0x9f).contains(&u) || u == 0x2028 || u == 0x2029 || (0x202a..=0x202e).contains(&u) || (0x2066..=0x2069).contains(&u)
    };
    let mut out = String::new();
    let mut gap = false;
    for c in v.chars() {
        if strip(c) {
            if !gap {
                out.push(' ');
                gap = true;
            }
        } else {
            out.push(c);
            gap = false;
        }
    }
    let t = out.trim();
    let mut taken = String::new();
    let mut units = 0;
    for c in t.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        taken.push(c);
    }
    taken
}

fn fail(error: &str) -> Value {
    json!({ "ok": false, "error": error })
}

fn int_in(v: &Value, lo: i64, hi: i64) -> Option<i64> {
    let f = v.as_f64()?;
    (f.fract() == 0.0 && f >= lo as f64 && f <= hi as f64).then_some(f as i64)
}

fn to_item(e: &Value) -> Value {
    let mut o = json!({ "path": e["path"], "name": e["name"], "size": e["size"], "dir": e["dir"], "mtime": e["mtime"] });
    if let Some(k) = e.get("kind").filter(|k| k.is_string()) {
        o["kind"] = k.clone();
    }
    o
}

fn items(v: &Value, max: usize) -> Vec<Value> {
    v["items"].as_array().map(|a| a.iter().take(max).map(to_item).collect()).unwrap_or_default()
}

fn host_error(e: &str) -> Value {
    fail(if e.starts_with("not in index") { "not-in-index" } else { "failed" })
}

fn ready(m: &DiskMap) -> Option<Value> {
    let Ok(st) = m.status() else { return Some(fail("unavailable")) };
    if st["indexed"] != json!(true) {
        return Some(fail(if st["scanning"] == json!(true) { "scanning" } else { "not-indexed" }));
    }
    None
}

fn search(m: &DiskMap, call: &Value, now: f64) -> Value {
    let Some(limit) = int_in(&call["limit"], 1, SEARCH_MAX) else { return fail("invalid") };
    let mut q = Map::new();
    for k in ["query", "kind", "minSize", "maxSize", "under"] {
        if let Some(v) = call.get(k).filter(|v| !v.is_null()) {
            q.insert(k.into(), v.clone());
        }
    }
    if let Some(d) = call.get("olderThanDays").filter(|v| !v.is_null()) {
        let Some(days) = int_in(d, 0, 36_500) else { return fail("invalid") };
        q.insert("before".into(), json!((now - days as f64 * DAY_MS).max(0.0)));
    }
    q.insert("sort".into(), json!("size"));
    q.insert("limit".into(), json!(limit));
    let Some(clean) = clean_search(&Value::Object(q)) else { return fail("invalid") };
    if let Some(early) = ready(m) {
        return early;
    }
    match m.host.call("disk.search", clean) {
        Ok(raw) => {
            let res = to_search(&raw);
            json!({ "ok": true, "tool": "disk_search", "total": res["total"], "ms": res["ms"], "items": items(&res, SEARCH_MAX as usize) })
        }
        Err(e) => host_error(&e),
    }
}

fn usage(m: &DiskMap, call: &Value) -> Value {
    let path = call["path"].as_str().unwrap_or("");
    let Some(limit) = int_in(&call["limit"], 1, USAGE_MAX).filter(|_| guard::is_absolute(path, Plat::host())) else { return fail("invalid") };
    if let Some(early) = ready(m) {
        return early;
    }
    match m.host.call("disk.children", json!({ "path": path, "sort": "size", "limit": limit })) {
        Ok(raw) => {
            let Some(res) = to_children(&raw) else { return fail("not-in-index") };
            let p = res["path"].as_str().filter(|s| !s.is_empty()).unwrap_or(path);
            json!({ "ok": true, "tool": "disk_usage", "path": p, "size": res["size"], "total": res["total"], "items": items(&res, USAGE_MAX as usize) })
        }
        Err(e) => host_error(&e),
    }
}

fn auto_confirm() -> bool {
    crate::disk_clean::e2e_env("CCM_E2E_DISKMAP_CONFIRM").is_some_and(|v| v == "1")
}

fn propose(app: &AppHandle, m: &DiskMap, call: &Value) -> Value {
    let plat = Plat::host();
    let paths: Vec<&Value> = call["paths"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    if paths.is_empty() || paths.len() > DELETE_MAX || !paths.iter().all(|p| p.as_str().is_some_and(|s| guard::is_absolute(s, plat))) {
        return fail("invalid");
    }
    let reason = call["reason"].as_str().map(|r| display_text(r, REASON_MAX)).unwrap_or_default();
    let Some(permanent) = call["permanent"].as_bool().filter(|_| !reason.is_empty()) else { return fail("invalid") };
    if m.is_deleting() {
        return fail("busy");
    }
    if let Some(early) = ready(m) {
        return early;
    }
    let pane = Some(display_text(call["name"].as_str().unwrap_or(""), 80)).filter(|s| !s.is_empty()).unwrap_or_else(|| "agent".into());
    let mut seen = HashSet::new();
    let list: Vec<Value> = paths.into_iter().filter(|p| seen.insert(p.as_str().unwrap_or("").to_owned())).cloned().collect();
    let confirm = |ask: &Ask, second: bool| -> bool {
        if auto_confirm() {
            return true;
        }
        let first: Vec<String> = ask.first.iter().map(|p| display_text(p, 400)).collect();
        let shown = Ask { count: ask.count, bytes: ask.bytes, first: &first, permanent: ask.permanent };
        let locale = crate::hosts::locale(app);
        let mac = cfg!(target_os = "macos");
        if second {
            let b = delete_box(&shown, true, &locale, mac);
            return crate::hosts::confirm(app, &b.message, &b.detail, &b.ok, &b.cancel);
        }
        ask_buttons(app, &agent_box(&shown, &pane, &reason, &locale, mac)) == 0
    };
    let res = m.remove_with(app, &list, permanent, &confirm, 5);
    let failed = res["failed"].as_array().cloned().unwrap_or_default();
    let done = res["done"].as_array().cloned().unwrap_or_default();
    let cancelled = res["cancelled"] == json!(true);
    if !cancelled && done.is_empty() && !failed.is_empty() && failed.iter().all(|f| f["reason"] == json!("busy")) {
        return fail("busy");
    }
    let mut out = json!({ "ok": true, "tool": "disk_propose_delete", "done": done, "failed": failed, "freed": res["freed"], "permanent": res["permanent"] });
    if cancelled {
        out["cancelled"] = json!(true);
    }
    out
}

pub fn call(app: &AppHandle, call: &Value) -> Value {
    let Some(state) = app.try_state::<DiskState>() else { return fail("unavailable") };
    let m = state.0.clone();
    let now = crate::disk_map::now_ms() as f64;
    match call["tool"].as_str().unwrap_or("") {
        "disk_search" => search(&m, call, now),
        "disk_usage" => usage(&m, call),
        "disk_propose_delete" => propose(app, &m, call),
        _ => fail("unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_text_strips_controls_and_bidi_and_clips() {
        assert_eq!(display_text("a\u{202e}b\n\nc", 10), "a b c");
        assert_eq!(display_text("  xin chào  ", 4), "xin ");
        assert_eq!(display_text("\u{0}", 5), "");
    }

    #[test]
    fn whole_numbers_in_range_only() {
        assert_eq!(int_in(&json!(5), 1, 200), Some(5));
        assert_eq!(int_in(&json!(201), 1, 200), None);
        assert_eq!(int_in(&json!(1.5), 1, 200), None);
        assert_eq!(int_in(&json!("5"), 1, 200), None);
    }

    #[test]
    fn items_keep_only_the_public_fields() {
        let v = json!({ "items": [{ "path": "C:\\a", "name": "a", "size": 3, "dir": false, "mtime": 1, "cloud": true, "kind": "image" }] });
        assert_eq!(items(&v, 5), vec![json!({ "path": "C:\\a", "name": "a", "size": 3, "dir": false, "mtime": 1, "kind": "image" })]);
    }
}
