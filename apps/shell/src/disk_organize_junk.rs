use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::disk_guard::{self as guard, GuardContext, Plat};
use crate::disk_map::DiskState;

const MAX_ITEMS: usize = 2000;
const SEARCH_LIMIT: usize = 10_000;
const SKIP_SEGMENTS: [&str; 11] = ["node_modules", "$recycle.bin", "system volume information", "windows", "program files", "program files (x86)", "__pycache__", "build", "dist", "target", "out"];
const APP_SEGMENTS: [&str; 3] = ["appdata", "library", "programdata"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Raw {
    pub path: String,
    pub name: String,
    pub dir: bool,
    pub count: Option<f64>,
    pub cloud: bool,
    pub link: bool,
    pub mount: bool,
    pub denied: bool,
    pub hidden: bool,
}

pub fn raw_items(v: &Value) -> Option<Vec<Raw>> {
    let items = v.get("items")?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|o| {
                let b = |k: &str| o.get(k) == Some(&Value::Bool(true));
                Some(Raw { path: o.get("path")?.as_str()?.to_owned(), name: o.get("name")?.as_str()?.to_owned(), dir: b("dir"), count: o.get("count").and_then(Value::as_f64), cloud: b("cloud"), link: b("link"), mount: b("mount"), denied: b("denied"), hidden: b("hidden") })
            })
            .collect(),
    )
}

pub fn skipped_path(p: &str, plat: Plat, skip_tree: &[String], allow_tree: &[String]) -> bool {
    let root = guard::root_of(p, plat);
    let rest = p.get(root.len()..).unwrap_or("");
    let allowed = allow_tree.iter().any(|a| !a.is_empty() && guard::is_under(p, a, plat));
    let bad = rest.split(['\\', '/']).filter(|s| !s.is_empty()).any(|s| {
        let l = s.to_lowercase();
        s.starts_with('.') || SKIP_SEGMENTS.contains(&l.as_str()) || (!allowed && APP_SEGMENTS.contains(&l.as_str()))
    });
    bad || skip_tree.iter().any(|k| !k.is_empty() && guard::is_under(p, k, plat))
}

fn u16_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]) as usize)
}

fn u32_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]) as usize)
}

fn ansi_at(b: &[u8], at: usize) -> Option<String> {
    let s = b.get(at..)?;
    let end = s.iter().position(|c| *c == 0)?;
    Some(String::from_utf8_lossy(&s[..end]).into_owned())
}

fn wide_at(b: &[u8], at: usize) -> Option<String> {
    let s = b.get(at..)?;
    let mut units = Vec::new();
    for ch in s.chunks_exact(2) {
        let u = u16::from_le_bytes([ch[0], ch[1]]);
        if u == 0 {
            return Some(String::from_utf16_lossy(&units));
        }
        units.push(u);
    }
    None
}

pub fn lnk_target(b: &[u8]) -> Option<String> {
    if u32_at(b, 0)? != 0x4C {
        return None;
    }
    let flags = u32_at(b, 0x14)?;
    let mut at = 0x4C;
    if flags & 0x1 != 0 {
        at += 2 + u16_at(b, at)?;
    }
    if flags & 0x2 == 0 {
        return None;
    }
    let info = b.get(at..)?;
    let size = u32_at(info, 0)?;
    let info = info.get(..size)?;
    let head = u32_at(info, 4)?;
    let lflags = u32_at(info, 8)?;
    let suffix = if head >= 0x24 && u32_at(info, 0x20)? > 0 { wide_at(info, u32_at(info, 0x20)?)? } else { ansi_at(info, u32_at(info, 0x18)?).unwrap_or_default() };
    let joined = |base: String| {
        if suffix.is_empty() {
            base
        } else if base.ends_with('\\') {
            format!("{base}{suffix}")
        } else {
            format!("{base}\\{suffix}")
        }
    };
    if lflags & 0x1 != 0 {
        let base = if head >= 0x24 && u32_at(info, 0x1C)? > 0 { wide_at(info, u32_at(info, 0x1C)?)? } else { ansi_at(info, u32_at(info, 0x10)?)? };
        return (!base.is_empty()).then(|| joined(base));
    }
    if lflags & 0x2 != 0 {
        let cn = info.get(u32_at(info, 0x14)?..)?;
        let net_off = u32_at(cn, 8)?;
        let net = if net_off > 0x14 && u32_at(cn, 0x14).is_some_and(|o| o > 0) { wide_at(cn, u32_at(cn, 0x14)?)? } else { ansi_at(cn, net_off)? };
        return (!net.is_empty()).then(|| joined(net));
    }
    None
}

fn read_shortcut(p: &str) -> Option<String> {
    let meta = std::fs::symlink_metadata(p).ok()?;
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return None;
    }
    lnk_target(&std::fs::read(p).ok()?)
}

fn drive_state(target: &str, plat: Plat, vols: &[(String, bool)]) -> &'static str {
    if plat == Plat::Win {
        if target.starts_with("\\\\") {
            return "unknown";
        }
        let b = target.as_bytes();
        if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || !(b[2] == b'\\' || b[2] == b'/') {
            return "unknown";
        }
        if vols.is_empty() {
            return "ok";
        }
        let letter = (b[0] as char).to_ascii_uppercase();
        return match vols.iter().find(|(m, _)| m.chars().next().map(|c| c.to_ascii_uppercase()) == Some(letter)) {
            Some((_, false)) => "ok",
            _ => "unknown",
        };
    }
    if !target.starts_with('/') {
        return "ok";
    }
    if let Some(rest) = target.strip_prefix("/Volumes/") {
        let vol = format!("/Volumes/{}", rest.split('/').next().unwrap_or(""));
        if !vols.is_empty() && !vols.iter().any(|(m, _)| *m == vol) {
            return "unknown";
        }
    }
    if target.starts_with("/net/") || target.starts_with("/Network/") {
        return "unknown";
    }
    match vols.iter().find(|(m, _)| m != "/" && guard::is_under(target, m, plat)) {
        Some((_, true)) => "unknown",
        _ => "ok",
    }
}

