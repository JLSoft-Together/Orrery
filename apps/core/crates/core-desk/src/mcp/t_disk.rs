use super::host::{tool_result, Args, ToolHost};
use super::t_common::{num_text, str_of};
use super::t_desk::desk_call;
use serde_json::{json, Map, Value};

pub const DELETE_RATE_MAX: usize = 3;
pub const DELETE_RATE_MS: u64 = 60_000;
pub const READ_RATE_MAX: usize = 30;
pub const READ_RATE_MS: u64 = 60_000;
pub const SEARCH_MAX: i64 = 200;
pub const SEARCH_DEFAULT: i64 = 50;
pub const USAGE_MAX: i64 = 100;
pub const USAGE_DEFAULT: i64 = 20;
pub const DELETE_MAX: usize = 200;
pub const REASON_MAX: usize = 500;
pub const KINDS: [&str; 7] = ["image", "video", "audio", "doc", "archive", "installer", "code"];

pub fn error_text(code: &str) -> &'static str {
    match code {
        "not-indexed" => "The disk has not been scanned yet. Ask the user to open Settings > Data > Disk and run a scan; Orrery never scans because an agent asked.",
        "scanning" => "The disk index is being built right now; try again when the scan has finished.",
        "not-in-index" => "That path is not in the disk index (outside the scanned folders, or it did not exist at the last scan).",
        "busy" => "Another delete is waiting for the user; try again after they answer it.",
        "rate" => "Too many requests from this session in the last minute; wait a little.",
        "rate-delete" => "Too many delete proposals from this session in the last minute (max 3); wait a little.",
        "unavailable" => "Orrery cannot use the disk index right now.",
        "invalid" => "Orrery rejected the arguments.",
        _ => "Orrery could not complete the disk request.",
    }
}

pub fn arg_text(field: &str) -> &'static str {
    match field {
        "query" => "query must be a string of at most 1024 characters.",
        "kind" => "kind must be one of image, video, audio, doc, archive, installer, code.",
        "size" => "minSize and maxSize must be whole numbers of bytes, 0 or more.",
        "days" => "olderThanDays must be a whole number from 0 to 36500.",
        "under" => "under must be an absolute path.",
        "limit" => "limit must be a whole number in the range the tool allows.",
        "path" => "path must be an absolute path.",
        "paths" => "paths must be a list of 1 to 200 absolute paths.",
        "reason" => "reason is required: a short text of at most 500 characters saying why these paths should go.",
        "permanent" => "permanent must be true or false.",
        _ => "Orrery rejected the arguments.",
    }
}

fn disk_fail(error: &str, field: Option<&str>) -> Value {
    let text = match field {
        Some(f) => arg_text(f),
        None => error_text(error),
    };
    let mut out = json!({ "ok": false, "error": error });
    if let Some(f) = field {
        out["detail"] = json!(f);
    }
    tool_result(text, out, true)
}

pub fn is_abs_path(v: &str) -> bool {
    let b = v.as_bytes();
    if v.is_empty() || v.chars().count() > 4096 || v.contains(['\0', '\r', '\n']) {
        return false;
    }
    if v.starts_with('/') {
        return true;
    }
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
        return true;
    }
    let unc = v.starts_with("\\\\") || v.starts_with("//");
    if !unc {
        return false;
    }
    let rest = &v[2..];
    let mut parts = rest.split(['\\', '/']);
    let host = parts.next().unwrap_or("");
    let share = parts.next().unwrap_or("");
    !host.is_empty() && !host.starts_with(['?', '.']) && !share.is_empty()
}

fn opt_int(v: Option<&Value>, lo: i64, hi: i64) -> Result<Option<i64>, ()> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(x) => {
            let f = x.as_f64().filter(|f| f.is_finite() && f.fract() == 0.0).ok_or(())?;
            if f < lo as f64 || f > hi as f64 {
                return Err(());
            }
            Ok(Some(f as i64))
        }
    }
}

fn opt_str<'a>(args: &'a Args, key: &str, max: usize) -> Result<Option<&'a str>, ()> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.chars().count() <= max && !s.contains('\0') => Ok(Some(s.as_str())),
        Some(_) => Err(()),
    }
}

