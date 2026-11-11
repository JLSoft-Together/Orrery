use std::sync::OnceLock;

use serde_json::{json, Value};
use tauri::AppHandle;

use core_desk::bridge::install::BRIDGE_EXE;

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
