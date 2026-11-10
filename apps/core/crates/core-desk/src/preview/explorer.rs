use super::paths::{dirname, is_absolute, is_within, join, real_or_null, resolve_str};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

fn inside(root: &str, target: &str) -> Option<String> {
    real_or_null(target).filter(|r| is_within(root, r))
}

pub fn check_entry_name(name: &Value) -> Result<(), &'static str> {
    static DEV: OnceLock<Regex> = OnceLock::new();
    let Some(n) = name.as_str().filter(|n| !n.is_empty()) else { return Err("empty") };
    if n != n.trim_matches(super::branches::js_space) {
        return Err("space");
    }
    if n.chars().any(|c| (c as u32) <= 0x1f || c == '\x7f') {
        return Err("control");
    }
    if n.contains('/') || n.contains('\\') {
        return Err("separator");
    }
    if n.contains(['<', '>', ':', '"', '|', '?', '*']) {
        return Err("character");
    }
    if n == "." || n == ".." {
        return Err("reserved");
    }
    if n.ends_with('.') {
        return Err("dot");
    }
    if DEV.get_or_init(|| Regex::new(r"(?i)^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\.|$)").expect("regex")).is_match(n) {
        return Err("device");
    }
    if n.len() > 255 {
        return Err("long");
    }
    Ok(())
}

pub fn list_dir(root: &str, dir: &Value) -> Value {
    let Some(dir) = dir.as_str().filter(|d| is_absolute(d) && !d.contains('\0')) else { return Value::Null };
    let Some(real) = inside(root, dir) else { return Value::Null };
    let Ok(rd) = std::fs::read_dir(&real) else { return Value::Null };
    let mut rows = Vec::new();
    for e in rd.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().to_string();
        let full = join(&real, &name);
        let ft = e.file_type().ok();
        if ft.map(|t| t.is_symlink()).unwrap_or(false) {
            let target = real_or_null(&full);
            let directory = target.as_ref().and_then(|t| std::fs::metadata(t).ok()).map(|m| m.is_dir()).unwrap_or(false);
            let out = target.as_ref().map(|t| !is_within(root, t)).unwrap_or(true);
            rows.push(json!({"name": name, "path": full, "directory": directory, "outOfRoot": out}));
        } else {
            rows.push(json!({"name": name, "path": full, "directory": ft.map(|t| t.is_dir()).unwrap_or(false), "outOfRoot": false}));
        }
    }
    Value::Array(rows)
}

pub fn fs_failure_code(e: &std::io::Error) -> &'static str {
    #[cfg(windows)]
    let mapped = match e.raw_os_error() {
        Some(5) | Some(32) | Some(33) | Some(170) => Some("busy"),
        Some(2) | Some(3) => Some("missing"),
        Some(80) | Some(183) | Some(145) => Some("exists"),
        _ => None,
    };
    #[cfg(not(windows))]
    let mapped = match e.raw_os_error() {
        Some(1) | Some(16) => Some("busy"),
        Some(13) => Some("denied"),
        Some(2) => Some("missing"),
        Some(17) | Some(39) | Some(66) => Some("exists"),
        _ => None,
    };
    if let Some(m) = mapped {
        return m;
    }
    fs_failure_text(&e.to_string())
}

pub fn fs_failure_text(message: &str) -> &'static str {
    static NAMED: OnceLock<Regex> = OnceLock::new();
    if message.to_lowercase().contains("being used by another process") {
        return "busy";
    }
    match NAMED.get_or_init(|| Regex::new(r"\b(EPERM|EBUSY|EACCES|ENOENT|EEXIST|ENOTEMPTY)\b").expect("regex")).captures(message).map(|c| c[1].to_string()).as_deref() {
        Some("EPERM") | Some("EBUSY") => "busy",
        Some("EACCES") => "denied",
        Some("ENOENT") => "missing",
        Some("EEXIST") | Some("ENOTEMPTY") => "exists",
        _ => "failed",
    }
}

