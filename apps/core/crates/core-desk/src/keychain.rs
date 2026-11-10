use crate::Ctx;
use serde_json::{json, Value};

pub fn available(ctx: &Ctx) -> bool {
    matches!(ctx.host("keychain.available", Value::Null), Ok(Value::Bool(true)))
}

pub fn seal(ctx: &Ctx, plain: &str) -> Option<String> {
    match ctx.host("keychain.seal", json!({"plain": plain})) {
        Ok(Value::String(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

pub fn unseal(ctx: &Ctx, sealed: &str) -> Option<String> {
    match ctx.host("keychain.unseal", json!({"sealed": sealed})) {
        Ok(Value::String(s)) => Some(s),
        _ => None,
    }
}
