mod batch;
mod settings;

use std::collections::{HashMap, VecDeque};

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{IdOnly, MonitorEvent, ServerMessage};
use crate::extra::{BackgroundJob, Dict, Favorite, NotifySettings, PlanLimits, ResponseStats, SensitiveSettings, SnoozeEntry, UsageBucket, UsageScan, UsageScanState};
use crate::model::{ActivityData, ActivityEvent, ActivityKind, Agent, AgentRole, PeerLink, TerminalSession};
use crate::time::{base36, iso_of_ms, now_ms};

pub use batch::EntityKind;

pub type Clock = Box<dyn Fn() -> i64 + Send>;
pub type Listener = Box<dyn FnMut(&ServerMessage) -> Result<(), String> + Send>;
pub type NewProjects = Box<dyn FnMut(Vec<String>) + Send>;

#[derive(Debug, Clone, Copy)]
pub struct StoreOptions {
    pub activity_limit: usize,
    pub batch_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewActivity {
    pub kind: ActivityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<ActivityData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

impl NewActivity {
    pub fn new(kind: ActivityKind) -> Self {
        Self { kind, terminal_id: None, agent_id: None, data: None, at: None }
    }
}

pub struct Store {
    options: StoreOptions,
    clock: Clock,
    terminals: IndexMap<String, TerminalSession>,
    agents: IndexMap<String, Agent>,
    agent_ids_by_terminal: HashMap<String, IndexSet<String>>,
    links: IndexMap<String, PeerLink>,
    activity: VecDeque<ActivityEvent>,
    tool_activity: VecDeque<ActivityEvent>,
    jobs: Vec<BackgroundJob>,
    limits: Option<PlanLimits>,
    codex_limits: Option<PlanLimits>,
    response: Option<ResponseStats>,
    model_names: Dict<String>,
    resources: Option<Value>,
    sensitive: Option<SensitiveSettings>,
    pub status_line_command: Option<String>,
    pub day_started_at: Option<String>,
    loading: bool,
    notify: Option<NotifySettings>,
    snoozes: Dict<SnoozeEntry>,
    usage: IndexMap<String, UsageBucket>,
    usage_scan: UsageScan,
    favorites: Vec<Favorite>,
    roots: IndexMap<String, String>,
    on_new_projects: Option<NewProjects>,
    pending: Vec<MonitorEvent>,
    pending_by_id: HashMap<(EntityKind, String), usize>,
    flush_at: Option<i64>,
    listeners: Vec<(u64, Listener)>,
    next_listener: u64,
    activity_counter: u64,
    seq: i64,
    started_at: String,
}

impl Store {
    pub fn new(options: StoreOptions) -> Self {
        Self::with_clock(options, Box::new(now_ms))
    }

    pub fn with_clock(options: StoreOptions, clock: Clock) -> Self {
        let started_at = iso_of_ms(clock());
        Self {
            options,
            clock,
            terminals: IndexMap::new(),
            agents: IndexMap::new(),
            agent_ids_by_terminal: HashMap::new(),
            links: IndexMap::new(),
            activity: VecDeque::new(),
            tool_activity: VecDeque::new(),
            jobs: Vec::new(),
            limits: None,
            codex_limits: None,
            response: None,
            model_names: Dict::new(),
            resources: None,
            sensitive: None,
            status_line_command: None,
            day_started_at: None,
            loading: false,
            notify: None,
            snoozes: Dict::new(),
            usage: IndexMap::new(),
            usage_scan: UsageScan { state: UsageScanState::Idle, files_done: 0, files_total: 0 },
            favorites: Vec::new(),
            roots: IndexMap::new(),
            on_new_projects: None,
            pending: Vec::new(),
            pending_by_id: HashMap::new(),
            flush_at: None,
            listeners: Vec::new(),
            next_listener: 0,
            activity_counter: 0,
            seq: 0,
            started_at,
        }
    }

    pub fn seq(&self) -> i64 {
        self.seq
    }

    pub fn started_at(&self) -> &str {
        &self.started_at
    }

    pub fn set_on_new_projects(&mut self, callback: Option<NewProjects>) {
        self.on_new_projects = callback;
    }

    pub fn terminal_list(&self) -> impl Iterator<Item = &TerminalSession> {
        self.terminals.values()
    }

    pub fn agent_list(&self) -> impl Iterator<Item = &Agent> {
        self.agents.values()
    }

    pub fn get_terminal(&self, id: &str) -> Option<&TerminalSession> {
        self.terminals.get(id)
    }

    pub fn get_agent(&self, id: &str) -> Option<&Agent> {
        self.agents.get(id)
    }

    pub fn agents_of(&self, terminal_id: &str) -> Vec<&Agent> {
        let Some(ids) = self.agent_ids_by_terminal.get(terminal_id) else { return Vec::new() };
        ids.iter().filter_map(|id| self.agents.get(id)).collect()
    }

    pub fn context_pct(&self, terminal_id: &str) -> Option<f64> {
        let a = self.agents.values().find(|a| a.terminal_id == terminal_id && a.role == AgentRole::Main)?;
        if let Some(pct) = a.context_pct {
            return Some(pct.0);
        }
        match (a.context_tokens, a.context_window) {
            (Some(tokens), Some(window)) if window > 0 => Some(context_used_pct(tokens as f64, window as f64)),
            _ => None,
        }
    }

    pub fn current_tool_of(&self, terminal_id: &str) -> Option<&str> {
        self.agents.values().find(|a| a.terminal_id == terminal_id && a.role == AgentRole::Main).and_then(|a| a.current_tool.as_deref())
    }

    pub fn tool_events(&self) -> &VecDeque<ActivityEvent> {
        &self.tool_activity
    }

    pub fn peer_links(&self) -> Vec<PeerLink> {
        self.links.values().cloned().collect()
    }

    pub fn upsert_terminal(&mut self, next: TerminalSession) {
        let Some(prev) = self.terminals.get(&next.id) else {
            self.terminals.insert(next.id.clone(), next.clone());
            return self.emit(MonitorEvent::TerminalCreated(Box::new(next)));
        };
        let Some(patch) = prev.diff(&next) else { return };
        self.terminals.insert(next.id.clone(), next);
        self.emit(MonitorEvent::TerminalUpdated(Box::new(patch)));
    }

    pub fn remove_terminal(&mut self, id: &str) {
        let agents: Vec<String> = self.agents_of(id).iter().map(|a| a.id.clone()).collect();
        for agent in agents {
            self.remove_agent(&agent);
        }
        if self.terminals.shift_remove(id).is_some() {
            self.emit(MonitorEvent::TerminalRemoved(IdOnly { id: id.to_string() }));
        }
    }

    pub fn upsert_agent(&mut self, next: Agent) {
        let Some(prev) = self.agents.get(&next.id) else {
            self.index_agent(&next.terminal_id, &next.id);
            self.agents.insert(next.id.clone(), next.clone());
            return self.emit(MonitorEvent::AgentCreated(Box::new(next)));
        };
        let Some(patch) = prev.diff(&next) else { return };
        let moved = (prev.terminal_id != next.terminal_id).then(|| prev.terminal_id.clone());
        if let Some(old) = moved {
            self.unindex_agent(&old, &next.id);
            self.index_agent(&next.terminal_id, &next.id);
        }
        self.agents.insert(next.id.clone(), next);
        self.emit(MonitorEvent::AgentUpdated(Box::new(patch)));
    }

    fn index_agent(&mut self, terminal_id: &str, id: &str) {
        self.agent_ids_by_terminal.entry(terminal_id.to_string()).or_default().insert(id.to_string());
    }

    fn unindex_agent(&mut self, terminal_id: &str, id: &str) {
        let Some(ids) = self.agent_ids_by_terminal.get_mut(terminal_id) else { return };
        ids.shift_remove(id);
        if ids.is_empty() {
            self.agent_ids_by_terminal.remove(terminal_id);
        }
    }

    pub fn remove_agent(&mut self, id: &str) {
        let Some(prev) = self.agents.shift_remove(id) else { return };
        self.unindex_agent(&prev.terminal_id, id);
        self.emit(MonitorEvent::AgentRemoved(IdOnly { id: id.to_string() }));
    }

    pub fn upsert_link(&mut self, next: PeerLink) {
        let prev = self.links.insert(next.id.clone(), next.clone());
        let Some(prev) = prev else {
            return self.emit(MonitorEvent::LinkCreated(next));
        };
        if let Some(patch) = prev.diff(&next) {
            self.emit(MonitorEvent::LinkUpdated(patch));
        }
    }

    pub fn remove_link(&mut self, id: &str) {
        if self.links.shift_remove(id).is_some() {
            self.emit(MonitorEvent::LinkRemoved(IdOnly { id: id.to_string() }));
        }
    }

    pub fn push_activity(&mut self, event: NewActivity) {
        let now = (self.clock)();
        let id = format!("{}-{}", base36(now.max(0) as u64), base36(self.activity_counter));
        self.activity_counter += 1;
        let value = ActivityEvent {
            id,
            at: event.at.unwrap_or_else(|| iso_of_ms(now)),
            kind: event.kind,
            terminal_id: event.terminal_id,
            agent_id: event.agent_id,
            data: event.data,
        };
        let limit = self.options.activity_limit;
        let ring = if value.kind.as_str().starts_with("tool.") { &mut self.tool_activity } else { &mut self.activity };
        ring.push_back(value.clone());
        while ring.len() > limit {
            ring.pop_front();
        }
        self.emit(MonitorEvent::Activity(Box::new(value)));
    }
}

pub fn context_used_pct(tokens: f64, window: f64) -> f64 {
    const BUFFER_PCT: f64 = 16.5;
    if window <= 0.0 {
        return 0.0;
    }
    let raw = tokens / window * 100.0;
    (raw / (100.0 - BUFFER_PCT) * 100.0).clamp(0.0, 100.0)
}
