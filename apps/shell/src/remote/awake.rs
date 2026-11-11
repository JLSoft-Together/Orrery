use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

use super::Remote;

pub const AWAKE_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows";

static AT_START: OnceLock<bool> = OnceLock::new();

pub fn args_for(remote_on: bool) -> Option<&'static str> {
    remote_on.then_some(AWAKE_ARGS)
}

pub fn browser_args(app: &AppHandle) -> Option<&'static str> {
    let on = *AT_START.get_or_init(|| app.try_state::<Remote>().is_some_and(|r| r.wants_on()));
    args_for(on)
}
