#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::path::{Path, PathBuf};

pub const MARKER: &str = "move-to-applications-asked";

pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf)
}

pub fn in_applications(bundle: &Path, home: Option<&Path>) -> bool {
    let Some(parent) = bundle.parent() else { return false };
    parent == Path::new("/Applications") || home.is_some_and(|h| parent == h.join("Applications"))
}

pub fn should_offer(packaged: bool, bundle: Option<&Path>, home: Option<&Path>, asked: bool) -> bool {
    packaged && !asked && bundle.is_some_and(|b| !in_applications(b, home))
}

pub fn target_of(bundle: &Path) -> Option<PathBuf> {
    bundle.file_name().map(|n| Path::new("/Applications").join(n))
}

pub fn removable_original(bundle: &Path) -> bool {
    let s = bundle.to_string_lossy();
    !s.starts_with("/Volumes/") && !s.contains("/AppTranslocation/")
}

pub fn trash_path(home: &Path, bundle: &Path, stamp: u64) -> Option<PathBuf> {
    let stem = bundle.file_stem()?.to_string_lossy().into_owned();
    Some(home.join(".Trash").join(format!("{stem} {stamp}.app")))
}

#[cfg(target_os = "macos")]
pub fn offer(app: &tauri::AppHandle) -> bool {
    use std::process::{Command, Stdio};

    use tauri::Manager;

    use crate::state::Shell;
    let shell = app.state::<Shell>();
    let marker = shell.data_dir.join(MARKER);
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let bundle = std::env::current_exe().ok().and_then(|e| bundle_of(&e));
    let packaged = !cfg!(debug_assertions) && !crate::config::e2e_isolated();
    if !should_offer(packaged, bundle.as_deref(), home.as_deref(), marker.exists()) {
        return false;
    }
    let (Some(bundle), Some(home)) = (bundle, home) else { return false };
    let Some(target) = target_of(&bundle) else { return false };
    let _ = std::fs::write(&marker, b"1");
    let t = shell.t();
    if !crate::hosts::confirm(app, t.move_title, t.move_detail, t.move_confirm, t.move_later) {
        return false;
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    if target.exists() {
        let Some(old) = trash_path(&home, &target, stamp) else { return false };
        if std::fs::rename(&target, &old).is_err() {
            return false;
        }
    }
    let copied = Command::new("/usr/bin/ditto").arg(&bundle).arg(&target).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success());
    if !copied {
        return false;
    }
    if removable_original(&bundle) {
        if let Some(old) = trash_path(&home, &bundle, stamp + 1) {
            let _ = std::fs::rename(&bundle, &old);
        }
    }
    Command::new("/bin/sh").args(["-c", "sleep 1; /usr/bin/open -n \"$0\""]).arg(&target).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundle_is_the_nearest_app_folder() {
        assert_eq!(bundle_of(Path::new("/Volumes/Orrery/Orrery.app/Contents/MacOS/orrery-shell")), Some(PathBuf::from("/Volumes/Orrery/Orrery.app")));
        assert_eq!(bundle_of(Path::new("/work/target/debug/orrery-shell")), None);
    }

    #[test]
    fn only_a_packaged_app_outside_applications_is_asked_once() {
        let home = Path::new("/Users/me");
        let dmg = Path::new("/Volumes/Orrery/Orrery.app");
        assert!(should_offer(true, Some(dmg), Some(home), false));
        assert!(!should_offer(true, Some(dmg), Some(home), true));
        assert!(!should_offer(false, Some(dmg), Some(home), false));
        assert!(!should_offer(true, None, Some(home), false));
        assert!(!should_offer(true, Some(Path::new("/Applications/Orrery.app")), Some(home), false));
        assert!(!should_offer(true, Some(Path::new("/Users/me/Applications/Orrery.app")), Some(home), false));
        assert!(should_offer(true, Some(Path::new("/Users/me/Downloads/Orrery.app")), Some(home), false));
    }

    #[test]
    fn dmg_and_translocated_copies_stay_where_they_are() {
        assert_eq!(target_of(Path::new("/Users/me/Downloads/Orrery.app")), Some(PathBuf::from("/Applications/Orrery.app")));
        assert!(!removable_original(Path::new("/Volumes/Orrery/Orrery.app")));
        assert!(!removable_original(Path::new("/private/var/folders/x/T/AppTranslocation/ABC/d/Orrery.app")));
        assert!(removable_original(Path::new("/Users/me/Downloads/Orrery.app")));
        assert_eq!(trash_path(Path::new("/Users/me"), Path::new("/Applications/Orrery.app"), 7), Some(PathBuf::from("/Users/me/.Trash/Orrery 7.app")));
    }
}
