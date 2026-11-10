use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::serve::Serve;
use crate::state::{lock, Shell};

pub fn stop(app: &AppHandle) -> Value {
    let shell = app.state::<Shell>();
    let taken = lock(&shell.serve).take();
    if let Some(mut serve) = taken {
        serve.stop();
    }
    Value::Null
}

pub fn start(app: &AppHandle) -> Value {
    let shell = app.state::<Shell>();
    if shell.stopped.load(std::sync::atomic::Ordering::SeqCst) {
        return Value::Null;
    }
    let mut slot = lock(&shell.serve);
    if slot.is_some() {
        return Value::Null;
    }
    match Serve::start(shell.serve_cfg.clone()) {
        Ok(serve) => *slot = Some(serve),
        Err(e) => eprintln!("[shell] collector restart failed: {e:?}"),
    }
    Value::Null
}
