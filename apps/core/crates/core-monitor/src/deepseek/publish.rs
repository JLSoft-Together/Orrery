use core_store::model::{ActivityData, ActivityKind, Agent, AgentRole, AgentStatus, Provider, TerminalSession, TerminalStatus};
use core_store::store::NewActivity;
use core_store::time::iso_of_ms;
use core_store::Store;

use super::monitor::{DeepseekMonitor, Thread, PREFIX};
use super::state::{StateChange, SubOutcome, SubagentState, TurnOutcome};
use crate::codex::win32_basename;
use crate::view::{blank_agent, blank_terminal};

fn draft(kind: ActivityKind, terminal_id: &str, agent_id: Option<String>, at: Option<String>, data: ActivityData) -> NewActivity {
    NewActivity { kind, terminal_id: Some(terminal_id.to_owned()), agent_id, data: Some(data), at }
}

impl DeepseekMonitor {
    pub(super) fn title_of(t: &Thread) -> String {
        match t.state.workspace.as_deref().filter(|w| !w.is_empty()) {
            Some(ws) => {
                let base = win32_basename(ws.trim_end_matches(['\\', '/']));
                if base.is_empty() {
                    "DeepSeek".to_owned()
                } else {
                    base
                }
            }
            None => "DeepSeek".to_owned(),
        }
    }

    pub(super) fn sub_id(id: &str, sub: &SubagentState) -> String {
        format!("{PREFIX}{id}:sub:{}", sub.id)
    }

    pub(super) fn drafts_of(id: &str, t: &Thread, changes: &[StateChange]) -> Vec<NewActivity> {
        let terminal_id = format!("{PREFIX}{id}");
        let main = Some(format!("{terminal_id}:main"));
        let title = Self::title_of(t);
        let mut out = Vec::new();
        for c in changes {
            match c {
                StateChange::TurnStart { at } => out.push(draft(ActivityKind::TurnStarted, &terminal_id, main.clone(), at.clone(), ActivityData { title: Some(title.clone()), ..ActivityData::default() })),
                StateChange::TurnEnd { outcome, at } => {
                    let kind = match outcome {
                        TurnOutcome::Failed => ActivityKind::TurnFailed,
                        TurnOutcome::Interrupted => ActivityKind::TurnInterrupted,
                        TurnOutcome::Completed => ActivityKind::TurnCompleted,
                    };
                    out.push(draft(kind, &terminal_id, main.clone(), at.clone(), ActivityData { title: Some(title.clone()), ..ActivityData::default() }));
                }
                StateChange::SubSpawn(sub) => {
                    let data = ActivityData { agent_type: Some(sub.name.clone().unwrap_or_else(|| "subagent".into())), ..ActivityData::default() };
                    out.push(draft(ActivityKind::AgentStarted, &terminal_id, Some(Self::sub_id(id, sub)), sub.started_at.clone(), data));
                }
                StateChange::SubDone(sub) => {
                    let kind = match sub.outcome {
                        Some(SubOutcome::Error) => ActivityKind::AgentFailed,
                        Some(SubOutcome::Cancelled) => ActivityKind::AgentCancelled,
                        _ => ActivityKind::AgentCompleted,
                    };
                    let data = ActivityData { agent_type: Some(sub.name.clone().unwrap_or_else(|| "subagent".into())), ..ActivityData::default() };
                    out.push(draft(kind, &terminal_id, Some(Self::sub_id(id, sub)), sub.completed_at.clone(), data));
                }
                StateChange::SessionEnd => {}
            }
        }
        out
    }

    pub(super) fn end(t: &mut Thread, now: i64) {
        t.state.ended_at = Some(iso_of_ms(now));
        t.state.turn_open = false;
    }

