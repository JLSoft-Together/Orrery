use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};
use tauri_plugin_notification::NotificationExt;

use crate::desk_text::desk_text;
use crate::state::{lock, Shell, MAIN};
use crate::vault::{UnsealError, Vault};
use crate::window::show_main;

pub const LOCAL_SNAPSHOT_JS: &str =
    "(() => { const o = {}; for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k && k.startsWith('ccm.')) o[k] = localStorage.getItem(k) } return o })()";
const READ_LOCAL: Duration = Duration::from_secs(3);

fn field<'a>(p: &'a Value, key: &str) -> &'a Value {
    p.get(key).unwrap_or(&Value::Null)
}

fn num(v: &Value) -> i64 {
    v.as_f64().filter(|n| n.is_finite()).map_or(0, |n| n as i64)
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

pub fn eval_value(webview: &Webview, js: &str, timeout: Duration) -> Option<Value> {
    let (tx, rx) = mpsc::channel();
    webview
        .eval_with_callback(js, move |raw| {
            let _ = tx.send(raw);
        })
        .ok()?;
    let raw = rx.recv_timeout(timeout).ok()?;
    serde_json::from_str(&raw).ok()
}

fn page(app: &AppHandle, caller: Option<&Webview>) -> Option<Webview> {
    caller.cloned().or_else(|| app.get_webview(MAIN))
}

pub fn read_local(app: &AppHandle, caller: Option<&Webview>) -> Value {
    page(app, caller).and_then(|w| eval_value(&w, LOCAL_SNAPSHOT_JS, READ_LOCAL)).filter(Value::is_object).unwrap_or_else(|| json!({}))
}

fn e2e_file() -> Option<String> {
    std::env::var("CCM_E2E_BACKUP_FILE").ok().filter(|v| !v.is_empty())
}

fn file_dialog(app: &AppHandle) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    let mut d = app.dialog().file().add_filter("Orrery backup", &["json"]);
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    d
}

fn as_path(p: Option<tauri_plugin_dialog::FilePath>) -> Value {
    p.and_then(|p| p.into_path().ok()).map_or(Value::Null, |p| json!(p.to_string_lossy()))
}

fn save_dialog(app: &AppHandle, p: &Value) -> Value {
    if let Some(f) = e2e_file() {
        return json!(f);
    }
    let mut d = file_dialog(app);
    if let Some(default) = field(p, "defaultPath").as_str().map(PathBuf::from) {
        if let Some(dir) = default.parent().filter(|d| d.is_dir()) {
            d = d.set_directory(dir);
        }
        if let Some(name) = default.file_name() {
            d = d.set_file_name(name.to_string_lossy());
        }
    }
    as_path(d.blocking_save_file())
}

fn open_dialog(app: &AppHandle) -> Value {
    if let Some(f) = e2e_file() {
        return json!(f);
    }
    as_path(file_dialog(app).blocking_pick_file())
}

fn e2e_answer(app: &AppHandle, message: &str) -> Option<bool> {
    let answer = std::env::var("CCM_E2E_CONFIRM").ok()?;
    let log = app.state::<Shell>().data_dir.join("e2e-confirms.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log) {
        let _ = writeln!(f, "{}", message.replace('\n', " "));
    }
    Some(answer == "ok")
}

pub fn confirm(app: &AppHandle, message: &str, detail: &str, ok: &str, cancel: &str) -> bool {
    if let Some(answer) = crate::remote::scope::confirm(message, detail, ok, cancel) {
        return answer;
    }
    if let Some(answer) = e2e_answer(app, message) {
        return answer;
    }
    if app.get_webview_window(MAIN).is_some_and(|w| !w.is_visible().unwrap_or(true)) {
        show_main(app);
    }
    let mut d = app
        .dialog()
        .message(format!("{message}\n\n{detail}"))
        .title("Orrery")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(ok.to_owned(), cancel.to_owned()));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.parent(&w);
    }
    d.blocking_show()
}

