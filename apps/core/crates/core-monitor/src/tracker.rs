use std::path::PathBuf;
use std::time::Instant;

use core_store::extra::ToolSpan;
use core_store::model::{Agent, TerminalSession};
use core_store::store::NewActivity;
use core_transcript::tracker::{Out, Plan, Step, SyncArgs, Tracker};
use serde_json::{Map, Value};

use crate::hints::ContextHints;
use crate::peers::{PeerInbound, PeerSent};
use crate::registry::RegistryRecord;
use crate::spans::{span_event, SpanEvent, SpanMirror};
use crate::view::{build_agents, build_terminal, codex_calls_of, last_device_call_of, last_port_conflict_of, PortConflict, TrackerView, ViewContext};

pub const PAGE_BYTES: u64 = 4 * 1024 * 1024;
pub const SAFETY_NET_MS: u64 = 4_000;

#[derive(Debug, Clone, PartialEq)]
pub enum SyncScope {
    All,
    SkipResolved,
    Files(Vec<String>),
}

pub trait SensitiveCheck {
    fn check(&mut self, terminal_id: &str, touch: &Value, cwd: Option<&str>) -> Vec<Map<String, Value>>;
}

pub struct MonTracker {
    pub terminal_id: String,
    pub record: RegistryRecord,
    pub shell: Option<String>,
    pub parent_pid: Option<u32>,
    pub ended_at: Option<String>,
    main_agent_id: String,
    core: Tracker,
    epoch: f64,
    view: TrackerView,
    mirror: SpanMirror,
    pending_spans: Vec<ToolSpan>,
    inbound: Vec<PeerInbound>,
    sent: Vec<PeerSent>,
    changed: bool,
    opened: bool,
    last_sync_at: Option<Instant>,
    safety_net_ms: Option<u64>,
}

fn base(f: &str) -> String {
    f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned()
}

pub fn terminal_id_of(r: &RegistryRecord) -> String {
    let start = match (&r.proc_start, r.started_at) {
        (Some(p), _) => p.clone(),
        (None, Some(s)) => core_transcript::jsstr::num(s).to_string(),
        (None, None) => "0".into(),
    };
    format!("{}:{}", r.pid, start)
}

