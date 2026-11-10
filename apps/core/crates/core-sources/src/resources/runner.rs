use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use core_store::extra::BackgroundJob;
use core_store::extra::JobState;
use core_store::model::{Agent, AgentRole, AgentStatus, TerminalSession, TerminalStatus};

pub const RES_SAMPLE_MS: u64 = 30_000;

enum Msg {
    Active(bool),
    Stop,
}

pub struct Runner {
    tx: Sender<Msg>,
    handle: Option<JoinHandle<()>>,
}

impl Runner {
    pub fn spawn(interval: Duration, mut tick: impl FnMut() + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Msg>();
        let handle = std::thread::spawn(move || {
            let mut active = false;
            let mut next = Instant::now();
            loop {
                let msg = if active {
                    match rx.recv_timeout(next.saturating_duration_since(Instant::now())) {
                        Ok(m) => Some(m),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                } else {
                    match rx.recv() {
                        Ok(m) => Some(m),
                        Err(_) => return,
                    }
                };
                match msg {
                    Some(Msg::Stop) => return,
                    Some(Msg::Active(on)) => {
                        if on && !active {
                            active = true;
                            tick();
                            next = Instant::now() + interval;
                        } else if !on {
                            active = false;
                        }
                    }
                    None => {
                        tick();
                        next = Instant::now() + interval;
                    }
                }
            }
        });
        Self { tx, handle: Some(handle) }
    }

    pub fn set_active(&self, on: bool) {
        let _ = self.tx.send(Msg::Active(on));
    }

    pub fn stop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn busy_counts(terminals: &[TerminalSession], agents: &[Agent], jobs: &[BackgroundJob]) -> (u64, u64) {
    let busy = terminals.iter().filter(|t| t.status == TerminalStatus::Working).count() + agents.iter().filter(|a| a.role == AgentRole::Subagent && a.status == AgentStatus::Working).count();
    let running = jobs.iter().filter(|j| j.state == JobState::Working).count();
    (busy as u64, running as u64)
}

#[derive(Default)]
pub struct BusyTracker {
    last: Option<(u64, u64)>,
}

impl BusyTracker {
    pub fn changed(&mut self, counts: (u64, u64)) -> Option<serde_json::Value> {
        if self.last == Some(counts) {
            return None;
        }
        self.last = Some(counts);
        Some(serde_json::json!({"type": "busy", "agents": counts.0, "jobs": counts.1}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn term(status: &str) -> TerminalSession {
        serde_json::from_value(serde_json::json!({"id": "t", "title": "t", "status": status})).unwrap()
    }

    #[test]
    fn counts_working_sessions_subagents_and_jobs_and_reports_changes_only() {
        let agents: Vec<Agent> = ["main", "subagent", "subagent"]
            .iter()
            .zip(["working", "working", "idle"])
            .map(|(role, st)| serde_json::from_value(serde_json::json!({"id": "a", "terminalId": "t", "role": role, "status": st})).unwrap())
            .collect();
        let jobs: Vec<BackgroundJob> = ["working", "done"].iter().map(|s| serde_json::from_value(serde_json::json!({"id": "j", "state": s})).unwrap()).collect();
        let counts = busy_counts(&[term("working"), term("idle")], &agents, &jobs);
        assert_eq!(counts, (2, 1));
        let mut t = BusyTracker::default();
        assert_eq!(t.changed(counts), Some(serde_json::json!({"type": "busy", "agents": 2, "jobs": 1})));
        assert_eq!(t.changed(counts), None);
        assert!(t.changed((0, 0)).is_some());
        assert!(t.changed((0, 0)).is_none());
    }

    #[test]
    fn runner_ticks_only_while_active_and_ticks_at_once_when_turned_on() {
        let n = Arc::new(AtomicUsize::new(0));
        let c = n.clone();
        let mut r = Runner::spawn(Duration::from_millis(40), move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(n.load(Ordering::SeqCst), 0);
        r.set_active(true);
        std::thread::sleep(Duration::from_millis(30));
        assert!(n.load(Ordering::SeqCst) >= 1);
        std::thread::sleep(Duration::from_millis(150));
        r.set_active(false);
        std::thread::sleep(Duration::from_millis(30));
        let after = n.load(Ordering::SeqCst);
        assert!(after >= 3, "{after}");
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(n.load(Ordering::SeqCst), after);
        r.stop();
    }
}
