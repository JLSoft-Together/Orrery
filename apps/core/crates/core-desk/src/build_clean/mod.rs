pub mod rules;
pub mod scan;
pub mod service;

use crate::git::{run_git, GitResult};
use crate::{arg, Ctx, Desk, DeskEnv};
use serde_json::{json, Value};
use service::{BuildCleanService, CleanHost, EVENT};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub const MODULE: &str = "buildClean";

pub struct CtxHost<'a, 'b>(pub &'a Ctx<'b>);

impl CleanHost for CtxHost<'_, '_> {
    fn git(&self, cwd: &str, args: &[String]) -> GitResult {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_git(Path::new(cwd), &refs, Duration::from_secs(30))
    }

    fn emit(&self, progress: Value) {
        self.0.emit(EVENT, progress)
    }

    fn running(&self, root: &str) -> Vec<String> {
        match self.0.host("buildClean.running", json!({"root": root})) {
            Ok(Value::Array(a)) => a.into_iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            _ => Vec::new(),
        }
    }

    fn daemons(&self) -> Vec<Value> {
        match self.0.host("buildClean.daemons", Value::Null) {
            Ok(Value::Array(a)) => a,
            _ => Vec::new(),
        }
    }

    fn trash(&self, path: &str) -> bool {
        matches!(self.0.host("buildClean.trash", json!({"path": path})), Ok(Value::Bool(true)))
    }

    fn confirm(&self, spec: Value) -> i64 {
        self.0.host("buildClean.confirm", spec).ok().and_then(|v| v.as_i64()).unwrap_or(1)
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    let svc = Arc::new(BuildCleanService::default());
    let s = svc.clone();
    desk.on(MODULE, "ccm:build-scan", move |ctx, args| {
        let Some(roots) = rules::parse_build_roots(arg(args, 0)) else { return Ok(Value::Null) };
        Ok(match s.scan(&roots, &CtxHost(ctx)) {
            Some(list) => Value::Array(list.iter().map(|p| p.to_value()).collect()),
            None => Value::Null,
        })
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:build-clean", move |ctx, args| {
        Ok(match rules::parse_build_clean(arg(args, 0)) {
            Some(req) => s.clean(&req, &CtxHost(ctx)),
            None => json!({"ok": false, "error": "invalid"}),
        })
    });
    desk.on(MODULE, "ccm:build-cancel", move |_, _| Ok(json!(svc.cancel())));
}