    pub(super) fn publish(&mut self, store: &mut Store, id: &str, drafts: Vec<NewActivity>) {
        let booting = self.booting;
        let Some(t) = self.threads.get_mut(id) else { return };
        if booting || t.awaiting_scan {
            t.pending.extend(drafts);
            return;
        }
        let terminal_id = format!("{PREFIX}{id}");
        let main_id = format!("{terminal_id}:main");
        let title = Self::title_of(t);
        let s = &t.state;
        let ended = s.ended_at.is_some();
        let status = if ended {
            TerminalStatus::Stale
        } else if s.turn_open {
            TerminalStatus::Working
        } else {
            TerminalStatus::Idle
        };
        let mut out = Vec::new();
        if !t.published && !t.initial && !ended {
            out.push(draft(ActivityKind::TerminalStarted, &terminal_id, None, None, ActivityData { title: Some(title.clone()), ..ActivityData::default() }));
        }
        let was_ended = store.get_terminal(&terminal_id).is_some_and(|x| x.ended_at.as_deref().is_some_and(|e| !e.is_empty()));
        if t.published && ended && !was_ended {
            out.push(draft(ActivityKind::TerminalEnded, &terminal_id, None, None, ActivityData { title: Some(title.clone()), ..ActivityData::default() }));
        }
        let started_at = s.session_start_at.clone().or_else(|| s.started_at.clone());
        let mut terminal: TerminalSession = blank_terminal(&terminal_id, &title, status);
        terminal.provider = Some(Provider::Deepseek);
        terminal.process_id = if ended { None } else { t.proc_.map(|p| u64::from(p.0)) };
        terminal.cwd.clone_from(&s.workspace);
        terminal.deepseek_session_id = Some(id.to_owned());
        terminal.status_since = match status {
            TerminalStatus::Working => s.turn_started_at.clone(),
            TerminalStatus::Idle => s.turn_ended_at.clone().or_else(|| started_at.clone()),
            _ => None,
        };
        terminal.started_at.clone_from(&started_at);
        terminal.last_activity_at.clone_from(&s.last_at);
        terminal.ended_at.clone_from(&s.ended_at);
        store.upsert_terminal(terminal);
        let main_status = if ended {
            AgentStatus::Completed
        } else if s.turn_open {
            AgentStatus::Working
        } else {
            AgentStatus::Idle
        };
        let mut main: Agent = blank_agent(&main_id, &terminal_id, AgentRole::Main, main_status);
        main.provider = Some(Provider::Deepseek);
        main.name = Some(title.clone());
        main.started_at.clone_from(&started_at);
        main.updated_at.clone_from(&s.last_at);
        main.completed_at.clone_from(&s.ended_at);
        store.upsert_agent(main);
        for sub in s.subagents.values() {
            let sub_status = sub.outcome.map_or(if ended { AgentStatus::Unknown } else { AgentStatus::Working }, SubOutcome::status);
            let mut a = blank_agent(&Self::sub_id(id, sub), &terminal_id, AgentRole::Subagent, sub_status);
            a.parent_id = Some(main_id.clone());
            a.provider = Some(Provider::Deepseek);
            a.name.clone_from(&sub.name);
            a.r#type = Some(sub.name.clone().unwrap_or_else(|| "subagent".into()));
            a.started_at.clone_from(&sub.started_at);
            a.updated_at = sub.completed_at.clone().or_else(|| sub.started_at.clone());
            a.completed_at.clone_from(&sub.completed_at);
            store.upsert_agent(a);
        }
        let mut keep: Vec<String> = vec![main_id];
        keep.extend(s.subagents.keys().map(|k| format!("{terminal_id}:sub:{k}")));
        let stale: Vec<String> = store.agents_of(&terminal_id).iter().filter(|a| !keep.contains(&a.id)).map(|a| a.id.clone()).collect();
        for sid in stale {
            store.remove_agent(&sid);
        }
        t.published = true;
        let pending: Vec<NewActivity> = std::mem::take(&mut t.pending);
        for d in out.into_iter().chain(pending).chain(drafts) {
            store.push_activity(d);
        }
    }

    pub(super) fn forget(&mut self, store: &mut Store, id: &str) {
        if self.threads.get(id).is_some_and(|t| t.published) {
            store.remove_terminal(&format!("{PREFIX}{id}"));
        }
        self.threads.shift_remove(id);
    }
}
