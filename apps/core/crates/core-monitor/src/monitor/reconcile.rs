use std::collections::HashSet;

use core_store::model::{ActivityData, ActivityKind, AgentRole, Provider};
use core_store::store::NewActivity;
use core_store::time::iso_of_ms;
use core_store::Store;
use core_transcript::time::parse_iso;
use indexmap::IndexSet;

use super::{Monitor, PROJECT_DEBOUNCE_MS, REGISTRY_DEBOUNCE_MS, RESOLVED_SWEEP_MS};
use crate::process::proc_start_matches;
use crate::registry::{read_registry, RegistryEntry, RegistryRecord};
use crate::tracker::{terminal_id_of, MonTracker, SyncScope};

pub fn session_id_in(file: &str) -> Option<String> {
    let b = file.as_bytes();
    if b.len() < 36 {
        return None;
    }
    (0..=b.len() - 36).find_map(|i| {
        let w = &b[i..i + 36];
        let ok = w.iter().enumerate().all(|(j, c)| match j {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        });
        ok.then(|| String::from_utf8_lossy(w).into_owned())
    })
}

pub(super) fn activity(kind: ActivityKind, terminal_id: Option<&str>, data: Option<ActivityData>) -> NewActivity {
    NewActivity { kind, terminal_id: terminal_id.map(str::to_owned), agent_id: None, data, at: None }
}

pub(super) fn titled(title: Option<String>) -> ActivityData {
    ActivityData { title, ..ActivityData::default() }
}

impl Monitor {
    pub fn reconcile(&mut self, store: &mut Store, initial: bool, watch: bool) {
        if watch && (!self.sessions_watched || !self.projects_watched) {
            self.setup_watchers(true);
        }
        let registry = read_registry(&self.sessions_dir);
        let now = self.deps.now();
        let mut seen: HashSet<String> = HashSet::new();
        let mut created: Vec<MonTracker> = Vec::new();
        for (pid, entry) in registry {
            let rec = match entry {
                RegistryEntry::Unreadable => {
                    for t in self.trackers.values().filter(|t| t.record.pid == pid && !t.ended()) {
                        seen.insert(t.terminal_id.clone());
                    }
                    continue;
                }
                RegistryEntry::Record(r) => *r,
            };
            let id = terminal_id_of(&rec);
            if self.rejected.contains(&id) || !self.deps.is_pid_alive(pid) {
                continue;
            }
            if !self.trackers.contains_key(&id) {
                if !created.iter().any(|t| t.terminal_id == id) {
                    created.push(MonTracker::new(&id, rec, self.projects_dir.clone(), self.config.safety_net_ms, self.config.reread_finished));
                }
                seen.insert(id);
                continue;
            }
            if self.trackers.get(&id).is_some_and(MonTracker::ended) {
                continue;
            }
            seen.insert(id.clone());
            self.apply_record_change(store, &id, rec);
        }
        let accepted = self.verify_new(created);
        let accepted_ids: Vec<String> = accepted.iter().map(|t| t.terminal_id.clone()).collect();
        for tracker in accepted {
            let id = tracker.terminal_id.clone();
            self.trackers.insert(id.clone(), tracker);
            self.swept_at.insert(id.clone(), now);
            let sensitive = self.sensitive.as_deref_mut().map(|s| s as &mut dyn crate::tracker::SensitiveCheck);
            if let Some(t) = self.trackers.get_mut(&id) {
                t.sync(true, &SyncScope::All, now, sensitive);
            }
            self.publish(store, &id);
            self.drain_peers(&id);
            self.schedule_diff(&id);
            if !initial {
                let title = self.trackers.get(&id).map(|t| t.build_terminal(&self.hints).title);
                store.push_activity(activity(ActivityKind::TerminalStarted, Some(&id), Some(titled(title))));
            }
        }
        let gone: Vec<String> = self.trackers.values().filter(|t| !t.ended() && !seen.contains(&t.terminal_id)).map(|t| t.terminal_id.clone()).collect();
        for id in gone {
            self.end_tracker(store, &id, now);
        }
        let ids: Vec<String> = self.trackers.keys().cloned().collect();
        for id in ids {
            let Some(t) = self.trackers.get(&id) else { continue };
            if !t.ended() {
                if !accepted_ids.contains(&id) {
                    let scope = self.reconcile_scope(&id, now);
                    self.sync_tracker(store, &id, &scope);
                }
                continue;
            }
            let ended_at = t.ended_at.as_deref().and_then(parse_iso);
            if ended_at.is_some_and(|e| now - e >= self.config.stale_ttl_ms) {
                self.forget(store, &id);
            }
        }
        self.flush_peers(store, true);
        self.prune_peers(store);
    }

    fn reconcile_scope(&mut self, id: &str, now: i64) -> SyncScope {
        if let Some(last) = self.swept_at.get(id) {
            if now - last < RESOLVED_SWEEP_MS {
                return SyncScope::SkipResolved;
            }
        }
        self.swept_at.insert(id.to_owned(), now);
        SyncScope::All
    }

