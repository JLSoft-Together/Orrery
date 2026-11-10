use std::sync::atomic::Ordering;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::state::Shell;
use crate::window::{open_compact, show_main};

pub const TRAY: &str = "main";

pub fn login_supported() -> bool {
    !cfg!(debug_assertions)
}

pub fn open_at_login(app: &AppHandle) -> bool {
    login_supported() && app.autolaunch().is_enabled().unwrap_or(false)
}

pub fn set_open_at_login(app: &AppHandle, on: bool) -> bool {
    if !login_supported() {
        return false;
    }
    let _ = if on { app.autolaunch().enable() } else { app.autolaunch().disable() };
    refresh(app);
    open_at_login(app)
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let t = app.state::<Shell>().t();
    let open = MenuItem::with_id(app, "open", t.tray_open, true, None::<&str>)?;
    let compact = MenuItem::with_id(app, "compact", t.tray_compact, true, None::<&str>)?;
    let login = CheckMenuItem::with_id(app, "login", t.tray_login, login_supported(), open_at_login(app), None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", t.tray_logs, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t.tray_quit, true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&open, &compact, &sep1, &login, &logs, &sep2, &quit])
}

pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(m)) = (app.tray_by_id(TRAY), menu(app)) {
        let _ = tray.set_menu(Some(m));
    }
}

pub fn quit(app: &AppHandle) {
    let shell = app.state::<Shell>();
    if shell.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(w) = app.get_window(crate::state::MAIN) {
        crate::window::save_bounds(&w);
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        handle.state::<Shell>().shutdown();
        handle.exit(0);
    });
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut b = TrayIconBuilder::with_id(TRAY).tooltip("Orrery").menu(&menu(app)?).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon().cloned() {
        b = b.icon(icon);
    }
    b.on_menu_event(|app, event| match event.id().as_ref() {
        "open" => show_main(app),
        "compact" => {
            let h = app.clone();
            std::thread::spawn(move || open_compact(&h));
        }
        "login" => {
            let on = !open_at_login(app);
            set_open_at_login(app, on);
        }
        "logs" => {
            let dir = app.state::<Shell>().data_dir.clone();
            let _ = app.opener().open_path(dir.to_string_lossy(), None::<&str>);
        }
        "quit" => quit(app),
        _ => {}
    })
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            show_main(tray.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}
