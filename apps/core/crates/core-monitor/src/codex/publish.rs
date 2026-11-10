use core_store::model::{ActivityData, ActivityKind, Agent, AgentRole, AgentStatus, Provider, TerminalSession, TerminalStatus};
use core_store::store::NewActivity;
use core_store::time::iso_of_ms;
use core_store::{JsNum, Store};

use super::monitor::{CodexMonitor, Published, Thread, IDLE_END_MS};
use crate::shared::js_round1;
use crate::view::{blank_agent, blank_terminal};

pub struct Place {
    pub terminal_id: String,
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub standalone: bool,
}

fn finished(s: AgentStatus) -> bool {
    matches!(s, AgentStatus::Completed | AgentStatus::Error | AgentStatus::Cancelled)
}

pub fn win32_basename(s: &str) -> String {
    let b = s.as_bytes();
    let from = if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() { 2 } else { 0 };
    let rest = &s[from..];
    rest.rsplit(['\\', '/']).next().unwrap_or(rest).to_owned()
}

fn draft(kind: ActivityKind, terminal_id: &str, agent_id: Option<&str>, title: &str, with_agent_type: bool) -> NewActivity {
    let data = if with_agent_type {
        ActivityData { agent_type: Some("codex".into()), description: Some(title.to_owned()), ..ActivityData::default() }
    } else {
        ActivityData { title: Some(title.to_owned()), ..ActivityData::default() }
    };
    NewActivity { kind, terminal_id: Some(terminal_id.to_owned()), agent_id: agent_id.map(str::to_owned), data: Some(data), at: None }
}

fn whole(v: f64) -> u64 {
    v.max(0.0) as u64
}

impl CodexMonitor {
    fn gone(&self, t: &Thread) -> bool {
        if t.superseded || t.proc_gone {
            return true;
        }
        if t.proc_.is_some() {
            return false;
        }
        if t.mode == super::state::CodexMode::Exec && t.state.turn_open == Some(false) {
            return true;
        }
        self.deps.now() - t.last_write_at > IDLE_END_MS
    }

    fn child_status(&self, t: &Thread) -> AgentStatus {
        let s = &t.state;
        if s.turn_open == Some(true) {
            return if self.gone(t) {
                if t.proc_gone {
                    AgentStatus::Cancelled
                } else {
                    AgentStatus::Unknown
                }
            } else {
                AgentStatus::Working
            };
        }
        if s.aborted {
            return AgentStatus::Cancelled;
        }
        if s.turn_ended_at.as_deref().is_some_and(|x| !x.is_empty()) {
            return AgentStatus::Completed;
        }
        if self.gone(t) {
            AgentStatus::Unknown
        } else {
            AgentStatus::Idle
        }
    }

    fn terminal_status(&self, t: &Thread) -> TerminalStatus {
        if self.gone(t) {
            TerminalStatus::Stale
        } else if t.state.turn_open == Some(true) {
            TerminalStatus::Working
        } else {
            TerminalStatus::Idle
        }
    }

    fn title(&self, t: &Thread) -> String {
        if let Some(n) = self.names.get(&t.id) {
            return n.clone();
        }
        match t.meta.cwd.as_deref().filter(|c| !c.is_empty()) {
            Some(c) => win32_basename(c.trim_end_matches(['\\', '/'])),
            None => "Codex".to_owned(),
        }
    }

    pub(super) fn placement(&self, id: &str, depth: u8) -> Place {
        let own = Place { terminal_id: format!("codex:{id}"), agent_id: format!("codex:{id}:main"), parent_id: None, standalone: true };
        let Some(t) = self.threads.get(id) else { return own };
        if let Some((terminal, parent)) = &t.link {
            return Place { terminal_id: terminal.clone(), agent_id: format!("codex:{id}"), parent_id: Some(parent.clone()), standalone: false };
        }
        let parent = t.meta.parent_thread_id.as_deref().filter(|p| *p != id && self.threads.contains_key(*p));
        if let (Some(parent), true, true) = (parent, t.mode == super::state::CodexMode::Subagent, depth < 16) {
            let p = self.placement(parent, depth + 1);
            return Place { terminal_id: p.terminal_id, agent_id: format!("codex:{id}"), parent_id: Some(p.agent_id), standalone: false };
        }
        own
    }

