use super::config_store::config_hash;
use super::configs::{drop_blocked_env, needs_trust};
use super::manager::{Inner, Live};
use super::types::{BuildError, EnvIssue, Hooks, RunContext, RunDevice, RunResult, RunnerFactory, SessionPatch};
use super::{android, custom, flutter};
use crate::preview::paths::{is_within, real_or_null};
use serde_json::{json, Map, Value};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

const LOG_FLUSH_MS: u64 = 100;
const MAX_ENDED: usize = 20;

pub fn is_live(state: &str) -> bool {
    matches!(state, "before-launch" | "building" | "installing" | "launching" | "starting" | "running" | "reloading" | "stopping")
}

fn pair_key(config_id: &str, device_id: Option<&str>) -> String {
    format!("{config_id}\u{0}{}", device_id.unwrap_or(""))
}

fn now() -> Value {
    json!(crate::util::now_ms() as i64)
}

fn factory(kind: &str) -> RunnerFactory {
    match kind {
        "flutter" => flutter::runner::create,
        "android" => android::runner::create,
        _ => custom::create,
    }
}

struct SessionHooks {
    m: Arc<Inner>,
    id: String,
}

impl Hooks for SessionHooks {
    fn state(&self, state: &str, patch: SessionPatch) {
        let mut p = Map::new();
        p.insert("state".into(), json!(state));
        for (k, v) in [("appId", patch.app_id), ("vmServiceUri", patch.vm_service_uri), ("devtoolsUri", patch.devtools_uri), ("progress", patch.progress)] {
            if let Some(v) = v {
                p.insert(k.into(), json!(v));
            }
        }
        if let Some(f) = patch.failure {
            p.insert("failure".into(), f.map(|e| e.to_value()).unwrap_or(Value::Null));
        }
        if !is_live(state) {
            p.insert("endedAt".into(), now());
            p.insert("progress".into(), Value::Null);
        }
        self.m.update(&self.id, p);
    }

    fn fail(&self, failure: EnvIssue) {
        let mut p = Map::new();
        p.insert("state".into(), json!("failed"));
        p.insert("failure".into(), failure.to_value());
        p.insert("endedAt".into(), now());
        p.insert("progress".into(), Value::Null);
        self.m.update(&self.id, p);
    }

    fn log(&self, stream: &str, text: &str, level: Option<&str>, tag: Option<&str>) {
        let mut line = Map::new();
        line.insert("stream".into(), json!(stream));
        line.insert("text".into(), json!((self.m.mask)(text)));
        if let Some(l) = level {
            line.insert("level".into(), json!(l));
        }
        if let Some(t) = tag {
            line.insert("tag".into(), json!(t));
        }
        line.insert("at".into(), now());
        self.m.queue_log(&self.id, Value::Object(line));
    }

    fn error(&self, error: BuildError) {
        let mut e = error;
        e.message = (self.m.mask)(&e.message);
        self.m.broadcast(json!({"type": "error", "sessionId": self.id, "error": e.to_value()}));
    }

    fn confirm(&self, code: &str, detail: Option<&str>) -> bool {
        self.m.confirm(code, detail)
    }
}

impl Inner {
    pub fn confirm(&self, code: &str, detail: Option<&str>) -> bool {
        matches!(self.host.call("run.confirm", json!({"code": code, "detail": detail})), Ok(Value::Bool(true)))
    }

    pub fn list(&self) -> Vec<Value> {
        self.state.lock().unwrap().sessions.iter().map(|(_, l)| l.session.clone()).collect()
    }

    pub fn live(&self) -> Vec<Value> {
        self.list().into_iter().filter(|s| is_live(s["state"].as_str().unwrap_or(""))).collect()
    }