impl MonTracker {
    pub fn new(terminal_id: &str, record: RegistryRecord, projects_dir: PathBuf, safety_net_ms: Option<u64>, reread_finished: bool) -> Self {
        let mut core = Tracker::new(terminal_id, projects_dir);
        core.reread_finished = reread_finished;
        MonTracker {
            terminal_id: terminal_id.to_owned(),
            record,
            shell: None,
            parent_pid: None,
            ended_at: None,
            main_agent_id: format!("{terminal_id}:main"),
            core,
            epoch: 0.0,
            view: TrackerView::default(),
            mirror: SpanMirror::default(),
            pending_spans: Vec::new(),
            inbound: Vec::new(),
            sent: Vec::new(),
            changed: true,
            opened: false,
            last_sync_at: None,
            safety_net_ms,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.record.session_id.as_deref()
    }

    pub fn main_agent_id(&self) -> &str {
        &self.main_agent_id
    }

    pub fn ended(&self) -> bool {
        self.ended_at.is_some()
    }

    pub fn last_reply(&self) -> Option<&str> {
        self.view.main.last_reply.as_deref()
    }

    pub fn pending_ask(&self) -> Option<&Value> {
        self.view.main.pending_ask.as_ref()
    }

    pub fn set_record(&mut self, record: RegistryRecord) -> bool {
        let session_changed = record.session_id != self.record.session_id;
        if record != self.record {
            self.changed = true;
        }
        self.record = record;
        if session_changed {
            self.epoch += 1.0;
            self.view = TrackerView::default();
            self.last_sync_at = None;
        }
        session_changed
    }

    pub fn mark_ended(&mut self, at: &str) {
        if self.ended_at.as_deref() != Some(at) {
            self.changed = true;
        }
        self.ended_at = Some(at.to_owned());
        self.core.release();
    }

    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    pub fn drain_spans(&mut self) -> Vec<ToolSpan> {
        std::mem::take(&mut self.pending_spans)
    }

    pub fn drain_peers(&mut self) -> (Vec<PeerInbound>, Vec<PeerSent>) {
        (std::mem::take(&mut self.inbound), std::mem::take(&mut self.sent))
    }

    pub fn close_open_spans(&mut self, at: &str) -> Vec<ToolSpan> {
        let changed = self.mirror.close_open(at);
        self.core.close_spans(at);
        changed
    }

    pub fn tool_spans(&mut self, now: i64, since_ms: Option<f64>) -> Vec<ToolSpan> {
        self.mirror.list(now, since_ms)
    }

    pub fn codex_calls(&self) -> Vec<(String, String)> {
        codex_calls_of(&self.view, &self.main_agent_id)
    }

    pub fn last_device_call_at(&self) -> Option<String> {
        last_device_call_of(&self.view)
    }

    pub fn last_port_conflict(&self) -> Option<PortConflict> {
        last_port_conflict_of(&self.view)
    }

    fn ctx<'a>(&'a self, hints: &'a ContextHints) -> ViewContext<'a> {
        ViewContext {
            terminal_id: &self.terminal_id,
            record: &self.record,
            shell: self.shell.as_deref(),
            parent_pid: self.parent_pid,
            ended_at: self.ended_at.as_deref(),
            hints,
            view: &self.view,
        }
    }

    pub fn build_terminal(&self, hints: &ContextHints) -> TerminalSession {
        build_terminal(&self.ctx(hints))
    }

    pub fn build_agents(&self, hints: &ContextHints, terminal: &TerminalSession) -> Vec<Agent> {
        build_agents(&self.ctx(hints), terminal)
    }

    fn recently_synced(&self) -> bool {
        let Some(ms) = self.safety_net_ms.filter(|m| *m > 0) else { return false };
        self.opened && self.last_sync_at.is_some_and(|t| t.elapsed().as_millis() < u128::from(ms))
    }

    pub fn sync(&mut self, catch_up: bool, scope: &SyncScope, now: i64, sensitive: Option<&mut (dyn SensitiveCheck + '_)>) -> Vec<NewActivity> {
        if !catch_up && *scope == SyncScope::SkipResolved && self.recently_synced() {
            return Vec::new();
        }
        let plan = match scope {
            SyncScope::All => Plan { all: true, ..Plan::default() },
            SyncScope::SkipResolved => Plan { unresolved: true, ..Plan::default() },
            SyncScope::Files(files) => Plan { files: files.iter().map(|f| base(f)).collect(), ..Plan::default() },
        };
        let full = plan.all || plan.unresolved;
        let mut drafts = Vec::new();
        let mut sensitive = sensitive;
        loop {
            let args = SyncArgs { session_id: self.record.session_id.clone(), cwd: self.record.cwd.clone(), epoch: self.epoch, catch_up, plan: plan.clone(), now };
            let mut out = Out::default();
            let step = self.core.sync(Some(args), PAGE_BYTES, &mut out);
            self.apply(out, catch_up, &mut drafts, sensitive.as_deref_mut());
            if let Step::Done = step {
                break;
            }
        }
        if self.core.take_view().is_some() {
            if let Some(next) = TrackerView::from_full(self.core.view(), &self.view) {
                self.view = next;
            }
            self.changed = true;
        }
        self.opened = true;
        if full {
            self.last_sync_at = Some(Instant::now());
        }
        drafts
    }

    fn apply(&mut self, out: Out, catch_up: bool, drafts: &mut Vec<NewActivity>, mut sensitive: Option<&mut (dyn SensitiveCheck + '_)>) {
        for v in out.spans {
            let Some(e) = span_event(v) else { continue };
            self.mirror.apply(&e);
            if let (false, SpanEvent::Span(s)) = (catch_up, e) {
                self.pending_spans.push(s);
            }
        }
        self.inbound.extend(out.inbound.into_iter().filter_map(|v| serde_json::from_value(v).ok()));
        self.sent.extend(out.sent.into_iter().filter_map(|v| serde_json::from_value(v).ok()));
        for v in out.activity {
            let Value::Object(mut m) = v else { continue };
            let touch = m.remove("touch");
            let Ok(d) = serde_json::from_value::<NewActivity>(Value::Object(m)) else { continue };
            let hits = match (&touch, sensitive.as_deref_mut()) {
                (Some(t), Some(s)) => s.check(&self.terminal_id, t, self.record.cwd.as_deref()),
                _ => Vec::new(),
            };
            let tool = d.data.as_ref().and_then(|x| x.tool.clone());
            let (agent, at) = (d.agent_id.clone(), d.at.clone());
            drafts.push(d);
            for h in hits {
                let mut data = h;
                if let Some(t) = &tool {
                    data.insert("tool".into(), Value::from(t.as_str()));
                }
                let raw = serde_json::json!({ "kind": "sensitive", "terminalId": self.terminal_id, "agentId": agent, "at": at, "data": data });
                if let Ok(s) = serde_json::from_value::<NewActivity>(strip_nulls(raw)) {
                    drafts.push(s);
                }
            }
        }
    }
}

fn strip_nulls(v: Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k, strip_nulls(v))).collect()),
        other => other,
    }
}
