use std::sync::atomic::Ordering;

use serde_json::json;
use tauri::webview::{NewWindowResponse, PageLoadEvent, PermissionKind, PermissionResponse};
use tauri::window::Color;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::appearance::{rgba, zoom_factor, Bounds};
use crate::guest;
use crate::state::{lock, Shell, MAIN};

pub const COMPACT: &str = "compact";
const SHIM: &str = include_str!("shim.js");
const HEADER: &str = include_str!("header.js");
const WEBVIEW: &str = include_str!("webview.js");

pub fn init_script(origin: &str) -> String {
    let platform = if cfg!(windows) { "win32" } else { "darwin" };
    let shell = json!({ "platform": platform, "origin": origin, "sync": { "ccm:core-serve": "rust", "ccm:deepseek-enabled": false } });
    format!("(function(){{const __ccmShell={shell};if(location.origin!==__ccmShell.origin)return;\n{SHIM};\n{HEADER};\n{WEBVIEW};\n}})();")
}

pub fn open_external(app: &AppHandle, url: &str) {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

fn background(shell: &Shell) -> Color {
    let hex = lock(&shell.look).background().unwrap_or_else(|| "#ececee".into());
    let (r, g, b) = rgba(&hex).unwrap_or((236, 236, 238));
    Color(r, g, b, 255)
}

fn builder<'a>(app: &'a AppHandle, label: &str, url: &str, shell: &Shell) -> Result<WebviewWindowBuilder<'a, tauri::Wry, AppHandle>, String> {
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{e}"))?;
    let nav = app.clone();
    let popup = app.clone();
    let mut b = WebviewWindowBuilder::new(app, label, WebviewUrl::External(parsed))
        .title("Orrery")
        .min_inner_size(420.0, 480.0)
        .data_directory(shell.webview_dir.clone())
        .initialization_script(shell.init_script.as_str())
        .background_color(background(shell))
        .zoom_hotkeys_enabled(false)
        .devtools(crate::config::devtools_allowed())
        .on_navigation(move |u| {
            let shell = nav.state::<Shell>();
            if shell.is_app_url(u) || u.scheme() == "about" {
                return true;
            }
            open_external(&nav, u.as_str());
            false
        })
        .on_new_window(move |u, _| {
            let shell = popup.state::<Shell>();
            if shell.is_app_url(&u) {
                let app = popup.clone();
                std::thread::spawn(move || open_compact(&app));
            } else {
                open_external(&popup, u.as_str());
            }
            NewWindowResponse::Deny
        })
        .on_permission_request(|webview, kind| {
            let app_page = webview.url().is_ok_and(|u| webview.app_handle().state::<Shell>().is_app_url(&u));
            if app_page && kind == PermissionKind::ClipboardRead {
                PermissionResponse::Allow
            } else {
                PermissionResponse::Default
            }
        })
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Started {
                guest::close_owned(webview.app_handle(), webview.label());
                crate::preview_ipc::release(webview.app_handle(), webview.label());
                release_owners(webview.app_handle(), webview.label());
            }
            if payload.event() == PageLoadEvent::Finished {
                if webview.label() == MAIN {
                    crate::webstorage::loaded(&webview, &webview.app_handle().state::<Shell>().data_dir);
                }
                let level = webview.app_handle().state::<Shell>().look.lock().map(|l| l.state.zoom).unwrap_or(0.0);
                let _ = webview.set_zoom(zoom_factor(level));
            }
        });
    #[cfg(target_os = "macos")]
    {
        b = b.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    }
    #[cfg(windows)]
    {
        b = b.decorations(false).shadow(true);
    }
    Ok(b.disable_drag_drop_handler())
}

