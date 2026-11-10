use super::deps::Deps;
use crate::capture::files::{is_orrery_file, shot_name, SHOT_MAX_BYTES, SHOT_SUB};
use crate::capture::folder::stamp;
use crate::preview::paths::is_within;
use crate::run::sessions::is_live;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const TEMP_SHOT_DIR: &str = "orrery-shots";
const TEMP_KEEP: usize = 30;

fn refuse(code: &str, devices: Option<Vec<Value>>) -> Value {
    let mut m = Map::new();
    m.insert("ok".into(), json!(true));
    m.insert("tool".into(), json!("device_screenshot"));
    m.insert("refused".into(), json!({"code": code}));
    if let Some(d) = devices {
        m.insert("devices".into(), Value::Array(d));
    }
    Value::Object(m)
}

const LENIENT: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true).with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

fn lenient_png(data: &str) -> Vec<u8> {
    let body = data.strip_prefix("data:image/png;base64,").unwrap_or(data);
    let clean: String = body.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/')).collect();
    let cut = if clean.len() % 4 == 1 { &clean[..clean.len() - 1] } else { &clean[..] };
    LENIENT.decode(cut.as_bytes()).unwrap_or_default()
}

pub fn write_temp_shot(tmp_root: &Path, png: &[u8], device: &str) -> Option<String> {
    let dir = tmp_root.join(TEMP_SHOT_DIR);
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = stamp();
    let mut file: Option<PathBuf> = None;
    for dup in 1..=50 {
        let target = dir.join(shot_name(device, &stamp, None, dup));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(mut f) => {
                use std::io::Write;
                if f.write_all(png).is_err() {
                    return None;
                }
                file = Some(target);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    let mut stats: Vec<(PathBuf, f64)> = std::fs::read_dir(&dir)
        .map(|r| r.flatten().collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|e| is_orrery_file(SHOT_SUB, &e.file_name().to_string_lossy()))
        .map(|e| {
            let at = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
            (e.path(), at)
        })
        .collect();
    stats.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (old, _) in stats.into_iter().skip(TEMP_KEEP) {
        if Some(&old) != file.as_ref() {
            let _ = std::fs::remove_file(old);
        }
    }
    file.map(|f| f.to_string_lossy().to_string())
}

pub fn screenshot(d: &dyn Deps, tmp_root: &Path, deadline_ms: u64, call: &Value) -> Value {
    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    let r = shoot(d, tmp_root, deadline, call);
    if Instant::now() >= deadline {
        return refuse("timeout", None);
    }
    r
}

fn shoot(d: &dyn Deps, tmp_root: &Path, deadline: Instant, call: &Value) -> Value {
    let Some(snap) = d.snapshot() else { return json!({"ok": false, "error": "unavailable"}) };
    if snap["adb"].is_null() || snap["adb"] == json!(false) || snap["adb"] == json!("") {
        return refuse("adb-missing", None);
    }
    let ready: Vec<&Value> = snap["devices"].as_array().map(|a| a.iter().filter(|x| x["online"] == json!(true) && x["state"] == "ready").collect()).unwrap_or_default();
    let listed: Vec<Value> = ready.iter().map(|x| json!({"serial": x["id"], "name": x["name"]})).collect();
    let id_of = |x: &&Value| x["id"].as_str().map(str::to_string);
    let wanted = call.get("serial").and_then(Value::as_str).filter(|s| !s.is_empty());
    let serial: String = if let Some(s) = wanted {
        if !ready.iter().any(|x| id_of(x).as_deref() == Some(s)) {
            return refuse("not-connected", Some(listed));
        }
        s.to_string()
    } else {
        let ids: Vec<String> = ready.iter().filter_map(id_of).collect();
        let cwd = call["cwd"].as_str().unwrap_or("");
        let mut runs: Vec<Value> = d
            .runs()
            .into_iter()
            .filter(|s| s["state"].as_str().is_some_and(is_live) && s["deviceId"].as_str().is_some_and(|id| !id.is_empty() && ids.iter().any(|i| i == id)) && s["cwd"].as_str().is_some_and(|c| is_within(cwd, c)))
            .collect();
        runs.sort_by(|a, b| b["startedAt"].as_f64().unwrap_or(0.0).total_cmp(&a["startedAt"].as_f64().unwrap_or(0.0)));
        match runs.first().and_then(|r| r["deviceId"].as_str().map(str::to_string)).or_else(|| (ids.len() == 1).then(|| ids[0].clone())) {
            Some(s) => s,
            None => return if ids.is_empty() { refuse("no-device", None) } else { refuse("several", Some(listed)) },
        }
    };
    let device = ready.iter().find(|x| id_of(x).as_deref() == Some(serial.as_str()));
    let cap = d.screencap(&serial);
    if cap["ok"] != json!(true) {
        let e = cap["error"].as_str().unwrap_or("");
        return refuse(if ["secure", "timeout", "adb-missing", "not-connected"].contains(&e) { e } else { "capture-failed" }, None);
    }
    let png = lenient_png(cap["png"].as_str().unwrap_or(""));
    if png.is_empty() || png.len() > SHOT_MAX_BYTES {
        return refuse("capture-failed", None);
    }
    if Instant::now() >= deadline {
        return refuse("timeout", None);
    }
    let label = device.and_then(|x| x["name"].as_str()).filter(|n| !n.is_empty()).unwrap_or(&serial).to_string();
    let active = d.capture_active();
    let file = if active { d.write_shot(&png, &label) } else { write_temp_shot(tmp_root, &png, &label) };
    let Some(file) = file else { return refuse("write-failed", None) };
    json!({"ok": true, "tool": "device_screenshot", "shot": {"path": file, "width": cap["width"], "height": cap["height"], "serial": serial, "name": label, "saved": if active { "capture" } else { "temp" }}})
}