    pub fn start(self: &Arc<Self>, root: &str, config_id: &str, device_id: Option<&str>) -> RunResult {
        let reserved = pair_key(config_id, device_id);
        {
            let mut g = self.state.lock().unwrap();
            if !g.starting.insert(reserved.clone()) {
                return RunResult::err("run-pending");
            }
        }
        let r = self.start_reserved(root, config_id, device_id, &reserved);
        self.state.lock().unwrap().starting.remove(&reserved);
        r
    }

    fn start_reserved(self: &Arc<Self>, root: &str, config_id: &str, device_pick: Option<&str>, reserved: &str) -> RunResult {
        let Some(found) = self.configs(root).into_iter().find(|c| c["id"] == config_id) else { return RunResult::err("config-missing") };
        let mut cfg = found.clone();
        if needs_trust(&found) {
            cfg["env"] = drop_blocked_env(&found["env"]);
        }
        let is_custom = cfg["type"] == "custom";
        let device_id: Option<String> = if is_custom {
            None
        } else if cfg["device"] != "selected" {
            cfg["device"].as_str().map(String::from)
        } else {
            device_pick.filter(|d| !d.is_empty()).map(String::from)
        };
        if !is_custom && device_id.is_none() {
            return RunResult::err("device-missing");
        }
        let device: Option<RunDevice> = match &device_id {
            Some(id) => {
                let hit = self.merged_devices().into_iter().find(|d| &d.id == id).or_else(|| self.devices().into_iter().find(|d| &d.id == id));
                match hit {
                    Some(d) => Some(d),
                    None => return RunResult::Err(EnvIssue::with("device-missing", id.clone())),
                }
            }
            None => None,
        };
        let key = pair_key(cfg["id"].as_str().unwrap_or(""), device_id.as_deref());
        if key != reserved && !self.state.lock().unwrap().starting.insert(key.clone()) {
            return RunResult::err("run-pending");
        }
        let r = self.start_checked(root, &cfg, device, &key, device_id.as_deref());
        if key != reserved {
            self.state.lock().unwrap().starting.remove(&key);
        }
        r
    }

    fn start_checked(self: &Arc<Self>, root: &str, cfg: &Value, device: Option<RunDevice>, key: &str, device_id: Option<&str>) -> RunResult {
        let cwd = cfg["cwd"].as_str().unwrap_or("");
        let Some(real_cwd) = real_or_null(cwd) else { return RunResult::Err(EnvIssue::with("cwd-missing", cwd)) };
        let real_root = real_or_null(root);
        if !real_root.as_deref().is_some_and(|r| is_within(r, &real_cwd)) {
            return RunResult::Err(EnvIssue::with("cwd-outside", cwd));
        }
        if needs_trust(cfg) && !self.approve(root, cfg, device_id) {
            return RunResult::err("untrusted");
        }
        let previous = {
            let g = self.state.lock().unwrap();
            g.sessions.iter().find(|(_, l)| l.key == key && is_live(l.session["state"].as_str().unwrap_or(""))).map(|(id, l)| (id.clone(), l.runner.clone()))
        };
        if let Some((id, runner)) = previous {
            self.stop_bounded(&id, runner);
        }
        self.launch(root, cfg, device, key)
    }

    fn approve(self: &Arc<Self>, root: &str, cfg: &Value, device_id: Option<&str>) -> bool {
        let hash = config_hash(cfg);
        if self.trust.has(&hash) {
            return true;
        }
        let preview = self.preview(root, cfg, device_id);
        let env_keys: Vec<&String> = cfg["env"].as_object().map(|o| o.keys().collect()).unwrap_or_default();
        let detail = crate::js::stringify(&json!({
            "name": cfg["name"], "cwd": cfg["cwd"], "command": preview.as_ref().and_then(|p| p["command"].as_str()).unwrap_or(""),
            "beforeLaunch": cfg["beforeLaunch"], "envKeys": env_keys,
        }));
        if !self.confirm("trust-config", Some(&detail)) {
            return false;
        }
        self.trust.add(&hash);
        true
    }

