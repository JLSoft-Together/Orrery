use std::path::PathBuf;

use serde_json::Value;

use super::json_file::JsonFile;
use super::jsstr::js_round;
use super::{system_clock, Clock};
use crate::extra::{Dict, SnoozeEntry};
use crate::model::{TerminalSession, TerminalStatus};
use crate::num::JsNum;

pub const SNOOZE_MAX_MIN: f64 = 24.0 * 60.0;

pub fn snoozed_until(entry: Option<&SnoozeEntry>, terminal: Option<&TerminalSession>, now_ms: f64) -> Option<f64> {
    let (entry, terminal) = (entry?, terminal?);
    if terminal.status != TerminalStatus::Waiting {
        return None;
    }
    (entry.until.0 > now_ms && entry.since == terminal.status_since).then_some(entry.until.0)
}

pub struct SnoozeStore {
    entries: Dict<SnoozeEntry>,
    file: Option<JsonFile>,
    clock: Clock,
}

impl SnoozeStore {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self::with_clock(file, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, clock: Clock) -> Self {
        Self { entries: Dict::new(), file: file.map(|f| JsonFile::new(f, 500)), clock }
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    pub fn load(&mut self) {
        let Some(Value::Object(raw)) = self.file.as_ref().and_then(JsonFile::read) else { return };
        let at = self.now() as f64;
        for (id, e) in raw {
            let Some(until) = e.get("until").and_then(Value::as_f64) else { continue };
            if until > at {
                let since = e.get("since").and_then(Value::as_str).map(String::from);
                self.entries.insert(id, SnoozeEntry { until: JsNum(until), since });
            }
        }
    }

    pub fn set(&mut self, terminal: &TerminalSession, minutes: Option<f64>) -> bool {
        let Some(minutes) = minutes else { return self.clear(&terminal.id) };
        if terminal.status != TerminalStatus::Waiting || !minutes.is_finite() || !(1.0..=SNOOZE_MAX_MIN).contains(&minutes) {
            return false;
        }
        let until = self.now() as f64 + js_round(minutes) * 60_000.0;
        self.entries.insert(terminal.id.clone(), SnoozeEntry { until: JsNum(until), since: terminal.status_since.clone() });
        self.save();
        true
    }

    pub fn clear(&mut self, id: &str) -> bool {
        if self.entries.shift_remove(id).is_none() {
            return false;
        }
        self.save();
        true
    }

    pub fn is_snoozed(&self, terminal: &TerminalSession, now_ms: Option<i64>) -> bool {
        let at = now_ms.unwrap_or_else(|| self.now()) as f64;
        snoozed_until(self.entries.get(&terminal.id), Some(terminal), at).is_some()
    }

    pub fn prune<'a>(&mut self, alive: impl IntoIterator<Item = &'a TerminalSession>) -> bool {
        let at = self.now() as f64;
        let by_id: std::collections::HashMap<&str, &TerminalSession> = alive.into_iter().map(|t| (t.id.as_str(), t)).collect();
        let before = self.entries.len();
        self.entries.retain(|id, e| snoozed_until(Some(e), by_id.get(id.as_str()).copied(), at).is_some());
        let changed = self.entries.len() != before;
        if changed {
            self.save();
        }
        changed
    }

    pub fn all(&self) -> Dict<SnoozeEntry> {
        self.entries.clone()
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let entries = &self.entries;
        self.file.as_mut().is_some_and(|f| f.poll(now, || serde_json::to_value(entries).unwrap_or(Value::Null)))
    }

    pub fn flush(&mut self) -> bool {
        let entries = &self.entries;
        self.file.as_mut().is_some_and(|f| f.flush_pending(|| serde_json::to_value(entries).unwrap_or(Value::Null)))
    }

    fn save(&mut self) {
        let now = self.now();
        if let Some(f) = &mut self.file {
            f.schedule(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use serde_json::json;
    use std::fs;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    fn terminal(id: &str, status: &str, since: &str) -> TerminalSession {
        serde_json::from_value(json!({ "id": id, "title": id, "status": status, "statusSince": since })).unwrap()
    }

    fn rig(file: Option<PathBuf>) -> (SnoozeStore, Arc<AtomicI64>) {
        let t = Arc::new(AtomicI64::new(1_800_000_000_000));
        let c = t.clone();
        (SnoozeStore::with_clock(file, Box::new(move || c.load(Ordering::SeqCst))), t)
    }

    #[test]
    fn snoozes_only_waiting_terminals_for_the_same_wait() {
        let (mut s, t) = rig(None);
        let waiting = terminal("a", "waiting", "s1");
        assert!(!s.set(&terminal("a", "working", "s1"), Some(5.0)));
        assert!(!s.set(&waiting, Some(0.5)));
        assert!(!s.set(&waiting, Some(SNOOZE_MAX_MIN + 1.0)));
        assert!(s.set(&waiting, Some(5.4)));
        assert_eq!(s.all()["a"].until.0, 1_800_000_000_000.0 + 5.0 * 60_000.0);
        assert!(s.is_snoozed(&waiting, None));
        assert!(!s.is_snoozed(&terminal("a", "waiting", "s2"), None));
        t.fetch_add(6 * 60_000, Ordering::SeqCst);
        assert!(!s.is_snoozed(&waiting, None));
        assert!(s.prune([&waiting]));
        assert!(s.all().is_empty());
        assert!(!s.set(&waiting, None));
    }

    #[test]
    fn prune_drops_gone_or_changed_terminals_and_persists() {
        let dir = temp_dir("snooze");
        let file = dir.join("snooze.json");
        let (mut s, t) = rig(Some(file.clone()));
        let a = terminal("a", "waiting", "s1");
        let b = terminal("b", "waiting", "s1");
        assert!(s.set(&a, Some(10.0)) && s.set(&b, Some(10.0)));
        assert_eq!(s.next_flush_at(), Some(1_800_000_000_500));
        assert!(!s.prune([&a, &b]));
        assert!(s.prune([&a]));
        assert!(s.poll(1_800_000_000_500));
        let saved: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(saved.as_object().unwrap().keys().collect::<Vec<_>>(), ["a"]);
        let (mut again, _) = rig(Some(file));
        again.load();
        assert!(again.is_snoozed(&a, None));
        t.fetch_add(11 * 60_000, Ordering::SeqCst);
        let (mut later, _) = rig(None);
        later.entries = again.all();
        assert!(!later.is_snoozed(&a, Some(1_800_000_000_000 + 11 * 60_000)));
        let _ = fs::remove_dir_all(&dir);
    }
}
