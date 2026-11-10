use core_store::model::AgentStatus;
use core_store::time::{iso_of_ms, now_ms};
use core_transcript::jsstr::utf16_cmp;
use indexmap::IndexMap;
use serde_json::Value;
use std::cmp::Ordering;

const MAX_SUBAGENTS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOutcome {
    Completed,
    Error,
    Cancelled,
}

impl SubOutcome {
    pub fn status(self) -> AgentStatus {
        match self {
            SubOutcome::Completed => AgentStatus::Completed,
            SubOutcome::Error => AgentStatus::Error,
            SubOutcome::Cancelled => AgentStatus::Cancelled,
        }
    }

    fn parse(s: &str) -> Option<SubOutcome> {
        match s {
            "completed" => Some(SubOutcome::Completed),
            "error" => Some(SubOutcome::Error),
            "cancelled" => Some(SubOutcome::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Default)]
pub struct SubRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub outcome: Option<SubOutcome>,
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub event: String,
    pub kind: Option<String>,
    pub thread_id: String,
    pub at: Option<String>,
    pub workspace: Option<String>,
    pub subagent: Option<SubRef>,
}

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

impl OutboxEvent {
    pub fn from_value(v: &Value) -> Option<OutboxEvent> {
        let sub = v.get("subagent").map(|s| SubRef { id: text(s, "id"), name: text(s, "name"), outcome: text(s, "outcome").as_deref().and_then(SubOutcome::parse) });
        Some(OutboxEvent { event: text(v, "event")?, kind: text(v, "kind"), thread_id: text(v, "threadId")?, at: text(v, "at"), workspace: text(v, "workspace"), subagent: sub })
    }
}

#[derive(Debug, Clone)]
pub struct SubagentState {
    pub id: String,
    pub name: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub outcome: Option<SubOutcome>,
}

#[derive(Debug, Clone)]
pub enum StateChange {
    TurnStart { at: Option<String> },
    TurnEnd { outcome: TurnOutcome, at: Option<String> },
    SubSpawn(SubagentState),
    SubDone(SubagentState),
    SessionEnd,
}

#[derive(Debug, Default)]
pub struct DeepseekThreadState {
    pub workspace: Option<String>,
    pub started_at: Option<String>,
    pub session_start_at: Option<String>,
    pub last_at: Option<String>,
    pub turn_open: bool,
    pub stalled: bool,
    pub turn_started_at: Option<String>,
    pub turn_ended_at: Option<String>,
    pub last_outcome: Option<TurnOutcome>,
    pub turns: u64,
    pub ended_at: Option<String>,
    pub subagents: IndexMap<String, SubagentState>,
    auto_id: u64,
}

fn before(a: &str, b: &str) -> bool {
    utf16_cmp(a, b) == Ordering::Less
}

impl DeepseekThreadState {
    pub fn apply(&mut self, e: &OutboxEvent) -> Vec<StateChange> {
        let at = e.at.clone();
        if e.workspace.is_some() {
            self.workspace.clone_from(&e.workspace);
        }
        if let Some(a) = &at {
            if self.started_at.as_deref().is_none_or(|s| before(a, s)) {
                self.started_at = Some(a.clone());
            }
            if self.last_at.as_deref().is_none_or(|l| before(l, a)) {
                self.last_at = Some(a.clone());
            }
        }
        match e.event.as_str() {
            "session_start" => {
                self.session_start_at = at;
                self.ended_at = None;
                Vec::new()
            }
            "session_end" => {
                self.ended_at = Some(at.clone().or_else(|| self.last_at.clone()).unwrap_or_else(|| iso_of_ms(now_ms())));
                self.turn_open = false;
                self.stalled = false;
                vec![StateChange::SessionEnd]
            }
            "turn_start" => {
                self.ended_at = None;
                self.turn_open = true;
                self.stalled = false;
                self.turn_started_at.clone_from(&at);
                self.turns += 1;
                vec![StateChange::TurnStart { at }]
            }
            "turn_stalled" => {
                if self.turn_open {
                    self.stalled = true;
                }
                Vec::new()
            }
            "turn_end" => {
                let outcome = match e.kind.as_deref() {
                    Some("turn.failed") => TurnOutcome::Failed,
                    Some("turn.interrupted") => TurnOutcome::Interrupted,
                    _ => TurnOutcome::Completed,
                };
                self.turn_open = false;
                self.stalled = false;
                self.turn_ended_at.clone_from(&at);
                self.last_outcome = Some(outcome);
                vec![StateChange::TurnEnd { outcome, at }]
            }
            "subagent_spawn" => {
                let id = e.subagent.as_ref().and_then(|s| s.id.clone()).unwrap_or_else(|| {
                    self.auto_id += 1;
                    format!("n{}", self.auto_id)
                });
                let prev = self.subagents.get(&id).cloned();
                let name = e.subagent.as_ref().and_then(|s| s.name.clone()).or_else(|| prev.as_ref().and_then(|p| p.name.clone()));
                let started_at = prev.and_then(|p| p.started_at).or(at);
                let sub = SubagentState { id: id.clone(), name, started_at, completed_at: None, outcome: None };
                self.subagents.insert(id, sub.clone());
                self.prune();
                vec![StateChange::SubSpawn(sub)]
            }
            "subagent_complete" => {
                let Some(id) = e.subagent.as_ref().and_then(|s| s.id.clone()).or_else(|| self.latest_open()) else { return Vec::new() };
                let prev = self.subagents.get(&id).cloned();
                let name = e.subagent.as_ref().and_then(|s| s.name.clone()).or_else(|| prev.as_ref().and_then(|p| p.name.clone()));
                let started_at = prev.and_then(|p| p.started_at).or_else(|| at.clone());
                let outcome = e.subagent.as_ref().and_then(|s| s.outcome).unwrap_or(SubOutcome::Completed);
                let sub = SubagentState { id: id.clone(), name, started_at, completed_at: at.or_else(|| self.last_at.clone()), outcome: Some(outcome) };
                self.subagents.insert(id, sub.clone());
                self.prune();
                vec![StateChange::SubDone(sub)]
            }
            _ => Vec::new(),
        }
    }

