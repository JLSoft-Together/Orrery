use core_store::diagnostics::{Diagnostics, SessionCounts, StatusLineDiagnostics, Watchers};
use core_store::event::{FavoriteRejectReason, FocusWindowResult, FolderPickResult, MonitorEvent, OpenApp, OpenResult, TerminalOpenResult};
use core_store::extra::{ToolSpansData, UsageScan};
use core_store::model::{ActivityData, ActivityKind};
use core_store::persist::aliases::{sanitize_alias, MAX_ALIAS_LENGTH, MAX_NOTE_LENGTH};
use core_store::persist::favorites::AddResult;
use core_store::persist::paths::is_absolute;
use core_store::time::now_ms;
use core_store::Store;
use core_transcript::jsstr::js_trim;
use core_transcript::time::parse_iso;
use serde_json::Value;

use super::reconcile::activity;
use super::Monitor;
use crate::repo::style;

pub use core_http::client::{LaunchMode, TerminalApp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    NotFound,
    Denied,
    NotInstalled,
    Failed,
}

impl LaunchError {
    pub fn as_str(self) -> &'static str {
        match self {
            LaunchError::NotFound => "notFound",
            LaunchError::Denied => "denied",
            LaunchError::NotInstalled => "notInstalled",
            LaunchError::Failed => "failed",
        }
    }
}

pub type Reply<'a> = &'a mut dyn FnMut(MonitorEvent);

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeTarget {
    pub terminal_id: String,
    pub pid: u32,
    pub cwd: Option<String>,
    pub main_agent_id: String,
    pub ended: bool,
    pub last_activity_at: Option<String>,
    pub codex_calls: Vec<(String, String)>,
}

fn has_control_chars(s: &str) -> bool {
    s.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f)
}

fn unquote(s: &str) -> &str {
    match s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) if !inner.contains(['\n', '\r', '\u{2028}', '\u{2029}']) => inner,
        _ => s,
    }
}

fn mode_str(m: LaunchMode) -> &'static str {
    match m {
        LaunchMode::New => "new",
        LaunchMode::Continue => "continue",
    }
}

impl Monitor {
    pub fn ask_of(&self, id: &str) -> Option<&Value> {
        self.trackers.get(id).and_then(|t| t.pending_ask())
    }

    pub fn reply_of(&self, id: &str) -> Option<&str> {
        self.trackers.get(id).and_then(|t| t.last_reply())
    }

    pub fn set_alias(&mut self, store: &mut Store, id: &str, value: &Value) -> bool {
        let Some(before) = self.title_of(id) else { return false };
        let alias = sanitize_alias(value, MAX_ALIAS_LENGTH);
        if !self.stores.aliases.set(id, alias.as_deref()) {
            return false;
        }
        self.publish(store, id);
        let after = self.title_of(id).unwrap_or_default();
        if after != before {
            store.push_activity(activity(ActivityKind::TerminalRenamed, Some(id), Some(ActivityData { from: Some(before), to: Some(after), ..ActivityData::default() })));
        }
        true
    }

    pub fn set_note(&mut self, store: &mut Store, id: &str, value: &Value) -> bool {
        if !self.trackers.contains_key(id) || !self.stores.notes.set(id, sanitize_alias(value, MAX_NOTE_LENGTH).as_deref()) {
            return false;
        }
        self.publish(store, id);
        true
    }

    pub fn claude_targets(&self, store: &Store) -> Vec<ClaudeTarget> {
        self.trackers
            .values()
            .map(|t| ClaudeTarget {
                terminal_id: t.terminal_id.clone(),
                pid: t.record.pid,
                cwd: t.record.cwd.clone(),
                main_agent_id: t.main_agent_id().to_owned(),
                ended: t.ended(),
                last_activity_at: store.get_terminal(&t.terminal_id).and_then(|x| x.last_activity_at.clone()),
                codex_calls: t.codex_calls(),
            })
            .collect()
    }

