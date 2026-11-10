use std::collections::HashSet;

use core_store::extra::JobState;
use core_store::model::{ActivityData, ActivityKind};
use core_store::Store;
use core_transcript::time::parse_iso;
use indexmap::IndexMap;
use serde_json::Value;

use super::reconcile::{activity, titled};
use super::{Done, Monitor};
use crate::catalog::{read_model_catalog, read_settings_model};
use crate::hints::SessionContext;
use crate::job_stop::{is_daemon_running, JobStopResult};
use crate::sidefiles::{latest_limits, read_hooks, read_status_lines, HookRecord, COMPACT_STALE_MS};

fn newest(values: impl Iterator<Item = String>) -> Option<String> {
    let mut best: Option<String> = None;
    for v in values.filter(|v| !v.is_empty()) {
        if best.as_ref().is_none_or(|b| v.encode_utf16().cmp(b.encode_utf16()).is_gt()) {
            best = Some(v);
        }
    }
    best
}

impl Monitor {
    pub fn refresh_hints(&mut self, store: &mut Store) {
        let settings_model = read_settings_model(&self.config.claude_root);
        let catalog = read_model_catalog(&self.config.claude_root);
        let changed = settings_model != self.hints.settings_model || self.hints_unset || catalog.windows != self.hints.model_windows;
        self.hints_unset = false;
        self.hints.settings_model = settings_model;
        self.hints.model_windows = catalog.windows;
        store.set_model_names(catalog.names);
        if changed {
            let ids: Vec<String> = self.trackers.keys().cloned().collect();
            for id in ids {
                self.publish(store, &id);
            }
        }
    }

    pub fn poll_extras(&mut self, store: &mut Store) {
        if self.stopped {
            return;
        }
        let now = self.deps.now();
        self.poll_jobs(store, now);
        let mut changed: HashSet<String> = self.poll_hooks(now);
        let records = read_status_lines(&self.config.data_dir.join("statusline"), now);
        self.status_reports = (records.len() as u64, newest(records.iter().map(|r| r.at.clone())).or_else(|| self.status_reports.1.clone()));
        let limits = latest_limits(&records);
        store.set_limits(self.stores.limit_history.apply(limits.as_ref()));
        let mut live: HashSet<String> = HashSet::new();
        for r in &records {
            if let Some(w) = r.context_window {
                if self.hints.session_windows.get(&r.session_id) != Some(&w) {
                    self.hints.session_windows.insert(r.session_id.clone(), w);
                    changed.insert(r.session_id.clone());
                }
            }
            let prev = self.hints.session_context.get(&r.session_id);
            if let Some(pct) = r.context_pct {
                if prev.is_none_or(|p| p.at != r.at || p.pct != pct) {
                    self.hints.session_context.insert(r.session_id.clone(), SessionContext { pct, buffer_pct: r.buffer_pct, at: r.at.clone() });
                    changed.insert(r.session_id.clone());
                }
            }
            let Some(cost) = r.cost_usd else { continue };
            live.insert(r.session_id.clone());
            if self.costs.get(&r.session_id) == Some(&cost) {
                continue;
            }
            self.costs.insert(r.session_id.clone(), cost);
            changed.insert(r.session_id.clone());
        }
        let tracked: HashSet<String> = self.trackers.values().filter_map(|t| t.session_id().filter(|s| !s.is_empty()).map(str::to_owned)).collect();
        self.costs.retain(|id, _| live.contains(id) || tracked.contains(id));
        let reported: HashSet<&str> = records.iter().map(|r| r.session_id.as_str()).collect();
        self.hints.session_windows.retain(|id, _| reported.contains(id.as_str()) || tracked.contains(id));
        self.hints.session_context.retain(|id, _| reported.contains(id.as_str()) || tracked.contains(id));
        if !changed.is_empty() {
            let ids: Vec<String> = self.trackers.values().filter(|t| t.session_id().is_some_and(|s| changed.contains(s))).map(|t| t.terminal_id.clone()).collect();
            for id in ids {
                self.publish(store, &id);
            }
        }
        if self.stores.response.roll_day() {
            store.set_response(self.stores.response.stats());
        }
    }

    fn poll_hooks(&mut self, now: i64) -> HashSet<String> {
        let mut changed = HashSet::new();
        let mut seen = HashSet::new();
        let stale = |x: Option<&HookRecord>| x.and_then(|h| h.compacting_since.as_deref()).filter(|s| !s.is_empty()).and_then(parse_iso).is_some_and(|at| now - at >= COMPACT_STALE_MS);
        for r in read_hooks(&self.config.data_dir.join("hooks"), now) {
            seen.insert(r.session_id.clone());
            let prev = self.hooks.get(&r.session_id);
            if prev.is_some_and(|p| p.at == r.at) && stale(prev) == stale(Some(&r)) {
                continue;
            }
            changed.insert(r.session_id.clone());
            self.hooks.insert(r.session_id.clone(), r);
        }
        let gone: Vec<String> = self.hooks.keys().filter(|id| !seen.contains(*id)).cloned().collect();
        for id in gone {
            self.hooks.remove(&id);
            changed.insert(id);
        }
        changed
    }

