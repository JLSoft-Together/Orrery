use std::collections::{HashSet, VecDeque};

use core_store::extra::ToolSpan;
use core_store::store::NewActivity;

use super::Monitor;
use crate::registry::RegistryEntry;
use crate::tracker::{terminal_id_of, SyncScope};
use crate::watch::{WatchEvent, Watcher};

#[derive(Default)]
pub(super) struct Staged {
    pub drafts: Vec<NewActivity>,
    pub spans: Vec<ToolSpan>,
    pub changed: bool,
}

pub(super) enum Synced {
    Staged(Staged),
    Covered,
    Read,
}

impl Monitor {
    pub fn presync(&mut self, now: i64) {
        self.clear_presync();
        if self.stopped {
            return;
        }
        self.tick_wall = Some(now);
        self.drain_watch(now);
        let mut gone: HashSet<String> = HashSet::new();
        if self.registry_due.is_some_and(|d| d <= now) {
            self.stage_reconcile(&mut gone);
        }
        let due: Vec<(String, Vec<String>)> = self.sync_due.iter().filter(|(_, (d, _))| *d <= now).map(|(k, (_, f))| (k.clone(), f.iter().cloned().collect())).collect();
        for (id, files) in due {
            self.stage(&id, &SyncScope::Files(files));
        }
        if self.timers.is_some_and(|t| t.reconcile <= now) {
            self.stage_reconcile(&mut gone);
        }
    }

    pub(super) fn clear_presync(&mut self) {
        self.tick_wall = None;
        self.staged.clear();
        self.covered.clear();
        self.pre_used.clear();
    }

    pub(super) fn drain_watch(&mut self, wall: i64) {
        let events = self.watcher.as_mut().map(Watcher::drain).unwrap_or_default();
        for e in events {
            match e {
                WatchEvent::Registry => self.schedule_registry(wall),
                WatchEvent::Project(rel) => self.project_changed(rel, wall),
                WatchEvent::SessionsLost => self.sessions_watched = false,
                WatchEvent::ProjectsLost => self.projects_watched = false,
            }
        }
    }

    pub fn project_changed(&mut self, rel: String, wall: i64) {
        self.route_project_change(&rel, wall);
        self.project_files.push(rel);
    }

    fn stage(&mut self, id: &str, scope: &SyncScope) {
        let now = self.deps.now();
        let sensitive = self.sensitive.as_deref_mut().map(|s| s as &mut dyn crate::tracker::SensitiveCheck);
        let Some(t) = self.trackers.get_mut(id) else { return };
        let drafts = t.sync(false, scope, now, sensitive);
        let staged = Staged { drafts, spans: t.drain_spans(), changed: t.take_changed() };
        self.staged.entry(id.to_owned()).or_default().push_back(staged);
    }

    fn stage_reconcile(&mut self, gone: &mut HashSet<String>) {
        let Some(registry) = self.pre_registry.as_ref() else { return };
        let mut seen: HashSet<String> = HashSet::new();
        let mut same: HashSet<String> = HashSet::new();
        for (pid, entry) in registry {
            match entry {
                RegistryEntry::Unreadable => {
                    for t in self.trackers.values().filter(|t| t.record.pid == *pid && !t.ended()) {
                        seen.insert(t.terminal_id.clone());
                        same.insert(t.terminal_id.clone());
                    }
                }
                RegistryEntry::Record(rec) => {
                    let id = terminal_id_of(rec);
                    if self.rejected.contains(&id) || !self.deps.is_pid_alive(*pid) {
                        continue;
                    }
                    let Some(t) = self.trackers.get(&id) else { continue };
                    if t.ended() {
                        continue;
                    }
                    if t.record.session_id == rec.session_id && t.record.cwd == rec.cwd {
                        same.insert(id.clone());
                    }
                    seen.insert(id);
                }
            }
        }
        let now = self.deps.now();
        let live: Vec<String> = self.trackers.values().filter(|t| !t.ended() && !gone.contains(&t.terminal_id)).map(|t| t.terminal_id.clone()).collect();
        for id in live {
            if !seen.contains(&id) {
                self.stage(&id, &SyncScope::All);
                self.covered.insert(id.clone());
                gone.insert(id);
            } else if same.contains(&id) {
                let scope = self.reconcile_scope(&id, now);
                self.stage(&id, &scope);
                self.covered.insert(id);
            }
        }
    }

    pub(super) fn take_synced(&mut self, id: &str) -> Synced {
        if let Some(s) = self.staged.get_mut(id).and_then(VecDeque::pop_front) {
            return Synced::Staged(s);
        }
        if self.covered.contains(id) {
            Synced::Covered
        } else {
            Synced::Read
        }
    }
}
