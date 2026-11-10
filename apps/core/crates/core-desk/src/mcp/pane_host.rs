use super::host::{Args, ToolHost};
use super::notes::NoteQueue;
use super::parse_desk::parse_mcp_desk_result;
use super::parse_page::parse_mcp_page_result;
use super::registry::{desk_reply_ms, Registry, PAGE_REPLY_MS};
use crate::util::uuid;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Duration;

pub fn tool_action(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "notify_owner" => "owner.notify",
        "open_in_workspace" => "workspace.show",
        "who_is_editing" => "file.editors",
        "hand_back" => "session.handBack",
        "list_sessions" => "session.list",
        "send_to_session" => "session.sendTo",
        "close_session" => "session.close",
        "run_app" => "run.app",
        "run_status" => "run.status",
        "run_log" => "run.log",
        "device_screenshot" => "device.screenshot",
        "preview_layout" => "layout.preview",
        "list_servers" => "port.list",
        "free_port" => "port.free",
        "telegram_notify" => "telegram.notify",
        "telegram_send" => "telegram.send",
        "telegram_receive" => "telegram.receive",
        _ => return None,
    })
}

pub struct PaneHost {
    pub reg: Arc<Registry>,
    pub id: String,
    pub cwd: String,
    pub preset: String,
}

fn unavailable() -> Value {
    json!({ "ok": false, "error": "unavailable" })
}

fn within(ms: u64, work: impl FnOnce() -> Value + Send + 'static) -> Value {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    rx.recv_timeout(Duration::from_millis(ms)).unwrap_or_else(|_| unavailable())
}

impl Registry {
    pub fn caller(self: &Arc<Self>, token: &str) -> Option<Arc<dyn ToolHost>> {
        let st = self.lock();
        let id = st.tokens.get(token)?;
        let p = st.panes.get(id)?;
        if !p.spawned || p.exited || st.disposed {
            return None;
        }
        Some(Arc::new(PaneHost { reg: self.clone(), id: p.id.clone(), cwd: p.cwd.clone(), preset: p.preset.clone() }))
    }

    pub(super) fn alive(&self, id: &str) -> bool {
        let st = self.lock();
        !st.disposed && st.panes.contains_key(id)
    }

    fn pid_of(&self, id: &str) -> Value {
        let st = self.lock();
        match st.panes.get(id) {
            Some(p) if !p.exited && p.pid != 0 => json!(p.pid),
            _ => Value::Null,
        }
    }

    pub fn ask_page(self: &Arc<Self>, from: &str, cwd: &str, mut call: Value) -> Value {
        let tool = call["tool"].as_str().unwrap_or("").to_string();
        if !self.alive(from) {
            return parse_mcp_page_result(&unavailable(), &tool);
        }
        call["id"] = json!(uuid());
        call["from"] = json!(from);
        call["cwd"] = json!(cwd);
        let raw = match tool_action(&tool) {
            Some("device.screenshot") => {
                let bridge = self.bridge.clone();
                within(PAGE_REPLY_MS, move || bridge.desk("desk:mcp-screenshot", call))
            }
            Some(action) => self.bridge.ask(action, call, from, PAGE_REPLY_MS),
            None => json!({ "ok": false, "error": "failed" }),
        };
        let raw = if self.alive(from) { raw } else { unavailable() };
        parse_mcp_page_result(&raw, &tool)
    }

    fn ask_desk(self: &Arc<Self>, from: &str, cwd: &str, mut call: Value) -> Value {
        let tool = call["tool"].as_str().unwrap_or("").to_string();
        if !self.alive(from) {
            return parse_mcp_desk_result(&unavailable(), &tool);
        }
        call["id"] = json!(uuid());
        call["from"] = json!(from);
        call["cwd"] = json!(cwd);
        call["pid"] = self.pid_of(from);
        let name = Path::new(cwd).file_name().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| cwd.to_string());
        call["name"] = json!(name);
        let bridge = self.bridge.clone();
        let raw = within(desk_reply_ms(&tool), move || bridge.desk("desk:mcp-desk", call));
        let raw = if self.alive(from) { raw } else { unavailable() };
        parse_mcp_desk_result(&raw, &tool)
    }

    fn note_from(self: &Arc<Self>, from: &str, cwd: &str, text: &str, level: &str, merged: u64) -> Value {
        let call = json!({ "tool": "notify_owner", "text": text, "level": level, "merged": merged });
        let mut sent = call.clone();
        sent["id"] = json!(uuid());
        let res = self.ask_page(from, cwd, call);
        if res["ok"] != json!(true) {
            return json!({ "ok": false, "error": if res["error"] == "unavailable" { "unavailable" } else { "failed" } });
        }
        if res["tool"] != "notify_owner" {
            return json!({ "ok": false, "error": "failed" });
        }
        let named = res["name"].as_str().filter(|n| !n.is_empty()).map(str::to_string).unwrap_or_else(|| {
            Path::new(cwd).file_name().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Agent".into())
        });
        let mut note = json!({ "type": "agent.note", "episode": sent["id"], "level": level, "text": text, "name": named });
        if let Some(t) = res["terminalId"].as_str() {
            note["terminalId"] = json!(t);
        }
        note["merged"] = json!(merged);
        self.bridge.note(note);
        json!({ "ok": true, "sent": true, "name": named, "inbox": res["inbox"] == json!(true) })
    }
}

impl ToolHost for PaneHost {
    fn root(&self) -> String {
        self.cwd.clone()
    }

    fn preset(&self) -> Option<String> {
        Some(self.preset.clone())
    }

    fn take(&self, key: &str, max: usize, window_ms: u64) -> bool {
        let now = self.reg.now();
        let mut st = self.reg.lock();
        let Some(p) = st.panes.get_mut(&self.id) else { return false };
        let times = p.rates.entry(key.to_string()).or_default();
        times.retain(|t| now - t < window_ms as f64);
        if times.len() >= max {
            return false;
        }
        times.push(now);
        true
    }

    fn open_session(&self, args: &Args) -> Value {
        self.reg.open_from(&self.id, args)
    }

    fn notify(&self, text: &str, level: &str) -> Value {
        if !self.reg.alive(&self.id) {
            return unavailable();
        }
        let queue = {
            let mut st = self.reg.lock();
            let Some(p) = st.panes.get_mut(&self.id) else { return unavailable() };
            if p.notes.is_none() {
                let (reg, id, cwd) = (self.reg.clone(), self.id.clone(), self.cwd.clone());
                let clock = self.reg.now.clone();
                p.notes = Some(NoteQueue::new(Arc::new(move |t: &str, l: &str, m: u64| reg.note_from(&id, &cwd, t, l, m)), clock));
            }
            p.notes.clone().unwrap()
        };
        queue.submit(text, level)
    }

    fn checkout_roots(&self) -> Vec<String> {
        self.reg.checkout_roots(&self.id)
    }

    fn page(&self, call: Value) -> Value {
        self.reg.ask_page(&self.id, &self.cwd, call)
    }

    fn desk(&self, call: Value) -> Option<Value> {
        Some(self.reg.ask_desk(&self.id, &self.cwd, call))
    }
}
