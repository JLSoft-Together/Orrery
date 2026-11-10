use super::host::{host_source, write_host, ERROR_MARKER, SHOW_EXTENSION};
use super::live::Live;
use super::project::scan_project;
use super::service::Inner;
use super::snap::{FwpError, Res};
use crate::run::flutter::env::flutter_spawn;
use crate::run::flutter::machine::MachineReply;
use crate::run::parse::parse_build_error;
use crate::run::proc::{pump_lines, spawn, SpawnSpec};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const MESSAGE_CAP: usize = 300;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_f64().filter(|f| f.fract() == 0.0).map_or(n.to_string(), |f| (f as i64).to_string()),
        other => other.to_string(),
    }
}

pub fn reply_error(res: &MachineReply) -> Option<String> {
    if let Some(e) = res.error.as_ref().filter(|e| !e.is_null()) {
        return Some(match e {
            Value::Object(o) if o.get("message").is_some_and(Value::is_string) => js_str(&o["message"]),
            other => js_str(other),
        });
    }
    let r = res.result.as_ref()?;
    let code = r.get("code")?.as_f64().filter(|c| *c != 0.0)?;
    Some(match r.get("message").filter(|m| !m.is_null()) {
        Some(m) => js_str(m),
        None => js_str(&json!(code)),
    })
}

fn lost_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)Lost connection to device").unwrap())
}

fn str_of(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from)
}

impl Inner {
    pub fn on_event(self: &Arc<Self>, live: &Arc<Live>, event: &str, params: &Map<String, Value>) {
        if !self.is_current(live) || live.st(|s| s.stop_requested) {
            return;
        }
        match event {
            "app.start" => {
                let id = str_of(params.get("appId")).or_else(|| live.st(|s| s.app_id.clone()));
                live.st(|s| s.app_id = id.clone());
                self.patch(move |s| s.app_id = id);
            }
            "app.progress" => {
                if params.get("finished") == Some(&Value::Bool(true)) {
                    return self.patch(|s| s.progress = None);
                }
                if let Some(m) = str_of(params.get("message")) {
                    self.log(&m);
                    self.patch(move |s| s.progress = Some(m));
                }
            }
            "app.started" => {
                live.st(|s| s.started = true);
                self.patch(|s| {
                    s.state = "running";
                    s.progress = None;
                    s.error = None;
                });
                let (me, lv) = (self.clone(), live.clone());
                let save = (self.deps.watch)(&Path::new(&live.root).join("lib"), Arc::new(move |changed| {
                    let (me, lv) = (me.clone(), lv.clone());
                    std::thread::spawn(move || me.on_save(&lv, changed));
                }));
                let (me, lv) = (self.clone(), live.clone());
                let pubspec = (self.deps.watch_pubspec)(Path::new(&live.root), Arc::new(move || {
                    let (me, lv) = (me.clone(), lv.clone());
                    std::thread::spawn(move || me.on_pubspec(&lv));
                }));
                live.st(|s| s.unwatch.extend([save, pubspec]));
                if let Some(cur) = self.snap.lock().unwrap().current.clone() {
                    let (me, lv) = (self.clone(), live.clone());
                    std::thread::spawn(move || {
                        let _ = me.send_show(&lv, &cur);
                    });
                }
            }
            "app.log" => {
                for l in params.get("log").and_then(Value::as_str).unwrap_or("").split('\n') {
                    self.text(live, l.strip_suffix('\r').unwrap_or(l));
                }
            }
            "daemon.logMessage" => {
                if let Some(m) = str_of(params.get("message")) {
                    for l in m.split('\n') {
                        self.text(live, l.strip_suffix('\r').unwrap_or(l));
                    }
                }
            }
            "app.stop" => {
                if let Some(e) = str_of(params.get("error")) {
                    self.text(live, &e);
                }
            }
            _ => {}
        }
    }