    pub fn last_device_call_at(&self) -> Option<i64> {
        self.trackers.values().filter_map(|t| t.last_device_call_at().as_deref().and_then(parse_iso)).max()
    }

    pub fn port_conflicts(&self) -> Vec<(String, f64, i64)> {
        self.trackers.iter().filter_map(|(id, t)| t.last_port_conflict().and_then(|c| parse_iso(&c.at).map(|at| (id.clone(), c.port, at)))).collect()
    }

    pub fn dismiss_ended(&mut self, store: &mut Store, id: Option<&str>) -> usize {
        let ids: Vec<String> = self.trackers.values().filter(|t| t.ended() && id.is_none_or(|i| i == t.terminal_id)).map(|t| t.terminal_id.clone()).collect();
        for t in &ids {
            self.forget(store, t);
        }
        ids.len()
    }

    pub fn toggle_favorite(&mut self, store: &mut Store, id: &str) -> bool {
        let Some(cwd) = self.trackers.get(id).and_then(|t| t.record.cwd.clone()).filter(|c| !c.is_empty()) else { return false };
        let title = self.title_of(id).unwrap_or_default();
        self.stores.favorites.toggle(&cwd, &title);
        store.set_favorites(self.stores.favorites.list());
        true
    }

    pub fn rename_favorite(&mut self, store: &mut Store, dir: &str, label: &Value) -> bool {
        if !self.stores.favorites.rename(dir, label) {
            return false;
        }
        store.set_favorites(self.stores.favorites.list());
        true
    }

    pub fn add_favorite(&mut self, store: &mut Store, dir: &str, label: &Value) -> bool {
        let trimmed = unquote(js_trim(dir));
        let invalid = trimmed.is_empty() || trimmed.encode_utf16().count() > 1024 || !is_absolute(trimmed, style()) || trimmed.starts_with("\\\\") || trimmed.starts_with("//") || trimmed.starts_with("\\/") || trimmed.starts_with("/\\") || has_control_chars(trimmed);
        if invalid {
            store.reject_favorite(dir.to_owned(), FavoriteRejectReason::Invalid);
            return false;
        }
        if !std::fs::metadata(trimmed).is_ok_and(|m| m.is_dir()) {
            store.reject_favorite(dir.to_owned(), FavoriteRejectReason::NotFound);
            return false;
        }
        match self.stores.favorites.add(trimmed, label) {
            AddResult::Ok => {
                store.set_favorites(self.stores.favorites.list());
                true
            }
            AddResult::Exists => {
                store.reject_favorite(dir.to_owned(), FavoriteRejectReason::Exists);
                false
            }
            AddResult::Limit => {
                store.reject_favorite(dir.to_owned(), FavoriteRejectReason::Limit);
                false
            }
        }
    }

    pub fn reorder_favorites(&mut self, store: &mut Store, dirs: &[String]) -> bool {
        if !self.stores.favorites.reorder(dirs) {
            return false;
        }
        store.set_favorites(self.stores.favorites.list());
        true
    }

    pub fn remove_favorite(&mut self, store: &mut Store, dir: &str) -> bool {
        if !self.stores.favorites.remove(dir) {
            return false;
        }
        store.set_favorites(self.stores.favorites.list());
        true
    }

    pub fn open_favorite(&mut self, store: &mut Store, dir: &str, mode: LaunchMode, app: Option<TerminalApp>) -> bool {
        let Some(fav) = self.stores.favorites.get(dir).cloned() else { return false };
        let Some(result) = self.deps.launch(&fav.dir, mode, app) else { return false };
        let mut data = ActivityData { title: Some(fav.label), mode: Some(mode_str(mode).into()), ..ActivityData::default() };
        match result {
            Ok(()) => {
                store.push_activity(activity(ActivityKind::LaunchStarted, None, Some(data)));
                true
            }
            Err(e) => {
                data.error = Some(e.as_str().into());
                store.push_activity(activity(ActivityKind::LaunchFailed, None, Some(data)));
                false
            }
        }
    }

