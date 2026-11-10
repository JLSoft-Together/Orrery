use core_store::extra::{BackgroundJob, JobState};
use core_store::model::{Agent, AgentRole, AgentStatus, TerminalSession, TerminalStatus};
use core_store::persist::localtime::{parse_iso, Zone};

pub const RES_IDLE_CPU_PCT: f64 = 1.0;

pub fn session_idle_since(t: &TerminalSession, agents: &[&Agent], jobs: &[&BackgroundJob]) -> Option<f64> {
    if t.ended_at.is_some() || !matches!(t.status, TerminalStatus::Idle | TerminalStatus::Waiting) {
        return None;
    }
    if agents.iter().any(|a| a.terminal_id == t.id && a.role == AgentRole::Subagent && a.status == AgentStatus::Working) {
        return None;
    }
    if let Some(sid) = &t.claude_session_id {
        if jobs.iter().any(|j| j.session_id.as_deref() == Some(sid.as_str()) && j.state == JobState::Working) {
            return None;
        }
    }
    let text = t.last_activity_at.as_deref().or(t.status_since.as_deref()).unwrap_or("");
    parse_iso(text, Zone::Local).map(|v| v as f64)
}

pub fn is_forgotten(idle_since: Option<f64>, now: f64, idle_minutes: f64) -> bool {
    idle_minutes > 0.0 && idle_since.is_some_and(|s| now - s >= idle_minutes * 60_000.0)
}

pub fn server_busy(cpu_pct: Option<f64>, ports: &[ResPort]) -> bool {
    cpu_pct.is_none_or(|c| c >= RES_IDLE_CPU_PCT) || ports.iter().any(|p| p.established > 0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResPort {
    pub port: i64,
    pub address: String,
    pub established: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusyState {
    pub first_seen: f64,
    pub last_busy: Option<f64>,
}

pub fn track_busy(prev: Option<BusyState>, busy: bool, now: f64) -> BusyState {
    let mut state = prev.unwrap_or(BusyState { first_seen: now, last_busy: None });
    if busy {
        state.last_busy = Some(now);
    }
    state
}

pub fn idle_since_of(state: &BusyState, now: f64) -> Option<f64> {
    if state.last_busy == Some(now) {
        return None;
    }
    Some(state.first_seen.max(state.last_busy.unwrap_or(0.0)))
}

pub fn emulator_idle_since(first_seen: f64, last_use: Option<f64>, last_call: Option<f64>) -> f64 {
    first_seen.max(last_use.unwrap_or(0.0)).max(last_call.unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: f64 = 1_793_527_200_000.0;

    fn term(status: TerminalStatus) -> TerminalSession {
        serde_json::from_value(serde_json::json!({"id": "a", "title": "proj", "status": status.as_str(), "lastActivityAt": "2026-11-01T10:00:00.000Z"})).unwrap()
    }

    fn agent(status: &str) -> Agent {
        serde_json::from_value(serde_json::json!({"id": "s", "terminalId": "a", "role": "subagent", "status": status})).unwrap()
    }

    fn job(state: &str) -> BackgroundJob {
        serde_json::from_value(serde_json::json!({"id": "j", "state": state, "sessionId": "c1"})).unwrap()
    }

    #[test]
    fn session_idle_only_when_idle_or_waiting_with_nothing_running() {
        assert_eq!(session_idle_since(&term(TerminalStatus::Idle), &[], &[]), Some(T0));
        assert_eq!(session_idle_since(&term(TerminalStatus::Working), &[], &[]), None);
        let mut ended = term(TerminalStatus::Idle);
        ended.ended_at = Some("x".into());
        assert_eq!(session_idle_since(&ended, &[], &[]), None);
        assert_eq!(session_idle_since(&term(TerminalStatus::Idle), &[&agent("working")], &[]), None);
        assert_eq!(session_idle_since(&term(TerminalStatus::Idle), &[&agent("idle")], &[]), Some(T0));
        let mut with_claude = term(TerminalStatus::Idle);
        with_claude.claude_session_id = Some("c1".into());
        assert_eq!(session_idle_since(&with_claude, &[], &[&job("working")]), None);
        assert_eq!(session_idle_since(&with_claude, &[], &[&job("done")]), Some(T0));
        assert!(is_forgotten(Some(T0), T0 + 3_600_000.0, 60.0));
        assert!(!is_forgotten(Some(T0), T0 + 59.0 * 60_000.0, 60.0));
        assert!(!is_forgotten(Some(T0), T0 + 999.0 * 60_000.0, 0.0));
    }

    #[test]
    fn server_quiet_streak_needs_low_cpu_and_no_connection() {
        let est = |n| ResPort { port: 1, address: "::".into(), established: n };
        assert!(!server_busy(Some(0.5), &[]));
        assert!(server_busy(Some(1.0), &[]));
        assert!(server_busy(None, &[]));
        assert!(server_busy(Some(0.0), &[est(1)]));
        let mut s = track_busy(None, false, T0);
        assert_eq!(idle_since_of(&s, T0), Some(T0));
        s = track_busy(Some(s), true, T0 + 1000.0);
        assert_eq!(idle_since_of(&s, T0 + 1000.0), None);
        s = track_busy(Some(s), false, T0 + 2000.0);
        assert_eq!(idle_since_of(&s, T0 + 2000.0), Some(T0 + 1000.0));
        assert_eq!(emulator_idle_since(T0, Some(T0 + 5.0), None), T0 + 5.0);
        assert_eq!(emulator_idle_since(T0, None, Some(T0 + 9.0)), T0 + 9.0);
    }
}
