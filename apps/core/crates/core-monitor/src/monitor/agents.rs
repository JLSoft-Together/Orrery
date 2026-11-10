use std::sync::Arc;

use core_store::persist::forecast::LimitHistory;
use core_store::Store;

use super::Monitor;
use crate::agentprocs::AgentProcs;
use crate::codex::{CodexConfig, CodexMonitor};
use crate::deepseek::{DeepseekConfig, DeepseekMonitor};
use crate::watch::Wake;

impl Monitor {
    pub fn enable_agents(&mut self, procs: Arc<dyn AgentProcs>, persist: bool, verify: bool) {
        let c = &self.config;
        if let Some(home) = c.codex_home.clone() {
            let history = persist.then(|| {
                let mut h = LimitHistory::new(Some(c.data_dir.join("codex-limit-history.json")));
                h.load();
                h
            });
            let config = CodexConfig { codex_home: home, stale_ttl_ms: c.stale_ttl_ms, reconcile_ms: c.reconcile_ms, verify_ms: c.process_verify_ms, verify_processes: verify };
            let mut codex = CodexMonitor::new(config, self.deps.clone(), procs.clone(), history);
            if let Some(w) = &self.wake {
                codex.set_wake(w.clone());
            }
            self.codex = Some(codex);
        }
        let c = &self.config;
        if let Some(home) = c.codewhale_home.clone() {
            let config = DeepseekConfig { codewhale_home: home, home_dir: c.home_dir.clone(), stale_ttl_ms: c.stale_ttl_ms, reconcile_ms: c.reconcile_ms, verify_ms: c.process_verify_ms, verify_processes: verify };
            let mut deepseek = DeepseekMonitor::new(config, self.deps.clone(), procs);
            if let Some(w) = &self.wake {
                deepseek.set_wake(w.clone());
            }
            self.deepseek = Some(deepseek);
        }
    }

    pub fn start_agents(&mut self, store: &mut Store, watch: bool) {
        if let Some(mut c) = self.codex.take() {
            c.start(store, &|s| self.claude_targets(s), watch);
            self.codex = Some(c);
        }
        if let Some(mut d) = self.deepseek.take() {
            d.start(store, watch);
            self.deepseek = Some(d);
        }
    }

    pub(super) fn tick_agents(&mut self, store: &mut Store) {
        if let Some(mut c) = self.codex.take() {
            c.tick(store, &|s| self.claude_targets(s));
            self.codex = Some(c);
        }
        if let Some(d) = self.deepseek.as_mut() {
            d.tick(store);
        }
    }

    pub fn with_codex<R>(&mut self, store: &mut Store, f: impl FnOnce(&mut CodexMonitor, &mut Store, &dyn Fn(&Store) -> Vec<super::ClaudeTarget>) -> R) -> Option<R> {
        let mut c = self.codex.take()?;
        let r = f(&mut c, store, &|s| self.claude_targets(s));
        self.codex = Some(c);
        Some(r)
    }

    pub(super) fn stop_agents(&mut self) {
        if let Some(c) = self.codex.as_mut() {
            c.stop();
        }
        if let Some(d) = self.deepseek.as_mut() {
            d.stop();
        }
    }

    pub(super) fn set_agents_wake(&mut self, wake: &Wake) {
        if let Some(c) = self.codex.as_mut() {
            c.set_wake(wake.clone());
        }
        if let Some(d) = self.deepseek.as_mut() {
            d.set_wake(wake.clone());
        }
    }

    pub(super) fn agents_due(&self) -> Vec<i64> {
        self.codex.as_ref().and_then(CodexMonitor::next_due).into_iter().chain(self.deepseek.as_ref().and_then(DeepseekMonitor::next_due)).collect()
    }

    pub(super) fn flush_agents(&mut self) {
        if let Some(c) = self.codex.as_mut() {
            c.flush_persist();
        }
    }

    pub(super) fn dismiss_agents(&mut self, store: &mut Store, id: Option<&str>) -> usize {
        let a = self.codex.as_mut().map_or(0, |c| c.dismiss_ended(store, id));
        let b = self.deepseek.as_mut().map_or(0, |d| d.dismiss_ended(store, id));
        a + b
    }

    pub(super) fn agent_cwd(&self, id: &str) -> Option<Option<String>> {
        if CodexMonitor::is_codex_terminal(id) {
            Some(self.codex.as_ref().and_then(|c| c.cwd_of(id)))
        } else if DeepseekMonitor::is_deepseek_terminal(id) {
            Some(self.deepseek.as_ref().and_then(|d| d.cwd_of(id)))
        } else {
            None
        }
    }

    pub(super) fn agent_pid(&self, id: &str) -> Option<Option<u32>> {
        if CodexMonitor::is_codex_terminal(id) {
            Some(self.codex.as_ref().and_then(|c| c.pid_of(id)))
        } else if DeepseekMonitor::is_deepseek_terminal(id) {
            Some(self.deepseek.as_ref().and_then(|d| d.pid_of(id)))
        } else {
            None
        }
    }
}
