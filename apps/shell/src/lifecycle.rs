use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};

use core_desk::bridge::install::BRIDGE_EXE;

use crate::state::{lock, Shell, MAIN};

const CENSUS_WAIT: Duration = Duration::from_secs(3);
const SAVE_WAIT: Duration = Duration::from_secs(15);

pub struct QuitText {
    en: bool,
}

impl QuitText {
    pub fn of(locale: &str) -> QuitText {
        QuitText { en: locale == "en" }
    }

    pub fn title(&self, n: usize) -> String {
        match (self.en, n) {
            (true, 0) => "Quit and close the open terminals?".into(),
            (true, 1) => "Quit and close 1 terminal?".into(),
            (true, n) => format!("Quit and close {n} terminals?"),
            (false, 0) => "Thoát và đóng các terminal đang mở?".into(),
            (false, n) => format!("Thoát và đóng {n} terminal?"),
        }
    }

    pub fn detail(&self) -> &'static str {
        if self.en { "These terminals run inside Orrery and stop when it quits:" } else { "Các terminal này chạy bên trong Orrery và sẽ dừng khi thoát:" }
    }

    pub fn confirm(&self) -> &'static str {
        if self.en { "Quit" } else { "Thoát" }
    }

    pub fn cancel(&self) -> &'static str {
        if self.en { "Cancel" } else { "Huỷ" }
    }

    pub fn runs_title(&self, n: usize) -> String {
        match (self.en, n) {
            (true, 1) => "Quit and stop 1 running app?".into(),
            (true, n) => format!("Quit and stop {n} running apps?"),
            (false, n) => format!("Thoát và dừng {n} app đang chạy?"),
        }
    }

    pub fn runs(&self) -> &'static str {
        if self.en {
            "Running from Orrery (stopped on quit, the app on the device keeps its data):"
        } else {
            "Đang chạy từ Orrery (sẽ dừng khi thoát, dữ liệu app trên thiết bị giữ nguyên):"
        }
    }

    pub fn unsaved_title(&self, n: usize) -> String {
        match (self.en, n) {
            (true, 1) => "Save changes to 1 file before quitting?".into(),
            (true, n) => format!("Save changes to {n} files before quitting?"),
            (false, n) => format!("Lưu thay đổi của {n} file trước khi thoát?"),
        }
    }

    pub fn unsaved_detail(&self) -> &'static str {
        if self.en { "Edited in Orrery and not saved yet:" } else { "Đã sửa trong Orrery và chưa lưu:" }
    }

    pub fn save(&self) -> &'static str {
        if self.en { "Save" } else { "Lưu" }
    }

    pub fn discard(&self) -> &'static str {
        if self.en { "Don't save" } else { "Không lưu" }
    }

    pub fn save_failed(&self) -> &'static str {
        if self.en {
            "Some files could not be saved. Quitting was cancelled; check the file tabs."
        } else {
            "Có file không lưu được. Đã huỷ thoát; xem lại các tab file."
        }
    }
}

fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_default()
}

