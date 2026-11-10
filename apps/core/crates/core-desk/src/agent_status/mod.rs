pub mod status;
pub mod watch;

use crate::{arg, js, Desk, DeskEnv};
use serde_json::{json, Value};
use watch::{AgentStatusWatch, Timing};

pub const MODULE: &str = "agentStatus";
const KEY_MAX: usize = 128;

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    let watch = AgentStatusWatch::new(desk.emitter(), Timing::default());
    desk.on(MODULE, "ccm:agent-status-watch", move |_, args| {
        let Some(key) = arg(args, 0).as_str().filter(|k| !k.is_empty() && js::len(k) <= KEY_MAX) else { return Ok(json!(false)) };
        watch.set(key, arg(args, 1).as_str().map(str::to_string));
        Ok(Value::Bool(true))
    });
}