pub fn locale(app: &AppHandle) -> String {
    lock(&app.state::<Shell>().look).locale().to_owned()
}

fn confirm_apply(app: &AppHandle, p: &Value) -> bool {
    let ids = strings(field(p, "sections"));
    if ids.is_empty() {
        return false;
    }
    let t = desk_text(&locale(app));
    let names: Vec<String> = ids.iter().map(|id| t.section(id)).collect();
    confirm(app, &t.backup_title(ids.len()), &t.backup_detail(&names), t.backup_confirm, t.cancel)
}

fn build_confirm(app: &AppHandle, p: &Value) -> i64 {
    let t = desk_text(&locale(app));
    let trash = field(p, "trash") == &Value::Bool(true);
    let size = match field(p, "size") {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    let message = t.build_title(num(field(p, "dirs")), &size);
    let detail = t.build_detail(&strings(field(p, "lines")), num(field(p, "more")), field(p, "reinstall") == &Value::Bool(true), trash);
    let ok = if trash { t.build_confirm_trash } else { t.build_confirm };
    if confirm(app, &message, &detail, ok, t.cancel) {
        0
    } else {
        1
    }
}

fn mcp_confirm_kill(app: &AppHandle, p: &Value) -> bool {
    let t = desk_text(&locale(app));
    let items = field(p, "items").as_array().cloned().unwrap_or_default();
    let pane = field(p, "pane").as_str().unwrap_or("");
    confirm(app, &t.free_port_title(pane, num(field(p, "port"))), &t.free_port_detail(&t.kill_lines(&items)), t.free_port_confirm(), t.cancel)
}

fn mcp_confirm(app: &AppHandle, p: &Value) -> i64 {
    let t = desk_text(&locale(app));
    let pane = field(p, "pane").as_str().unwrap_or("");
    let owner = field(p, "owner").as_str().unwrap_or("");
    if confirm(app, &t.tg_receive_title(pane), &t.tg_receive_detail(owner), t.tg_receive_confirm(), t.cancel) {
        0
    } else {
        1
    }
}

fn trash(p: &Value) -> bool {
    let Some(file) = field(p, "path").as_str().map(Path::new).filter(|f| f.is_absolute()) else { return false };
    trash::delete(file).is_ok()
}

fn agent_confirm(app: &AppHandle, p: &Value) -> i64 {
    let Some((message, detail, buttons)) = desk_text(&locale(app)).agent_data_spec(p) else { return 1 };
    let last = buttons.len() as i64 - 1;
    if buttons.len() == 2 {
        return if confirm(app, &message, &detail, &buttons[0], &buttons[1]) { 0 } else { 1 };
    }
    if let Some(answer) = e2e_answer(app, &message) {
        return if answer { 0 } else { last };
    }
    if app.get_webview_window(MAIN).is_some_and(|w| !w.is_visible().unwrap_or(true)) {
        show_main(app);
    }
    let mut d = app
        .dialog()
        .message(format!("{message}\n\n{detail}"))
        .title("Orrery")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(buttons[0].clone(), buttons[1].clone(), buttons[2].clone()));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.parent(&w);
    }
    match d.blocking_show_with_result() {
        MessageDialogResult::Yes => 0,
        MessageDialogResult::No => 1,
        MessageDialogResult::Custom(s) if s == buttons[0] => 0,
        MessageDialogResult::Custom(s) if s == buttons[1] => 1,
        _ => last,
    }
}

fn agent_pick(app: &AppHandle, p: &Value) -> Value {
    let purpose = match field(p, "purpose").as_str() {
        Some(k @ ("target" | "backup")) => k,
        _ => return Value::Null,
    };
    let forced = std::env::var(if purpose == "target" { "CCM_E2E_AGENT_TARGET" } else { "CCM_E2E_AGENT_BACKUP" }).ok().filter(|v| !v.is_empty());
    if let Some(dir) = forced {
        return json!(dir);
    }
    let mut d = app.dialog().file().set_title(desk_text(&locale(app)).agent_pick_title(purpose));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    as_path(d.blocking_pick_folder())
}

