use std::cmp::Ordering;
use std::collections::HashSet;

use core_store::model::{ActivityEvent, ActivityKind};
use core_store::num::JsNum;
use core_store::persist::jsstr::slice_units;
use core_store::persist::localtime::Zone;
use core_store::time::utf16_cmp;
use indexmap::IndexMap;
use serde_json::Value;

use super::{Draft, Toaster};
use crate::message::{AgentNote, DiskAlert, PhoneEpisode};
use crate::util::{date_ms, format_bytes, format_gb, js_num, js_value_str, name_of, phone_name, server_label};

pub const LOOP_WINDOW_MS: f64 = 5.0 * 60_000.0;
pub const LOOP_MIN_FAILURES: u64 = 5;
const IDLE_LIST_MAX: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorLoop {
    pub failures: u64,
    pub calls: u64,
    pub tool: Option<String>,
    pub since: String,
}

pub fn error_loops<'a>(items: impl IntoIterator<Item = &'a ActivityEvent>, now: f64, zone: Zone) -> IndexMap<String, ErrorLoop> {
    let mut acc: IndexMap<String, (u64, u64, IndexMap<String, u64>, String)> = IndexMap::new();
    for e in items {
        let Some(id) = e.terminal_id.as_ref().filter(|id| !id.is_empty()) else { continue };
        if e.kind != ActivityKind::ToolStarted && e.kind != ActivityKind::ToolFailed {
            continue;
        }
        let at = date_ms(Some(&e.at), zone);
        if !at.is_finite() || now - at > LOOP_WINDOW_MS {
            continue;
        }
        let a = acc.entry(id.clone()).or_insert_with(|| (0, 0, IndexMap::new(), String::new()));
        if e.kind == ActivityKind::ToolStarted {
            a.1 += 1;
            continue;
        }
        a.0 += 1;
        if a.3.is_empty() || utf16_cmp(&e.at, &a.3) == Ordering::Less {
            a.3 = e.at.clone();
        }
        let tool = e.data.as_ref().and_then(|d| d.tool.clone()).unwrap_or_default();
        *a.2.entry(tool).or_insert(0) += 1;
    }
    let mut out = IndexMap::new();
    for (id, (failures, calls, tools, since)) in acc {
        if failures < LOOP_MIN_FAILURES || failures * 2 < calls {
            continue;
        }
        let mut top: Option<(&String, u64)> = None;
        for (name, n) in &tools {
            if top.is_none_or(|(_, best)| *n > best) {
                top = Some((name, *n));
            }
        }
        let tool = top.map(|(n, _)| n.clone()).filter(|n| !n.is_empty());
        out.insert(id, ErrorLoop { failures, calls: calls.max(failures), tool, since });
    }
    out
}