pub fn short(p: &str, home: &Path) -> String {
    if home.as_os_str().is_empty() {
        return p.to_owned();
    }
    match Path::new(p).strip_prefix(home) {
        Ok(rel) if rel.as_os_str().is_empty() => "~".into(),
        Ok(rel) => Path::new("~").join(rel).to_string_lossy().into_owned(),
        Err(_) => p.to_owned(),
    }
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn alive_panes(census: &[Value]) -> Vec<Value> {
    census.iter().filter(|s| s.get("alive") != Some(&Value::Bool(false))).cloned().collect()
}

pub fn quit_prompt(t: &QuitText, census: Option<&[Value]>, runs: &[Value], home: &Path) -> Option<(String, String)> {
    let live = census.map(alive_panes);
    if live.as_ref().is_some_and(Vec::is_empty) && runs.is_empty() {
        return None;
    }
    let live = live.unwrap_or_default();
    let terms = census.is_none() || !live.is_empty();
    let lines: Vec<String> = live.iter().map(|s| format!("• {} — {}", text(s, "preset"), short(text(s, "cwd"), home))).collect();
    let run_lines: Vec<String> = runs
        .iter()
        .map(|r| {
            let device = text(r, "deviceName");
            let to = if device.is_empty() { String::new() } else { format!(" → {device}") };
            format!("• {}{to} — {}", text(r, "configName"), short(text(r, "cwd"), home))
        })
        .collect();
    let mut detail = Vec::new();
    if terms {
        detail.push(if lines.is_empty() { t.detail().to_owned() } else { format!("{}\n{}", t.detail(), lines.join("\n")) });
    }
    if !run_lines.is_empty() {
        detail.push(format!("{}\n{}", t.runs(), run_lines.join("\n")));
    }
    let message = if terms { t.title(live.len()) } else { t.runs_title(runs.len()) };
    Some((message, detail.join("\n\n")))
}

pub fn unsaved_prompt(t: &QuitText, files: &[Value]) -> (String, String) {
    let lines: Vec<String> = files.iter().map(|f| format!("• {} — {}", text(f, "name"), text(f, "path"))).collect();
    (t.unsaved_title(files.len()), format!("{}\n{}", t.unsaved_detail(), lines.join("\n")))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Unsaved {
    Save,
    Discard,
    Cancel,
}

pub fn unsaved_choice(result: &MessageDialogResult, t: &QuitText) -> Unsaved {
    match result {
        MessageDialogResult::Yes => Unsaved::Save,
        MessageDialogResult::No => Unsaved::Discard,
        MessageDialogResult::Custom(s) if s == t.save() => Unsaved::Save,
        MessageDialogResult::Custom(s) if s == t.discard() => Unsaved::Discard,
        _ => Unsaved::Cancel,
    }
}

fn page_labels() -> &'static Mutex<Vec<String>> {
    static LABELS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    LABELS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn page_of(labels: &mut Vec<String>, label: &str) -> u64 {
    match labels.iter().position(|l| l == label) {
        Some(i) => i as u64 + 1,
        None => {
            labels.push(label.to_owned());
            labels.len() as u64
        }
    }
}

pub fn label_of(labels: &[String], page: u64) -> Option<String> {
    labels.get(usize::try_from(page).ok()?.checked_sub(1)?).cloned()
}

pub fn save_reply_key(label: &str, id: u64) -> String {
    format!("editor-save:{label}:{id}")
}

pub fn editor_dirty(app: &AppHandle, label: &str, files: &Value, auto_save: &Value) {
    let page = page_of(&mut lock(page_labels()), label);
    if let Err(e) = crate::desk::call_inside(app, "desk:editor-dirty", json!([page, files, auto_save])) {
        eprintln!("[shell] editor dirty: {e}");
    }
}

fn page_ids(state: &Value) -> Vec<u64> {
    state["pages"].as_array().map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default()
}

fn editor_state(app: &AppHandle) -> (Vec<Value>, bool, Vec<String>) {
    let read = || crate::desk::call_inside(app, "desk:editor-state", json!([])).unwrap_or(Value::Null);
    let mut state = read();
    let labels = lock(page_labels()).clone();
    let gone: Vec<u64> = page_ids(&state).into_iter().filter(|p| label_of(&labels, *p).is_none_or(|l| app.get_webview(&l).is_none())).collect();
    if !gone.is_empty() {
        for p in gone {
            let _ = crate::desk::call_inside(app, "desk:editor-forget", json!([p]));
        }
        state = read();
    }
    let files = state["files"].as_array().cloned().unwrap_or_default();
    let pages = page_ids(&state).into_iter().filter_map(|p| label_of(&labels, p)).collect();
    (files, state["autoSave"] == Value::Bool(true), pages)
}

fn save_all(app: &AppHandle, labels: &[String]) -> bool {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let shell = app.state::<Shell>();
    let mut ok = true;
    let mut waits = Vec::new();
    for label in labels {
        let id = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
        let key = save_reply_key(label, id);
        let (tx, rx) = mpsc::channel();
        lock(&shell.replies).insert(key.clone(), tx);
        if app.emit_to(label.as_str(), "ccm:editor-save-all", id).is_err() {
            lock(&shell.replies).remove(&key);
            ok = false;
            continue;
        }
        waits.push((key, rx));
    }
    let deadline = Instant::now() + SAVE_WAIT;
    for (key, rx) in waits {
        let got = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).ok();
        lock(&shell.replies).remove(&key);
        ok &= got == Some(Value::Bool(true));
    }
    ok
}

fn e2e_unsaved() -> Option<Unsaved> {
    let answer = std::env::var("CCM_E2E_CONFIRM").ok()?;
    Some(if answer == "ok" { Unsaved::Save } else { Unsaved::Cancel })
}

fn ask_unsaved(app: &AppHandle, t: &QuitText, files: &[Value]) -> Unsaved {
    if let Some(answer) = e2e_unsaved() {
        return answer;
    }
    if app.get_webview_window(MAIN).is_some_and(|w| !w.is_visible().unwrap_or(true)) {
        crate::window::show_main(app);
    }
    let (message, detail) = unsaved_prompt(t, files);
    let mut d = app
        .dialog()
        .message(format!("{message}\n\n{detail}"))
        .title("Orrery")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(t.save().into(), t.discard().into(), t.cancel().into()));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.parent(&w);
    }
    unsaved_choice(&d.blocking_show_with_result(), t)
}