fn relaunch(app: &AppHandle) {
    let shell = app.state::<Shell>();
    shell.quitting.store(true, Ordering::SeqCst);
    shell.shutdown();
    app.restart();
}

fn keychain(app: &AppHandle, method: &str, p: &Value) -> Result<Value, String> {
    let vault = app.try_state::<Vault>().ok_or("no keychain")?;
    let text = field(p, if method == "keychain.seal" { "plain" } else { "sealed" }).as_str().unwrap_or_default();
    match method {
        "keychain.available" => Ok(json!(vault.available())),
        "keychain.seal" => vault.seal(text).map(|s| json!(s)).ok_or_else(|| "seal failed".into()),
        _ => vault.unseal(text).map(|s| json!(s)).map_err(|e| {
            if e == UnsealError::Legacy && vault.first_warning() {
                let t = app.state::<Shell>().t();
                let _ = app.notification().builder().title(t.app_name).body(t.tokens_lost).show();
            }
            "unseal failed".into()
        }),
    }
}

pub fn call(app: &AppHandle, caller: Option<&Webview>, method: &str, p: &Value) -> Result<Value, String> {
    if let Some(v) = crate::update_host::call(app, method, p) {
        return Ok(v);
    }
    for module in [crate::host_run::call, crate::host_devices::call, crate::host_extra::call] {
        if let Some(r) = module(app, caller, method, p) {
            return r;
        }
    }
    match method {
        "keychain.available" | "keychain.seal" | "keychain.unseal" => keychain(app, method, p),
        "backup.readLocal" => Ok(read_local(app, caller)),
        "backup.github" | "buildClean.running" | "buildClean.daemons" => Ok(json!([])),
        "backup.saveDialog" => Ok(save_dialog(app, p)),
        "backup.openDialog" => Ok(open_dialog(app)),
        "backup.confirmApply" => Ok(json!(confirm_apply(app, p))),
        "backup.relaunch" => {
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                relaunch(&handle);
            });
            Ok(json!(true))
        }
        "buildClean.trash" => Ok(json!(trash(p))),
        "buildClean.confirm" => Ok(json!(build_confirm(app, p))),
        "mcpDesk.confirmKill" => Ok(json!(mcp_confirm_kill(app, p))),
        "mcpDesk.confirm" => Ok(json!(mcp_confirm(app, p))),
        "agentData.confirm" => Ok(json!(agent_confirm(app, p))),
        "agentData.pickFolder" => Ok(agent_pick(app, p)),
        "agentData.stopCollector" => Ok(crate::collector::stop(app)),
        "agentData.startCollector" => Ok(crate::collector::start(app)),
        "remote.link" => Ok(crate::remote::pane_link::call(app, p)),
        "telegram.notice" => Ok(json!(crate::notice::telegram_notice(app, &locale(app), p))),
        "resources.report" => Ok(core_serve::sampler::latest_resources().unwrap_or(Value::Null)),
        m if m.starts_with("preview.") => crate::preview_host::call(app, m, p),
        _ => Err(format!("no host for {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_string_lists_are_read_leniently() {
        assert_eq!(num(&json!(3)), 3);
        assert_eq!(num(&json!("3")), 0);
        assert_eq!(num(&Value::Null), 0);
        assert_eq!(strings(&json!(["a", 1, "b"])), vec!["a".to_owned(), "b".to_owned()]);
        assert!(strings(&json!("x")).is_empty());
    }

    #[test]
    fn trash_needs_an_absolute_path() {
        assert!(!trash(&json!({ "path": "relative/dir" })));
        assert!(!trash(&json!({})));
    }

    #[test]
    fn the_local_snapshot_reads_only_ccm_keys() {
        assert!(LOCAL_SNAPSHOT_JS.contains("k.startsWith('ccm.')"));
    }
}
