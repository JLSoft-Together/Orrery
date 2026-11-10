use super::gate::{self, denied};
use super::{envcheck, i18n, scripts};
use crate::{arg, Desk, DeskEnv};
use serde_json::{json, Value};

pub const MODULE: &str = "previewTools";

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key).unwrap_or(&Value::Null)
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    desk.on(MODULE, "ccm:env-check", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| envcheck::check_env(&d)).unwrap_or_else(denied)));
    desk.on(MODULE, "ccm:env-reveal", |ctx, args| {
        let req = arg(args, 0);
        Ok(gate::check_dir(ctx, field(req, "root")).map(|d| envcheck::reveal_env(&d, field(req, "file"), field(req, "key"))).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:i18n-scan", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| i18n::scan_i18n(&d)).unwrap_or_else(denied)));
    desk.on(MODULE, "ccm:scripts", |ctx, args| {
        let dir = arg(args, 0);
        let real = gate::repo_of(ctx, dir).or_else(|| gate::allowed(ctx, dir, false).map(|a| a.real));
        Ok(match real.filter(|r| std::fs::metadata(r).map(|m| m.is_dir()).unwrap_or(false)) {
            Some(r) => scripts::detect_scripts(&r),
            None => json!(null),
        })
    });
}
