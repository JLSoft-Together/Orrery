#[path = "host_extra_text.rs"]
mod text;
#[path = "host_telegram.rs"]
mod telegram;
#[cfg(test)]
#[path = "host_extra_tests.rs"]
mod tests;

use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::desk_text::desk_text;
use crate::hosts::{confirm, locale};
use crate::state::{lock, Shell, MAIN};
use text::extra_text;

fn field<'a>(p: &'a Value, key: &str) -> &'a Value {
    p.get(key).unwrap_or(&Value::Null)
}

fn str_of<'a>(p: &'a Value, key: &str) -> &'a str {
    field(p, key).as_str().unwrap_or("")
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

fn ask(app: &AppHandle, (title, detail, ok): (String, String, String)) -> bool {
    let cancel = extra_text(&locale(app)).cancel();
    confirm(app, &title, &detail, &ok, &cancel)
}

pub fn ext_plan(p: &Value) -> (String, Vec<String>, Option<String>, Option<String>) {
    let kind = match str_of(p, "kind") {
        k @ ("install" | "remove" | "source") => k,
        _ => "install",
    };
    let scope = field(p, "scope").as_str().filter(|s| matches!(*s, "user" | "project" | "local")).map(str::to_owned);
    let dir = field(p, "dir").as_str().map(str::to_owned);
    (kind.to_owned(), strings(field(p, "commands")), scope, dir)
}

fn ext_confirm(app: &AppHandle, p: &Value) -> bool {
    let t = extra_text(&locale(app));
    let (kind, commands, scope, dir) = ext_plan(p);
    let place = t.ext_where(scope.as_deref(), dir.as_deref());
    ask(app, t.ext(&kind, &place, &commands))
}

fn res_confirm(app: &AppHandle, p: &Value) -> bool {
    let items = field(p, "items").as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        return false;
    }
    let loc = locale(app);
    let lines = desk_text(&loc).kill_lines(&items);
    ask(app, extra_text(&loc).res_kill(&items, &lines))
}

fn avd_stop(app: &AppHandle, p: &Value) -> bool {
    let avd = str_of(p, "avd");
    !avd.is_empty() && crate::desk::call_inside(app, "ccm:avd-stop", json!([avd])).is_ok_and(|v| v["ok"] == Value::Bool(true))
}

fn e2e_prompts() -> Option<String> {
    std::env::var("CCM_E2E_PROMPTS_FILE").ok().filter(|v| !v.is_empty())
}

fn prompts_dialog(app: &AppHandle, save: bool) -> Value {
    if let Some(f) = e2e_prompts() {
        return json!(f);
    }
    let (title, filter) = extra_text(&locale(app)).prompts(save);
    let mut d = app.dialog().file().set_title(title).add_filter(filter, &["json"]);
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    let picked = if save {
        if let Ok(docs) = app.path().document_dir() {
            d = d.set_directory(docs);
        }
        d.set_file_name("orrery-prompts.json").blocking_save_file()
    } else {
        d.blocking_pick_file()
    };
    picked.and_then(|p| p.into_path().ok()).map_or(Value::Null, |p| json!(p.to_string_lossy()))
}

fn previous(app: &AppHandle) -> Value {
    let unclean = crate::lifecycle::previous_unclean().unwrap_or_else(|| {
        let file = app.state::<Shell>().data_dir.join(core_desk::restore::RUN_STATE_FILE);
        core_desk::restore::read_previous_run(&file)
    });
    json!({ "unclean": unclean })
}

pub fn parse_ioreg_idle(out: &str) -> Option<f64> {
    let line = out.lines().find(|l| l.contains("\"HIDIdleTime\""))?;
    let ns: f64 = line.rsplit('=').next()?.trim().parse().ok()?;
    Some(ns / 1e9)
}