    pub fn text(self: &Arc<Self>, live: &Arc<Live>, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        if let Some(we) = self.widget_error(line) {
            self.log(&format!("{ERROR_MARKER}{}: {}", we.entry.as_deref().unwrap_or(""), we.detail.as_deref().unwrap_or("")));
            let ready = live.st(|s| s.started && !s.stop_requested);
            if ready && self.is_current(live) {
                self.patch(move |s| s.error = Some(we));
            }
            return;
        }
        self.log(line);
        if lost_re().is_match(line) {
            live.st(|s| s.device_lost = true);
        }
        let Some(err) = parse_build_error(line, &live.root).filter(|e| e.severity == "error") else { return };
        live.st(|s| {
            if !s.started && s.build_error.is_none() {
                s.build_error = Some(err);
            } else if s.reloading && s.reload_error.is_none() {
                s.reload_error = Some(err);
            }
        });
    }

    fn widget_error(&self, line: &str) -> Option<FwpError> {
        let at = line.find(ERROR_MARKER)?;
        let parsed: Value = serde_json::from_str(&line[at + ERROR_MARKER.len()..]).ok()?;
        let id = parsed.get("id")?.as_str()?;
        let message = parsed.get("message")?.as_str()?;
        let masked = core_sources::mask::mask_secrets(message, &self.deps.home, self.deps.platform);
        let flat: String = {
            let mut out = String::new();
            let mut gap = false;
            for c in masked.chars() {
                if (c as u32) < 0x20 || c == '\u{7f}' {
                    if !gap {
                        out.push(' ');
                    }
                    gap = true;
                } else {
                    out.push(c);
                    gap = false;
                }
            }
            out
        };
        Some(FwpError { code: "widget".into(), detail: Some(crate::js::slice(&flat, MESSAGE_CAP)), build: None, entry: Some(crate::js::slice(id, 512)) })
    }

    pub fn send_show(self: &Arc<Self>, live: &Arc<Live>, id: &str) -> Res {
        let app = live.st(|s| s.app_id.clone());
        let res = live.machine.request("app.callServiceExtension", json!({"appId": app, "methodName": SHOW_EXTENSION, "params": {"id": id}}), Some(REQUEST_TIMEOUT));
        if let Some(failed) = reply_error(&res) {
            self.log(&failed);
            return Err(FwpError::with("show-failed", failed));
        }
        if res.result.as_ref().and_then(|r| r.get("ok")) == Some(&Value::Bool(false)) {
            return Err(FwpError::new("unknown-id"));
        }
        Ok(())
    }

    pub fn on_save(self: &Arc<Self>, live: &Arc<Live>, mut changed: Vec<String>) {
        loop {
            let ready = live.st(|s| s.started && !s.stop_requested && !s.syncing);
            if !changed.iter().any(|f| f.ends_with(".dart")) || !self.is_current(live) || !ready {
                return;
            }
            if live.st(|s| std::mem::replace(&mut s.reloading, true)) {
                live.st(|s| s.reload_queued = true);
                return;
            }
            live.st(|s| s.reload_error = None);
            self.patch(|s| s.state = "reloading");
            let error = self.reload(live);
            live.st(|s| s.reloading = false);
            if !self.is_current(live) || live.st(|s| s.stop_requested || s.syncing) {
                return;
            }
            self.patch(move |s| {
                s.state = "running";
                s.error = error;
            });
            if !live.st(|s| std::mem::replace(&mut s.reload_queued, false)) {
                return;
            }
            changed = vec![".dart".to_string()];
        }
    }

