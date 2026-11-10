pub mod awake;
pub mod native;

use crate::{arg, js, Desk, DeskEnv};
use awake::{mode_of, KeepAwake, AWAKE_FILE, DEFAULT_MODE};
use serde_json::{json, Value};

pub const MODULE: &str = "power";
pub const EVENT: &str = "ccm:awake-event";

pub fn read_mode(file: &std::path::Path) -> &'static str {
    std::fs::read_to_string(file).ok().and_then(|t| js::parse(&t)).and_then(|v| v.get("mode").and_then(mode_of)).unwrap_or(DEFAULT_MODE)
}

fn count(v: &Value, k: &str) -> Option<i64> {
    let f = v.get(k)?.as_f64()?;
    (f.fract() == 0.0 && (0.0..100_000.0).contains(&f)).then_some(f as i64)
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let file = env.data_dir.join(AWAKE_FILE);
    let emitter = desk.emitter();
    let keep = KeepAwake::new(native::power(), read_mode(&file), native::grace_ms(), Box::new(move |state| emitter.emit(EVENT, state)));
    let k = keep.clone();
    desk.on(MODULE, "ccm:awake-state", move |_, _| Ok(k.state()));
    let k = keep.clone();
    desk.on(MODULE, "ccm:awake-mode", move |_, args| {
        let Some(mode) = mode_of(arg(args, 0)) else { return Ok(Value::Null) };
        k.set_mode(mode);
        let _ = std::fs::write(&file, js::stringify(&json!({"mode": mode})));
        Ok(k.state())
    });
    let k = keep.clone();
    desk.on(MODULE, "desk:awake-busy", move |_, args| {
        let p = arg(args, 0);
        let (Some(agents), Some(jobs)) = (count(p, "agents"), count(p, "jobs")) else { return Ok(json!(false)) };
        k.set_busy(agents, jobs);
        Ok(json!(true))
    });
    let k = keep.clone();
    desk.on(MODULE, "desk:awake-power", move |_, _| {
        k.power_changed();
        Ok(json!(true))
    });
    desk.on(MODULE, "desk:awake-dispose", move |_, _| {
        keep.dispose();
        Ok(json!(true))
    });
}
