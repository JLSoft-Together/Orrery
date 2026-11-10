use super::Inner;
use crate::run::parse::parse_build_error;
use crate::run::types::{RunResult, SessionPatch, EnvIssue};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

fn device_lost_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)lost connection to device|device (?:was )?disconnected|device .* (?:not found|is offline)|no connected devices").expect("regex"))
}

fn str_of(p: &Map<String, Value>, k: &str) -> Option<String> {
    p.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from)
}

fn error_text(e: &Value) -> String {
    if let Some(s) = e.as_str() {
        return s.to_string();
    }
    if let Some(m) = e.get("message").and_then(Value::as_str) {
        return m.to_string();
    }
    crate::js::stringify(e)
}

fn split_lines(s: &str) -> Vec<&str> {
    s.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

fn patch() -> SessionPatch {
    SessionPatch::default()
}

impl Inner {
    fn started(&self) -> bool {
        self.st.lock().unwrap().started
    }

    pub(super) fn text(&self, line: &str, stream: &str, level: Option<&str>) {
        if device_lost_re().is_match(line) {
            self.st.lock().unwrap().device_lost = true;
        }
        self.ctx.hooks.log(stream, line, level, None);
        let Some(err) = parse_build_error(line, &self.cwd) else { return };
        {
            let mut s = self.st.lock().unwrap();
            if err.severity == "error" && !s.started {
                s.saw_build_error = true;
            }
        }
        self.ctx.hooks.error(err);
    }

    pub(super) fn plain(&self, line: &str) {
        if !line.trim().is_empty() {
            let stream = if self.started() { "app" } else { "build" };
            self.text(line, stream, None);
        }
    }

    pub(super) fn on_event(&self, event: &str, params: &Map<String, Value>) {
        let hooks = &self.ctx.hooks;
        let cur = self.current();
        match event {
            "daemon.connected" => {
                let pid = params.get("pid").and_then(Value::as_f64).map(|p| format!(" (pid {})", crate::js::number(p))).unwrap_or_default();
                hooks.log("build", &format!("flutter daemon connected{pid}"), None, None);
            }
            "app.start" => {
                let id = {
                    let mut s = self.st.lock().unwrap();
                    if let Some(a) = str_of(params, "appId") {
                        s.app_id = Some(a);
                    }
                    s.app_id.clone()
                };
                self.set_state(&cur, SessionPatch { app_id: Some(id), ..patch() });
            }
            "app.progress" => {
                if params.get("finished") == Some(&Value::Bool(true)) {
                    return self.set_state(&cur, SessionPatch { progress: Some(None), ..patch() });
                }
                if let Some(m) = str_of(params, "message") {
                    hooks.log("build", &m, None, None);
                    self.set_state(&cur, SessionPatch { progress: Some(Some(m)), ..patch() });
                }
            }
            "app.debugPort" => {
                if let Some(u) = str_of(params, "wsUri") {
                    self.set_state(&cur, SessionPatch { vm_service_uri: Some(Some(u)), ..patch() });
                }
            }
            "app.devTools" => {
                if let Some(u) = str_of(params, "uri") {
                    self.set_state(&cur, SessionPatch { devtools_uri: Some(Some(u)), ..patch() });
                }
            }
            "app.started" => {
                self.st.lock().unwrap().started = true;
                self.set_state("running", SessionPatch { progress: Some(None), ..patch() });
            }
            "app.log" => {
                let log = params.get("log").and_then(Value::as_str).unwrap_or("");
                let mut lines = split_lines(log);
                if lines.len() > 1 && lines.last() == Some(&"") {
                    lines.pop();
                }
                let level = (params.get("error") == Some(&Value::Bool(true))).then_some("E");
                for l in lines {
                    self.text(l, "app", level);
                }
            }
            "app.webLaunchUrl" => {
                if let Some(u) = str_of(params, "url") {
                    hooks.log("app", &u, None, None);
                }
            }
            "app.warning" => {
                if let Some(w) = str_of(params, "warning") {
                    hooks.log(if self.started() { "app" } else { "build" }, &w, Some("W"), None);
                }
            }
            "app.stop" => {
                if let Some(e) = str_of(params, "error") {
                    self.text(&e, if self.started() { "app" } else { "build" }, Some("E"));
                }
            }
            "daemon.logMessage" => {
                let Some(m) = str_of(params, "message") else { return };
                let level = match params.get("level").and_then(Value::as_str) {
                    Some("error") => Some("E"),
                    Some("warning") => Some("W"),
                    _ => None,
                };
                for l in split_lines(&m) {
                    self.text(l, "build", level);
                }
            }
            _ => {}
        }
    }

    pub(super) fn reload(&self, full: bool) -> RunResult {
        if self.mode != "debug" {
            return RunResult::err("reload-unsupported");
        }
        let app_id = {
            let s = self.st.lock().unwrap();
            if !s.started || s.ended || s.state != "running" {
                None
            } else {
                s.app_id.clone()
            }
        };
        let Some(app_id) = app_id else {
            return RunResult::Err(EnvIssue::with("reload-failed", "not-running"));
        };
        self.set_state("reloading", patch());
        let machine = self.machine.lock().unwrap().clone().filter(|_| !self.st.lock().unwrap().ended);
        let res = match machine {
            Some(m) => m.request("app.restart", json!({"appId": app_id, "fullRestart": full, "pause": false, "reason": "manual"}), None),
            None => super::super::machine::MachineReply::error("not-running"),
        };
        let back = {
            let s = self.st.lock().unwrap();
            !s.ended && s.state == "reloading"
        };
        if back {
            self.set_state("running", patch());
        }
        let failed = if res.failed() {
            res.error.as_ref().map(error_text)
        } else {
            match res.result.as_ref().and_then(|r| r.get("code")).and_then(Value::as_f64) {
                Some(c) if c != 0.0 => {
                    let msg = res.result.as_ref().and_then(|r| r.get("message")).filter(|m| !m.is_null());
                    Some(match msg {
                        Some(Value::String(s)) => s.clone(),
                        Some(v) => crate::js::stringify(v),
                        None => crate::js::number(c),
                    })
                }
                _ => None,
            }
        };
        match failed {
            None => RunResult::ok(),
            Some(f) => {
                self.ctx.hooks.log("app", &f, Some("E"), None);
                RunResult::Err(EnvIssue::with("reload-failed", f))
            }
        }
    }
}