impl Toaster {
    pub fn check_loops<'a>(&mut self, events: impl IntoIterator<Item = &'a ActivityEvent>) {
        let loops = error_loops(events, self.now(), self.zone);
        let mut current = HashSet::new();
        for (id, l) in loops {
            let episode = format!("{id}|{}", l.since);
            current.insert(episode.clone());
            if !self.loop_sent.insert(episode) {
                continue;
            }
            let Some(t) = self.terms.get(&id).cloned() else { continue };
            if !self.can("loop") {
                continue;
            }
            let text = self.text();
            let d = Draft { title: text.loop_title(&name_of(&t)), body: text.loop_body(l.failures, l.tool.as_deref().unwrap_or("?")), url: self.link_for(Some(&id)), open: text.open(), persistent: false, snooze: false };
            self.show(&format!("loop:{id}"), d);
        }
        self.loop_sent.retain(|e| current.contains(e));
    }

    pub fn observe_sensitive(&mut self, e: &ActivityEvent) {
        if e.kind != ActivityKind::Sensitive {
            return;
        }
        let (Some(id), Some(data)) = (e.terminal_id.as_ref().filter(|i| !i.is_empty()), e.data.as_ref()) else { return };
        let Some(rule) = data.rule.as_ref().filter(|r| !r.is_empty()) else { return };
        let Some(t) = self.terms.get(id).cloned() else { return };
        if !self.can("sensitive") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.sensitive_title(&name_of(&t)), body: text.sensitive_body(rule, data.tool.as_deref().unwrap_or("?"), data.path.as_deref()), url: self.link_for(Some(&t.id)), open: text.open(), persistent: false, snooze: false };
        self.show(&format!("sensitive:{}", t.id), d);
    }

    pub fn observe_disk(&mut self, m: &DiskAlert) {
        let danger = m.level == "danger";
        if danger {
            let p = PhoneEpisode { episode: format!("disk:{}:{}", m.label, js_num(self.now())), reason: "disk", name: m.label.clone(), free_bytes: Some(JsNum(m.free_bytes)), ..Default::default() };
            self.phone(p, Some("disk"));
        }
        if !self.can("disk") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.disk_title(&m.label, &format_gb(m.free_bytes)), body: text.disk_body(danger), url: self.link_for(None), open: text.open_dashboard(), persistent: false, snooze: false };
        self.show(&format!("disk:{}", m.label), d);
    }

    pub fn observe_note(&mut self, m: &AgentNote, snoozed: Option<&dyn Fn(&core_store::TerminalSession) -> bool>) {
        if m.level == "info" {
            return;
        }
        let done = m.level == "done";
        let kind = if done { "done" } else { "waiting" };
        let t = m.terminal_id.as_ref().filter(|i| !i.is_empty()).and_then(|id| self.terms.get(id)).cloned();
        let text = self.text();
        let more = if m.merged != 0.0 && !m.merged.is_nan() { text.note_more(&js_num(m.merged)) } else { String::new() };
        let body = format!("{}{more}", m.text);
        let p = PhoneEpisode { episode: format!("n:{}", m.episode), reason: "note", name: t.as_ref().map_or_else(|| m.name.clone(), phone_name), text: Some(slice_units(&body, 200)), ..Default::default() };
        self.phone(p, Some(kind));
        if !self.can(kind) || t.as_ref().is_some_and(|t| snoozed.is_some_and(|f| f(t))) {
            return;
        }
        let key = format!("note:{}", t.as_ref().map_or(m.name.as_str(), |t| t.id.as_str()));
        let title = text.note_title(&t.as_ref().map_or_else(|| m.name.clone(), name_of), done);
        let open = if t.is_some() { text.open() } else { text.open_dashboard() };
        let d = Draft { title, body, url: self.link_for(t.as_ref().map(|t| t.id.as_str())), open, persistent: false, snooze: false };
        self.show(&key, d);
    }

    pub fn observe_idle(&mut self, report: &Value) {
        let list = |k: &str| report.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
        let forgotten = |v: &Value| v.get("forgotten").is_some_and(|f| !matches!(f, Value::Null | Value::Bool(false)));
        let mut items: Vec<(String, String)> = Vec::new();
        for s in list("sessions").iter().filter(|s| forgotten(s)) {
            let id = js_value_str(s.get("sessionId"), "undefined");
            let label = self.terms.get(&id).map_or_else(|| id.clone(), name_of);
            items.push((format!("s:{id}|{}", js_value_str(s.get("idleSinceMs"), "")), label));
        }
        for s in list("servers").iter().filter(|s| forgotten(s)) {
            let orphan = s.get("orphan").is_some_and(|o| !matches!(o, Value::Null | Value::Bool(false)));
            let idle = if orphan { "orphan".to_string() } else { js_value_str(s.get("idleSinceMs"), "") };
            items.push((format!("p:{}:{}|{idle}", js_value_str(s.get("pid"), "undefined"), js_value_str(s.get("start"), "undefined")), server_label(s)));
        }
        let fresh = items.iter().filter(|(k, _)| !self.idle_sent.contains(k)).count();
        let current: HashSet<String> = items.iter().map(|(k, _)| k.clone()).collect();
        self.idle_sent = current;
        let primed = self.idle_primed;
        self.idle_primed = true;
        if items.is_empty() {
            return self.hide_key("idle:all");
        }
        let idle_minutes = report.get("idleMinutes").and_then(Value::as_f64).unwrap_or(f64::NAN);
        if !primed || fresh == 0 || idle_minutes <= 0.0 || !self.can("idle") {
            return;
        }
        let text = self.text();
        let names: Vec<&str> = items.iter().take(IDLE_LIST_MAX).map(|(_, l)| l.as_str()).collect();
        let more = items.len() - names.len();
        let joined = if more > 0 { format!("{} +{more}", names.join(", ")) } else { names.join(", ") };
        let ram = report.pointer("/totals/forgottenRamBytes").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let d = Draft { title: text.idle_title(items.len(), &format_bytes(ram, self.locale() == core_store::persist::notify_file::ToastLocale::Vi)), body: text.idle_body(&joined), url: self.link_for(None), open: text.open_dashboard(), persistent: false, snooze: false };
        self.show("idle:all", d);
    }
}