fn save_failed(app: &AppHandle, t: &QuitText) {
    if std::env::var("CCM_E2E_CONFIRM").is_ok() {
        return;
    }
    let mut d = app.dialog().message(t.save_failed()).title("Orrery").kind(MessageDialogKind::Error).buttons(MessageDialogButtons::OkCustom(t.cancel().into()));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.parent(&w);
    }
    d.blocking_show();
}

fn confirm_unsaved(app: &AppHandle, t: &QuitText) -> bool {
    let (files, auto, pages) = editor_state(app);
    if files.is_empty() {
        return true;
    }
    if auto && save_all(app, &pages) {
        return true;
    }
    match ask_unsaved(app, t, &files) {
        Unsaved::Discard => true,
        Unsaved::Cancel => false,
        Unsaved::Save => {
            if save_all(app, &pages) {
                return true;
            }
            save_failed(app, t);
            false
        }
    }
}

fn live_runs(app: &AppHandle) -> Vec<Value> {
    let all = crate::desk::call_inside(app, "desk:run-sessions", Value::Null).unwrap_or(Value::Null);
    all.as_array().map(|a| a.iter().filter(|s| core_desk::run::sessions::is_live(text(s, "state"))).cloned().collect()).unwrap_or_default()
}

pub fn confirm_quit(app: &AppHandle) -> bool {
    let t = QuitText::of(&crate::hosts::locale(app));
    if !confirm_unsaved(app, &t) {
        return false;
    }
    let census = app.state::<Shell>().pty.census(CENSUS_WAIT);
    let runs = live_runs(app);
    let Some((message, detail)) = quit_prompt(&t, census.as_deref(), &runs, &home()) else { return true };
    crate::hosts::confirm(app, &message, &detail, t.confirm(), t.cancel())
}

fn previous_cell() -> &'static OnceLock<bool> {
    static PREVIOUS: OnceLock<bool> = OnceLock::new();
    &PREVIOUS
}

pub fn previous_unclean() -> Option<bool> {
    previous_cell().get().copied()
}

pub fn unclean_of(reply: &Value) -> Option<bool> {
    reply.get("unclean").and_then(Value::as_bool)
}

pub fn boot(app: &AppHandle) {
    match crate::desk::call_inside(app, "desk:run-boot", json!([])) {
        Ok(v) => {
            if let Some(u) = unclean_of(&v) {
                let _ = previous_cell().set(u);
            }
        }
        Err(e) => eprintln!("[shell] run boot: {e}"),
    }
}

pub fn clean(app: &AppHandle) {
    crate::disk_map::stop(app);
    crate::disk_organize::stop(app);
    crate::disk_clean::stop(app);
    for channel in ["desk:run-clean", "desk:tg-stop"] {
        if let Err(e) = crate::desk::call_inside(app, channel, json!([])) {
            eprintln!("[shell] {channel}: {e}");
        }
    }
}

pub fn migrate_args(next: &Value) -> Option<Value> {
    next["statusline"].as_str().filter(|s| s.contains(BRIDGE_EXE))?;
    Some(json!([{ "statusline": next["statusline"], "hook": next["hook"] }]))
}

pub fn moved(reply: &Value) -> bool {
    reply.get("result").and_then(Value::as_str).is_some_and(|r| r != "none")
}