    fn reload(self: &Arc<Self>, live: &Arc<Live>) -> Option<FwpError> {
        let entries = scan_project(Path::new(&live.root));
        if let Err(e) = write_host(Path::new(&live.root), &host_source(&live.pkg, &entries, &live.host)) {
            let detail = e.to_string();
            self.log(&detail);
            return Some(FwpError::with("reload-failed", detail));
        }
        if self.is_current(live) {
            self.patch(move |s| s.entries = entries);
        }
        let app = live.st(|s| s.app_id.clone());
        let res = live.machine.request("app.restart", json!({"appId": app, "fullRestart": false, "pause": false, "reason": "save"}), Some(REQUEST_TIMEOUT));
        let failed = reply_error(&res);
        if let Some(f) = &failed {
            self.log(f);
        }
        if let Some(b) = live.st(|s| s.reload_error.clone()) {
            return Some(FwpError { build: Some(Box::new(b)), ..FwpError::new("build") });
        }
        failed.map(|f| FwpError::with("reload-failed", f))
    }

    pub fn on_pubspec(self: &Arc<Self>, live: &Arc<Live>) {
        if !self.is_current(live) || live.st(|s| !s.started || s.stop_requested) {
            return;
        }
        if live.st(|s| std::mem::replace(&mut s.syncing, true)) {
            live.st(|s| s.sync_queued = true);
            return;
        }
        self.patch(|s| {
            s.state = "syncing";
            s.progress = None;
            s.error = None;
        });
        let mut error;
        loop {
            live.st(|s| s.sync_queued = false);
            error = self.pub_get(live);
            if error.is_some() || !live.st(|s| s.sync_queued) || !self.is_current(live) || live.st(|s| s.stop_requested) {
                break;
            }
        }
        live.st(|s| s.syncing = false);
        if !self.is_current(live) || live.st(|s| s.stop_requested) {
            return;
        }
        if let Some(e) = error {
            return self.patch(move |s| {
                s.state = "running";
                s.error = Some(e);
            });
        }
        let _ = self.start(&live.root, &live.device_id, live.flavor.as_deref());
    }

    fn pub_get(self: &Arc<Self>, live: &Arc<Live>) -> Option<FwpError> {
        let d = &self.deps;
        let args = vec!["pub".to_string(), "get".to_string()];
        self.log("> flutter pub get");
        let spec = match flutter_spawn(&live.sdk, &args, d.win, &d.env) {
            Some(r) => SpawnSpec { file: r.file, args: r.args, cwd: live.root.clone(), verbatim: r.verbatim, ..Default::default() },
            None => return Some(FwpError::with("pub-get", "bad-arg")),
        };
        let mut child = match spawn(&spec, &d.env) {
            Ok(c) => c,
            Err(e) => return Some(FwpError::with("pub-get", e.to_string())),
        };
        let pid = child.id();
        live.st(|s| s.pub_pid = Some(pid));
        let first: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let make = |me: Arc<Self>, first: Arc<Mutex<Option<String>>>| {
            let (home, platform) = (me.deps.home.clone(), me.deps.platform);
            move |line: &str| {
                if line.trim().is_empty() {
                    return;
                }
                me.log(line);
                let mut f = first.lock().unwrap();
                if f.is_none() && line.to_lowercase().contains("error") {
                    *f = Some(crate::js::slice(&core_sources::mask::mask_secrets(line, &home, platform), MESSAGE_CAP));
                }
            }
        };
        let pumps: Vec<_> = [child.stdout.take().map(|o| pump_lines(o, make(self.clone(), first.clone()))), child.stderr.take().map(|e| pump_lines(e, make(self.clone(), first.clone())))].into_iter().flatten().collect();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            let _ = tx.send(code);
        });
        let Ok(code) = rx.recv_timeout(d.pub_get_timeout) else {
            self.kill_pub_get(live, false);
            return Some(FwpError::with("pub-get", "timeout"));
        };
        for p in pumps {
            let _ = p.join();
        }
        live.st(|s| {
            if s.pub_pid == Some(pid) {
                s.pub_pid = None;
            }
        });
        if code == Some(0) {
            return None;
        }
        let detail = first.lock().unwrap().clone().unwrap_or_else(|| format!("exit {}", code.map_or("signal".to_string(), |c| c.to_string())));
        Some(FwpError::with("pub-get", detail))
    }
}
