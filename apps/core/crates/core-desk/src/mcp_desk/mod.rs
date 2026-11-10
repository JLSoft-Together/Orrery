pub mod deps;
pub mod desk;
pub mod scope;
pub mod shot;

#[cfg(test)]
mod tests;

use crate::{arg, Caller, Ctx, Desk, DeskEnv};
use deps::{HostDeps, NativeDeps};
use serde_json::Value;
use std::path::PathBuf;

pub const MODULE: &str = "mcp";
pub const SHOT_DEADLINE_MS: u64 = 7_000;

fn home() -> String {
    std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default()
}

pub fn opts_of(o: &Value) -> desk::Opts {
    let text = |k: &str| o.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    let mut self_pids = vec![std::process::id() as i64];
    match o.get("selfPid") {
        Some(Value::Number(n)) => self_pids.extend(n.as_i64()),
        Some(Value::Array(a)) => self_pids.extend(a.iter().filter_map(Value::as_i64)),
        _ => {}
    }
    desk::Opts {
        win: match text("platform") {
            Some(p) => p == "win32",
            None => cfg!(windows),
        },
        home: text("home").unwrap_or_else(home),
        self_pids,
        fallback_dir: text("fallbackDir").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("orrery-layout")),
        tmp_root: text("tmpRoot").map(PathBuf::from).unwrap_or_else(std::env::temp_dir),
        deadline_ms: o.get("deadlineMs").and_then(Value::as_u64).unwrap_or(SHOT_DEADLINE_MS),
    }
}

fn native(o: &Value) -> bool {
    o.get("deps").and_then(Value::as_str) == Some("native")
}

pub fn call_desk(ctx: &Ctx, caller: &Caller, args: &[Value]) -> Value {
    let (call, o) = (arg(args, 0), arg(args, 1));
    if !call.is_object() {
        return desk::fail("unavailable");
    }
    let opts = opts_of(o);
    if native(o) {
        desk::run(&NativeDeps { ctx, caller }, &opts, call)
    } else {
        desk::run(&HostDeps { ctx }, &opts, call)
    }
}

pub fn call_screenshot(ctx: &Ctx, caller: &Caller, args: &[Value]) -> Value {
    let (call, o) = (arg(args, 0), arg(args, 1));
    let opts = opts_of(o);
    if native(o) {
        shot::screenshot(&NativeDeps { ctx, caller }, &opts.tmp_root, opts.deadline_ms, call)
    } else {
        shot::screenshot(&HostDeps { ctx }, &opts.tmp_root, opts.deadline_ms, call)
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    let caller = desk.caller();
    let c = caller.clone();
    desk.on(MODULE, "desk:mcp-desk", move |ctx, args| Ok(call_desk(ctx, &c, args)));
    desk.on(MODULE, "desk:mcp-screenshot", move |ctx, args| Ok(call_screenshot(ctx, &caller, args)));
}