#[cfg(target_os = "macos")]
fn system_idle() -> f64 {
    std::process::Command::new("/usr/sbin/ioreg")
        .args(["-c", "IOHIDSystem", "-r", "-d", "1", "-k", "HIDIdleTime"])
        .output()
        .ok()
        .and_then(|o| parse_ioreg_idle(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(0.0)
}

#[cfg(windows)]
fn system_idle() -> f64 {
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        dw_time: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount() -> u32;
    }
    let mut info = LastInputInfo { cb_size: std::mem::size_of::<LastInputInfo>() as u32, dw_time: 0 };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return 0.0;
    }
    f64::from(unsafe { GetTickCount() }.wrapping_sub(info.dw_time)) / 1000.0
}

#[cfg(not(any(target_os = "macos", windows)))]
fn system_idle() -> f64 {
    0.0
}

pub fn e2e_idle(raw: Option<String>) -> Option<f64> {
    raw.filter(|v| !v.trim().is_empty()).and_then(|v| v.trim().parse::<f64>().ok()).filter(|n| n.is_finite() && *n >= 0.0)
}

fn idle_seconds() -> f64 {
    e2e_idle(std::env::var("CCM_E2E_IDLE_S").ok()).unwrap_or_else(system_idle)
}

fn bridge_cache() -> &'static Mutex<Option<Value>> {
    static CACHE: Mutex<Option<Value>> = Mutex::new(None);
    &CACHE
}

pub fn bridge_command(install: &Value, key: &str) -> Value {
    install[key]["command"].as_str().filter(|c| !c.is_empty()).map_or(Value::Null, |c| json!(c))
}

fn bridge(app: &AppHandle, key: &str) -> Value {
    let mut cache = lock(bridge_cache());
    if cache.is_none() {
        if let Ok(v) = crate::desk::call_inside(app, "desk:bridge-install", json!([{}])) {
            if v["ok"] == Value::Bool(true) {
                *cache = Some(v);
            }
        }
    }
    cache.as_ref().map_or(Value::Null, |v| bridge_command(v, key))
}

pub fn bridge_commands(app: &AppHandle) -> Value {
    json!({ "statusline": bridge(app, "statusline"), "hook": bridge(app, "hook") })
}

pub fn call(app: &AppHandle, caller: Option<&Webview>, method: &str, p: &Value) -> Option<Result<Value, String>> {
    let _ = caller;
    telegram::listen(app);
    if method.starts_with("telegram.") {
        return telegram::call(app, method, p);
    }
    let t = || extra_text(&locale(app));
    let v = match method {
        "extensions.confirm" => json!(ext_confirm(app, p)),
        "statusline.command" => bridge(app, "statusline"),
        "hooks.command" => bridge(app, "hook"),
        "statusline.confirm" => json!(ask(app, t().statusline(str_of(p, "file"), str_of(p, "planned"), field(p, "other") == &Value::Bool(true)))),
        "hooks.confirm" => json!(ask(app, t().hooks(str_of(p, "file"), str_of(p, "planned"), field(p, "on") == &Value::Bool(true)))),
        "claudeHooks.confirmWrite" => json!(ask(app, t().hook_toggle(str_of(p, "action") == "enable", str_of(p, "event"), str_of(p, "file"), str_of(p, "diff")))),
        "claudeHooks.confirmTest" => json!(ask(app, t().hook_test(str_of(p, "event"), str_of(p, "text"), str_of(p, "cwd")))),
        "github.confirmRemove" => json!(ask(app, t().gh_remove(str_of(p, "login")))),
        "resources.confirm" => json!(res_confirm(app, p)),
        "resources.avdStop" => json!(avd_stop(app, p)),
        "deepseek.confirm" => json!(ask(app, t().deepseek(p))),
        "restore.saveDialog" => prompts_dialog(app, true),
        "restore.openDialog" => prompts_dialog(app, false),
        "restore.previous" => previous(app),
        "disk.open" => json!(!str_of(p, "dir").is_empty() && app.opener().open_path(str_of(p, "dir"), None::<&str>).is_ok()),
        "phone.idle" => json!(idle_seconds()),
        "phone.locale" => json!(locale(app)),
        _ => return None,
    };
    Some(Ok(v))
}