    fn stop_bounded(self: &Arc<Self>, id: &str, runner: Arc<dyn super::types::Runner>) {
        let (tx, rx) = mpsc::channel::<()>();
        let r = runner.clone();
        std::thread::spawn(move || {
            r.stop();
            let _ = tx.send(());
        });
        if rx.recv_timeout(Duration::from_millis(self.restart_stop_ms)).is_ok() {
            return;
        }
        runner.kill();
        let mut p = Map::new();
        p.insert("state".into(), json!("stopped"));
        p.insert("endedAt".into(), now());
        p.insert("progress".into(), Value::Null);
        self.update(id, p);
    }

    fn launch(self: &Arc<Self>, root: &str, cfg: &Value, device: Option<RunDevice>, key: &str) -> RunResult {
        let id = crate::util::uuid();
        let kind = cfg["type"].as_str().unwrap_or("custom").to_string();
        let session = json!({
            "id": id, "configId": cfg["id"], "configName": cfg["name"], "type": kind, "cwd": cfg["cwd"],
            "deviceId": device.as_ref().map(|d| d.id.clone()), "deviceName": device.as_ref().map(|d| d.name.clone()),
            "mode": if kind == "flutter" { cfg["mode"].clone() } else { Value::Null },
            "state": "starting", "startedAt": now(), "endedAt": null, "appId": null, "vmServiceUri": null, "devtoolsUri": null, "failure": null, "progress": null,
        });
        let device_value = device.as_ref().map(RunDevice::to_value).unwrap_or(Value::Null);
        let hooks = Arc::new(SessionHooks { m: self.clone(), id: id.clone() });
        let host = self.host.clone();
        let ctx = RunContext {
            root: root.to_string(),
            config: cfg.clone(),
            device,
            settings: self.get_settings(),
            env: self.env.clone(),
            win: self.win,
            home: self.home.clone(),
            hooks: hooks.clone(),
            on_adb_use: Some(Arc::new(move |serial| {
                let _ = host.call("run.adbUse", json!({"serial": serial}));
            })),
        };
        let runner = factory(&kind)(ctx);
        {
            let mut g = self.state.lock().unwrap();
            g.sessions.push((id.clone(), Live { config: cfg.clone(), device: device_value, session: session.clone(), runner: runner.clone(), key: key.to_string() }));
            Self::prune(&mut g.sessions);
            self.broadcast(json!({"type": "session", "session": session}));
        }
        std::thread::spawn(move || {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.start())).is_err() {
                hooks.fail(EnvIssue::new("spawn"));
            }
        });
        RunResult::Ok(Some(id))
    }

    fn prune(sessions: &mut Vec<(String, Live)>) {
        let mut ended: Vec<(String, i64)> = sessions.iter().filter(|(_, l)| !is_live(l.session["state"].as_str().unwrap_or(""))).map(|(id, l)| (id.clone(), l.session["endedAt"].as_i64().unwrap_or(0))).collect();
        ended.sort_by_key(|e| std::cmp::Reverse(e.1));
        for (id, _) in ended.into_iter().skip(MAX_ENDED) {
            sessions.retain(|(i, _)| *i != id);
        }
    }

    pub fn update(self: &Arc<Self>, id: &str, mut patch: Map<String, Value>) {
        let mut g = self.state.lock().unwrap();
        let Some(idx) = g.sessions.iter().position(|(i, _)| i == id) else { return };
        let cur_state = g.sessions[idx].1.session["state"].as_str().unwrap_or("").to_string();
        if !is_live(&cur_state) {
            return;
        }
        if let Some(Value::String(p)) = patch.get("progress") {
            let m = (self.mask)(p);
            patch.insert("progress".into(), json!(m));
        }
        if let Some(Value::String(d)) = patch.get("failure").and_then(|f| f.get("detail")).cloned() {
            patch.get_mut("failure").unwrap()["detail"] = json!((self.mask)(&d));
        }
        let progress_only = patch.keys().all(|k| k == "progress" || k == "state") && patch.get("state").is_none_or(|s| s.as_str() == Some(&cur_state));
        for (k, v) in patch {
            g.sessions[idx].1.session[&k] = v;
        }
        if progress_only {
            g.dirty.insert(id.to_string());
            self.schedule_flush(&mut g);
            return;
        }
        g.dirty.remove(id);
        self.flush_locked(&mut g);
        let session = g.sessions[idx].1.session.clone();
        self.broadcast(json!({"type": "session", "session": session}));
        let changed = session["state"].as_str().is_some_and(|s| s != cur_state);
        let ended = changed && !is_live(session["state"].as_str().unwrap_or(""));
        if changed {
            let live = &g.sessions[idx].1;
            self.emitter.emit(super::manager::STATE_EVENT, json!({"session": session, "config": live.config, "device": live.device}));
        }
        drop(g);
        if ended {
            self.release_devices_if_idle();
        }
    }

    fn schedule_flush(self: &Arc<Self>, g: &mut super::manager::State) {
        if g.flush_scheduled {
            return;
        }
        g.flush_scheduled = true;
        let m = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(LOG_FLUSH_MS));
            let mut g = m.state.lock().unwrap();
            m.flush_locked(&mut g);
        });
    }

    fn flush_locked(&self, g: &mut super::manager::State) {
        g.flush_scheduled = false;
        for (session_id, lines) in std::mem::take(&mut g.pending) {
            self.broadcast(json!({"type": "log", "sessionId": session_id, "lines": lines}));
        }
        for id in std::mem::take(&mut g.dirty) {
            if let Some((_, l)) = g.sessions.iter().find(|(i, _)| *i == id) {
                self.broadcast(json!({"type": "session", "session": l.session}));
            }
        }
    }

    fn queue_log(self: &Arc<Self>, id: &str, line: Value) {
        let mut g = self.state.lock().unwrap();
        match g.pending.iter_mut().find(|(i, _)| i == id) {
            Some((_, list)) => list.push(line),
            None => g.pending.push((id.to_string(), vec![line])),
        }
        self.schedule_flush(&mut g);
    }

    pub fn stop(self: &Arc<Self>, id: &str) -> RunResult {
        let runner = self.state.lock().unwrap().sessions.iter().find(|(i, _)| i == id).map(|(_, l)| l.runner.clone());
        match runner {
            Some(r) => {
                r.stop();
                RunResult::ok()
            }
            None => RunResult::err("session-missing"),
        }
    }

    pub fn reload(self: &Arc<Self>, id: &str, full: bool) -> RunResult {
        let runner = self.state.lock().unwrap().sessions.iter().find(|(i, _)| i == id).map(|(_, l)| l.runner.clone());
        match runner {
            Some(r) => r.reload(full),
            None => RunResult::err("reload-unsupported"),
        }
    }

    pub fn stop_all(self: &Arc<Self>) {
        let runners: Vec<Arc<dyn super::types::Runner>> = {
            let g = self.state.lock().unwrap();
            g.sessions.iter().filter(|(_, l)| is_live(l.session["state"].as_str().unwrap_or(""))).map(|(_, l)| l.runner.clone()).collect()
        };
        let handles: Vec<_> = runners.into_iter().map(|r| std::thread::spawn(move || r.stop())).collect();
        for h in handles {
            let _ = h.join();
        }
        self.restart_devices(false);
        self.watchers.close_all();
    }

    pub fn kill_all(self: &Arc<Self>) {
        let (runners, pids) = {
            let mut g = self.state.lock().unwrap();
            let r: Vec<_> = g.sessions.iter().filter(|(_, l)| is_live(l.session["state"].as_str().unwrap_or(""))).map(|(_, l)| l.runner.clone()).collect();
            (r, std::mem::take(&mut g.pub_pids))
        };
        for r in runners {
            r.kill();
        }
        for p in pids {
            super::proc::kill_pid(p, true);
        }
        self.restart_devices(true);
        self.watchers.close_all();
    }
}
