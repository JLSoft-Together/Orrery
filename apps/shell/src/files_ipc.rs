use std::path::{Path, PathBuf};

use core_desk::preview::paths::dirname;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::config::APP_VERSION;
use crate::preview_host::{pane_roots, Gate};
use crate::state::{Shell, MAIN};

pub const CHANNELS: [&str; 8] = ["ccm:preview-url", "ccm:open-path", "ccm:reveal", "ccm:pick-file", "ccm:pick-files", "ccm:logs", "ccm:bug-report-preview", "ccm:bug-report-save"];

const MAX_PICKED: usize = 50;
const MAX_PAGE_REPORT: usize = 20_000;
const LOG_TAIL_BYTES: u64 = 256 * 1024;
const LOG_FILES: [&str; 2] = [crate::log::FILE, "pty.log"];

const EXECUTABLE_EXT: [&str; 42] = [
    "exe", "com", "bat", "cmd", "msi", "msp", "scr", "pif", "cpl", "msc", "lnk", "url", "reg", "inf", "hta", "jar", "appref-ms", "ps1", "psm1", "psd1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "py", "pyw", "pl", "rb", "app",
    "command", "tool", "sh", "bash", "zsh", "pkg", "dmg", "workflow", "terminal", "scpt", "applescript",
];

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

pub fn is_executable(file: &str) -> bool {
    let ext = Path::new(file).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    EXECUTABLE_EXT.contains(&ext.as_str())
}

fn arg(args: &[Value], i: usize) -> &Value {
    args.get(i).unwrap_or(&Value::Null)
}

fn start_dir(near: &Value) -> Option<PathBuf> {
    let p = PathBuf::from(near.as_str()?);
    (p.is_absolute() && p.is_dir()).then_some(p)
}

fn dialog(app: &AppHandle) -> FileDialogBuilder<tauri::Wry> {
    let mut d = app.dialog().file();
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    d
}

fn allowed(app: &AppHandle, path: &Value) -> Option<(String, Vec<String>)> {
    let gate = app.try_state::<Gate>()?;
    let roots = |_fresh: bool| pane_roots(app);
    gate.allowed(&roots, path, false)
}

fn preview_url(app: &AppHandle, path: &Value) -> Value {
    let Some((real, roots)) = allowed(app, path) else { return Value::Null };
    let Some(server) = crate::preview_serve::server() else { return Value::Null };
    server.target_for(&real, &roots).map_or(Value::Null, |t| json!({"url": t.url, "root": t.root, "token": t.token, "origin": server.origin()}))
}

fn open_path(app: &AppHandle, path: &Value) -> Value {
    let Some((real, _)) = allowed(app, path).filter(|(r, _)| !is_executable(r)) else { return json!("denied") };
    match app.opener().open_path(real, None::<&str>) {
        Ok(()) => json!("ok"),
        Err(e) => json!(e.to_string()),
    }
}

fn reveal(app: &AppHandle, path: &Value) -> Value {
    let Some((real, _)) = allowed(app, path) else { return json!(false) };
    json!(app.opener().reveal_item_in_dir(real).is_ok())
}

fn pick_file(app: &AppHandle, kind: &Value, near: &Value) -> Value {
    let mut d = dialog(app);
    if kind.as_str() == Some("html") {
        d = d.add_filter("HTML", &["html", "htm"]);
    }
    if let Some(dir) = start_dir(near) {
        d = d.set_directory(dir);
    }
    let Some(file) = d.blocking_pick_file().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()) else { return Value::Null };
    if let Some(gate) = app.try_state::<Gate>() {
        gate.grants.add(&dirname(&file));
    }
    json!(file)
}

fn pick_files(app: &AppHandle) -> Value {
    let picked = dialog(app).blocking_pick_files().unwrap_or_default();
    json!(picked.into_iter().filter_map(|p| p.into_path().ok()).take(MAX_PICKED).map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>())
}

pub fn tail_of(file: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(file) else { return "(missing)".into() };
    let len = f.metadata().map_or(0, |m| m.len());
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(LOG_TAIL_BYTES)));
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return "(missing)".into();
    }
    String::from_utf8_lossy(&buf).into_owned()
}

pub fn page_report(raw: &Value) -> String {
    raw.as_str().map_or_else(String::new, |s| s.chars().take(MAX_PAGE_REPORT).collect())
}

pub fn platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn home() -> String {
    std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()
}

pub fn build_report(head: &[String], page: &str, logs: &[(&str, String)], home: &str, platform: &str) -> String {
    let mut sections = vec![head.join("\n")];
    if !page.is_empty() {
        sections.push(format!("## Diagnostics\n{page}"));
    }
    for (name, text) in logs {
        sections.push(format!("## {name}\n{}", text.trim_end()));
    }
    format!("{}\n", core_desk::claude::mask::mask_hook_text(&sections.join("\n\n"), home, platform))
}

pub fn save_dialog(app: &AppHandle, name: &str, filter: &str, exts: &[&str]) -> Option<PathBuf> {
    let mut d = dialog(app).set_file_name(name).add_filter(filter, exts);
    if let Ok(dir) = app.path().download_dir() {
        d = d.set_directory(dir);
    }
    d.blocking_save_file().and_then(|p| p.into_path().ok())
}

