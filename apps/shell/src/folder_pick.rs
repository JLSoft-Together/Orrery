use std::sync::OnceLock;

use core_monitor::native::Picker;
use core_store::event::FolderPickResult;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::state::MAIN;

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn install(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn e2e_answer(get: &dyn Fn(&str) -> Option<String>, test_build: bool) -> Option<(FolderPickResult, Option<String>)> {
    if !test_build {
        return None;
    }
    let raw = get("CCM_E2E_PICK_FOLDER")?.trim().to_owned();
    match raw.as_str() {
        "" => None,
        "cancel" => Some((FolderPickResult::Cancelled, None)),
        _ => Some((FolderPickResult::Ok, Some(raw))),
    }
}

fn pick() -> (FolderPickResult, Option<String>) {
    let test_build = cfg!(debug_assertions) || crate::config::e2e_isolated();
    if let Some(answer) = e2e_answer(&|k| std::env::var(k).ok(), test_build) {
        return answer;
    }
    let Some(app) = APP.get() else { return (FolderPickResult::Failed, None) };
    let mut d = app.dialog().file();
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    match d.blocking_pick_folder().map(|p| p.into_path()) {
        None => (FolderPickResult::Cancelled, None),
        Some(Ok(p)) => (FolderPickResult::Ok, Some(p.to_string_lossy().into_owned())),
        Some(Err(_)) => (FolderPickResult::Failed, None),
    }
}

pub fn picker() -> Picker {
    Box::new(pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_e2e_answer_only_exists_in_test_builds() {
        let env = |v: &'static str| move |k: &str| (k == "CCM_E2E_PICK_FOLDER").then(|| v.to_owned());
        assert_eq!(e2e_answer(&env("C:\\w"), false), None);
        assert_eq!(e2e_answer(&env("C:\\w"), true), Some((FolderPickResult::Ok, Some("C:\\w".into()))));
        assert_eq!(e2e_answer(&env("cancel"), true), Some((FolderPickResult::Cancelled, None)));
        assert_eq!(e2e_answer(&env("  "), true), None);
        assert_eq!(e2e_answer(&|_| None, true), None);
    }
}
