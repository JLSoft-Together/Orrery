use std::sync::{Arc, Mutex, MutexGuard};

use core_monitor::Monitor;
use core_store::extra::ToastHealth;
use core_store::model::{ActivityEvent, TerminalSession};
use core_store::persist::snooze::SnoozeStore;
use core_store::Store;
use core_toast::{AgentNote, DiskAlert, ToastOptions, ToastSnapshot, Toaster};
use serde_json::Value;

use crate::config::ServeConfig;

pub type Emit = Arc<dyn Fn(&Value) + Send + Sync>;

pub struct Alerts {
    pub toaster: Toaster,
    pub snoozes: SnoozeStore,
    full: bool,
    emit: Emit,
}

pub type SharedAlerts = Arc<Mutex<Alerts>>;

pub fn lock(a: &SharedAlerts) -> MutexGuard<'_, Alerts> {
    a.lock().unwrap_or_else(|e| e.into_inner())
}

impl Alerts {
    pub fn open(cfg: &ServeConfig, port: u16, emit: Emit) -> Alerts {
        let data = &cfg.monitor.data_dir;
        let options = ToastOptions { supported: cfg.toast_supported(), port, file: Some(data.join("notify.json")), link: cfg.toast_link.clone() };
        let mut toaster = Toaster::system(options);
        toaster.load();
        toaster.phone_on = cfg.host_rpc;
        toaster.telegram_on = cfg.host_rpc;
        let mut snoozes = SnoozeStore::new(Some(data.join("snooze.json")));
        snoozes.load();
        Alerts { toaster, snoozes, full: cfg.toast_full(), emit }
    }

    pub fn pump(&mut self) {
        for m in self.toaster.drain() {
            (self.emit)(&m.to_message(self.full));
        }
    }

    pub fn observe(&mut self, s: &mut Store, monitor: &Monitor) {
        if self.snoozes.prune(s.terminal_list()) {
            s.set_snoozes(self.snoozes.all());
        }
        let s: &Store = s;
        let Alerts { toaster, snoozes, .. } = self;
        let terminals: Vec<&TerminalSession> = s.terminal_list().collect();
        let context_pct = |id: &str| s.context_pct(id);
        let tool_of = |id: &str| s.current_tool_of(id).map(str::to_owned);
        let reply_of = |id: &str| monitor.reply_of(id).map(str::to_owned);
        let ask_of = |id: &str| monitor.ask_of(id).cloned();
        let snoozed = |t: &TerminalSession| snoozes.is_snoozed(t, None);
        let snap = ToastSnapshot {
            terminals,
            jobs: Some(s.job_list()),
            limits: Some(s.current_limits()),
            context_pct: Some(&context_pct),
            tool_of: Some(&tool_of),
            reply_of: Some(&reply_of),
            ask_of: Some(&ask_of),
            snoozed: Some(&snoozed),
        };
        toaster.observe(&snap);
        self.pump();
    }

    pub fn check_loops(&mut self, s: &Store) {
        self.toaster.check_loops(s.tool_events().iter());
        self.pump();
    }

    pub fn sensitive(&mut self, events: &[ActivityEvent]) {
        for e in events {
            self.toaster.observe_sensitive(e);
        }
        self.pump();
    }

    pub fn control(&mut self, s: &mut Store, v: &Value) {
        match v.get("type").and_then(Value::as_str) {
            Some("disk.alert") => {
                if let Some(m) = disk_alert(v) {
                    self.toaster.observe_disk(&m);
                }
            }
            Some("agent.note") => {
                if let Ok(m) = serde_json::from_value::<AgentNote>(v.clone()) {
                    let snoozes = &self.snoozes;
                    let snoozed = |t: &TerminalSession| snoozes.is_snoozed(t, None);
                    self.toaster.observe_note(&m, Some(&snoozed));
                }
            }
            Some("toast.health") => {
                if let Some(h) = v.get("health").and_then(|h| serde_json::from_value::<ToastHealth>(h.clone()).ok()) {
                    self.toaster.set_health(h);
                    s.set_notify(self.toaster.settings());
                }
            }
            _ => {}
        }
        self.pump();
    }

    pub fn next_due(&self) -> Option<i64> {
        [self.toaster.next_flush_at(), self.snoozes.next_flush_at()].into_iter().flatten().min()
    }

    pub fn poll(&mut self, now: i64) {
        self.toaster.poll(now);
        self.snoozes.poll(now);
    }

    pub fn close(&mut self) {
        self.toaster.clear();
        self.pump();
        self.toaster.flush();
        self.snoozes.flush();
    }
}

fn disk_alert(v: &Value) -> Option<DiskAlert> {
    let m = serde_json::from_value::<DiskAlert>(v.clone()).ok()?;
    let ok_label = !m.label.is_empty() && m.label.chars().count() <= 200 && !m.label.chars().any(|c| (c as u32) < 0x20);
    let ok_num = |n: f64| n.is_finite() && n >= 0.0;
    (ok_label && (m.level == "warn" || m.level == "danger") && ok_num(m.free_bytes) && ok_num(m.total_bytes)).then_some(m)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn disk_alerts_are_checked_like_parse_disk_alert() {
        let ok = json!({"type": "disk.alert", "label": "C:", "level": "danger", "freeBytes": 1, "totalBytes": 9});
        assert!(disk_alert(&ok).is_some());
        for bad in [json!({"type": "disk.alert", "label": "", "level": "warn", "freeBytes": 1, "totalBytes": 9}), json!({"type": "disk.alert", "label": "C:", "level": "x", "freeBytes": 1, "totalBytes": 9}), json!({"type": "disk.alert", "label": "C:", "level": "warn", "freeBytes": -1, "totalBytes": 9}), json!({"type": "disk.alert", "label": "a\nb", "level": "warn", "freeBytes": 1, "totalBytes": 9})] {
            assert!(disk_alert(&bad).is_none(), "{bad}");
        }
    }
}