    fn latest_open(&self) -> Option<String> {
        let mut best: Option<&SubagentState> = None;
        for s in self.subagents.values() {
            if s.outcome.is_some() {
                continue;
            }
            let later = best.is_none_or(|b| utf16_cmp(s.started_at.as_deref().unwrap_or(""), b.started_at.as_deref().unwrap_or("")) != Ordering::Less);
            if later {
                best = Some(s);
            }
        }
        best.map(|s| s.id.clone())
    }

    fn prune(&mut self) {
        if self.subagents.len() <= MAX_SUBAGENTS {
            return;
        }
        let ids: Vec<String> = self.subagents.keys().cloned().collect();
        for id in ids {
            if self.subagents.len() <= MAX_SUBAGENTS {
                break;
            }
            if self.subagents.get(&id).is_some_and(|s| s.outcome.is_some()) {
                self.subagents.shift_remove(&id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(event: &str, at: &str) -> OutboxEvent {
        OutboxEvent { event: event.into(), kind: None, thread_id: "t".into(), at: Some(at.into()), workspace: None, subagent: None }
    }

    #[test]
    fn folds_a_session_with_a_subagent() {
        let mut st = DeepseekThreadState::default();
        st.apply(&ev("session_start", "2026-11-01T10:00:00.000Z"));
        let changes = st.apply(&ev("turn_start", "2026-11-01T10:00:01.000Z"));
        assert!(st.turn_open && matches!(changes[0], StateChange::TurnStart { .. }));
        let mut spawn = ev("subagent_spawn", "2026-11-01T10:00:02.000Z");
        spawn.subagent = Some(SubRef { id: None, name: Some("explore".into()), outcome: None });
        st.apply(&spawn);
        assert_eq!(st.subagents.keys().collect::<Vec<_>>(), vec!["n1"]);
        let done = st.apply(&ev("subagent_complete", "2026-11-01T10:00:03.000Z"));
        let StateChange::SubDone(s) = &done[0] else { panic!("sub done") };
        assert_eq!((s.id.as_str(), s.outcome, s.name.as_deref()), ("n1", Some(SubOutcome::Completed), Some("explore")));
        let mut end = ev("turn_end", "2026-11-01T10:00:04.000Z");
        end.kind = Some("turn.failed".into());
        let c = st.apply(&end);
        assert!(matches!(c[0], StateChange::TurnEnd { outcome: TurnOutcome::Failed, .. }));
        st.apply(&ev("session_end", "2026-11-01T10:00:05.000Z"));
        assert_eq!(st.ended_at.as_deref(), Some("2026-11-01T10:00:05.000Z"));
        assert_eq!(st.started_at.as_deref(), Some("2026-11-01T10:00:00.000Z"));
        st.apply(&ev("session_start", "2026-11-01T10:01:00.000Z"));
        assert!(st.ended_at.is_none());
    }
}
