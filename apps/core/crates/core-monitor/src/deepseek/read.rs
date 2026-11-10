use core_store::store::NewActivity;
use core_store::time::now_ms;
use core_store::Store;
use indexmap::IndexMap;

use super::monitor::{DeepseekMonitor, Thread, MAX_LINE, SCAN_DEBOUNCE_MS, SCAN_WAIT_MS, TAIL_BYTES};
use super::state::{DeepseekThreadState, OutboxEvent};
use super::tail::OutboxTail;

fn file_key(file: &std::path::Path) -> Option<String> {
    let m = std::fs::metadata(file).ok()?;
    let birth = m.created().ok().and_then(|c| c.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis());
    #[cfg(unix)]
    let ino = std::os::unix::fs::MetadataExt::ino(&m);
    #[cfg(not(unix))]
    let ino = 0u64;
    Some(format!("{ino}:{birth}"))
}

impl DeepseekMonitor {
    pub fn read_outbox(&mut self, store: &mut Store, catch_up: bool) {
        let Some(file) = self.outbox.clone() else { return };
        let Some(key) = file_key(&file) else { return };
        if self.file_key.as_deref() != Some(key.as_str()) || self.tailer.is_none() {
            let from_end = if self.ever_opened { None } else { Some(TAIL_BYTES) };
            self.tailer = Some(OutboxTail::new(file, from_end, MAX_LINE));
            self.file_key = Some(key);
            self.ever_opened = true;
        }
        let Some(mut tailer) = self.tailer.take() else { return };
        let mut touched: IndexMap<String, Vec<NewActivity>> = IndexMap::new();
        let mut fresh = false;
        tailer.read_each(|e| self.on_event(e, catch_up, &mut touched, &mut fresh));
        self.tailer = Some(tailer);
        if catch_up {
            return;
        }
        for (id, drafts) in touched {
            if self.threads.contains_key(&id) {
                self.publish(store, &id, drafts);
            }
        }
        if fresh {
            self.schedule_scan(SCAN_DEBOUNCE_MS);
            self.publish_due.get_or_insert(now_ms() + SCAN_WAIT_MS);
        }
    }

    fn on_event(&mut self, e: OutboxEvent, catch_up: bool, touched: &mut IndexMap<String, Vec<NewActivity>>, fresh: &mut bool) {
        if !self.threads.contains_key(&e.thread_id) {
            if e.event == "session_end" {
                return;
            }
            let t = Thread { id: e.thread_id.clone(), state: DeepseekThreadState::default(), first_seen_at: self.deps.now(), proc_: None, awaiting_scan: !catch_up, initial: catch_up, published: false, pending: Vec::new() };
            self.threads.insert(e.thread_id.clone(), t);
            *fresh = true;
        }
        let Some(t) = self.threads.get_mut(&e.thread_id) else { return };
        let was_ended = t.state.ended_at.is_some();
        let changes = t.state.apply(&e);
        if !catch_up && t.published && ((was_ended && t.state.ended_at.is_none()) || e.event == "session_start") {
            t.proc_ = None;
            t.awaiting_scan = true;
            *fresh = true;
        }
        let drafts = if catch_up { Vec::new() } else { Self::drafts_of(&e.thread_id, t, &changes) };
        touched.entry(e.thread_id.clone()).or_default().extend(drafts);
    }
}
