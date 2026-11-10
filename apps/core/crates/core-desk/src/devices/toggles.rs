use crate::js;
use crate::run::android::jsre::{g, Js};
use serde_json::{json, Value};

pub fn is_package(v: &str) -> bool {
    static RE: Js = Js::new(r"^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$", "");
    js::len(v) <= 255 && RE.test(v)
}

pub fn is_activity(v: &str) -> bool {
    static RE: Js = Js::new(r"^\.?[A-Za-z_$][\w$]*(\.[A-Za-z_$][\w$]*)*$", "");
    js::len(v) <= 255 && RE.test(v)
}

pub fn is_launch_uri(v: &str) -> bool {
    static RE: Js = Js::new(r"^[A-Za-z][A-Za-z0-9+.-]*:\S+$", "");
    js::len(v) <= 2048 && RE.test(v) && !v.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f)
}

pub fn clean_launch(v: &Value) -> Option<Value> {
    let l = v.as_object()?;
    match l.get("kind").and_then(Value::as_str) {
        Some("uri") => {
            let uri = l.get("uri").and_then(Value::as_str).filter(|u| is_launch_uri(u))?;
            let pkg = match l.get("pkg") {
                None | Some(Value::Null) => Value::Null,
                Some(Value::String(p)) if is_package(p) => json!(p),
                _ => return None,
            };
            Some(json!({"kind": "uri", "uri": uri, "pkg": pkg}))
        }
        Some("activity") => {
            let pkg = l.get("pkg").and_then(Value::as_str).filter(|p| is_package(p))?;
            let activity = l.get("activity").and_then(Value::as_str).filter(|a| is_activity(a))?;
            Some(json!({"kind": "activity", "pkg": pkg, "activity": activity}))
        }
        _ => None,
    }
}

pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn launch_command(l: &Value) -> Vec<String> {
    if l["kind"] == json!("uri") {
        let mut out: Vec<String> = ["am", "start", "-W", "-a", "android.intent.action.VIEW", "-d"].iter().map(|s| s.to_string()).collect();
        out.push(sh_quote(l["uri"].as_str().unwrap_or("")));
        if let Some(p) = l["pkg"].as_str().filter(|p| !p.is_empty()) {
            out.push(p.to_string());
        }
        return out;
    }
    ["am", "start", "-W", "-n"].iter().map(|s| s.to_string()).chain([format!("{}/{}", l["pkg"].as_str().unwrap_or(""), l["activity"].as_str().unwrap_or(""))]).collect()
}

pub fn parse_launch(out: &str) -> (bool, Option<String>) {
    static ERR: Js = Js::new(r"^(Error(?: type \d+)?:.*|Exception occurred.*|java\.lang\.\w+.*)$", "m");
    static STATUS: Js = Js::new(r"Status:\s*ok", "i");
    static WARN: Js = Js::new(r"^Warning:\s*(.*)$", "m");
    if let Some(m) = ERR.exec(out) {
        return (false, Some(js::slice(g(&m, 1).unwrap_or("").trim(), 300)));
    }
    if STATUS.test(out) {
        let warn = WARN.exec(out).and_then(|m| g(&m, 1).map(|w| js::slice(w.trim(), 300)));
        return (true, warn);
    }
    let t = js::slice(out.trim(), 300);
    (false, Some(if t.is_empty() { "no output".into() } else { t }))
}

pub fn parse_density(out: &str) -> (Option<i64>, Option<i64>) {
    static PHYS: Js = Js::new(r"Physical density:\s*(\d+)", "i");
    static OVER: Js = Js::new(r"Override density:\s*(\d+)", "i");
    let num = |re: &Js| re.exec(out).and_then(|m| g(&m, 1).and_then(|s| s.parse::<i64>().ok()));
    (num(&PHYS), num(&OVER))
}

pub const ANIMATION_KEYS: [&str; 3] = ["window_animation_scale", "transition_animation_scale", "animator_duration_scale"];
pub const LOCALE_MIN_SDK: i64 = 33;

pub fn is_locale_tag(v: &str) -> bool {
    static TAG: Js = Js::new(r"^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$", "");
    v.is_empty() || (js::len(v) <= 35 && TAG.test(v))
}

pub fn is_locale_list(v: &str) -> bool {
    static TAG: Js = Js::new(r"^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*$", "");
    js::len(v) <= 200 && (v.is_empty() || v.split(',').all(|t| TAG.test(t)))
}

fn is_integer(v: &Value) -> bool {
    v.as_f64().is_some_and(|f| f.is_finite() && f.fract() == 0.0)
}

pub fn clean_change(v: &Value) -> Option<Value> {
    let c = v.as_object()?;
    match c.get("key").and_then(Value::as_str)? {
        "night" => match c.get("value").and_then(Value::as_str) {
            Some(x @ ("yes" | "no")) => Some(json!({"key": "night", "value": x})),
            _ => None,
        },
        "fontScale" => {
            let f = c.get("value").and_then(Value::as_f64).filter(|f| f.is_finite() && *f >= 0.5 && *f <= 3.0)?;
            Some(json!({"key": "fontScale", "value": crate::devices::history::num((f * 100.0).round() / 100.0)}))
        }
        "density" => match c.get("value") {
            Some(Value::Null) => Some(json!({"key": "density", "value": null})),
            Some(n) if is_integer(n) && n.as_f64().is_some_and(|f| (72.0..=1000.0).contains(&f)) => Some(json!({"key": "density", "value": n.as_f64().map(|f| f as i64)})),
            _ => None,
        },
        "animations" => c.get("off").and_then(Value::as_bool).map(|off| json!({"key": "animations", "off": off})),
        "locale" => {
            let pkg = c.get("pkg").and_then(Value::as_str).filter(|p| is_package(p))?;
            let value = c.get("value").and_then(Value::as_str).filter(|v| is_locale_tag(v))?;
            Some(json!({"key": "locale", "pkg": pkg, "value": value}))
        }
        _ => None,
    }
}

pub fn parse_night(out: &str) -> Option<String> {
    static RE: Js = Js::new(r"Night mode:\s*(\w+)", "i");
    let v = RE.exec(out).and_then(|m| g(&m, 1).map(|s| s.to_lowercase()))?;
    Some(if matches!(v.as_str(), "yes" | "no" | "auto") { v } else { "custom".to_string() })
}

pub fn parse_setting(out: &str) -> Option<String> {
    static NUM: Js = Js::new(r"^\d+(\.\d+)?$", "");
    let v = out.trim().split('\n').next_back().unwrap_or("").trim();
    (!v.is_empty() && v != "null" && NUM.test(v)).then(|| v.to_string())
}

pub fn parse_sdk(out: &str) -> Option<i64> {
    let t = out.trim();
    (!t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())).then(|| t.parse::<i64>().ok()).flatten().filter(|n| *n > 0)
}

pub fn parse_app_locales(out: &str) -> Option<String> {
    static RE: Js = Js::new(r"Locales for \S+ for user \d+ are \[([^\]]*)\]", "");
    RE.exec(out).and_then(|m| g(&m, 1).map(|s| s.split_whitespace().collect::<String>()))
}
