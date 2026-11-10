use core_store::extra::QuietHours;
use core_store::model::{TerminalSession, TerminalStatus};
use core_store::persist::jsstr::{js_round, slice_units, trim};
use core_store::persist::localtime::{parse_iso, Zone, DAY_MS};
use serde_json::Value;
use sha1::{Digest, Sha1};
use sha2::Sha256;

pub const LONG_WAIT_MS: f64 = 5.0 * 60_000.0;
pub const MAX_REMINDERS: f64 = 3.0;
pub const CONTEXT_CRITICAL: f64 = 95.0;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitReason {
    Permission,
    Question,
    Other,
}

pub fn wait_reason(waiting_for: Option<&str>) -> WaitReason {
    match waiting_for.map(|w| trim(w).to_lowercase()).as_deref() {
        Some("permission prompt" | "sandbox request" | "worker request") => WaitReason::Permission,
        Some("input needed" | "dialog open") => WaitReason::Question,
        _ => WaitReason::Other,
    }
}

pub fn date_ms(s: Option<&str>, zone: Zone) -> f64 {
    s.and_then(|v| parse_iso(v, zone)).map_or(f64::NAN, |v| v as f64)
}

pub fn local_minutes(now_ms: f64, zone: Zone) -> i64 {
    let ms = now_ms as i64;
    (ms + zone.offset_ms(ms)).rem_euclid(DAY_MS) / 60_000
}

fn minutes_of(hm: &str) -> i64 {
    let b = hm.as_bytes();
    if b.len() != 5 {
        return 0;
    }
    let d = |i: usize| i64::from(b[i].wrapping_sub(b'0'));
    (d(0) * 10 + d(1)) * 60 + d(3) * 10 + d(4)
}