pub fn create_main(app: &AppHandle, hidden: bool) -> Result<WebviewWindow, String> {
    let shell = app.state::<Shell>();
    let saved = lock(&shell.look).state.bounds.clone();
    let url = format!("{}/?k={}", shell.origin, shell.key);
    let mut b = builder(app, MAIN, &url, &shell)?.inner_size(1360.0, 860.0).visible(false);
    if let Some(s) = saved.as_ref().filter(|s| s.width >= 420 && s.height >= 480) {
        b = b.inner_size(f64::from(s.width), f64::from(s.height));
    }
    let win = b.build().map_err(|e| e.to_string())?;
    crate::drop_paths::attach(&win);
    if let Some(s) = saved.filter(|s| s.width >= 420 && s.height >= 480) {
        if on_screen(&win, &s) {
            let _ = win.set_position(PhysicalPosition::new(s.x, s.y));
            let _ = win.set_size(PhysicalSize::new(s.width, s.height));
        }
        if s.maximized {
            let _ = win.maximize();
        }
    }
    if !hidden {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(win)
}

fn on_screen(win: &WebviewWindow, s: &Bounds) -> bool {
    win.available_monitors().unwrap_or_default().iter().any(|m| {
        let (p, z) = (m.position(), m.size());
        s.x < p.x + z.width as i32 && s.x + s.width as i32 > p.x && s.y >= p.y - 8 && s.y < p.y + z.height as i32
    })
}

pub fn save_bounds(win: &tauri::Window) {
    let shell = win.state::<Shell>();
    let maximized = win.is_maximized().unwrap_or(false);
    let mut look = lock(&shell.look);
    let mut bounds = look.state.bounds.clone().unwrap_or_default();
    bounds.maximized = maximized;
    if !maximized && !win.is_minimized().unwrap_or(false) {
        if let (Ok(p), Ok(z)) = (win.outer_position(), win.inner_size()) {
            bounds = Bounds { x: p.x, y: p.y, width: z.width, height: z.height, maximized };
        }
    }
    look.state.bounds = Some(bounds);
    look.save();
}

pub fn open_compact(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(COMPACT) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let shell = app.state::<Shell>();
    let url = format!("{}/?mode=compact", shell.origin);
    if let Ok(b) = builder(app, COMPACT, &url, &shell) {
        if let Ok(win) = b.inner_size(440.0, 720.0).min_inner_size(320.0, 360.0).build() {
            crate::drop_paths::attach(&win);
        }
    }
}

fn release_owners(app: &AppHandle, label: &str) {
    let app = app.clone();
    let prefix = format!("{label}:");
    std::thread::spawn(move || {
        for channel in ["desk:dev-release-owner", "desk:run-release-owner"] {
            let _ = crate::desk::call_inside(&app, channel, serde_json::json!([prefix]));
        }
    });
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if matches!(event, WindowEvent::Destroyed) {
        crate::preview_ipc::release(window.app_handle(), window.label());
        release_owners(window.app_handle(), window.label());
    }
    if window.label() != MAIN {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        save_bounds(window);
        let shell = window.state::<Shell>();
        if shell.quitting.load(Ordering::SeqCst) {
            return;
        }
        api.prevent_close();
        let _ = window.hide();
        let first = {
            let mut look = lock(&shell.look);
            let first = !look.state.hidden_notice;
            if first {
                look.state.hidden_notice = true;
                look.save();
            }
            first
        };
        if first && cfg!(windows) {
            let t = shell.t();
            let _ = window.app_handle().notification().builder().title(t.app_name).body(t.hidden_notice).show();
        }
    }
}

pub fn window_action(window: &WebviewWindow, action: &str) -> serde_json::Value {
    match action {
        "min" => {
            let _ = window.minimize();
        }
        "max" => {
            if window.is_maximized().unwrap_or(false) {
                let _ = window.unmaximize();
            } else {
                let _ = window.maximize();
            }
        }
        "close" => {
            let _ = window.close();
        }
        _ => {}
    }
    json!({ "maximized": window.is_maximized().unwrap_or(false) })
}

pub fn set_zoom_all(app: &AppHandle, level: f64) {
    for w in app.webview_windows().values() {
        let _ = w.set_zoom(zoom_factor(level));
    }
    guest::refresh_all(app);
}

pub fn set_background_all(app: &AppHandle, hex: &str) {
    if let Some((r, g, b)) = rgba(hex) {
        for w in app.webview_windows().values() {
            let _ = w.set_background_color(Some(Color(r, g, b, 255)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_init_script_ends_every_part_with_a_semicolon() {
        let script = init_script("http://127.0.0.1:4419");
        for part in [SHIM, HEADER, WEBVIEW] {
            assert!(script.contains(&format!("{part};\n")));
        }
        assert!(script.contains("Document.prototype.createElement"));
        assert!(script.contains("\"origin\":\"http://127.0.0.1:4419\""));
    }
}
