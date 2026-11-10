use std::path::PathBuf;

use core_store::Store;
use indexmap::IndexMap;

use super::link::{caller_of, claude_target_for, match_processes, ThreadFacts, ThreadKey};
use super::monitor::{CodexMonitor, Targets};
use super::process::CodexProcess;
use super::state::CodexMode;
use crate::monitor::ClaudeTarget;

fn sorted_desc(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).map(|r| r.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned())).collect()).unwrap_or_default();
    names.sort();
    names.reverse();
    names
}

impl CodexMonitor {
    pub(super) fn scan_once(&mut self, store: &mut Store, targets: Targets) {
        let procs = if self.config.verify_processes { self.procs.codex() } else { None };
        self.last_scan_ok = procs.is_some();
        let pending: Vec<String> = self.threads.values().filter(|t| t.awaiting_scan).map(|t| t.id.clone()).collect();
        let Some(procs) = procs else {
            let tg = targets(store);
            for id in pending {
                if let Some(t) = self.threads.get_mut(&id) {
                    t.awaiting_scan = false;
                }
                self.link(&id, None, &tg);
                self.publish(store, &id, Vec::new());
            }
            return;
        };
        self.processes = procs.len();
        self.find_resumed(store, &procs);
        let live: Vec<String> = self.threads.values().filter(|t| t.ended_at.is_none() && !t.proc_gone && !t.superseded).map(|t| t.id.clone()).collect();
        let keys: Vec<ThreadKey> = live
            .iter()
            .filter_map(|id| self.threads.get(id))
            .map(|t| ThreadKey { id: t.id.clone(), created_at: t.meta.created_at, mode: t.mode, parent_thread_id: t.meta.parent_thread_id.clone(), pid: t.proc_.map(|p| p.0), start: t.proc_.map(|p| p.1) })
            .collect();
        let matches = match_processes(&keys, &procs);
        let mut owners: IndexMap<u32, Vec<String>> = IndexMap::new();
        let tg = targets(store);
        for id in &live {
            let p = matches.get(id);
            if let Some(t) = self.threads.get_mut(id) {
                if let Some(p) = p {
                    t.proc_ = Some((p.pid, p.start));
                    t.proc_gone = false;
                    if matches!(t.mode, CodexMode::Tui | CodexMode::Exec) {
                        owners.entry(p.pid).or_default().push(id.clone());
                    }
                } else if t.proc_.is_some() {
                    t.proc_gone = true;
                }
            }
            self.link(id, p, &tg);
        }
        for group in owners.values_mut() {
            if group.len() < 2 {
                continue;
            }
            group.sort_by_key(|id| std::cmp::Reverse(self.threads.get(id).map_or(0, |t| t.meta.created_at)));
            for old in group.iter().skip(1) {
                if let Some(t) = self.threads.get_mut(old) {
                    t.superseded = true;
                }
            }
        }
        for id in &pending {
            if let Some(t) = self.threads.get_mut(id) {
                t.awaiting_scan = false;
            }
        }
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            self.publish(store, &id, Vec::new());
        }
    }

    fn link(&mut self, id: &str, proc: Option<&CodexProcess>, targets: &[ClaudeTarget]) {
        let Some(t) = self.threads.get(id) else { return };
        if t.link.is_some() || t.mode == CodexMode::Subagent || t.published.as_ref().is_some_and(|p| !p.standalone) {
            return;
        }
        let facts = ThreadFacts { mode: t.mode, cwd: t.meta.cwd.as_deref(), created_at: t.meta.created_at };
        let found = claude_target_for(proc, &facts, targets, cfg!(windows)).map(|target| (target.terminal_id.clone(), caller_of(target, t.meta.created_at)));
        if let (Some(link), Some(t)) = (found, self.threads.get_mut(id)) {
            t.link = Some(link);
        }
    }

    fn find_resumed(&mut self, store: &mut Store, procs: &[CodexProcess]) {
        for p in procs {
            let Some(id) = p.resume_id.as_deref() else { continue };
            if self.threads.contains_key(id) || self.skipped.contains(id) || self.resume_searched.contains(id) {
                continue;
            }
            self.resume_searched.insert(id.to_owned());
            if let Some(file) = self.find_rollout(id) {
                let now = self.deps.now();
                self.add_thread(store, &file, true, Some(now));
            }
        }
    }

    fn find_rollout(&self, id: &str) -> Option<PathBuf> {
        let suffix = format!("-{id}.jsonl");
        for year in sorted_desc(&self.sessions_dir) {
            let ydir = self.sessions_dir.join(&year);
            for month in sorted_desc(&ydir) {
                let mdir = ydir.join(&month);
                for day in sorted_desc(&mdir) {
                    let dir = mdir.join(&day);
                    if let Some(hit) = sorted_desc(&dir).into_iter().find(|n| n.ends_with(&suffix)) {
                        return Some(dir.join(hit));
                    }
                }
            }
        }
        None
    }
}
