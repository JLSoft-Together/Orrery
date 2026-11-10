pub mod clean;
pub mod moves;
pub mod ops;
pub mod procs;
pub mod rules;
pub mod service;
pub mod strip;
pub mod tree;

#[cfg(test)]
mod tests;

use crate::{arg, Ctx, Desk, DeskEnv, DeskError};
use procs::AgentProc;
use rules::{agent_of, parse_clean_request, parse_strip_days, Agent};
use serde_json::{json, Value};
use service::{fail, AgentDataService, AgentHost, Roots, EVENT};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const MODULE: &str = "agentData";
pub const STORE_FILE: &str = "agent-data-move.json";

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

fn home() -> PathBuf {
    let h = if cfg!(windows) { var("USERPROFILE").or_else(|| var("HOME")) } else { var("HOME") };
    PathBuf::from(h.unwrap_or_default())
}

fn app_data(home: &Path) -> PathBuf {
    if cfg!(windows) {
        var("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData").join("Roaming"))
    } else {
        home.join("Library").join("Application Support")
    }
}

pub fn roots_from_env() -> Roots {
    let home = home();
    let claude = var("CLAUDE_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".claude"));
    let codex = var("CODEX_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".codex"));
    let app = app_data(&home).join("Claude");
    let claude_app = (app.exists() && var("CCM_E2E_AGENT_TARGET").is_none()).then(|| crate::util::path_str(&app));
    Roots { claude: crate::util::path_str(&claude), codex: crate::util::path_str(&codex), claude_app }
}

pub struct CtxHost<'a, 'b>(pub &'a Ctx<'b>);

impl AgentHost for CtxHost<'_, '_> {
    fn roots(&self) -> Roots {
        roots_from_env()
    }

    fn procs(&self, agent: Agent) -> Vec<AgentProc> {
        procs::agent_procs(agent)
    }

    fn confirm(&self, spec: Value) -> Option<i64> {
        self.0.host("agentData.confirm", spec).ok().and_then(|v| v.as_i64())
    }

    fn pick_folder(&self, purpose: &str) -> Option<String> {
        match self.0.host("agentData.pickFolder", json!({"purpose": purpose})) {
            Ok(Value::String(s)) if !s.is_empty() => Some(s),
            _ => None,
        }
    }

    fn stop_collector(&self) {
        let _ = self.0.host("agentData.stopCollector", Value::Null);
    }

    fn start_collector(&self) {
        let _ = self.0.host("agentData.startCollector", Value::Null);
    }

    fn emit(&self, progress: Value) {
        self.0.emit(EVENT, progress)
    }

    fn is_local(&self, dir: &str) -> bool {
        moves::is_local_dir(dir)
    }

    fn free_bytes(&self, dir: &str) -> Option<f64> {
        moves::free_bytes(dir)
    }

    fn alive(&self, pid: f64) -> bool {
        procs::alive(pid)
    }

    fn now(&self) -> f64 {
        crate::util::now_ms()
    }
}

type Op = fn(&AgentDataService, Agent, &dyn AgentHost) -> service::Out;

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let svc = Arc::new(AgentDataService::new(env.data_dir.join(STORE_FILE)));
    if var("CCM_AGENT_DATA_RECOVER").is_none_or(|v| v.trim() != "0") {
        svc.recover();
    }
    let s = svc.clone();
    desk.on(MODULE, "desk:agentdata-recover", move |_, _| Ok(json!(s.recover().iter().map(|a| a.as_str()).collect::<Vec<_>>())));
    let s = svc.clone();
    desk.on(MODULE, "ccm:agentdata-state", move |ctx, _| Ok(s.state(&CtxHost(ctx))));
    let s = svc.clone();
    desk.on(MODULE, "ccm:agentdata-scan", move |ctx, _| s.scan(&CtxHost(ctx)).map_err(DeskError::Failed));
    let s = svc.clone();
    desk.on(MODULE, "ccm:agentdata-cancel", move |_, _| Ok(json!(s.cancel())));
    let s = svc.clone();
    desk.on(MODULE, "ccm:agentdata-clean", move |ctx, args| match parse_clean_request(arg(args, 0)) {
        Some(req) => s.clean(&req, &CtxHost(ctx)).map_err(DeskError::Failed),
        None => Ok(fail("invalid")),
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:agentdata-strip", move |ctx, args| match parse_strip_days(arg(args, 0)) {
        Some(days) => s.strip(days, &CtxHost(ctx)).map_err(DeskError::Failed),
        None => Ok(fail("invalid")),
    });
    let ops: [(&str, Op); 4] = [
        ("ccm:agentdata-move", AgentDataService::move_out),
        ("ccm:agentdata-undo", AgentDataService::undo),
        ("ccm:agentdata-delete-original", AgentDataService::delete_original),
        ("ccm:agentdata-discard", AgentDataService::discard),
    ];
    for (channel, op) in ops {
        let s = svc.clone();
        desk.on(MODULE, channel, move |ctx, args| match agent_of(arg(args, 0)) {
            Some(agent) => op(&s, agent, &CtxHost(ctx)).map_err(DeskError::Failed),
            None => Ok(fail("invalid")),
        });
    }
}
