use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::state::Shell;

pub const POWER_POLL: Duration = Duration::from_secs(30);
const COUNT_MAX: u64 = 100_000;

pub fn busy_args(msg: &Value) -> Option<Value> {
    if msg.get("type").and_then(Value::as_str) != Some("busy") {
        return None;
    }
    let count = |k: &str| msg.get(k).and_then(Value::as_u64).filter(|n| *n < COUNT_MAX);
    Some(json!([{ "agents": count("agents")?, "jobs": count("jobs")? }]))
}

fn call(app: &AppHandle, channel: &str, args: Value) {
    if let Err(e) = crate::desk::call_inside(app, channel, args) {
        eprintln!("[shell] {channel}: {e}");
    }
}

fn stopped(app: &AppHandle) -> bool {
    app.try_state::<Shell>().is_none_or(|s| s.stopped.load(Ordering::SeqCst))
}

pub fn busy(app: &AppHandle, msg: &Value) {
    if stopped(app) {
        return;
    }
    if let Some(args) = busy_args(msg) {
        call(app, "desk:awake-busy", args);
    }
}

pub fn idle(app: &AppHandle) {
    call(app, "desk:awake-busy", json!([{ "agents": 0, "jobs": 0 }]));
}

pub fn dispose(app: &AppHandle) {
    call(app, "desk:awake-dispose", json!([null]));
}

pub fn changed(last: &mut Option<bool>, now: bool) -> bool {
    let moved = last.is_some_and(|l| l != now);
    *last = Some(now);
    moved
}

pub fn watch(app: &AppHandle) {
    let app = app.clone();
    let _ = std::thread::Builder::new().name("power-watch".into()).spawn(move || {
        let mut last = None;
        while !stopped(&app) {
            if changed(&mut last, core_desk::power::native::on_battery_now()) {
                call(&app, "desk:awake-power", json!([null]));
            }
            std::thread::sleep(POWER_POLL);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_counts_pass_to_the_keep_awake_handler() {
        assert_eq!(busy_args(&json!({ "type": "busy", "agents": 2, "jobs": 1 })), Some(json!([{ "agents": 2, "jobs": 1 }])));
        assert_eq!(busy_args(&json!({ "type": "busy", "agents": -1, "jobs": 0 })), None);
        assert_eq!(busy_args(&json!({ "type": "busy", "agents": 1.5, "jobs": 0 })), None);
        assert_eq!(busy_args(&json!({ "type": "busy", "agents": 100_000, "jobs": 0 })), None);
        assert_eq!(busy_args(&json!({ "type": "phone", "agents": 1, "jobs": 0 })), None);
    }

    #[test]
    fn a_power_change_is_only_a_flip_after_the_first_reading() {
        let mut last = None;
        assert!(!changed(&mut last, false));
        assert!(!changed(&mut last, false));
        assert!(changed(&mut last, true));
        assert!(!changed(&mut last, true));
        assert!(changed(&mut last, false));
    }
}
