use crate::{js, util};
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Component, Path};

pub const CAPTURE_FILE: &str = "capture.json";
pub const REC_SUB: &str = "recordings";
pub const SHOT_SUB: &str = "shots";
pub const SHOT_MAX_BYTES: usize = 32 * 1024 * 1024;
const DAY_MS: f64 = 24.0 * 60.0 * 60_000.0;
pub const KEEP_DAYS: [i64; 5] = [1, 3, 7, 14, 30];
pub const MAX_GB: [i64; 5] = [1, 2, 5, 10, 20];

fn digits(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn is_rec_file(name: &str) -> bool {
    name.strip_prefix("orrery-rec-").and_then(|r| r.strip_suffix(".mp4")).is_some_and(|d| digits(d, 10, 16))
}

fn is_shot_file(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".png") else { return false };
    let mut parts: Vec<&str> = stem.split('-').collect();
    if parts.last().is_some_and(|p| p.len() >= 2 && p.starts_with('f') && digits(&p[1..], 1, 3)) {
        parts.pop();
    }
    if parts.len() >= 4 && digits(parts[parts.len() - 1], 1, 3) && digits(parts[parts.len() - 2], 6, 6) && digits(parts[parts.len() - 3], 8, 8) && shot_prefix(&parts[..parts.len() - 3]) {
        return true;
    }
    parts.len() >= 3 && digits(parts[parts.len() - 1], 6, 6) && digits(parts[parts.len() - 2], 8, 8) && shot_prefix(&parts[..parts.len() - 2])
}

fn shot_prefix(parts: &[&str]) -> bool {
    let p = parts.join("-");
    (1..=40).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

pub fn is_orrery_file(sub: &str, name: &str) -> bool {
    match sub {
        REC_SUB => is_rec_file(name),
        SHOT_SUB => is_shot_file(name),
        _ => false,
    }
}

pub fn sanitize_device(name: &str) -> String {
    let mut s = String::new();
    let mut run = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            s.push(c);
            run = false;
        } else if !run {
            s.push('-');
            run = true;
        }
    }
    let mut out = String::new();
    for c in s.chars() {
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    let trimmed = out.trim_matches(['-', '.']);
    let cut: String = trimmed.chars().take(40).collect();
    let cut = cut.trim_end_matches(['-', '.']);
    if cut.is_empty() {
        "device".into()
    } else {
        cut.to_string()
    }
}

pub fn shot_name(device: &str, stamp: &str, frame: Option<i64>, dup: u32) -> String {
    let d = if dup > 1 { format!("-{dup}") } else { String::new() };
    let f = frame.filter(|f| *f != 0).map(|f| format!("-f{f}")).unwrap_or_default();
    format!("{}-{stamp}{d}{f}.png", sanitize_device(device))
}

#[derive(Debug, Clone)]
pub struct CapturedFile {
    pub path: String,
    pub size: f64,
    pub mtime_ms: f64,
}

pub fn pick_expired(files: &[CapturedFile], now: f64, keep_days: f64, max_bytes: f64) -> Vec<CapturedFile> {
    let mut out = Vec::new();
    let mut kept = Vec::new();
    for f in files {
        if now - f.mtime_ms > keep_days * DAY_MS {
            out.push(f.clone());
        } else {
            kept.push(f.clone());
        }
    }
    kept.sort_by(|a, b| b.mtime_ms.partial_cmp(&a.mtime_ms).unwrap_or(std::cmp::Ordering::Equal));
    let mut total = 0.0;
    for f in kept {
        total += f.size;
        if total > max_bytes {
            out.push(f);
        }
    }
    out
}

fn win_abs(raw: &str) -> bool {
    let b = raw.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/') {
        return true;
    }
    let Some(rest) = raw.strip_prefix("\\\\") else { return false };
    let mut it = rest.splitn(3, ['\\', '/']);
    let (server, share) = (it.next().unwrap_or(""), it.next());
    !server.is_empty() && share.is_some_and(|s| !s.is_empty())
}

pub fn clean_dir(raw: &Value) -> Option<String> {
    let s = raw.as_str()?;
    if s.is_empty() || js::len(s) > 1024 || s.contains(['\0', '\r', '\n']) {
        return None;
    }
    if cfg!(windows) {
        if !win_abs(s) {
            return None;
        }
    } else if !s.starts_with('/') {
        return None;
    }
    Some(util::path_str(&util::resolve(s)))
}

fn parts(p: &str) -> Vec<String> {
    util::resolve(p)
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| {
            let s = c.as_os_str().to_string_lossy().to_string();
            if cfg!(windows) {
                s.to_lowercase()
            } else {
                s
            }
        })
        .collect()
}

pub fn is_inside(root: &str, file: &str) -> bool {
    let (a, b) = (parts(root), parts(file));
    b.len() > a.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y) && !b[a.len()].starts_with("..")
}

fn pick(r: &Map<String, Value>, k: &str, list: &[i64]) -> Option<Value> {
    let f = r.get(k)?.as_f64()?;
    list.iter().find(|n| **n as f64 == f).map(|n| json!(n))
}