pub fn size_text(bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes.max(0.0);
    let mut u = 0;
    while v >= 1024.0 && u < units.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} B", v as i64)
    } else if v >= 100.0 {
        format!("{} {}", (v + 0.5).floor() as i64, units[u])
    } else {
        format!("{:.1} {}", v, units[u])
    }
}

fn item_line(it: &Value) -> String {
    let size = it.get("size").and_then(Value::as_f64).unwrap_or(0.0);
    let mtime = it.get("mtime").and_then(Value::as_f64).unwrap_or(0.0);
    let date = if mtime > 0.0 { crate::util::iso_string(mtime.min(8.64e15))[..10].to_string() } else { "unknown date".to_string() };
    let what = if it.get("dir") == Some(&json!(true)) { "folder".to_string() } else { it.get("kind").and_then(Value::as_str).unwrap_or("file").to_string() };
    format!("- {} {} [{what}, modified {date}]", size_text(size), str_of(it, "path"))
}

fn items_of(res: &Value) -> Vec<Value> {
    res.get("items").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn ask(host: &dyn ToolHost, call: Value, tool: &str) -> Result<Value, Value> {
    let Some(res) = desk_call(host, call) else { return Err(disk_fail("unavailable", None)) };
    if res.get("ok") != Some(&json!(true)) {
        return Err(disk_fail(str_of(&res, "error"), None));
    }
    if str_of(&res, "tool") != tool {
        return Err(disk_fail("failed", None));
    }
    Ok(res)
}

pub fn disk_search(args: &Args, host: &dyn ToolHost) -> Value {
    let Ok(query) = opt_str(args, "query", 1024) else { return disk_fail("invalid", Some("query")) };
    let kind = match args.get("kind") {
        None | Some(Value::Null) => None,
        Some(Value::String(k)) if KINDS.contains(&k.as_str()) => Some(k.as_str()),
        Some(_) => return disk_fail("invalid", Some("kind")),
    };
    let (Ok(min_size), Ok(max_size)) = (opt_int(args.get("minSize"), 0, 9_007_199_254_740_991), opt_int(args.get("maxSize"), 0, 9_007_199_254_740_991)) else { return disk_fail("invalid", Some("size")) };
    let Ok(days) = opt_int(args.get("olderThanDays"), 0, 36_500) else { return disk_fail("invalid", Some("days")) };
    let under = match opt_str(args, "under", 4096) {
        Ok(Some(u)) if is_abs_path(u) => Some(u),
        Ok(None) => None,
        _ => return disk_fail("invalid", Some("under")),
    };
    let Ok(limit) = opt_int(args.get("limit"), 1, SEARCH_MAX) else { return disk_fail("invalid", Some("limit")) };
    if !host.take("disk-read", READ_RATE_MAX, READ_RATE_MS) {
        return disk_fail("rate", None);
    }
    let mut call = Map::new();
    call.insert("tool".into(), json!("disk_search"));
    if let Some(q) = query {
        call.insert("query".into(), json!(q));
    }
    if let Some(k) = kind {
        call.insert("kind".into(), json!(k));
    }
    if let Some(n) = min_size {
        call.insert("minSize".into(), json!(n));
    }
    if let Some(n) = max_size {
        call.insert("maxSize".into(), json!(n));
    }
    if let Some(n) = days {
        call.insert("olderThanDays".into(), json!(n));
    }
    if let Some(u) = under {
        call.insert("under".into(), json!(u));
    }
    call.insert("limit".into(), json!(limit.unwrap_or(SEARCH_DEFAULT)));
    let res = match ask(host, Value::Object(call), "disk_search") {
        Ok(r) => r,
        Err(e) => return e,
    };
    let items = items_of(&res);
    let total = num_text(res.get("total"));
    if items.is_empty() {
        return tool_result(&format!("No entries match ({total} total)."), res, false);
    }
    let word = if total == "1" { "entry" } else { "entries" };
    let mut lines = vec![format!("{total} matching {word} ({} ms), showing {}, biggest first:", num_text(res.get("ms")), items.len())];
    lines.extend(items.iter().map(item_line));
    tool_result(&lines.join("\n"), res, false)
}

pub fn disk_usage(args: &Args, host: &dyn ToolHost) -> Value {
    let path = match args.get("path") {
        Some(Value::String(p)) if is_abs_path(p) => p.as_str(),
        _ => return disk_fail("invalid", Some("path")),
    };
    let Ok(limit) = opt_int(args.get("limit"), 1, USAGE_MAX) else { return disk_fail("invalid", Some("limit")) };
    if !host.take("disk-read", READ_RATE_MAX, READ_RATE_MS) {
        return disk_fail("rate", None);
    }
    let res = match ask(host, json!({ "tool": "disk_usage", "path": path, "limit": limit.unwrap_or(USAGE_DEFAULT) }), "disk_usage") {
        Ok(r) => r,
        Err(e) => return e,
    };
    let items = items_of(&res);
    let total = num_text(res.get("total"));
    let word = if total == "1" { "entry" } else { "entries" };
    let tail = if items.is_empty() { ".".to_string() } else { format!(", showing {}, biggest first:", items.len()) };
    let mut lines = vec![format!("{}: {} in total, {total} {word} directly inside{tail}", str_of(&res, "path"), size_text(res.get("size").and_then(Value::as_f64).unwrap_or(0.0)))];
    lines.extend(items.iter().map(item_line));
    tool_result(&lines.join("\n"), res, false)
}

pub fn disk_propose_delete(args: &Args, host: &dyn ToolHost) -> Value {
    let paths: Vec<&str> = match args.get("paths") {
        Some(Value::Array(a)) if !a.is_empty() && a.len() <= DELETE_MAX => {
            let mut out = Vec::new();
            for p in a {
                match p.as_str() {
                    Some(s) if is_abs_path(s) => out.push(s),
                    _ => return disk_fail("invalid", Some("paths")),
                }
            }
            out
        }
        _ => return disk_fail("invalid", Some("paths")),
    };
    let reason = match args.get("reason") {
        Some(Value::String(r)) if r.chars().count() <= REASON_MAX && !r.contains('\0') && !r.trim().is_empty() => r.as_str(),
        _ => return disk_fail("invalid", Some("reason")),
    };
    let permanent = match args.get("permanent") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return disk_fail("invalid", Some("permanent")),
    };
    if !host.take("disk-delete", DELETE_RATE_MAX, DELETE_RATE_MS) {
        return tool_result(error_text("rate-delete"), json!({ "ok": false, "error": "rate" }), true);
    }
    let res = match ask(host, json!({ "tool": "disk_propose_delete", "paths": paths, "reason": reason, "permanent": permanent }), "disk_propose_delete") {
        Ok(r) => r,
        Err(e) => return e,
    };
    let done = res.get("done").and_then(Value::as_array).cloned().unwrap_or_default();
    let failed = res.get("failed").and_then(Value::as_array).cloned().unwrap_or_default();
    if res.get("cancelled") == Some(&json!(true)) {
        return tool_result("The user declined: nothing was deleted.", res, false);
    }
    let mut tail: Vec<String> = Vec::new();
    if !failed.is_empty() {
        tail.push("Not removed:".to_string());
        for f in failed.iter().take(10) {
            let p = str_of(f, "path");
            tail.push(format!("- {}: {}", if p.is_empty() { "(invalid path)" } else { p }, str_of(f, "reason")));
        }
        if failed.len() > 10 {
            tail.push(format!("...and {} more", failed.len() - 10));
        }
    }
    if done.is_empty() {
        let mut lines = vec!["Nothing was deleted.".to_string()];
        lines.extend(tail);
        return tool_result(&lines.join("\n"), res, true);
    }
    let bin = if res.get("permanent") == Some(&json!(true)) { "deleted permanently" } else { "moved to the Recycle Bin / Trash" };
    let item = if done.len() == 1 { "item" } else { "items" };
    let mut lines = vec![format!("The user confirmed: {} {item} {bin}, {} freed.", done.len(), size_text(res.get("freed").and_then(Value::as_f64).unwrap_or(0.0)))];
    lines.extend(tail);
    tool_result(&lines.join("\n"), res, false)
}
