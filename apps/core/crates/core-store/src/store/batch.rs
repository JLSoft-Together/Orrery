use std::cmp::Ordering;

use super::{Listener, Store};
use crate::event::{MonitorEvent, MonitorSnapshot, ServerMessage};
use crate::model::ActivityEvent;
use crate::time::utf16_cmp;
use crate::{PROTOCOL_VERSION, SUPPORTED_CLAUDE_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Terminal,
    Agent,
    Link,
}

enum Verb {
    Created,
    Updated,
    Removed,
}

fn entity_of(event: &MonitorEvent) -> Option<(EntityKind, &str, Verb)> {
    use MonitorEvent as E;
    Some(match event {
        E::TerminalCreated(t) => (EntityKind::Terminal, &t.id, Verb::Created),
        E::TerminalUpdated(p) => (EntityKind::Terminal, &p.id, Verb::Updated),
        E::TerminalRemoved(i) => (EntityKind::Terminal, &i.id, Verb::Removed),
        E::AgentCreated(a) => (EntityKind::Agent, &a.id, Verb::Created),
        E::AgentUpdated(p) => (EntityKind::Agent, &p.id, Verb::Updated),
        E::AgentRemoved(i) => (EntityKind::Agent, &i.id, Verb::Removed),
        E::LinkCreated(l) => (EntityKind::Link, &l.id, Verb::Created),
        E::LinkUpdated(p) => (EntityKind::Link, &p.id, Verb::Updated),
        E::LinkRemoved(i) => (EntityKind::Link, &i.id, Verb::Removed),
        _ => return None,
    })
}

fn fold(into: &mut MonitorEvent, patch: &MonitorEvent) -> bool {
    use MonitorEvent as E;
    match (into, patch) {
        (E::TerminalCreated(full), E::TerminalUpdated(p)) => full.apply(p),
        (E::TerminalUpdated(prev), E::TerminalUpdated(p)) => prev.merge(p),
        (E::AgentCreated(full), E::AgentUpdated(p)) => full.apply(p),
        (E::AgentUpdated(prev), E::AgentUpdated(p)) => prev.merge(p),
        (E::LinkCreated(full), E::LinkUpdated(p)) => full.apply(p),
        (E::LinkUpdated(prev), E::LinkUpdated(p)) => prev.merge(p),
        _ => return false,
    }
    true
}

fn by_at(a: &ActivityEvent, b: &ActivityEvent) -> Ordering {
    utf16_cmp(&a.at, &b.at)
}

impl Store {
    pub fn subscribe(&mut self, listener: Listener) -> u64 {
        self.next_listener += 1;
        self.listeners.push((self.next_listener, listener));
        self.next_listener
    }

    pub fn unsubscribe(&mut self, id: u64) -> bool {
        let before = self.listeners.len();
        self.listeners.retain(|(i, _)| *i != id);
        self.listeners.len() != before
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.flush_at
    }

    pub fn poll(&mut self, now: i64) -> bool {
        match self.flush_at {
            Some(at) if now >= at => {
                self.flush();
                true
            }
            _ => false,
        }
    }

    pub fn flush(&mut self) {
        self.flush_at = None;
        if self.pending.is_empty() {
            return;
        }
        self.seq += 1;
        let message = ServerMessage::broadcast(self.seq, std::mem::take(&mut self.pending));
        self.pending_by_id.clear();
        for (_, listener) in &mut self.listeners {
            if let Err(err) = listener(&message) {
                eprintln!("[collector] store listener: {err}");
            }
        }
    }

    pub(super) fn emit(&mut self, event: MonitorEvent) {
        if let Some(event) = self.coalesce(event) {
            self.pending.push(event);
        }
        if self.flush_at.is_none() {
            self.flush_at = Some((self.clock)() + self.options.batch_ms);
        }
    }

    fn coalesce(&mut self, event: MonitorEvent) -> Option<MonitorEvent> {
        let Some((kind, id, verb)) = entity_of(&event) else { return Some(event) };
        let key = (kind, id.to_string());
        match verb {
            Verb::Removed => {
                self.pending_by_id.remove(&key);
            }
            Verb::Created => {
                self.pending_by_id.insert(key, self.pending.len());
            }
            Verb::Updated => match self.pending_by_id.get(&key) {
                Some(&index) => {
                    if fold(&mut self.pending[index], &event) {
                        return None;
                    }
                }
                None => {
                    self.pending_by_id.insert(key, self.pending.len());
                }
            },
        }
        Some(event)
    }

    pub fn snapshot(&self) -> MonitorSnapshot {
        let mut activity: Vec<ActivityEvent> = self.activity.iter().chain(self.tool_activity.iter()).cloned().collect();
        activity.sort_by(by_at);
        MonitorSnapshot {
            protocol_version: PROTOCOL_VERSION,
            collector_started_at: self.started_at.clone(),
            supported_claude_version: SUPPORTED_CLAUDE_VERSION.to_string(),
            terminals: self.terminals.values().cloned().collect(),
            agents: self.agents.values().cloned().collect(),
            activity,
            usage: Some(self.usage_snapshot()),
            favorites: Some(self.favorites.clone()),
            jobs: Some(self.jobs.clone()),
            limits: self.limits.clone(),
            codex_limits: self.codex_limits.clone(),
            response: self.response.clone(),
            model_names: Some(self.model_names.clone()),
            status_line_command: self.status_line_command.clone(),
            day_started_at: self.day_started_at.clone(),
            notify: self.notify.clone(),
            snoozes: Some(self.snoozes.clone()),
            loading: self.loading.then_some(true),
            peer_links: Some(self.peer_links()),
            resources: self.resources.clone(),
            sensitive: self.sensitive.clone(),
        }
    }

    pub fn snapshot_now(&self) -> ServerMessage {
        ServerMessage::broadcast(self.seq, vec![MonitorEvent::Snapshot(Box::new(self.snapshot()))])
    }

    pub fn snapshot_message(&mut self) -> ServerMessage {
        self.flush();
        self.snapshot_now()
    }
}
