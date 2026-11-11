use std::path::Path;
use std::time::Duration;

use core_desk::preview::gitmisc::image_mime;
use core_desk::preview::paths::{is_absolute, real_or_null};
use serde_json::{json, Value};
use tauri::{AppHandle, Webview};

use crate::hosts;
use crate::state::MAIN;

const RUN_USE_GAP: Duration = Duration::from_secs(30);
const KIT_HOST: &str = "dl.google.com";

fn field<'a>(p: &'a Value, key: &str) -> &'a Value {
    p.get(key).unwrap_or(&Value::Null)
}

fn text(p: &Value, key: &str) -> String {
    match field(p, key) {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn owner_key(label: &str, key: &str) -> String {
    format!("{label}:{key}")
}

#[derive(Debug, PartialEq)]
pub struct Dialog {
    pub message: String,
    pub detail: String,
    pub ok: &'static str,
    pub cancel: &'static str,
}

fn cancel(vi: bool) -> &'static str {
    if vi {
        "Huỷ"
    } else {
        "Cancel"
    }
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

fn trust_detail(vi: bool, cwd: &str, command: &str, before: &[String], env_keys: &[String]) -> String {
    let command = if command.is_empty() { "—" } else { command };
    let mut lines = Vec::new();
    if vi {
        lines.push("Cấu hình này đến từ repository (dùng chung hoặc được import). Chỉ chạy khi bạn tin người viết nó.".to_owned());
        lines.push(format!("Thư mục: {cwd}"));
        lines.push(format!("Lệnh: {command}"));
    } else {
        lines.push("This configuration comes from the repository (shared or imported). Run it only if you trust its author.".to_owned());
        lines.push(format!("Folder: {cwd}"));
        lines.push(format!("Command: {command}"));
    }
    if !before.is_empty() {
        lines.push((if vi { "Trước khi chạy:" } else { "Before launch:" }).to_owned());
        lines.extend(before.iter().map(|l| format!("  {l}")));
    }
    if !env_keys.is_empty() {
        lines.push(format!("{}: {}", if vi { "Biến môi trường" } else { "Environment variables" }, env_keys.join(", ")));
    }
    lines.push((if vi { "Mỗi khi cấu hình thay đổi, app sẽ hỏi lại." } else { "You are asked again whenever the configuration changes." }).to_owned());
    lines.join("\n")
}

pub fn run_confirm_spec(vi: bool, code: &str, detail: Option<&str>) -> Option<Dialog> {
    match code {
        "reinstall" => {
            let pkg = detail.unwrap_or("");
            Some(if vi {
                Dialog { message: "Gỡ bản cũ và cài lại?".into(), detail: format!("{pkg} trên máy được ký bằng key khác. Cài lại sẽ xoá dữ liệu của app trên thiết bị."), ok: "Gỡ và cài lại", cancel: cancel(vi) }
            } else {
                Dialog { message: "Uninstall the old build and reinstall?".into(), detail: format!("The installed {pkg} was signed with a different key. Reinstalling deletes its data on the device."), ok: "Uninstall and reinstall", cancel: cancel(vi) }
            })
        }
        "trust-config" => {
            let plan: Value = serde_json::from_str(detail?).ok()?;
            let (name, cwd, command) = (plan.get("name")?.as_str()?, plan.get("cwd")?.as_str()?, plan.get("command")?.as_str()?);
            let detail = trust_detail(vi, cwd, command, &strs(field(&plan, "beforeLaunch")), &strs(field(&plan, "envKeys")));
            Some(if vi {
                Dialog { message: format!("Chạy \"{name}\" từ file của project?"), detail, ok: "Tin và chạy", cancel: cancel(vi) }
            } else {
                Dialog { message: format!("Run \"{name}\" from the project files?"), detail, ok: "Trust and run", cancel: cancel(vi) }
            })
        }
        _ => None,
    }
}

pub fn layout_confirm_spec(vi: bool, bytes: f64, host: &str) -> Dialog {
    let mb = if bytes.is_finite() { (bytes / 1_000_000.0).round() as i64 } else { 0 };
    let host = if host.is_empty() { KIT_HOST } else { host };
    if vi {
        Dialog {
            message: "Tải bộ dựng layout Android?".into(),
            detail: format!("Xem trước layout XML cần layoutlib của Google và render daemon (khoảng {mb} MB) từ {host}. Tải một lần qua HTTPS, kiểm tra theo SHA-256 cố định và lưu trong thư mục dữ liệu của app."),
            ok: "Tải về",
            cancel: cancel(vi),
        }
    } else {
        Dialog {
            message: "Download the Android layout renderer?".into(),
            detail: format!("The XML layout preview needs Google's layoutlib and the render daemon (about {mb} MB) from {host}. It is downloaded once over HTTPS, checked against pinned SHA-256 hashes and stored in the app data folder."),
            ok: "Download",
            cancel: cancel(vi),
        }
    }
}

fn is_vi(app: &AppHandle) -> bool {
    hosts::locale(app).to_lowercase().starts_with("vi")
}

fn ask(app: &AppHandle, d: &Dialog) -> bool {
    hosts::confirm(app, &d.message, &d.detail, d.ok, d.cancel)
}

fn run_confirm(app: &AppHandle, p: &Value) -> bool {
    let Some(code) = field(p, "code").as_str() else { return false };
    run_confirm_spec(is_vi(app), code, field(p, "detail").as_str()).is_some_and(|d| ask(app, &d))
}

fn layout_confirm(app: &AppHandle, p: &Value) -> bool {
    let bytes = field(p, "bytes").as_f64().unwrap_or(0.0);
    ask(app, &layout_confirm_spec(is_vi(app), bytes, &text(p, "host")))
}

fn layout_url(p: &Value) -> Value {
    let roots: Vec<String> = strs(field(p, "roots")).iter().filter_map(|r| real_or_null(r)).collect();
    let file = text(p, "file");
    if !is_absolute(&file) || !Path::new(&file).is_file() || image_mime(&file).is_none() {
        return Value::Null;
    }
    crate::preview_serve::url_for(&file, &roots).map_or(Value::Null, |url| json!({"url": url}))
}

fn gate_file(app: &AppHandle, p: &Value) -> Result<Value, String> {
    crate::preview_host::call(app, "preview.allowed", &json!({"path": field(p, "path")})).map(|v| v.get("real").cloned().unwrap_or(Value::Null))
}

pub fn call(app: &AppHandle, caller: Option<&Webview>, method: &str, p: &Value) -> Option<Result<Value, String>> {
    Some(Ok(match method {
        "run.owner" => json!(owner_key(caller.map_or(MAIN, |w| w.label()), &text(p, "key"))),
        "run.confirm" => json!(run_confirm(app, p)),
        "run.adbUse" => {
            if let Some(serial) = field(p, "serial").as_str().filter(|s| !s.is_empty()) {
                crate::host_devices::device_used(app, serial, RUN_USE_GAP);
            }
            json!(true)
        }
        "layout.urlFor" => layout_url(p),
        "layout.confirm" => json!(caller.is_some() && layout_confirm(app, p)),
        "gate.dir" => return Some(crate::preview_host::call(app, "preview.explorerRoot", &json!({"root": field(p, "dir")}))),
        "gate.file" => return Some(gate_file(app, p)),
        _ => return None,
    }))
}

#[cfg(test)]
#[path = "host_run_tests.rs"]
mod tests;
