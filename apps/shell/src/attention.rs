use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;
use tauri::{AppHandle, Manager, UserAttentionType};

use crate::state::{lock, MAIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Bounce,
    Cancel,
}

#[derive(Default)]
pub struct Attention {
    shown: HashMap<String, bool>,
    bouncing: bool,
}

impl Attention {
    pub fn handle(&mut self, msg: &Value) -> Option<Op> {
        let tag = msg.get("tag").and_then(Value::as_str).filter(|t| !t.is_empty() && t.len() <= 200)?;
        match msg.get("type").and_then(Value::as_str)? {
            "toast.show" => {
                let persistent = msg.get("persistent").and_then(Value::as_bool).unwrap_or(false);
                let fresh = self.shown.insert(tag.to_owned(), persistent).is_none();
                fresh.then(|| {
                    self.bouncing = true;
                    Op::Bounce
                })
            }
            "toast.hide" => {
                self.shown.remove(tag);
                None
            }
            _ => None,
        }
    }

    pub fn settle(&mut self) -> Option<Op> {
        self.shown.retain(|_, persistent| *persistent);
        std::mem::take(&mut self.bouncing).then_some(Op::Cancel)
    }

    pub fn clear(&mut self) -> Option<Op> {
        self.shown.clear();
        std::mem::take(&mut self.bouncing).then_some(Op::Cancel)
    }
}

fn state() -> &'static Mutex<Attention> {
    static STATE: OnceLock<Mutex<Attention>> = OnceLock::new();
    STATE.get_or_init(Mutex::default)
}

fn apply(app: &AppHandle, op: Option<Op>) {
    let (Some(op), Some(w)) = (op, app.get_webview_window(MAIN)) else { return };
    let _ = match op {
        Op::Bounce if !w.is_focused().unwrap_or(false) => w.request_user_attention(Some(UserAttentionType::Critical)),
        Op::Bounce => Ok(()),
        Op::Cancel => w.request_user_attention(None),
    };
}

pub fn handle(app: &AppHandle, msg: &Value) {
    let op = lock(state()).handle(msg);
    apply(app, op);
}

pub fn settle(app: &AppHandle) {
    let op = lock(state()).settle();
    apply(app, op);
}

pub fn clear(app: &AppHandle) {
    let op = lock(state()).clear();
    apply(app, op);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_new_toast_bounces_once_and_focus_cancels_like_attention_ts() {
        let mut a = Attention::default();
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t1", "persistent": false})), Some(Op::Bounce));
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t1", "persistent": false})), None);
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t2", "persistent": true})), Some(Op::Bounce));
        assert_eq!(a.settle(), Some(Op::Cancel));
        assert_eq!(a.settle(), None);
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t2", "persistent": true})), None);
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t1"})), Some(Op::Bounce));
        assert_eq!(a.handle(&json!({"type": "toast.hide", "tag": "t1"})), None);
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t1"})), Some(Op::Bounce));
        assert_eq!(a.clear(), Some(Op::Cancel));
        assert_eq!(a.handle(&json!({"type": "toast.show", "tag": "t2", "persistent": true})), Some(Op::Bounce));
        assert_eq!(a.handle(&json!({"type": "busy", "tag": "x"})), None);
        assert_eq!(a.handle(&json!({"type": "toast.show"})), None);
    }
}
