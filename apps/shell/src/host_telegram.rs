use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use base64::Engine;
use serde_json::{json, Value};
use tauri::{AppHandle, Listener, Manager};

use super::text::extra_text;
use crate::state::{lock, Shell};

const ASK_TIMEOUT: Duration = Duration::from_secs(5);
const LOG_LINES: usize = 200;
const LOG_SESSIONS: usize = 20;
const SHOT_KEEP: Duration = Duration::from_secs(600);
pub const RUN_EVENT: &str = "ccm:run-event";

pub fn orrery_action(inbound: &str) -> Option<&'static str> {
    Some(match inbound {
        "status" => "session.status",
        "send" => "session.send",
        "all" => "session.sendAll",
        "keys" => "session.keys",
        "tail" => "session.tail",
        "compact" => "session.compact",
        "close" => "session.closeOwner",
        "focus" => "session.focus",
        "new" => "session.create",
        "resumeList" => "session.resumeList",
        "resume" => "session.resume",
        "answer" => "session.answer",
        _ => return None,
    })
}

fn fail(error: &str) -> Value {
    json!({ "ok": false, "error": error })
}

fn text<'a>(p: &'a Value, key: &str) -> &'a str {
    p.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn ask_request(id: &str, req: &Value) -> Option<Value> {
    let action = orrery_action(req.get("action")?.as_str()?)?;
    Some(json!({ "id": id, "action": action, "args": req, "caller": { "source": "telegram" } }))
}

fn ask(app: &AppHandle, p: &Value) -> Value {
    let id = crate::config::uuid();
    let Some(req) = p.get("req").filter(|r| r.is_object()).and_then(|r| ask_request(&id, r)) else { return fail("unavailable") };
    app.state::<Shell>().ask(app, &id, "ccm:orrery", req, ASK_TIMEOUT).unwrap_or_else(|| fail("unavailable"))
}

pub fn pick_device(snapshot: &Value, serial: Option<&str>) -> Option<(String, String)> {
    snapshot["devices"].as_array()?.iter().find_map(|d| {
        let id = d["id"].as_str()?;
        let ready = d["online"] == Value::Bool(true) && d["state"] == "ready";
        (ready && serial.is_none_or(|s| s == id)).then(|| (id.to_owned(), d["name"].as_str().unwrap_or(id).to_owned()))
    })
}

pub fn png_bytes(raw: &str) -> Option<Vec<u8>> {
    let b64 = raw.strip_prefix("data:image/png;base64,").unwrap_or(raw);
    base64::engine::general_purpose::STANDARD.decode(b64).ok().filter(|b| b.starts_with(&[0x89, 0x50, 0x4e, 0x47]))
}

fn shot_dir() -> PathBuf {
    std::env::temp_dir().join("orrery-telegram")
}

fn sweep(dir: &Path, keep: Duration) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for e in entries.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| now.duration_since(t).ok()).is_some_and(|age| age > keep);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

pub fn save_shot(dir: &Path, png: &[u8]) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    sweep(dir, SHOT_KEEP);
    let file = dir.join(format!("{}.png", crate::config::uuid()));
    std::fs::write(&file, png).ok()?;
    Some(file)
}

fn screenshot(app: &AppHandle, p: &Value) -> Value {
    let serial = p.get("serial").and_then(Value::as_str).filter(|s| !s.is_empty());
    let Ok(snapshot) = crate::desk::call_inside(app, "ccm:dev-snapshot", json!([])) else { return fail("none") };
    let Some((id, name)) = pick_device(&snapshot, serial) else { return fail("none") };
    let cap = crate::desk::call_inside(app, "ccm:dev-screencap", json!([id])).unwrap_or_else(|_| fail("failed"));
    let png = (cap["ok"] == Value::Bool(true)).then(|| cap["png"].as_str().and_then(png_bytes)).flatten();
    match png.and_then(|b| save_shot(&shot_dir(), &b)) {
        Some(file) => json!({ "ok": true, "png": file.to_string_lossy(), "device": name }),
        None => fail("failed"),
    }
}

pub fn trusted_configs(list: &Value) -> Vec<Value> {
    list.as_array()
        .map(|a| {
            a.iter()
                .filter(|c| !core_desk::run::configs::needs_trust(c))
                .filter_map(|c| Some(json!({ "id": c["id"].as_str()?, "name": c["name"].as_str()? })))
                .collect()
        })
        .unwrap_or_default()
}

fn configs(app: &AppHandle, project: &str) -> Value {
    crate::desk::call_inside_allowing(app, "ccm:run-configs", json!([project]), project).unwrap_or_else(|_| json!([]))
}

pub fn wants_device(cfg: &Value) -> bool {
    cfg["type"] != "custom" && cfg["device"] == "selected"
}

pub fn first_ready(devices: &Value) -> Option<String> {
    devices.as_array()?.iter().find(|d| d["state"] == "ready").and_then(|d| d["id"].as_str()).map(str::to_owned)
}

