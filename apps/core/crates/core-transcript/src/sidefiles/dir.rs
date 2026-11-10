use super::common::file_name;
use super::hook::hook;
use super::status::status;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

const MAX_BYTES: u64 = 4096;
const HOOK_KEEP_MS: f64 = 24.0 * 3_600_000.0;
const STATUS_KEEP_MS: f64 = 7.0 * 24.0 * 3_600_000.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Hooks,
    Status,
}

fn keep_ms(kind: Kind) -> f64 {
    match kind {
        Kind::Hooks => HOOK_KEEP_MS,
        Kind::Status => STATUS_KEEP_MS,
    }
}

fn mtime_ms(meta: &fs::Metadata) -> Option<f64> {
    let d = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(d.as_secs_f64() * 1000.0)
}

fn parse(kind: Kind, bytes: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(bytes);
    let v: Value = serde_json::from_str(&text).ok()?;
    match kind {
        Kind::Hooks => hook(&v),
        Kind::Status => status(&v),
    }
}

pub fn read_dir(dir: &str, now: f64, kind: Kind) -> Result<Vec<Value>, &'static str> {
    let root = Path::new(dir);
    if !root.is_absolute() {
        return Err("dir");
    }
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| file_name(n))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let path = root.join(&name);
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if mtime_ms(&meta).is_some_and(|t| now - t > keep_ms(kind)) {
            let _ = fs::remove_file(&path);
            continue;
        }
        if meta.len() > MAX_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if let Some(r) = parse(kind, &bytes) {
            out.push(r);
        }
    }
    Ok(out)
}