    fn poll_jobs(&mut self, store: &mut Store, now: i64) {
        if !self.jobs_loaded {
            self.load_dismissed_jobs();
        }
        let list = self.job_reader.read(now);
        let mut drafts = Vec::new();
        if self.jobs_loaded {
            for job in &list {
                let prev = self.jobs.get(&job.id);
                if prev.is_some_and(|p| p.state == job.state) || (prev.is_none() && job.state == JobState::Done) {
                    continue;
                }
                let title = Some(job.name.clone().unwrap_or_else(|| job.id.clone()));
                match job.state {
                    JobState::Blocked => drafts.push(activity(ActivityKind::JobBlocked, None, Some(titled(title)))),
                    JobState::Done => drafts.push(activity(ActivityKind::JobDone, None, Some(titled(title)))),
                    _ => {}
                }
            }
        }
        self.jobs = list.into_iter().map(|j| (j.id.clone(), j)).collect();
        self.jobs_loaded = true;
        let stale: Vec<String> = self.dismissed_jobs.iter().filter(|(id, at)| !self.jobs.get(*id).is_some_and(|j| j.updated_at.as_deref() == Some(at.as_str()))).map(|(id, _)| id.clone()).collect();
        for id in stale {
            self.dismissed_jobs.shift_remove(&id);
            self.save_dismissed_jobs();
        }
        self.publish_jobs(store);
        for d in drafts {
            store.push_activity(d);
        }
    }

    fn load_dismissed_jobs(&mut self) {
        let Some(Value::Object(saved)) = self.stores.dismissed_jobs.as_ref().and_then(|f| f.read()) else { return };
        for (id, at) in saved {
            if let Value::String(at) = at {
                self.dismissed_jobs.insert(id, at);
            }
        }
    }

    fn save_dismissed_jobs(&mut self) {
        let now = core_store::time::now_ms();
        if let Some(f) = self.stores.dismissed_jobs.as_mut() {
            f.schedule(now);
        }
    }

    fn publish_jobs(&self, store: &mut Store) {
        store.set_jobs(self.jobs.values().filter(|j| !self.dismissed_jobs.contains_key(&j.id)).cloned().collect());
    }

    pub fn dismiss_jobs(&mut self, store: &mut Store, job_id: Option<&str>) -> usize {
        let mut removed = 0;
        let picks: IndexMap<String, String> = self
            .jobs
            .values()
            .filter(|j| match job_id {
                None => j.state == JobState::Done,
                Some(id) => j.id == id,
            })
            .filter(|j| !self.dismissed_jobs.contains_key(&j.id))
            .map(|j| (j.id.clone(), j.updated_at.clone().unwrap_or_default()))
            .collect();
        for (id, at) in picks {
            self.dismissed_jobs.insert(id, at);
            removed += 1;
        }
        if removed > 0 {
            self.save_dismissed_jobs();
            self.publish_jobs(store);
        }
        removed
    }

    pub fn stop_job(&mut self, job_id: &str) -> bool {
        let Some(job) = self.jobs.get(job_id) else { return false };
        if job.state == JobState::Done || self.stopping_jobs.contains(job_id) {
            return false;
        }
        self.stopping_jobs.insert(job_id.to_owned());
        let title = job.name.clone().unwrap_or_else(|| job.id.clone());
        let (deps, tx, id) = (self.deps.clone(), self.done_tx.clone(), job_id.to_owned());
        std::thread::spawn(move || {
            let r = deps.stop_job(&id);
            let _ = tx.send(Done::Stop(id, title, r));
        });
        true
    }

    pub(super) fn drain_done(&mut self, store: &mut Store) -> Vec<(String, JobStopResult)> {
        let mut out = Vec::new();
        while let Ok(Done::Stop(id, title, result)) = self.done_rx.try_recv() {
            out.push((id.clone(), self.finish_stop(store, &id, title, result)));
        }
        out
    }

    fn finish_stop(&mut self, store: &mut Store, job_id: &str, title: String, mut result: JobStopResult) -> JobStopResult {
        self.stopping_jobs.remove(job_id);
        let deps = self.deps.clone();
        if result == JobStopResult::Unconfirmed && !is_daemon_running(&self.config.claude_root, &|p| deps.is_pid_alive(p)) {
            result = JobStopResult::NotRunning;
        }
        let stopped = matches!(result, JobStopResult::Ok | JobStopResult::NotRunning);
        let data = ActivityData { title: Some(title), job_id: Some(job_id.to_owned()), error: (result != JobStopResult::Ok).then(|| result.as_str().to_owned()), ..ActivityData::default() };
        store.push_activity(activity(if stopped { ActivityKind::JobStopped } else { ActivityKind::JobStopFailed }, None, Some(data)));
        if stopped || result == JobStopResult::NotFound {
            self.dismiss_jobs(store, Some(job_id));
        }
        if result == JobStopResult::Ok {
            self.poll_extras(store);
        }
        result
    }
}