pub fn start_result(res: &Value) -> Value {
    match (res["ok"].as_bool(), res["sessionId"].as_str()) {
        (Some(true), Some(id)) => json!({ "ok": true, "sessionId": id }),
        (Some(true), None) => fail("failed"),
        _ => fail(res["error"]["code"].as_str().or_else(|| res["error"].as_str()).unwrap_or("failed")),
    }
}

fn run_start(app: &AppHandle, p: &Value) -> Value {
    let (project, config_id) = (text(p, "project"), text(p, "configId"));
    let list = configs(app, project);
    let Some(cfg) = list.as_array().and_then(|a| a.iter().find(|c| c["id"] == config_id)).cloned() else { return fail("config-missing") };
    if core_desk::run::configs::needs_trust(&cfg) {
        return fail("config-missing");
    }
    let device = if wants_device(&cfg) { crate::desk::call_inside(app, "ccm:run-devices", json!([])).ok().as_ref().and_then(first_ready) } else { None };
    match crate::desk::call_inside_allowing(app, "ccm:run-start", json!([{ "root": project, "configId": config_id, "deviceId": device }]), project) {
        Ok(res) => start_result(&res),
        Err(_) => fail("failed"),
    }
}

#[derive(Default)]
pub struct RunLogs {
    order: VecDeque<String>,
    lines: HashMap<String, Vec<String>>,
}

impl RunLogs {
    pub fn push(&mut self, id: &str, add: &[Value]) {
        if !self.lines.contains_key(id) {
            self.order.push_back(id.to_owned());
        }
        let list = self.lines.entry(id.to_owned()).or_default();
        list.extend(add.iter().filter_map(|l| l.as_str().or_else(|| l["text"].as_str())).map(str::to_owned));
        if list.len() > LOG_LINES {
            list.drain(..list.len() - LOG_LINES);
        }
        while self.order.len() > LOG_SESSIONS {
            if let Some(old) = self.order.pop_front() {
                self.lines.remove(&old);
            }
        }
    }

    pub fn get(&self, id: &str) -> Vec<String> {
        self.lines.get(id).cloned().unwrap_or_default()
    }
}

struct Feed {
    logs: Mutex<RunLogs>,
    last: Mutex<String>,
}

fn feed() -> &'static Feed {
    static FEED: OnceLock<Feed> = OnceLock::new();
    FEED.get_or_init(|| Feed { logs: Mutex::new(RunLogs::default()), last: Mutex::new(String::new()) })
}

pub fn fresh(last: &mut String, payload: &str) -> bool {
    if last == payload {
        return false;
    }
    payload.clone_into(last);
    true
}

fn on_run_event(app: &AppHandle, payload: &str) {
    if !fresh(&mut lock(&feed().last), payload) {
        return;
    }
    let Ok(event) = serde_json::from_str::<Value>(payload) else { return };
    match event["type"].as_str() {
        Some("log") => {
            if let (Some(id), Some(lines)) = (event["sessionId"].as_str(), event["lines"].as_array()) {
                lock(&feed().logs).push(id, lines);
            }
        }
        Some("session") if event["session"].is_object() => {
            let app = app.clone();
            let session = event["session"].clone();
            std::thread::spawn(move || {
                let _ = crate::desk::call_inside(&app, "desk:tg-run-state", json!([{ "session": session }]));
                if session["state"] == "failed" {
                    let _ = crate::desk::call_inside(&app, "desk:tg-run-failed", json!([{ "session": session }]));
                }
            });
        }
        _ => {}
    }
}

pub fn listen(app: &AppHandle) {
    static ON: OnceLock<()> = OnceLock::new();
    ON.get_or_init(|| {
        let handle = app.clone();
        app.listen_any(RUN_EVENT, move |e| on_run_event(&handle, e.payload()));
    });
}

fn confirm_command(app: &AppHandle, p: &Value) -> bool {
    let t = extra_text(&crate::hosts::locale(app));
    let (title, detail, ok) = t.tg_command(text(p, "kind") == "receive", text(p, "file"));
    crate::hosts::confirm(app, &title, &detail, &ok, &t.cancel())
}

pub fn call(app: &AppHandle, method: &str, p: &Value) -> Option<Result<Value, String>> {
    let v = match method {
        "telegram.locale" => json!(if crate::hosts::locale(app) == "en" { "en" } else { "vi" }),
        "telegram.ask" => ask(app, p),
        "telegram.focus" => {
            crate::window::show_main(app);
            json!(true)
        }
        "telegram.screenshot" => screenshot(app, p),
        "telegram.runConfigs" => json!(trusted_configs(&configs(app, text(p, "project")))),
        "telegram.runStart" => run_start(app, p),
        "telegram.runLog" => json!(lock(&feed().logs).get(text(p, "sessionId"))),
        "telegram.confirmCommand" => json!(confirm_command(app, p)),
        "telegram.bigUpload" => json!({ "result": fail("failed") }),
        "telegram.user" => fail("failed"),
        _ => return None,
    };
    Some(Ok(v))
}