pub fn in_quiet_hours(quiet: &QuietHours, now_ms: f64, zone: Zone) -> bool {
    if !quiet.enabled || quiet.from == quiet.to {
        return false;
    }
    let now = local_minutes(now_ms, zone);
    let (from, to) = (minutes_of(&quiet.from), minutes_of(&quiet.to));
    if from < to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

pub fn reminders_due(status_since: Option<&str>, now_ms: f64, zone: Zone) -> f64 {
    let since = date_ms(status_since, zone);
    if !since.is_finite() {
        return 0.0;
    }
    ((now_ms - since) / LONG_WAIT_MS).floor().clamp(0.0, MAX_REMINDERS)
}

pub fn quiet_for(t: &TerminalSession, now_ms: f64, threshold_min: f64, zone: Zone) -> Option<f64> {
    if threshold_min <= 0.0 || t.status != TerminalStatus::Working {
        return None;
    }
    let last = date_ms(t.last_activity_at.as_deref(), zone);
    if !last.is_finite() {
        return None;
    }
    let quiet = now_ms - last;
    (quiet >= threshold_min * 60_000.0).then_some(quiet)
}

pub fn to_fixed1(x: f64) -> String {
    let quarter = x * 4.0;
    if x >= 0.0 && quarter.fract() == 0.0 && quarter % 2.0 == 1.0 {
        return format!("{:.1}", (x * 10.0 + 0.5).floor() / 10.0);
    }
    format!("{x:.1}")
}

pub fn js_num(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if v.fract() == 0.0 && v.abs() < 1e21 {
        return format!("{}", v as i128);
    }
    format!("{v}")
}

pub fn js_value_str(v: Option<&Value>, missing: &str) -> String {
    match v {
        None | Some(Value::Null) => missing.into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.as_f64().map_or_else(|| n.to_string(), js_num),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

pub fn format_gb(bytes: f64) -> String {
    let gb = bytes / GB;
    if gb >= 100.0 {
        format!("{} GB", js_num(js_round(gb)))
    } else {
        format!("{} GB", to_fixed1(gb))
    }
}

pub fn format_bytes(bytes: f64, vi: bool) -> String {
    let gb = bytes / GB;
    let (value, unit) = if gb >= 1.0 { (to_fixed1(gb), "GB") } else { (js_num(js_round(bytes / (1024.0 * 1024.0)).max(1.0)), "MB") };
    let value = if vi { value.replacen('.', ",", 1) } else { value };
    format!("{value} {unit}")
}

pub fn toast_tag(key: &str) -> String {
    Sha1::digest(key.as_bytes()).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub fn same_key(a: &str, b: &str) -> bool {
    let (x, y) = (Sha256::digest(a.as_bytes()), Sha256::digest(b.as_bytes()));
    x.iter().zip(y.iter()).fold(0u8, |acc, (p, q)| acc | (p ^ q)) == 0
}

pub fn random_key() -> String {
    let mut buf = [0u8; 16];
    if getrandom::fill(&mut buf).is_err() {
        let seed = format!("{}:{}:{:?}", core_store::time::now_ms(), std::process::id(), std::thread::current().id());
        buf.copy_from_slice(&Sha256::digest(seed.as_bytes())[..16]);
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn name_of(t: &TerminalSession) -> String {
    let raw = [t.alias.as_deref(), Some(t.title.as_str())].into_iter().flatten().find(|s| !s.is_empty()).unwrap_or(&t.id);
    slice_units(raw, 80)
}

fn win32_basename(p: &str) -> &str {
    let rest = match p.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => &p[2..],
        _ => p,
    };
    rest.rsplit(['/', '\\']).next().unwrap_or("")
}

pub fn phone_name(t: &TerminalSession) -> String {
    if let Some(alias) = t.alias.as_deref().filter(|a| !a.is_empty()) {
        return slice_units(alias, 80);
    }
    let base = t.cwd.as_deref().filter(|c| !c.is_empty()).map(|c| win32_basename(c.trim_end_matches(['/', '\\']))).unwrap_or("");
    slice_units(if base.is_empty() { "Claude" } else { base }, 80)
}

pub fn server_label(s: &Value) -> String {
    let Some(d) = s.get("daemon").filter(|d| d.is_object()) else {
        let name = s.get("name").and_then(Value::as_str).unwrap_or("");
        let lower = name.to_ascii_lowercase();
        return if lower.ends_with(".exe") { name[..name.len() - 4].to_string() } else { name.to_string() };
    };
    let kind = d.get("type").and_then(Value::as_str).unwrap_or("");
    let label = match kind {
        "gradle" => "Gradle daemon",
        "kotlin" => "Kotlin daemon",
        "dart" => "Dart analysis server",
        "flutter" => "Flutter daemon",
        "emulator" => "Emulator",
        _ => "undefined",
    };
    let extra = d.get(if kind == "emulator" { "avd" } else { "version" }).and_then(Value::as_str).filter(|v| !v.is_empty());
    extra.map_or_else(|| label.to_string(), |e| format!("{label} {e}"))
}

pub fn local_hhmm(ms: f64, zone: Zone) -> String {
    let m = local_minutes(ms, zone);
    format!("{:02}:{:02}", m / 60, m % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn term(v: Value) -> TerminalSession {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn number_text_matches_js() {
        assert_eq!(to_fixed1(1.25), "1.3");
        assert_eq!(to_fixed1(1.75), "1.8");
        assert_eq!(to_fixed1(0.05), "0.1");
        assert_eq!(to_fixed1(8.0), "8.0");
        assert_eq!(format_gb(1.5 * GB), "1.5 GB");
        assert_eq!(format_gb(250.5 * GB), "251 GB");
        assert_eq!(format_bytes(1.25 * GB, true), "1,3 GB");
        assert_eq!(format_bytes(10.0, false), "1 MB");
        assert_eq!(format_bytes(300.0 * 1024.0 * 1024.0, false), "300 MB");
        assert_eq!(js_num(1_800_000_000_000.0), "1800000000000");
        assert_eq!(js_num(1.5), "1.5");
    }

    #[test]
    fn tags_links_and_keys() {
        assert_eq!(toast_tag("waiting:a").len(), 16);
        assert_eq!(toast_tag("test"), "a94a8fe5ccb19ba6");
        assert_eq!(encode_uri_component("a b/:é!*'()~"), "a%20b%2F%3A%C3%A9!*'()~");
        assert!(same_key("abc", "abc") && !same_key("abc", "ab"));
        let k = random_key();
        assert!(k.len() == 32 && k.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn names_for_toasts_and_phone() {
        assert_eq!(name_of(&term(json!({"id": "x", "title": "", "status": "idle"}))), "x");
        assert_eq!(name_of(&term(json!({"id": "x", "title": "T", "alias": "", "status": "idle"}))), "T");
        assert_eq!(phone_name(&term(json!({"id": "x", "title": "T", "status": "idle", "cwd": "D:\\work\\app\\"}))), "app");
        assert_eq!(phone_name(&term(json!({"id": "x", "title": "T", "status": "idle", "cwd": "/Users/me/proj"}))), "proj");
        assert_eq!(phone_name(&term(json!({"id": "x", "title": "T", "status": "idle", "cwd": "C:\\"}))), "Claude");
        assert_eq!(phone_name(&term(json!({"id": "x", "title": "T", "status": "idle"}))), "Claude");
    }

    #[test]
    fn waits_quiet_hours_and_labels() {
        assert_eq!(wait_reason(Some(" Permission Prompt ")), WaitReason::Permission);
        assert_eq!(wait_reason(Some("dialog open")), WaitReason::Question);
        assert_eq!(wait_reason(None), WaitReason::Other);
        let z = Zone::Fixed(0);
        let q = QuietHours { enabled: true, from: "22:00".into(), to: "08:00".into() };
        assert!(in_quiet_hours(&q, 23.0 * 3_600_000.0, z));
        assert!(in_quiet_hours(&q, 7.0 * 3_600_000.0, z));
        assert!(!in_quiet_hours(&q, 12.0 * 3_600_000.0, z));
        assert_eq!(local_hhmm(9.0 * 3_600_000.0 + 5.0 * 60_000.0, z), "09:05");
        assert_eq!(server_label(&json!({"name": "node.EXE"})), "node");
        assert_eq!(server_label(&json!({"name": "java", "daemon": {"type": "gradle", "version": "8.14"}})), "Gradle daemon 8.14");
        assert_eq!(server_label(&json!({"name": "qemu", "daemon": {"type": "emulator", "avd": "Pixel"}})), "Emulator Pixel");
        assert_eq!(server_label(&json!({"name": "dart", "daemon": {"type": "dart"}})), "Dart analysis server");
    }
}
