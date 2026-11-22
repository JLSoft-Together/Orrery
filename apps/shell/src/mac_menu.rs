#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::strings::Strings;

pub const QUIT_ID: &str = "app-quit";
pub const CLOSE_ID: &str = "app-close";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    About,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Minimize,
    Zoom,
    Close,
    Front,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: Option<&'static str>,
    pub window_menu: bool,
    pub items: Vec<Item>,
}

pub fn model(t: &Strings) -> Vec<Section> {
    use Item::*;
    vec![
        Section { title: None, window_menu: false, items: vec![About, Separator, Services, Separator, Hide, HideOthers, ShowAll, Separator, Quit] },
        Section { title: Some(t.menu_edit), window_menu: false, items: vec![Undo, Redo, Separator, Cut, Copy, Paste, SelectAll] },
        Section { title: Some(t.menu_window), window_menu: true, items: vec![Minimize, Zoom, Close, Separator, Front] },
    ]
}

pub fn quit_label(t: &Strings, app_name: &str) -> String {
    format!("{} {app_name}", t.tray_quit)
}

pub fn accelerator(item: Item) -> Option<&'static str> {
    match item {
        Item::Quit => Some("CmdOrCtrl+Q"),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
pub fn build(app: &tauri::AppHandle, t: &Strings) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem as P, Submenu, WINDOW_SUBMENU_ID};
    let info = app.package_info();
    let about = AboutMetadata { name: Some(info.name.clone()), version: Some(info.version.to_string()), copyright: app.config().bundle.copyright.clone(), ..Default::default() };
    let menu = Menu::new(app)?;
    for section in model(t) {
        let title = section.title.map_or_else(|| info.name.clone(), str::to_owned);
        let sub = if section.window_menu { Submenu::with_id(app, WINDOW_SUBMENU_ID, title, true)? } else { Submenu::new(app, title, true)? };
        for item in section.items {
            match item {
                Item::About => sub.append(&P::about(app, None, Some(about.clone()))?)?,
                Item::Services => sub.append(&P::services(app, None)?)?,
                Item::Hide => sub.append(&P::hide(app, None)?)?,
                Item::HideOthers => sub.append(&P::hide_others(app, None)?)?,
                Item::ShowAll => sub.append(&P::show_all(app, None)?)?,
                Item::Quit => sub.append(&MenuItem::with_id(app, QUIT_ID, quit_label(t, &info.name), true, accelerator(item))?)?,
                Item::Undo => sub.append(&P::undo(app, None)?)?,
                Item::Redo => sub.append(&P::redo(app, None)?)?,
                Item::Cut => sub.append(&P::cut(app, None)?)?,
                Item::Copy => sub.append(&P::copy(app, None)?)?,
                Item::Paste => sub.append(&P::paste(app, None)?)?,
                Item::SelectAll => sub.append(&P::select_all(app, None)?)?,
                Item::Minimize => sub.append(&P::minimize(app, None)?)?,
                Item::Zoom => sub.append(&P::maximize(app, None)?)?,
                Item::Close => sub.append(&MenuItem::with_id(app, CLOSE_ID, t.menu_close, true, accelerator(item))?)?,
                Item::Front => sub.append(&P::bring_all_to_front(app, None)?)?,
                Item::Separator => sub.append(&P::separator(app)?)?,
            }
        }
        menu.append(&sub)?;
    }
    Ok(menu)
}

#[cfg(target_os = "macos")]
pub fn close_focused(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.webview_windows().into_values().find(|w| w.is_focused().unwrap_or(false)) {
        let _ = w.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::strings;

    #[test]
    fn the_menu_matches_the_electron_roles() {
        let m = model(strings("en"));
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].title, None);
        assert_eq!(m[0].items.first(), Some(&Item::About));
        assert_eq!(m[0].items.last(), Some(&Item::Quit));
        assert!(m[0].items.contains(&Item::Hide) && m[0].items.contains(&Item::HideOthers) && m[0].items.contains(&Item::ShowAll));
        assert_eq!(m[1].title, Some("Edit"));
        for i in [Item::Undo, Item::Redo, Item::Cut, Item::Copy, Item::Paste, Item::SelectAll] {
            assert!(m[1].items.contains(&i), "{i:?}");
        }
        assert!(m[2].window_menu);
        assert_eq!(m[2].items, vec![Item::Minimize, Item::Zoom, Item::Close, Item::Separator, Item::Front]);
        assert_eq!(model(strings("vi"))[2].title, Some("Cửa sổ"));
    }

    #[test]
    fn only_quit_has_a_shortcut_so_cmd_w_stays_with_the_page() {
        assert_eq!(accelerator(Item::Quit), Some("CmdOrCtrl+Q"));
        assert_eq!(accelerator(Item::Close), None);
        assert_eq!(accelerator(Item::Copy), None);
        assert_eq!(quit_label(strings("en"), "Orrery"), "Quit Orrery");
        assert_eq!(quit_label(strings("vi"), "Orrery"), "Thoát Orrery");
    }
}