pub fn migrate_bridges(app: &AppHandle) {
    let app = app.clone();
    let spawned = std::thread::Builder::new().name("bridge-migrate".into()).spawn(move || {
        let Some(args) = migrate_args(&crate::host_extra::bridge_commands(&app)) else { return };
        match crate::desk::call_inside(&app, "desk:bridge-migrate", args) {
            Ok(v) if moved(&v) => eprintln!("[shell] bridge entries in settings.json moved to orrery-bridge: {v}"),
            Ok(_) => {}
            Err(e) => eprintln!("[shell] bridge migrate failed: {e}"),
        }
    });
    if let Err(e) = spawned {
        eprintln!("[shell] bridge migrate: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_runs_only_when_the_statusline_is_the_rust_bridge() {
        let rust = format!("\"/data/bin/{BRIDGE_EXE}\" statusline-bridge");
        let hook = format!("\"/data/bin/{BRIDGE_EXE}\" hook-bridge");
        assert_eq!(migrate_args(&json!({"statusline": rust, "hook": hook})), Some(json!([{"statusline": rust, "hook": hook}])));
        assert_eq!(migrate_args(&json!({"statusline": rust, "hook": null})), Some(json!([{"statusline": rust, "hook": null}])));
        assert_eq!(migrate_args(&json!({"statusline": "node \"/x/statusline-bridge.mjs\"", "hook": hook})), None);
        assert_eq!(migrate_args(&json!({"statusline": null, "hook": hook})), None);
    }

    #[test]
    fn only_a_real_move_is_logged() {
        assert!(moved(&json!({"result": "moved"})));
        assert!(!moved(&json!({"result": "none"})));
        assert!(!moved(&json!(null)));
    }

    #[test]
    fn run_boot_reply_gives_the_previous_run() {
        assert_eq!(unclean_of(&json!({"unclean": true})), Some(true));
        assert_eq!(unclean_of(&json!({"unclean": false})), Some(false));
        assert_eq!(unclean_of(&json!(null)), None);
    }

    #[test]
    fn nothing_live_means_no_quit_prompt() {
        let t = QuitText::of("en");
        let home = Path::new("/h");
        assert_eq!(quit_prompt(&t, Some(&[]), &[], home), None);
        assert_eq!(quit_prompt(&t, Some(&[json!({"preset": "shell", "cwd": "/x", "alive": false})]), &[], home), None);
    }

    #[test]
    fn the_quit_prompt_lists_terminals_and_runs() {
        let t = QuitText::of("en");
        let home = PathBuf::from("/h");
        let census = vec![json!({"preset": "claude", "cwd": "/h/app", "alive": true}), json!({"preset": "shell", "cwd": "/opt", "alive": true})];
        let runs = vec![json!({"configName": "app", "deviceName": "Pixel", "cwd": "/h/app"})];
        let (message, detail) = quit_prompt(&t, Some(&census), &runs, &home).unwrap();
        assert_eq!(message, "Quit and close 2 terminals?");
        let app = Path::new("~").join("app").to_string_lossy().into_owned();
        assert_eq!(detail, format!("{}\n• claude — {app}\n• shell — /opt\n\n{}\n• app → Pixel — {app}", t.detail(), t.runs()));
    }

    #[test]
    fn only_runs_use_the_runs_title_and_no_census_still_names_terminals() {
        let t = QuitText::of("vi");
        let home = Path::new("/h");
        let runs = vec![json!({"configName": "web", "cwd": "/w"})];
        let (message, detail) = quit_prompt(&t, Some(&[]), &runs, home).unwrap();
        assert_eq!(message, "Thoát và dừng 1 app đang chạy?");
        assert_eq!(detail, format!("{}\n• web — /w", t.runs()));
        let (message, detail) = quit_prompt(&t, None, &[], home).unwrap();
        assert_eq!(message, "Thoát và đóng các terminal đang mở?");
        assert_eq!(detail, t.detail());
    }

    #[test]
    fn paths_under_home_are_shortened() {
        let home = Path::new("/h");
        assert_eq!(short("/h", home), "~");
        assert_eq!(short("/hx/a", home), "/hx/a");
        assert_eq!(short("/other", Path::new("")), "/other");
    }

    #[test]
    fn titles_follow_counts_in_both_locales() {
        let en = QuitText::of("en");
        assert_eq!(en.title(1), "Quit and close 1 terminal?");
        assert_eq!(en.runs_title(3), "Quit and stop 3 running apps?");
        assert_eq!(en.unsaved_title(1), "Save changes to 1 file before quitting?");
        assert_eq!(en.unsaved_title(2), "Save changes to 2 files before quitting?");
        assert_eq!(QuitText::of("vi").unsaved_title(1), "Lưu thay đổi của 1 file trước khi thoát?");
        assert_eq!(QuitText::of("xx").confirm(), "Thoát");
    }

    #[test]
    fn the_unsaved_prompt_names_each_file() {
        let t = QuitText::of("en");
        let (message, detail) = unsaved_prompt(&t, &[json!({"name": "a.ts", "path": "/w/a.ts"})]);
        assert_eq!(message, "Save changes to 1 file before quitting?");
        assert_eq!(detail, "Edited in Orrery and not saved yet:\n• a.ts — /w/a.ts");
    }

    #[test]
    fn unsaved_buttons_map_to_save_discard_cancel() {
        let t = QuitText::of("en");
        assert_eq!(unsaved_choice(&MessageDialogResult::Yes, &t), Unsaved::Save);
        assert_eq!(unsaved_choice(&MessageDialogResult::No, &t), Unsaved::Discard);
        assert_eq!(unsaved_choice(&MessageDialogResult::Custom("Save".into()), &t), Unsaved::Save);
        assert_eq!(unsaved_choice(&MessageDialogResult::Custom("Don't save".into()), &t), Unsaved::Discard);
        assert_eq!(unsaved_choice(&MessageDialogResult::Custom("Cancel".into()), &t), Unsaved::Cancel);
        assert_eq!(unsaved_choice(&MessageDialogResult::Cancel, &t), Unsaved::Cancel);
    }

    #[test]
    fn pages_get_stable_ids_per_label() {
        let mut labels = Vec::new();
        assert_eq!(page_of(&mut labels, MAIN), 1);
        assert_eq!(page_of(&mut labels, "compact"), 2);
        assert_eq!(page_of(&mut labels, MAIN), 1);
        assert_eq!(label_of(&labels, 2).as_deref(), Some("compact"));
        assert_eq!(label_of(&labels, 0), None);
        assert_eq!(label_of(&labels, 3), None);
        assert_ne!(save_reply_key(MAIN, 1), save_reply_key("compact", 1));
    }

    #[test]
    fn editor_dirty_pages_reach_the_desk_tracker() {
        let dir = std::env::temp_dir().join(format!("orrery-shell-editors-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let state = crate::desk::boot(&crate::desk::env_for(&dir, "9.9.9"));
        let files = json!([{"name": "a.ts", "path": "/w/a.ts"}]);
        state.desk.dispatch("desk:editor-dirty", json!([1, files, true])).unwrap();
        let got = state.desk.dispatch("desk:editor-state", json!([])).unwrap();
        assert_eq!(got["files"], files);
        assert_eq!(got["autoSave"], true);
        assert_eq!(page_ids(&got), vec![1]);
        state.desk.dispatch("desk:editor-dirty", json!([1, [], false])).unwrap();
        assert!(page_ids(&state.desk.dispatch("desk:editor-state", json!([])).unwrap()).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn boot_then_clean_round_trips_through_the_desk() {
        let dir = std::env::temp_dir().join(format!("orrery-shell-lifecycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let state = crate::desk::boot(&crate::desk::env_for(&dir, "9.9.9"));
        let first = state.desk.dispatch("desk:run-boot", json!([])).unwrap();
        assert_eq!(unclean_of(&first), Some(false));
        let crashed = crate::desk::boot(&crate::desk::env_for(&dir, "9.9.9"));
        assert_eq!(unclean_of(&crashed.desk.dispatch("desk:run-boot", json!([])).unwrap()), Some(true));
        crashed.desk.dispatch("desk:run-clean", json!([])).unwrap();
        let next = crate::desk::boot(&crate::desk::env_for(&dir, "9.9.9"));
        assert_eq!(unclean_of(&next.desk.dispatch("desk:run-boot", json!([])).unwrap()), Some(false));
        let _ = std::fs::remove_dir_all(dir);
    }
}
