use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::diagnostics::Diagnostics;
use crate::extra::{
    BackgroundJob, DayTimeline, Dict, Favorite, NotifySettings, PlanLimits, ResponseStats, SensitiveSettings, SessionRecord, SnoozeEntry, ToastHealth,
    ToastTestResult, ToolSpan, ToolSpansData, ToolStats, UsageBucket, UsageHeatmap, UsageScan, UsageSnapshot,
};
use crate::model::{ActivityEvent, Agent, AgentPatch, PeerLink, PeerLinkPatch, TerminalPatch, TerminalSession};

string_enum! {
    pub enum FavoriteRejectReason { Invalid = "invalid", NotFound = "notFound", Exists = "exists", Limit = "limit" }
}

string_enum! {
    pub enum FocusWindowResult {
        Ok = "ok",
        NotFound = "notFound",
        Ambiguous = "ambiguous",
        Failed = "failed",
        Unsupported = "unsupported",
        AppOnly = "appOnly",
        Denied = "denied",
    }
}

string_enum! {
    pub enum FolderPickResult { Ok = "ok", Cancelled = "cancelled", Busy = "busy", Failed = "failed", Unsupported = "unsupported" }
}

string_enum! {
    pub enum OpenApp { Explorer = "explorer", Vscode = "vscode" }
}

string_enum! {
    pub enum OpenResult { Ok = "ok", NotFound = "notFound", NoApp = "noApp", Failed = "failed", Unsupported = "unsupported" }
}

record! {
    pub struct MonitorSnapshot {
        req {
            protocol_version: u64,
            collector_started_at: String,
            supported_claude_version: String,
            terminals: Vec<TerminalSession>,
            agents: Vec<Agent>,
            activity: Vec<ActivityEvent>,
        }
        opt {
            usage: UsageSnapshot,
            favorites: Vec<Favorite>,
            jobs: Vec<BackgroundJob>,
            limits: PlanLimits,
            codex_limits: PlanLimits,
            response: ResponseStats,
            model_names: Dict<String>,
            status_line_command: String,
            day_started_at: String,
            notify: NotifySettings,
            snoozes: Dict<SnoozeEntry>,
            loading: bool,
            peer_links: Vec<PeerLink>,
            resources: Value,
            sensitive: SensitiveSettings,
        }
    }
}