    pub fn send_history(&self, store: &mut Store, reply: Option<Reply>) {
        let sessions = self.stores.history.list();
        match reply {
            Some(r) => r(MonitorEvent::HistoryData(core_store::event::HistoryData { sessions })),
            None => store.history_data(sessions),
        }
    }

    pub fn send_tool_spans(&mut self, reply: Reply, terminal_id: Option<&str>, since: Option<&str>) {
        let since_ms = since.map(|s| parse_iso(s).map_or(f64::NAN, |v| v as f64));
        let now = self.deps.now();
        let mut spans = Vec::new();
        for t in self.trackers.values_mut().filter(|t| terminal_id.is_none_or(|id| id == t.terminal_id)) {
            spans.extend(t.tool_spans(now, since_ms));
        }
        reply(MonitorEvent::ToolSpansData(ToolSpansData { spans, terminal_id: terminal_id.map(str::to_owned), since: since.map(str::to_owned) }));
    }

    pub fn send_timeline(&mut self, store: &mut Store, reply: Option<Reply>) {
        let data = self.stores.timeline.data();
        match reply {
            Some(r) => r(MonitorEvent::TimelineData(data)),
            None => store.timeline_data(data),
        }
    }

    pub fn open_folder(&self, id: &str, app: OpenApp, reply: Reply) {
        let cwd = self.trackers.get(id).and_then(|t| t.record.cwd.clone()).filter(|c| !c.is_empty());
        let result = match cwd {
            None => OpenResult::NotFound,
            Some(c) => self.deps.open_folder(&c, app).unwrap_or(OpenResult::Unsupported),
        };
        reply(MonitorEvent::TerminalOpenResult(TerminalOpenResult { terminal_id: id.to_owned(), app, result }));
    }

    pub fn diagnostics(&self, usage: Option<UsageScan>, clients: u64) -> Diagnostics {
        let ended = self.trackers.values().filter(|t| t.ended()).count() as u64;
        let live = self.trackers.len() as u64 - ended;
        Diagnostics {
            started_at: self.started_at.clone(),
            platform: platform(),
            node: String::new(),
            claude_root: self.config.claude_root.to_string_lossy().into_owned(),
            claude_root_found: self.sessions_watched || self.projects_watched,
            data_dir: self.config.data_dir.to_string_lossy().into_owned(),
            watchers: Watchers { sessions: self.sessions_watched, projects: self.projects_watched },
            verify_processes: self.config.verify_processes,
            sessions: SessionCounts { live, ended },
            status_line: StatusLineDiagnostics { reports: self.status_reports.0, last_at: self.status_reports.1.clone() },
            clients,
            collector_version: None,
            codex: None,
            deepseek: None,
            usage,
        }
    }

    pub fn pick_folder(&mut self, store: &mut Store, request_id: &str, reply: Option<Reply>) {
        let (result, dir) = self.deps.pick_folder().unwrap_or((FolderPickResult::Unsupported, None));
        match reply {
            Some(r) => r(MonitorEvent::FolderPicked(core_store::event::FolderPicked { request_id: request_id.to_owned(), result, dir })),
            None => store.folder_picked(request_id.to_owned(), result, dir),
        }
    }

    pub fn focus_window(&mut self, store: &mut Store, id: &str, reply: Option<Reply>) -> FocusWindowResult {
        let pid = self.trackers.get(id).filter(|t| !t.ended()).map(|t| t.record.pid);
        let result = match pid {
            None => FocusWindowResult::NotFound,
            Some(p) => self.deps.focus_window(p).unwrap_or(FocusWindowResult::Unsupported),
        };
        match reply {
            Some(r) => r(MonitorEvent::FocusResult(core_store::event::FocusResult { terminal_id: id.to_owned(), result })),
            None => store.focus_result(id.to_owned(), result),
        }
        result
    }

    pub fn wall_now(&self) -> i64 {
        now_ms()
    }
}

pub fn platform() -> String {
    let os = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        other => other,
    };
    format!("{os} {arch}")
}