pub fn report(app: &AppHandle, page: &str) -> String {
    let shell = app.state::<Shell>();
    let generated = core_desk::util::iso_string(core_desk::util::now_ms());
    let head = vec![
        format!("Orrery {APP_VERSION}{}", if cfg!(debug_assertions) { " dev" } else { "" }),
        format!("Tauri {}, {} {}, locale {}", tauri::VERSION, platform(), std::env::consts::ARCH, crate::hosts::locale(app)),
        format!("collector port {}", shell.port),
        format!("generated {generated}"),
    ];
    let logs: Vec<(&str, String)> = LOG_FILES.iter().map(|n| (*n, tail_of(&shell.data_dir.join(n)))).collect();
    build_report(&head, page, &logs, &home(), platform())
}

fn logs(app: &AppHandle, action: &Value, page: &Value) -> Value {
    let action = action.as_str().unwrap_or("");
    if action != "copy" && action != "save" {
        return Value::Null;
    }
    let text = report(app, &page_report(page));
    if action == "copy" {
        return json!({"ok": arboard::Clipboard::new().and_then(|mut c| c.set_text(text)).is_ok()});
    }
    let stamp = core_desk::util::local_file_stamp();
    let mut d = dialog(app).set_file_name(format!("agent-monitor-log-{stamp}.txt")).add_filter("Text", &["txt"]);
    if let Ok(dir) = app.path().download_dir() {
        d = d.set_directory(dir);
    }
    let Some(file) = d.blocking_save_file().and_then(|p| p.into_path().ok()) else { return json!({"ok": false}) };
    if std::fs::write(&file, text).is_err() {
        return json!({"ok": false});
    }
    let _ = app.opener().reveal_item_in_dir(&file);
    json!({"ok": true, "path": file.to_string_lossy()})
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    let a0 = arg(args, 0);
    Ok(match channel {
        "ccm:preview-url" => preview_url(app, a0),
        "ccm:open-path" => open_path(app, a0),
        "ccm:reveal" => reveal(app, a0),
        "ccm:pick-file" => pick_file(app, a0, arg(args, 1)),
        "ccm:pick-files" => pick_files(app),
        "ccm:logs" => logs(app, a0, arg(args, 1)),
        "ccm:bug-report-preview" => crate::bug_report::preview(app),
        "ccm:bug-report-save" => crate::bug_report::save(app, a0),
        _ => return Err(format!("not ported: {channel}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_channels_exist_in_the_generated_shim_and_not_in_the_desk() {
        let shim = include_str!("shim.js");
        let dir = std::env::temp_dir().join(format!("orrery-files-ipc-{}", std::process::id()));
        let desk = crate::desk::boot(&crate::desk::env_for(&dir, "1.0.0")).desk.channels();
        for c in CHANNELS {
            assert!(shim.contains(&format!("\"{c}\"")), "{c}");
            assert!(!desk.iter().any(|d| d == c), "{c}");
            assert!(!crate::preview_ipc::handles(c), "{c}");
            assert!(handles(c));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn executables_are_never_opened() {
        for f in ["/a/x.EXE", "/a/run.sh", "/a/Tool.app", "/a/s.command", "/a/x.ps1", "/a/i.dmg", "/a/k.js"] {
            assert!(is_executable(f), "{f}");
        }
        for f in ["/a/x.png", "/a/README.md", "/a/index.html", "/a/noext", "/a/doc.pdf"] {
            assert!(!is_executable(f), "{f}");
        }
    }

    #[test]
    fn the_page_report_is_capped_and_only_text() {
        assert_eq!(page_report(&json!("x".repeat(MAX_PAGE_REPORT + 5))).chars().count(), MAX_PAGE_REPORT);
        assert_eq!(page_report(&json!(5)), "");
        assert_eq!(page_report(&Value::Null), "");
    }

    #[test]
    fn the_report_joins_head_diagnostics_and_logs_and_masks_secrets() {
        let head = vec!["Orrery 4.0.1".to_owned(), "generated now".to_owned()];
        let logs = vec![("shell.log", "line\nAPI_KEY=abcdef123456\n\n".to_owned()), ("pty.log", "(missing)".to_owned())];
        let text = build_report(&head, "page /Users/me/p", &logs, "/Users/me", "darwin");
        assert!(text.starts_with("Orrery 4.0.1\ngenerated now\n\n## Diagnostics\npage "), "{text}");
        assert!(!text.contains("/Users/me/p"), "{text}");
        assert!(text.contains("## shell.log\nline\n"));
        assert!(!text.contains("abcdef123456"), "{text}");
        assert!(text.ends_with("## pty.log\n(missing)\n"));
        let bare = build_report(&head, "", &[], "/Users/me", "darwin");
        assert_eq!(bare, "Orrery 4.0.1\ngenerated now\n");
    }

    #[test]
    fn the_log_tail_keeps_the_end_of_a_big_file() {
        let dir = std::env::temp_dir().join(format!("orrery-files-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("x.log");
        let mut body = vec![b'a'; LOG_TAIL_BYTES as usize];
        body.extend_from_slice(b"END");
        std::fs::write(&file, &body).unwrap();
        let tail = tail_of(&file);
        assert_eq!(tail.len(), LOG_TAIL_BYTES as usize);
        assert!(tail.ends_with("END"));
        assert_eq!(tail_of(&dir.join("gone.log")), "(missing)");
        let _ = std::fs::remove_dir_all(dir);
    }
}