record! { pub struct Loading { req { loading: bool } opt {} } }
record! { pub struct IdOnly { req { id: String } opt {} } }
record! { pub struct Buckets { req { buckets: Vec<UsageBucket> } opt {} } }
record! { pub struct Roots { req { roots: Dict<String> } opt {} } }
record! { pub struct Favorites { req { favorites: Vec<Favorite> } opt {} } }
record! { pub struct FavoriteRejected { req { dir: String, reason: FavoriteRejectReason } opt {} } }
record! { pub struct Jobs { req { jobs: Vec<BackgroundJob> } opt {} } }
record! { pub struct Models { req { model_names: Dict<String> } opt {} } }
record! { pub struct FocusResult { req { terminal_id: String, result: FocusWindowResult } opt {} } }
record! { pub struct FolderPicked { req { request_id: String, result: FolderPickResult } opt { dir: String } } }
record! { pub struct HistoryData { req { sessions: Vec<SessionRecord> } opt {} } }
record! { pub struct PromptOpenResult { req { search_id: String, session_id: String, result: OpenResult } opt {} } }
record! { pub struct TerminalOpenResult { req { terminal_id: String, app: OpenApp, result: OpenResult } opt {} } }
record! { pub struct Snoozes { req { snoozes: Dict<SnoozeEntry> } opt {} } }
record! { pub struct NotifyTestResult { req { result: ToastTestResult } opt { health: ToastHealth } } }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", deny_unknown_fields)]
pub enum MonitorEvent {
    #[serde(rename = "snapshot")]
    Snapshot(Box<MonitorSnapshot>),
    #[serde(rename = "sessions.loading")]
    SessionsLoading(Loading),
    #[serde(rename = "terminal.created")]
    TerminalCreated(Box<TerminalSession>),
    #[serde(rename = "terminal.updated")]
    TerminalUpdated(Box<TerminalPatch>),
    #[serde(rename = "terminal.removed")]
    TerminalRemoved(IdOnly),
    #[serde(rename = "agent.created")]
    AgentCreated(Box<Agent>),
    #[serde(rename = "agent.updated")]
    AgentUpdated(Box<AgentPatch>),
    #[serde(rename = "agent.removed")]
    AgentRemoved(IdOnly),
    #[serde(rename = "link.created")]
    LinkCreated(PeerLink),
    #[serde(rename = "link.updated")]
    LinkUpdated(PeerLinkPatch),
    #[serde(rename = "link.removed")]
    LinkRemoved(IdOnly),
    #[serde(rename = "activity")]
    Activity(Box<ActivityEvent>),
    #[serde(rename = "usage.reset")]
    UsageReset(UsageSnapshot),
    #[serde(rename = "usage.updated")]
    UsageUpdated(Buckets),
    #[serde(rename = "usage.scan")]
    UsageScan(UsageScan),
    #[serde(rename = "usage.roots")]
    UsageRoots(Roots),
    #[serde(rename = "favorites.updated")]
    FavoritesUpdated(Favorites),
    #[serde(rename = "favorite.rejected")]
    FavoriteRejected(FavoriteRejected),
    #[serde(rename = "tool.span")]
    ToolSpan(ToolSpan),
    #[serde(rename = "office.snapshot")]
    OfficeSnapshot(Value),
    #[serde(rename = "toolSpans.data")]
    ToolSpansData(ToolSpansData),
    #[serde(rename = "jobs.updated")]
    JobsUpdated(Jobs),
    #[serde(rename = "limits.updated")]
    LimitsUpdated(PlanLimits),
    #[serde(rename = "codexLimits.updated")]
    CodexLimitsUpdated(PlanLimits),
    #[serde(rename = "response.updated")]
    ResponseUpdated(ResponseStats),
    #[serde(rename = "models.updated")]
    ModelsUpdated(Models),
    #[serde(rename = "terminal.focusResult")]
    FocusResult(FocusResult),
    #[serde(rename = "folder.picked")]
    FolderPicked(FolderPicked),
    #[serde(rename = "history.data")]
    HistoryData(HistoryData),
    #[serde(rename = "history.added")]
    HistoryAdded(Box<SessionRecord>),
    #[serde(rename = "timeline.data")]
    TimelineData(DayTimeline),
    #[serde(rename = "heatmap.data")]
    HeatmapData(UsageHeatmap),
    #[serde(rename = "toolstats.data")]
    ToolStatsData(ToolStats),
    #[serde(rename = "promptSearch.progress")]
    PromptSearchProgress(Value),
    #[serde(rename = "promptSearch.done")]
    PromptSearchDone(Value),
    #[serde(rename = "promptSearch.openResult")]
    PromptSearchOpenResult(PromptOpenResult),
    #[serde(rename = "terminal.openResult")]
    TerminalOpenResult(TerminalOpenResult),
    #[serde(rename = "diagnostics.data")]
    DiagnosticsData(Box<Diagnostics>),
    #[serde(rename = "notify.settings")]
    NotifySettings(NotifySettings),
    #[serde(rename = "snooze.updated")]
    SnoozeUpdated(Snoozes),
    #[serde(rename = "notify.testResult")]
    NotifyTestResult(NotifyTestResult),
    #[serde(rename = "resources")]
    Resources(Value),
    #[serde(rename = "sensitive.settings")]
    SensitiveSettings(SensitiveSettings),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerMessage {
    pub seq: i64,
    pub events: Vec<MonitorEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct: Option<bool>,
}

impl ServerMessage {
    pub fn broadcast(seq: i64, events: Vec<MonitorEvent>) -> Self {
        Self { seq, events, direct: None }
    }

    pub fn direct(event: MonitorEvent) -> Self {
        Self { seq: -1, events: vec![event], direct: Some(true) }
    }
}