    pub(super) fn publish(&mut self, store: &mut Store, id: &str, mut drafts: Vec<NewActivity>) {
        if self.booting || !self.threads.get(id).is_some_and(|t| !t.awaiting_scan) {
            return;
        }
        let place = self.placement(id, 0);
        if !place.standalone && store.get_terminal(&place.terminal_id).is_none() {
            if self.threads.get(id).is_some_and(|t| t.link.is_some()) {
                self.forget(store, id);
            }
            return;
        }
        let prev = self.threads.get(id).and_then(|t| t.published.clone());
        if prev.as_ref().is_some_and(|p| p.terminal_id != place.terminal_id || p.agent_id != place.agent_id) {
            self.unpublish(store, id);
        }
        let now = self.deps.now();
        let (term_status, child_status, title) = {
            let Some(t) = self.threads.get(id) else { return };
            (self.terminal_status(t), self.child_status(t), self.title(t))
        };
        let Some(t) = self.threads.get_mut(id) else { return };
        let fresh = t.published.is_none();
        let status;
        let terminal = &place.terminal_id;
        if place.standalone {
            if term_status == TerminalStatus::Stale && t.ended_at.is_none() {
                t.ended_at = Some(iso_of_ms(now));
                if prev.is_some() {
                    drafts.push(draft(ActivityKind::TerminalEnded, terminal, None, &title, false));
                }
            }
            status = if term_status == TerminalStatus::Stale {
                if t.state.turn_open != Some(true) {
                    AgentStatus::Completed
                } else if t.proc_gone {
                    AgentStatus::Cancelled
                } else {
                    AgentStatus::Unknown
                }
            } else if term_status == TerminalStatus::Working {
                AgentStatus::Working
            } else {
                AgentStatus::Idle
            };
            store.upsert_terminal(terminal_of(t, terminal, term_status, &title));
            if fresh && !t.initial && t.ended_at.is_none() {
                drafts.push(draft(ActivityKind::TerminalStarted, terminal, None, &title, false));
            }
        } else {
            status = child_status;
            if fresh && !t.initial {
                drafts.push(draft(ActivityKind::AgentStarted, terminal, Some(&place.agent_id), &title, true));
            }
        }
        if finished(status) && t.finished_at.is_none() && !place.standalone {
            t.finished_at = Some(now);
        }
        if !finished(status) && !place.standalone {
            t.finished_at = None;
        }
        if !place.standalone && finished(status) && t.last_status != Some(status) && (t.last_status.is_some() || !t.initial) {
            let kind = match status {
                AgentStatus::Completed => ActivityKind::AgentCompleted,
                AgentStatus::Cancelled => ActivityKind::AgentCancelled,
                _ => ActivityKind::AgentFailed,
            };
            drafts.push(draft(kind, terminal, Some(&place.agent_id), &title, true));
        }
        t.last_status = Some(status);
        store.upsert_agent(agent_of(t, &place, status, &title));
        t.published = Some(Published { terminal_id: place.terminal_id.clone(), agent_id: place.agent_id.clone(), standalone: place.standalone });
        for d in drafts {
            store.push_activity(d);
        }
    }

    pub(super) fn unpublish(&mut self, store: &mut Store, id: &str) {
        let Some(t) = self.threads.get_mut(id) else { return };
        let Some(prev) = t.published.take() else { return };
        if prev.standalone {
            store.remove_terminal(&prev.terminal_id);
        } else {
            store.remove_agent(&prev.agent_id);
        }
    }

    pub(super) fn forget(&mut self, store: &mut Store, id: &str) {
        self.unpublish(store, id);
        self.threads.shift_remove(id);
        while let Some(child) = self.threads.iter().find(|(_, c)| c.meta.parent_thread_id.as_deref() == Some(id) && c.link.is_none()).map(|(k, _)| k.clone()) {
            self.forget(store, &child);
        }
    }
}

fn terminal_of(t: &Thread, id: &str, status: TerminalStatus, title: &str) -> TerminalSession {
    let s = &t.state;
    let mut term = blank_terminal(id, title, status);
    term.provider = Some(Provider::Codex);
    term.process_id = t.proc_.map(|p| u64::from(p.0));
    term.entrypoint = Some(t.mode.as_str().to_owned());
    term.cwd.clone_from(&t.meta.cwd);
    term.codex_session_id = Some(t.id.clone());
    term.codex_version.clone_from(&t.meta.version);
    term.status_since = match status {
        TerminalStatus::Working => s.turn_started_at.clone(),
        TerminalStatus::Idle => s.turn_ended_at.clone().or_else(|| Some(iso_of_ms(t.meta.created_at))),
        _ => None,
    };
    term.compactions = (s.compactions > 0).then_some(s.compactions);
    term.started_at = Some(iso_of_ms(t.meta.created_at));
    term.last_activity_at.clone_from(&s.last_at);
    term.ended_at.clone_from(&t.ended_at);
    term
}

fn agent_of(t: &Thread, place: &Place, status: AgentStatus, title: &str) -> Agent {
    let s = &t.state;
    let role = if place.standalone { AgentRole::Main } else { AgentRole::Subagent };
    let mut a = blank_agent(&place.agent_id, &place.terminal_id, role, status);
    a.parent_id.clone_from(&place.parent_id);
    a.provider = Some(Provider::Codex);
    a.name = Some(title.to_owned());
    a.r#type = Some(t.mode.as_str().to_owned());
    a.model.clone_from(&s.model);
    a.effort.clone_from(&s.effort);
    if status == AgentStatus::Working {
        a.current_tool = s.current_tool().map(str::to_owned);
        a.current_tool_summary = s.current_tool_summary().map(str::to_owned);
        a.current_tool_started_at = s.current_tool_started_at().map(str::to_owned);
    }
    a.started_at = Some(iso_of_ms(t.meta.created_at));
    a.updated_at.clone_from(&s.last_at);
    if finished(status) {
        a.completed_at = s.turn_ended_at.clone().or_else(|| s.last_at.clone());
    }
    if let Some(total) = &s.total {
        let input = total.input - total.cached;
        a.input_tokens = Some(whole(input));
        a.output_tokens = Some(whole(total.output));
        a.cache_read_tokens = Some(whole(total.cached));
        a.total_tokens = Some(whole(input + total.output));
    }
    let context = s.last.as_ref().map(|l| l.total);
    a.context_tokens = context.map(whole);
    a.context_window = s.context_window.map(whole);
    if let (Some(c), Some(w)) = (context, s.context_window) {
        a.context_pct = Some(JsNum(js_round1((c / w * 100.0).min(100.0))));
    }
    a
}

#[cfg(test)]
mod tests {
    use super::win32_basename;

    #[test]
    fn takes_the_last_segment_like_path_win32_basename() {
        assert_eq!(win32_basename("C:\\Temp\\art-hero-x1"), "art-hero-x1");
        assert_eq!(win32_basename("/home/me/proj"), "proj");
        assert_eq!(win32_basename("C:"), "");
        assert_eq!(win32_basename("plain"), "plain");
    }
}
