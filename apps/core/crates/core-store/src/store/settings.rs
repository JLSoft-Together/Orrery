use indexmap::IndexSet;
use serde::Serialize;
use serde_json::Value;

use super::Store;
use crate::event::{
    Buckets, FavoriteRejectReason, FavoriteRejected, Favorites, FocusResult, FocusWindowResult, FolderPickResult, FolderPicked, HistoryData, Jobs, Loading,
    Models, MonitorEvent, Roots, Snoozes,
};
use crate::extra::{
    BackgroundJob, DayTimeline, Dict, Favorite, NotifySettings, PlanLimits, ResponseStats, SensitiveSettings, SessionRecord, SnoozeEntry, ToolSpan, UsageBucket,
    UsageScan, UsageSnapshot,
};

fn same_json<T: Serialize>(a: &T, b: &T) -> bool {
    match (serde_json::to_string(a), serde_json::to_string(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn without_at(report: &Value) -> Value {
    let mut out = report.clone();
    if let Value::Object(map) = &mut out {
        map.insert("at".into(), Value::String(String::new()));
    }
    out
}

impl Store {
    pub fn job_list(&self) -> &[BackgroundJob] {
        &self.jobs
    }

    pub fn current_limits(&self) -> Option<&PlanLimits> {
        self.limits.as_ref()
    }

    pub fn notify(&self) -> Option<&NotifySettings> {
        self.notify.as_ref()
    }

    pub fn snoozes(&self) -> &Dict<SnoozeEntry> {
        &self.snoozes
    }

    pub fn project_root(&self, dir: &str) -> Option<&str> {
        self.roots.get(dir).map(String::as_str)
    }

    pub fn set_loading(&mut self, loading: bool) {
        if loading == self.loading {
            return;
        }
        self.loading = loading;
        self.emit(MonitorEvent::SessionsLoading(Loading { loading }));
    }

    pub fn set_notify(&mut self, settings: NotifySettings) {
        if self.notify.as_ref().is_some_and(|n| same_json(n, &settings)) {
            return;
        }
        self.notify = Some(settings.clone());
        self.emit(MonitorEvent::NotifySettings(settings));
    }

    pub fn set_snoozes(&mut self, snoozes: Dict<SnoozeEntry>) {
        if same_json(&snoozes, &self.snoozes) {
            return;
        }
        self.snoozes = snoozes.clone();
        self.emit(MonitorEvent::SnoozeUpdated(Snoozes { snoozes }));
    }

    pub fn reset_usage(&mut self, snapshot: UsageSnapshot) {
        self.usage.clear();
        for b in &snapshot.buckets {
            self.usage.insert(b.key.clone(), b.clone());
        }
        self.usage_scan = snapshot.scan;
        let payload = self.usage_snapshot();
        self.emit(MonitorEvent::UsageReset(payload));
        let dirs = self.new_projects(&snapshot.buckets);
        self.report_new_projects(dirs);
    }

    pub fn set_project_roots(&mut self, roots: Dict<String>) {
        let mut added = Dict::new();
        for (dir, root) in roots {
            if self.roots.get(&dir) == Some(&root) {
                continue;
            }
            self.roots.insert(dir.clone(), root.clone());
            added.insert(dir, root);
        }
        if !added.is_empty() {
            self.emit(MonitorEvent::UsageRoots(Roots { roots: added }));
        }
    }

    fn new_projects(&self, buckets: &[UsageBucket]) -> Vec<String> {
        if self.on_new_projects.is_none() {
            return Vec::new();
        }
        let dirs: IndexSet<&str> = buckets.iter().map(|b| b.project.as_str()).filter(|d| !self.roots.contains_key(*d)).collect();
        dirs.into_iter().map(str::to_string).collect()
    }

    fn report_new_projects(&mut self, dirs: Vec<String>) {
        if dirs.is_empty() {
            return;
        }
        if let Some(callback) = self.on_new_projects.as_mut() {
            callback(dirs);
        }
    }

    pub fn update_usage(&mut self, buckets: Vec<UsageBucket>) {
        if buckets.is_empty() {
            return;
        }
        for b in &buckets {
            self.usage.insert(b.key.clone(), b.clone());
        }
        let dirs = self.new_projects(&buckets);
        self.emit(MonitorEvent::UsageUpdated(Buckets { buckets }));
        self.report_new_projects(dirs);
    }

    pub fn set_usage_scan(&mut self, scan: UsageScan) {
        self.usage_scan = scan.clone();
        self.emit(MonitorEvent::UsageScan(scan));
    }

    pub fn set_favorites(&mut self, favorites: Vec<Favorite>) {
        self.favorites = favorites.clone();
        self.emit(MonitorEvent::FavoritesUpdated(Favorites { favorites }));
    }

    pub fn reject_favorite(&mut self, dir: String, reason: FavoriteRejectReason) {
        self.emit(MonitorEvent::FavoriteRejected(FavoriteRejected { dir, reason }));
    }

    pub fn history_data(&mut self, sessions: Vec<SessionRecord>) {
        self.emit(MonitorEvent::HistoryData(HistoryData { sessions }));
    }

    pub fn history_added(&mut self, record: SessionRecord) {
        self.emit(MonitorEvent::HistoryAdded(Box::new(record)));
    }

    pub fn tool_span(&mut self, span: ToolSpan) {
        self.emit(MonitorEvent::ToolSpan(span));
    }

    pub fn timeline_data(&mut self, timeline: DayTimeline) {
        self.emit(MonitorEvent::TimelineData(timeline));
    }

    pub fn folder_picked(&mut self, request_id: String, result: FolderPickResult, dir: Option<String>) {
        self.emit(MonitorEvent::FolderPicked(FolderPicked { request_id, result, dir }));
    }

    pub fn focus_result(&mut self, terminal_id: String, result: FocusWindowResult) {
        self.emit(MonitorEvent::FocusResult(FocusResult { terminal_id, result }));
    }

    pub fn set_jobs(&mut self, jobs: Vec<BackgroundJob>) {
        if same_json(&jobs, &self.jobs) {
            return;
        }
        self.jobs = jobs.clone();
        self.emit(MonitorEvent::JobsUpdated(Jobs { jobs }));
    }

    pub fn set_limits(&mut self, limits: Option<PlanLimits>) {
        let Some(limits) = limits else { return };
        if self.limits.as_ref().is_some_and(|l| same_json(l, &limits)) {
            return;
        }
        self.limits = Some(limits.clone());
        self.emit(MonitorEvent::LimitsUpdated(limits));
    }

    pub fn set_codex_limits(&mut self, limits: PlanLimits) {
        if self.codex_limits.as_ref().is_some_and(|l| same_json(l, &limits)) {
            return;
        }
        self.codex_limits = Some(limits.clone());
        self.emit(MonitorEvent::CodexLimitsUpdated(limits));
    }

    pub fn set_response(&mut self, stats: ResponseStats) {
        if self.response.as_ref().is_some_and(|r| same_json(r, &stats)) {
            return;
        }
        self.response = Some(stats.clone());
        self.emit(MonitorEvent::ResponseUpdated(stats));
    }

    pub fn set_model_names(&mut self, names: Dict<String>) {
        if same_json(&names, &self.model_names) {
            return;
        }
        self.model_names = names.clone();
        self.emit(MonitorEvent::ModelsUpdated(Models { model_names: names }));
    }

    pub fn set_sensitive(&mut self, settings: SensitiveSettings) {
        if self.sensitive.as_ref().is_some_and(|s| same_json(s, &settings)) {
            return;
        }
        self.sensitive = Some(settings.clone());
        self.emit(MonitorEvent::SensitiveSettings(settings));
    }

    pub fn set_resources(&mut self, report: Value) {
        let same = self.resources.as_ref().is_some_and(|r| same_json(&without_at(r), &without_at(&report)));
        self.resources = Some(report.clone());
        if !same {
            self.emit(MonitorEvent::Resources(report));
        }
    }

    pub fn usage_snapshot(&self) -> UsageSnapshot {
        UsageSnapshot {
            buckets: self.usage.values().cloned().collect(),
            scan: self.usage_scan.clone(),
            roots: Some(self.roots.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        }
    }
}
