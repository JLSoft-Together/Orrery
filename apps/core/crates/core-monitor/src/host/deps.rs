use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use core_store::event::{FocusWindowResult, FolderPickResult, OpenApp, OpenResult};
use serde_json::{json, Value};

use super::client::{HostClient, HostError};
use super::protocol::{valid_dir, valid_pid, FOCUS_TIMEOUT_MS, LAUNCH_TIMEOUT_MS, OPEN_TIMEOUT_MS, PICK_TIMEOUT_MS};
use crate::job_stop::JobStopResult;
use crate::monitor::{Deps, LaunchError, LaunchMode, SystemDeps, TerminalApp};
use crate::process::ProcessInfo;

pub struct HostDeps {
    client: Arc<HostClient>,
    system: SystemDeps,
}

impl HostDeps {
    pub fn new(client: Arc<HostClient>) -> HostDeps {
        HostDeps { client, system: SystemDeps }
    }

    fn call(&self, method: &str, params: Value, ms: u64) -> Result<Value, HostError> {
        self.client.call(method, params, Duration::from_millis(ms))
    }
}

fn result_of<T: serde::de::DeserializeOwned>(body: &Value, fallback: T) -> T {
    body.get("result").and_then(|r| serde_json::from_value(r.clone()).ok()).unwrap_or(fallback)
}

fn launch_error(s: &str) -> LaunchError {
    match s {
        "notFound" => LaunchError::NotFound,
        "denied" => LaunchError::Denied,
        "notInstalled" => LaunchError::NotInstalled,
        _ => LaunchError::Failed,
    }
}

impl Deps for HostDeps {
    fn now(&self) -> i64 {
        self.system.now()
    }

    fn is_pid_alive(&self, pid: u32) -> bool {
        self.system.is_pid_alive(pid)
    }

    fn query_processes(&self, pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
        self.system.query_processes(pids)
    }

    fn stop_job(&self, job_id: &str) -> JobStopResult {
        self.system.stop_job(job_id)
    }

    fn launch(&self, dir: &str, mode: LaunchMode, app: Option<TerminalApp>) -> Option<Result<(), LaunchError>> {
        if !valid_dir(dir) {
            return Some(Err(LaunchError::Failed));
        }
        let mut params = json!({ "dir": dir, "mode": mode_str(mode) });
        if let Some(a) = app {
            params["app"] = json!(if a == TerminalApp::Iterm { "iterm" } else { "terminal" });
        }
        match self.call("launch", params, LAUNCH_TIMEOUT_MS) {
            Ok(body) if body.get("ok").and_then(Value::as_bool) == Some(true) => Some(Ok(())),
            Ok(body) => Some(Err(launch_error(body.get("error").and_then(Value::as_str).unwrap_or("failed")))),
            Err(HostError::Closed) => None,
            Err(_) => Some(Err(LaunchError::Failed)),
        }
    }

    fn focus_window(&self, pid: u32) -> Option<FocusWindowResult> {
        if !valid_pid(pid) {
            return Some(FocusWindowResult::NotFound);
        }
        match self.call("focus", json!({ "pid": pid }), FOCUS_TIMEOUT_MS) {
            Ok(body) => Some(result_of(&body, FocusWindowResult::Failed)),
            Err(HostError::Closed) => None,
            Err(_) => Some(FocusWindowResult::Failed),
        }
    }

    fn open_folder(&self, dir: &str, app: OpenApp) -> Option<OpenResult> {
        if !valid_dir(dir) {
            return Some(OpenResult::NotFound);
        }
        match self.call("open", json!({ "dir": dir, "app": app.as_str() }), OPEN_TIMEOUT_MS) {
            Ok(body) => Some(result_of(&body, OpenResult::Failed)),
            Err(HostError::Closed) => None,
            Err(_) => Some(OpenResult::Failed),
        }
    }

    fn pick_folder(&self) -> Option<(FolderPickResult, Option<String>)> {
        match self.call("pickFolder", json!({}), PICK_TIMEOUT_MS) {
            Ok(body) => {
                let result = result_of(&body, FolderPickResult::Failed);
                let dir = body.get("dir").and_then(Value::as_str).map(str::to_owned);
                match (result, dir) {
                    (FolderPickResult::Ok, Some(d)) if valid_dir(&d) => Some((FolderPickResult::Ok, Some(d))),
                    (FolderPickResult::Ok, _) => Some((FolderPickResult::Failed, None)),
                    (r, _) => Some((r, None)),
                }
            }
            Err(HostError::Closed) => None,
            Err(_) => Some((FolderPickResult::Failed, None)),
        }
    }
}

fn mode_str(m: LaunchMode) -> &'static str {
    match m {
        LaunchMode::New => "new",
        LaunchMode::Continue => "continue",
    }
}