pub fn create_entry(root: &str, parent: &Value, name: &Value, kind: &Value) -> Value {
    if let Err(reason) = check_entry_name(name) {
        return json!({"ok": false, "error": "invalid", "reason": reason});
    }
    let kind = kind.as_str().filter(|k| *k == "file" || *k == "directory");
    let parent = parent.as_str().filter(|p| is_absolute(p));
    let (Some(kind), Some(parent)) = (kind, parent) else { return json!({"ok": false, "error": "denied"}) };
    let Some(dir) = inside(root, parent).filter(|d| std::fs::metadata(d).map(|m| m.is_dir()).unwrap_or(false)) else { return json!({"ok": false, "error": "denied"}) };
    let dest = join(&dir, name.as_str().unwrap_or(""));
    if !is_within(&dir, &dest) {
        return json!({"ok": false, "error": "denied"});
    }
    let res = if kind == "directory" { std::fs::create_dir(&dest) } else { std::fs::OpenOptions::new().write(true).create_new(true).open(&dest).map(|_| ()) };
    match res {
        Ok(()) => json!({"ok": true, "path": dest}),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => json!({"ok": false, "error": "exists"}),
        Err(_) => json!({"ok": false, "error": "failed"}),
    }
}

fn entry_in(root: &str, target: &Value) -> Option<String> {
    let t = target.as_str().filter(|t| is_absolute(t) && !t.contains('\0'))?;
    let abs = resolve_str(t);
    let name = crate::util::basename(&abs);
    if name.is_empty() || name == "." || name == ".." || t.split(['\\', '/']).any(|p| p == "..") {
        return None;
    }
    let parent = inside(root, &dirname(&abs))?;
    Some(join(&parent, &name))
}

pub fn rename_entry(root: &str, target: &Value, name: &Value) -> Value {
    if let Err(reason) = check_entry_name(name) {
        return json!({"ok": false, "error": "invalid", "reason": reason});
    }
    let Some(from) = entry_in(root, target) else { return json!({"ok": false, "error": "denied"}) };
    if std::fs::symlink_metadata(&from).is_err() {
        return json!({"ok": false, "error": "missing"});
    }
    let dest = join(&dirname(&from), name.as_str().unwrap_or(""));
    if dest == from {
        return json!({"ok": true, "path": dest});
    }
    if std::fs::symlink_metadata(&dest).is_ok() {
        let same = dest.to_lowercase() == from.to_lowercase() && real_or_null(&dest).is_some() && real_or_null(&dest) == real_or_null(&from);
        if !same {
            return json!({"ok": false, "error": "exists"});
        }
    }
    match std::fs::rename(&from, &dest) {
        Ok(()) => json!({"ok": true, "path": dest}),
        Err(e) => json!({"ok": false, "error": fs_failure_code(&e)}),
    }
}

pub fn trash_entry(root: &str, target: &Value, trash: &dyn Fn(&str) -> Result<(), String>) -> Value {
    let Some(from) = entry_in(root, target) else { return json!({"ok": false, "error": "denied"}) };
    if std::fs::symlink_metadata(&from).is_err() {
        return json!({"ok": false, "error": "missing"});
    }
    match trash(&from) {
        Ok(()) => json!({"ok": true}),
        Err(m) => {
            let why = fs_failure_text(&m);
            json!({"ok": false, "error": if why == "exists" { "failed" } else { why }})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_names() {
        assert!(check_entry_name(&json!("a.txt")).is_ok());
        for (bad, why) in [("", "empty"), (" a", "space"), ("a/b", "separator"), ("a:b", "character"), ("..", "reserved"), ("a.", "dot"), ("CON.txt", "device"), ("a\u{1}", "control")] {
            assert_eq!(check_entry_name(&json!(bad)), Err(why), "{bad}");
        }
        assert_eq!(fs_failure_text("Error: EBUSY: resource busy"), "busy");
        assert_eq!(fs_failure_text("weird"), "failed");
    }
}
