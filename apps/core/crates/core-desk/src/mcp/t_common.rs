use super::host::ToolHost;
use super::text::{mask_run, quote};
use serde_json::{json, Value};

pub fn page_error(code: &str) -> &'static str {
    match code {
        "unavailable" => "The Orrery window is not ready right now.",
        "caller" => "This session is not an Orrery workspace pane the window knows.",
        "limit" => "Orrery could not do that right now.",
        "target" => "No such session in this project. Call list_sessions for the current ones.",
        "busy" => "That session already has a prompt waiting; try again once the user has sent it.",
        _ => "Orrery could not do that.",
    }
}

pub fn ask(host: &dyn ToolHost, call: Value) -> Value {
    let res = host.page(call);
    if res.is_object() {
        res
    } else {
        json!({ "ok": false, "error": "failed" })
    }
}

pub fn refusal_text(r: &Value) -> String {
    let code = str_of(r, "code");
    let base = match code {
        "no-config" => "This project has no Run configuration. The user can add one in the Run pane.".to_string(),
        "config" => "No Run configuration of this project has that id; call run_status for the list.".to_string(),
        "device" => "That device is not one this configuration can run on right now; call run_status for the connected devices.".to_string(),
        "device-missing" => "No connected device for this configuration; ask the user to connect or start one.".to_string(),
        "device-unauthorized" => "The device has not authorised this computer for USB debugging yet.".to_string(),
        "running" => "That configuration is already running on that device. Use action reload (hot reload) or restart.".to_string(),
        "no-session" => "Nothing of this project is running.".to_string(),
        "not-running" => "The app is not in the running state yet; wait and check run_status.".to_string(),
        "waiting" => "Orrery is still on it (the user may be asked to trust the configuration, or the old run is stopping); check run_status in a few seconds.".to_string(),
        "run-pending" => "A start of that configuration is already in progress.".to_string(),
        "reload-unsupported" => "Hot reload works only for Flutter debug runs.".to_string(),
        "reload-failed" => "The hot reload failed; see run_log.".to_string(),
        "untrusted" => "The user declined to trust this shared Run configuration.".to_string(),
        "config-missing" => "That Run configuration no longer exists.".to_string(),
        "session-missing" => "That run already ended.".to_string(),
        "cwd-missing" => "The configuration's folder does not exist.".to_string(),
        "cwd-outside" => "The configuration's folder is outside the project.".to_string(),
        "no-device" => "No Android device is connected over adb.".to_string(),
        "several" => "More than one device is connected; pass serial.".to_string(),
        "not-connected" => "That device is not connected and ready.".to_string(),
        "adb-missing" => "adb was not found; the Android SDK is not set up.".to_string(),
        "secure" => "The app on screen blocks screenshots (secure window).".to_string(),
        "timeout" => "The device did not answer in time.".to_string(),
        "capture-failed" => "The device returned no screenshot.".to_string(),
        "write-failed" => "Orrery could not save the screenshot.".to_string(),
        other => format!("Orrery refused it ({other})."),
    };
    match r.get("detail").and_then(Value::as_str).filter(|d| !d.is_empty()) {
        Some(d) => format!("{base} ({})", mask_run(d)),
        None => base,
    }
}

pub fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(true),
        Some(_) => true,
    }
}

pub fn jnum(x: f64) -> Value {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 9e15 {
        json!(x as i64)
    } else {
        json!(x)
    }
}

pub fn num_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => match n.as_i64() {
            Some(i) => i.to_string(),
            None => match n.as_f64() {
                Some(f) if f.fract() == 0.0 && f.abs() < 1e21 => format!("{}", f as i128),
                Some(f) => format!("{f}"),
                None => n.to_string(),
            },
        },
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => "undefined".to_string(),
    }
}

pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn as_int(v: Option<&Value>) -> Option<i64> {
    let f = v?.as_f64()?;
    if f.is_finite() && f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_991.0 {
        Some(f as i64)
    } else {
        None
    }
}

pub fn mask_session(s: &Value) -> Value {
    if !s.is_object() {
        return Value::Null;
    }
    let mut out = s.clone();
    let name = mask_run(str_of(s, "configName"));
    out["configName"] = json!(name);
    if let Some(e) = s.get("lastError").and_then(Value::as_str).filter(|e| !e.is_empty()) {
        out["lastError"] = json!(mask_run(e));
    }
    out
}

pub fn session_line(s: &Value) -> String {
    let device_id = str_of(s, "deviceId");
    let on = if device_id.is_empty() {
        String::new()
    } else {
        let name = s.get("deviceName").and_then(Value::as_str).unwrap_or(device_id);
        format!(" on {} ({device_id})", quote(name))
    };
    let at = crate::util::iso_string(s.get("startedAt").and_then(Value::as_f64).unwrap_or(0.0).clamp(-8.64e15, 8.64e15));
    let err = match s.get("lastError").and_then(Value::as_str).filter(|e| !e.is_empty()) {
        Some(e) => format!("; last error: {e}"),
        None => String::new(),
    };
    let state = str_of(s, "state");
    let raw = str_of(s, "rawState");
    let raw_part = if state == raw { String::new() } else { format!(" ({raw})") };
    format!("{} [{}, {}]{on}: {state}{raw_part}, started {at}{err}", quote(str_of(s, "configName")), str_of(s, "configId"), str_of(s, "type"))
}
