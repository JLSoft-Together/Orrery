use super::ipc::LazySvc;
use super::ipc_render::{explorer, head_of};
use super::xml::is_layout_target;
use super::MODULE;
use crate::{arg, Ctx, Desk};
use serde_json::{json, Value};
use std::sync::Arc;

fn gate_file(ctx: &Ctx, req: &Value) -> Option<String> {
    explorer(ctx, &req["file"]).filter(|f| is_layout_target(f, None, || head_of(f)))
}

pub fn register(desk: &mut Desk, lazy: Arc<LazySvc>) {
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-model-status", move |ctx, args| {
        Ok(gate_file(ctx, arg(args, 0)).map(|f| l.get().model.info(&f)).unwrap_or(Value::Null))
    });
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-model-refresh", move |ctx, args| {
        Ok(gate_file(ctx, arg(args, 0)).map(|f| l.get().model.refresh_file(&f)).unwrap_or(Value::Null))
    });
    let l = lazy.clone();
    desk.on(MODULE, "ccm:layout-model-pick", move |ctx, args| {
        let req = arg(args, 0);
        let file = gate_file(ctx, req);
        let module = req["module"].as_str().filter(|m| m.chars().count() <= 300);
        Ok(match (file, module) {
            (Some(f), Some(m)) => l.get().model.pick_app(&f, m),
            _ => Value::Null,
        })
    });
    let l = lazy;
    desk.on(MODULE, "desk:layout-after-build", move |_, args| {
        let text = |i: usize| arg(args, i).as_str().unwrap_or("").to_string();
        l.get().model.after_build(&text(0), &text(1), &text(2));
        Ok(json!(true))
    });
}