pub fn parse_config(raw: &Value) -> Value {
    let empty = Map::new();
    let r = raw.as_object().unwrap_or(&empty);
    json!({
        "dir": clean_dir(r.get("dir").unwrap_or(&Value::Null)),
        "asFile": r.get("asFile").and_then(Value::as_bool).unwrap_or(true),
        "keepDays": pick(r, "keepDays", &KEEP_DAYS).unwrap_or(json!(7)),
        "maxGb": pick(r, "maxGb", &MAX_GB).unwrap_or(json!(2)),
    })
}

pub fn parse_patch(raw: &Value) -> Option<Map<String, Value>> {
    let r = raw.as_object()?;
    let mut out = Map::new();
    if let Some(d) = r.get("dir") {
        if !d.is_null() {
            return None;
        }
        out.insert("dir".into(), Value::Null);
    }
    if let Some(v) = r.get("asFile") {
        out.insert("asFile".into(), json!(v.as_bool()?));
    }
    if r.contains_key("keepDays") {
        out.insert("keepDays".into(), pick(r, "keepDays", &KEEP_DAYS)?);
    }
    if r.contains_key("maxGb") {
        out.insert("maxGb".into(), pick(r, "maxGb", &MAX_GB)?);
    }
    Some(out)
}

const LENIENT: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true).with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

pub fn decode_png(v: &Value) -> Option<Vec<u8>> {
    let prefix = "data:image/png;base64,";
    let s = v.as_str()?;
    if !s.starts_with(prefix) || js::len(s) as f64 > (SHOT_MAX_BYTES as f64 * 4.0) / 3.0 + prefix.len() as f64 + 4.0 {
        return None;
    }
    let clean: String = s[prefix.len()..].chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/')).collect();
    let cut = if clean.len() % 4 == 1 { &clean[..clean.len() - 1] } else { &clean[..] };
    let buf = LENIENT.decode(cut.as_bytes()).ok()?;
    (buf.len() > PNG_SIG.len() && buf[..8] == PNG_SIG).then_some(buf)
}

pub struct ShotReq {
    pub png: Vec<u8>,
    pub device: String,
    pub frame: Option<i64>,
}

pub fn parse_shot_request(raw: &Value) -> Option<ShotReq> {
    let r = raw.as_object()?;
    let device = r.get("device")?.as_str().filter(|d| js::len(d) <= 200)?.to_string();
    let frame = match r.get("frame") {
        None | Some(Value::Null) => None,
        Some(f) => {
            let n = f.as_f64().filter(|n| n.fract() == 0.0 && (1.0..=999.0).contains(n))?;
            Some(n as i64)
        }
    };
    let png = decode_png(r.get("png").unwrap_or(&Value::Null))?;
    Some(ShotReq { png, device, frame })
}

pub fn join(dir: &str, sub: &str) -> String {
    util::path_str(&Path::new(dir).join(sub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_rules() {
        assert!(is_rec_file("orrery-rec-1700000000000.mp4"));
        assert!(!is_rec_file("orrery-rec-17.mp4"));
        assert!(is_orrery_file(SHOT_SUB, "Pixel_7-20261110-213000.png"));
        assert!(is_orrery_file(SHOT_SUB, "Pixel_7-20261110-213000-2-f12.png"));
        assert!(is_orrery_file(SHOT_SUB, "a-b-20261110-213000-f3.png"));
        assert!(!is_orrery_file(SHOT_SUB, "x.png"));
        assert!(!is_orrery_file("other", "orrery-rec-1700000000000.mp4"));
        assert_eq!(sanitize_device("  Pixel 7 (API 34)!! "), "Pixel-7-API-34");
        assert_eq!(sanitize_device("..."), "device");
        assert_eq!(shot_name("Pixel 7", "20261110-213000", Some(3), 2), "Pixel-7-20261110-213000-2-f3.png");
        let f = |p: &str, size: f64, m: f64| CapturedFile { path: p.into(), size, mtime_ms: m };
        let gone = pick_expired(&[f("old", 1.0, 0.0), f("a", 6.0, 1e10 - 10.0), f("b", 6.0, 1e10 - 20.0)], 1e10, 1.0, 10.0);
        assert_eq!(gone.iter().map(|g| g.path.as_str()).collect::<Vec<_>>(), vec!["old", "b"]);
        let png = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2]));
        assert_eq!(decode_png(&json!(png)).unwrap().len(), 10);
        assert!(decode_png(&json!("data:image/png;base64,AAAA")).is_none());
        assert!(parse_patch(&json!({"dir": "/x"})).is_none());
        assert!(parse_patch(&json!({"keepDays": 2})).is_none());
        assert_eq!(parse_config(&json!({"keepDays": 3, "asFile": false})), json!({"dir": null, "asFile": false, "keepDays": 3, "maxGb": 2}));
        if !cfg!(windows) {
            assert_eq!(clean_dir(&json!("/a/b/../c/")).as_deref(), Some("/a/c"));
            assert!(clean_dir(&json!("rel")).is_none());
            assert!(is_inside("/a/shots", "/a/shots/x.png"));
            assert!(!is_inside("/a/shots", "/a/shots"));
            assert!(!is_inside("/a/shots", "/a/shots2/x"));
        }
    }
}
