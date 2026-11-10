pub mod checker;
pub mod http;
pub mod osv;
pub mod registry;
pub mod versions;

use crate::{arg, Desk, DeskEnv};
use checker::LibChecker;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub const MODULE: &str = "libs";
pub const LIBS_TIMEOUT: Duration = Duration::from_secs(10);

fn request(args: &[Value]) -> (Value, bool) {
    let r = arg(args, 0);
    let field = |k: &str| r.as_object().and_then(|o| o.get(k)).cloned().unwrap_or(Value::Null);
    (field("queries"), field("fresh") == Value::Bool(true))
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let checker = Arc::new(LibChecker::new(Box::new(http::UreqFetch::new(LIBS_TIMEOUT)), env.data_dir.clone(), env.libs_registry.clone(), env.osv_url.clone()));
    let c = checker.clone();
    desk.on(MODULE, "ccm:libs-check", move |_, args| {
        let (queries, fresh) = request(args);
        Ok(c.check(&queries, fresh))
    });
    desk.on(MODULE, "ccm:libs-vulns", move |_, args| {
        let (queries, fresh) = request(args);
        Ok(checker.vulns(&queries, fresh))
    });
}