fn gone_code(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::NotFound || e.raw_os_error() == Some(if cfg!(windows) { 267 } else { 20 })
}

pub fn scan(app: &AppHandle, app_dirs: &[String]) -> Value {
    let none = json!({ "indexed": false, "items": [] });
    let Some(state) = app.try_state::<DiskState>() else { return none };
    let m = state.0.clone();
    let Ok(st) = m.status() else { return none };
    if st["indexed"] != json!(true) || st["scanning"] == json!(true) {
        return none;
    }
    let plat = Plat::host();
    let vol_list = m.volumes();
    let vols: Vec<(String, bool)> = vol_list.as_array().map(|a| a.iter().filter_map(|v| Some((v["mount"].as_str()?.to_owned(), v["network"] == json!(true)))).collect()).unwrap_or_default();
    let ctx = GuardContext { home: crate::disk_map::home_of(app), keep: crate::disk_map::keep_of(app), roots: crate::disk_map::status_roots(&st), volumes: vols.iter().map(|(m, _)| m.clone()).filter(|m| !m.is_empty()).collect() };
    let allow = vec![std::env::temp_dir().to_string_lossy().into_owned()];
    let allowed = |p: &str| !skipped_path(p, plat, app_dirs, &allow) && guard::check_deletable(p, &ctx, plat).is_none();
    let search = |q: Value| -> Option<Vec<Raw>> {
        if !m.host.running() {
            return None;
        }
        m.host.call("disk.search", q).ok().and_then(|v| raw_items(&v))
    };
    let mut items: Vec<Value> = Vec::new();
    let mut truncated = false;
    let Some(dirs) = search(json!({ "only": "dir", "maxSize": 0, "sort": "name", "limit": SEARCH_LIMIT })) else { return none };
    if dirs.len() >= SEARCH_LIMIT {
        truncated = true;
    }
    for d in dirs.iter().filter(|d| d.dir && d.count == Some(0.0) && !d.link && !d.mount && !d.denied && !d.hidden && !d.cloud && allowed(&d.path)) {
        let ok = std::fs::symlink_metadata(&d.path).is_ok_and(|s| s.is_dir() && !s.file_type().is_symlink()) && std::fs::read_dir(&d.path).is_ok_and(|mut r| r.next().is_none());
        if ok {
            items.push(json!({ "path": d.path, "name": d.name, "kind": "empty", "state": "empty" }));
        }
    }
    if plat == Plat::Win {
        let links = search(json!({ "query": "*.lnk", "only": "file", "sort": "name", "limit": SEARCH_LIMIT })).unwrap_or_default();
        if links.len() >= SEARCH_LIMIT {
            truncated = true;
        }
        for l in links.iter().filter(|l| !l.dir && l.name.to_lowercase().ends_with(".lnk") && !l.cloud && !l.denied && allowed(&l.path)) {
            let Some(target) = read_shortcut(&l.path) else { continue };
            if drive_state(&target, plat, &vols) == "unknown" {
                items.push(json!({ "path": l.path, "name": l.name, "kind": "shortcut", "state": "unknown", "target": target }));
                continue;
            }
            if let Err(e) = std::fs::metadata(&target) {
                items.push(json!({ "path": l.path, "name": l.name, "kind": "shortcut", "state": if gone_code(&e) { "broken" } else { "unknown" }, "target": target }));
            }
        }
    } else if plat == Plat::Mac {
        let files = search(json!({ "only": "file", "maxSize": 0, "sort": "name", "limit": SEARCH_LIMIT })).unwrap_or_default();
        if files.len() >= SEARCH_LIMIT {
            truncated = true;
        }
        for f in files.iter().filter(|f| f.link && !f.dir && allowed(&f.path)) {
            if !std::fs::symlink_metadata(&f.path).is_ok_and(|s| s.file_type().is_symlink()) {
                continue;
            }
            let Ok(raw) = std::fs::read_link(&f.path) else { continue };
            let raw = raw.to_string_lossy().into_owned();
            let target = if raw.starts_with('/') { guard::normalize(&raw, plat) } else { guard::normalize(&guard::join(&guard::parent(&f.path, plat), &raw, plat), plat) };
            if drive_state(&target, plat, &vols) == "unknown" {
                items.push(json!({ "path": f.path, "name": f.name, "kind": "shortcut", "state": "unknown", "target": target }));
                continue;
            }
            if let Err(e) = std::fs::metadata(&f.path) {
                if e.kind() == std::io::ErrorKind::NotFound {
                    items.push(json!({ "path": f.path, "name": f.name, "kind": "shortcut", "state": "broken", "target": target }));
                }
            }
        }
    }
    if items.len() > MAX_ITEMS {
        truncated = true;
        items.truncate(MAX_ITEMS);
    }
    let mut out = json!({ "indexed": true, "items": items });
    if truncated {
        out["truncated"] = json!(true);
    }
    out
}