    fn apply_record_change(&mut self, store: &mut Store, id: &str, entry: RegistryRecord) {
        let Some(t) = self.trackers.get_mut(id) else { return };
        let prev = t.record.clone();
        let prev_title = t.build_terminal(&self.hints).title;
        let session_changed = t.set_record(entry.clone());
        let title = t.build_terminal(&self.hints).title;
        let mut drafts = Vec::new();
        if title != prev_title {
            drafts.push(activity(ActivityKind::TerminalRenamed, Some(id), Some(ActivityData { from: Some(prev_title), to: Some(title.clone()), ..ActivityData::default() })));
        }
        if prev.status.as_deref() == Some("waiting") && entry.status.as_deref() == Some("busy") {
            if let Some(since) = prev.status_updated_at {
                let replied = entry.status_updated_at.unwrap_or_else(|| self.deps.now() as f64);
                if self.stores.response.record(replied - since) {
                    store.set_response(self.stores.response.stats());
                }
            }
        }
        if entry.status != prev.status && entry.status.as_deref() != Some("busy") {
            self.schedule_diff(id);
        }
        if entry.status != prev.status && entry.status.as_deref() == Some("waiting") {
            drafts.push(activity(ActivityKind::TerminalStatus, Some(id), Some(ActivityData { to: Some("waiting".into()), waiting_for: entry.waiting_for.clone(), ..ActivityData::default() })));
        }
        if session_changed {
            drafts.push(activity(ActivityKind::SessionCleared, Some(id), Some(titled(Some(title)))));
            let subs: Vec<String> = store.agents_of(id).iter().filter(|a| a.role != AgentRole::Main && a.provider != Some(Provider::Codex)).map(|a| a.id.clone()).collect();
            for a in subs {
                store.remove_agent(&a);
            }
        }
        for d in drafts {
            store.push_activity(d);
        }
    }

    fn verify_new(&mut self, created: Vec<MonTracker>) -> Vec<MonTracker> {
        if created.is_empty() || !self.config.verify_processes {
            return created;
        }
        let pids: Vec<u32> = created.iter().map(|t| t.record.pid).collect();
        let Some(info) = self.deps.query_processes(&pids) else { return created };
        let mut accepted = Vec::new();
        for mut t in created {
            match info.get(&t.record.pid) {
                Some(p) if proc_start_matches(t.record.proc_start.as_deref(), &p.start) => {
                    t.parent_pid = p.parent_pid;
                    t.shell = p.parent_name.clone();
                    accepted.push(t);
                }
                _ => {
                    self.rejected.insert(t.terminal_id.clone());
                }
            }
        }
        accepted
    }

    pub(super) fn verify_live(&mut self, store: &mut Store) {
        if !self.config.verify_processes || self.stopped {
            return;
        }
        let live: Vec<(String, u32, Option<String>)> = self.trackers.values().filter(|t| !t.ended()).map(|t| (t.terminal_id.clone(), t.record.pid, t.record.proc_start.clone())).collect();
        if live.is_empty() {
            return;
        }
        let pids: Vec<u32> = live.iter().map(|l| l.1).collect();
        let Some(info) = self.deps.query_processes(&pids) else { return };
        let now = self.deps.now();
        for (id, pid, start) in live {
            if !info.get(&pid).is_some_and(|p| proc_start_matches(start.as_deref(), &p.start)) {
                self.end_tracker(store, &id, now);
            }
        }
    }

    pub(super) fn end_tracker(&mut self, store: &mut Store, id: &str, now: i64) {
        self.sync_tracker(store, id, &SyncScope::All);
        let at = iso_of_ms(now);
        let Some(t) = self.trackers.get_mut(id) else { return };
        t.mark_ended(&at);
        for span in t.close_open_spans(&at) {
            store.tool_span(span);
        }
        self.publish(store, id);
        if let Some(record) = self.record_of(store, id) {
            self.stores.history.add(record.clone());
            store.history_added(record);
        }
        let title = self.trackers.get(id).map(|t| t.build_terminal(&self.hints).title);
        store.push_activity(activity(ActivityKind::TerminalEnded, Some(id), Some(titled(title))));
    }

    pub(super) fn sync_tracker(&mut self, store: &mut Store, id: &str, scope: &SyncScope) {
        let now = self.deps.now();
        let sensitive = self.sensitive.as_deref_mut().map(|s| s as &mut dyn crate::tracker::SensitiveCheck);
        let Some(t) = self.trackers.get_mut(id) else { return };
        let drafts = t.sync(false, scope, now, sensitive);
        if t.take_changed() {
            self.publish(store, id);
        }
        if let Some(t) = self.trackers.get_mut(id) {
            for span in t.drain_spans() {
                store.tool_span(span);
            }
        }
        for d in drafts {
            store.push_activity(d);
        }
        self.drain_peers(id);
        self.flush_peers(store, false);
    }

    pub(super) fn forget(&mut self, store: &mut Store, id: &str) {
        self.trackers.shift_remove(id);
        self.diffs.remove(id);
        self.diff_due.shift_remove(id);
        self.sync_due.shift_remove(id);
        self.swept_at.remove(id);
        self.drop_peer_terminal(store, id);
        store.remove_terminal(id);
        self.stores.aliases.delete(id);
        self.stores.notes.delete(id);
    }

    pub(super) fn schedule_registry(&mut self, wall: i64) {
        if self.registry_due.is_none() {
            self.registry_due = Some(wall + REGISTRY_DEBOUNCE_MS);
        }
    }

    pub(super) fn route_project_change(&mut self, file: &str, wall: i64) {
        let Some(sid) = session_id_in(file) else { return };
        let Some(id) = self.trackers.values().find(|t| !t.ended() && t.session_id() == Some(sid.as_str())).map(|t| t.terminal_id.clone()) else { return };
        let entry = self.sync_due.entry(id).or_insert_with(|| (wall + PROJECT_DEBOUNCE_MS, IndexSet::new()));
        entry.1.insert(file.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::session_id_in;

    #[test]
    fn finds_a_session_id_in_a_project_path() {
        assert_eq!(session_id_in("D--w\\11111111-2222-3333-4444-555555555555\\subagents\\agent-a.jsonl").as_deref(), Some("11111111-2222-3333-4444-555555555555"));
        assert_eq!(session_id_in("D--w\\notes.md"), None);
    }
}
