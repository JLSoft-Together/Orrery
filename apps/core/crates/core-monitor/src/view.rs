use std::sync::Arc;

use core_store::model::{Agent, AgentRole, AgentStatus, CacheTtl, LimitHit, TerminalSession, TerminalStatus};
use core_store::time::iso_of_ms;
use core_store::JsNum;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::hints::ContextHints;
use crate::registry::RegistryRecord;
use crate::shared::{context_window_of, js_round1, main_agent_status, map_registry_status, usable_pct, WindowHints};

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PortConflict {
    pub port: f64,
    pub at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StateView {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub current_tool: Option<String>,
    pub last_at: Option<String>,
    pub tokens: Option<Tokens>,
    pub context_tokens: Option<f64>,
    pub peak_context: f64,
    pub cache_at: Option<String>,
    pub codex_calls: Vec<String>,
    pub device_call_at: Option<String>,
    pub port_conflict: Option<PortConflict>,
    pub first_context: Option<f64>,
    pub cache_ttl: Option<CacheTtl>,
    pub permission_mode: Option<String>,
    pub git_branch: Option<String>,
    pub compactions: Option<f64>,
    pub limit_hit: Option<Arc<LimitHit>>,
    pub last_reply: Option<String>,
    pub pending_ask: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SubMeta {
    pub agent_type: Option<String>,
    pub description: Option<String>,
    pub tool_use_id: Option<String>,
    pub spawn_depth: Option<f64>,
    pub parent_agent_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubView {
    pub agent_id: String,
    #[serde(default)]
    pub meta: SubMeta,
    pub started_at: Option<String>,
    pub status: AgentStatus,
    pub resolved: bool,
    pub completed_at: Option<String>,
    pub msg_in: Option<f64>,
    pub msg_out: Option<f64>,
    #[serde(default)]
    pub state: StateView,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct TrackerView {
    #[serde(default)]
    pub main: StateView,
    #[serde(default)]
    pub subs: Vec<SubView>,
}

impl TrackerView {
    pub fn from_full(v: Value, prev: &TrackerView) -> Option<TrackerView> {
        let mut next: TrackerView = serde_json::from_value(v).ok()?;
        if let (Some(a), Some(b)) = (&prev.main.limit_hit, &next.main.limit_hit) {
            if a == b {
                next.main.limit_hit = Some(a.clone());
            }
        }
        Some(next)
    }
}

pub struct ViewContext<'a> {
    pub terminal_id: &'a str,
    pub record: &'a RegistryRecord,
    pub shell: Option<&'a str>,
    pub parent_pid: Option<u32>,
    pub ended_at: Option<&'a str>,
    pub hints: &'a ContextHints,
    pub view: &'a TrackerView,
}

pub fn to_iso(ms: Option<f64>) -> Option<String> {
    let ms = ms?;
    (ms > 0.0 && ms.is_finite()).then(|| iso_of_ms(ms.trunc() as i64))
}

pub fn later_of(values: &[Option<&str>]) -> Option<String> {
    let mut best: Option<&str> = None;
    for v in values.iter().flatten() {
        if v.is_empty() {
            continue;
        }
        if best.is_none_or(|b| v.encode_utf16().cmp(b.encode_utf16()).is_gt()) {
            best = Some(v);
        }
    }
    best.map(str::to_owned)
}

pub fn basename(p: &str) -> String {
    let s = if cfg!(windows) {
        let b = p.as_bytes();
        if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
            &p[2..]
        } else {
            p
        }
    } else {
        p
    };
    let sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    let trimmed = s.trim_end_matches(sep);
    trimmed.rsplit(sep).next().unwrap_or("").to_owned()
}

pub fn blank_terminal(id: &str, title: &str, status: TerminalStatus) -> TerminalSession {
    serde_json::from_value(json!({ "id": id, "title": title, "status": status.as_str() })).expect("terminal")
}

pub fn blank_agent(id: &str, terminal_id: &str, role: AgentRole, status: AgentStatus) -> Agent {
    serde_json::from_value(json!({ "id": id, "terminalId": terminal_id, "role": role.as_str(), "status": status.as_str() })).expect("agent")
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|s| !s.is_empty())
}

pub fn build_terminal(c: &ViewContext) -> TerminalSession {
    let r = c.record;
    let main = &c.view.main;
    let (mapped, background) = map_registry_status(r.status.as_deref());
    let ended = c.ended_at.is_some();
    let title = match (non_empty(&r.name), non_empty(&r.cwd)) {
        (Some(n), _) => n.to_owned(),
        (None, Some(cwd)) => basename(cwd),
        (None, None) => format!("PID {}", r.pid),
    };
    let mut last_sub: Option<String> = None;
    for e in &c.view.subs {
        last_sub = later_of(&[last_sub.as_deref(), e.state.last_at.as_deref()]);
    }
    let status_since = to_iso(r.status_updated_at);
    let mut t = blank_terminal(c.terminal_id, &title, if ended { TerminalStatus::Stale } else { mapped });
    t.process_id = Some(u64::from(r.pid));
    t.parent_process_id = c.parent_pid.map(u64::from);
    t.shell = c.shell.map(str::to_owned);
    t.name_source = r.name_source.clone();
    t.kind = r.kind.clone();
    t.entrypoint = r.entrypoint.clone();
    t.cwd = r.cwd.clone();
    t.claude_session_id = r.session_id.clone();
    t.claude_version = r.version.clone();
    t.raw_status = r.status.clone();
    t.status_since = if ended { None } else { status_since.clone() };
    t.waiting_for = if !ended && mapped == TerminalStatus::Waiting { r.waiting_for.clone() } else { None };
    t.background_shell = (!ended && background).then_some(true);
    t.permission_mode = main.permission_mode.clone();
    t.git_branch = main.git_branch.clone();
    t.compactions = main.compactions.filter(|n| *n != 0.0 && !n.is_nan()).map(|n| n as u64);
    t.limit_hit = if ended { None } else { main.limit_hit.clone() };
    t.started_at = to_iso(r.started_at);
    t.last_activity_at = later_of(&[status_since.as_deref(), main.last_at.as_deref(), last_sub.as_deref()]);
    t.ended_at = c.ended_at.map(str::to_owned);
    t
}

struct Context {
    tokens: Option<u64>,
    window: Option<u64>,
    pct: Option<f64>,
    first: Option<u64>,
}

fn context_of(c: &ViewContext, state: &StateView, model: Option<&str>, main: bool) -> Option<Context> {
    let tokens = state.context_tokens?;
    let sid = if main { c.record.session_id.as_deref() } else { None };
    let catalog = model.and_then(|m| c.hints.model_windows.get(&m.replacen("[1m]", "", 1)).copied());
    let window = context_window_of(
        model,
        WindowHints {
            settings_model: c.hints.settings_model.as_deref(),
            peak: Some(state.peak_context),
            reported: sid.and_then(|s| c.hints.session_windows.get(s).copied()),
            catalog,
        },
    );
    let report = sid.and_then(|s| c.hints.session_context.get(s));
    let buffer = report.and_then(|r| r.buffer_pct).unwrap_or(c.hints.buffer_pct);
    let fresh = report.filter(|r| state.cache_at.as_deref().is_none_or(|at| at.is_empty() || r.at.encode_utf16().cmp(at.encode_utf16()).is_ge()));
    let raw = fresh.map(|r| r.pct).or_else(|| window.filter(|w| *w != 0.0).map(|w| tokens / w * 100.0));
    Some(Context {
        tokens: Some(tokens as u64),
        window: window.map(|w| w as u64),
        pct: raw.map(|r| js_round1(usable_pct(r, buffer))),
        first: if main { state.first_context.map(|f| f as u64) } else { None },
    })
}

fn apply_context(a: &mut Agent, ctx: Option<Context>) {
    let Some(ctx) = ctx else { return };
    a.context_tokens = ctx.tokens;
    a.context_window = ctx.window;
    a.context_pct = ctx.pct.map(JsNum);
    a.first_context_tokens = ctx.first;
}

fn apply_tokens(a: &mut Agent, t: &Option<Tokens>) {
    let Some(t) = t else { return };
    a.input_tokens = Some(t.input as u64);
    a.output_tokens = Some(t.output as u64);
    a.cache_read_tokens = Some(t.cache_read as u64);
    a.total_tokens = Some((t.input + t.output) as u64);
}

pub fn build_agents(c: &ViewContext, terminal: &TerminalSession) -> Vec<Agent> {
    let main_id = format!("{}:main", c.terminal_id);
    let main = &c.view.main;
    let main_status = main_agent_status(terminal.status);
    let show_tool = matches!(main_status, AgentStatus::Working | AgentStatus::Waiting);
    let mut m = blank_agent(&main_id, c.terminal_id, AgentRole::Main, main_status);
    m.name = Some(terminal.title.clone());
    m.r#type = Some("main".into());
    m.model = main.model.clone();
    m.effort = main.effort.clone();
    m.current_tool = if show_tool { main.current_tool.clone() } else { None };
    m.started_at = terminal.started_at.clone();
    m.updated_at = main.last_at.clone();
    m.completed_at = terminal.ended_at.clone();
    apply_tokens(&mut m, &main.tokens);
    apply_context(&mut m, context_of(c, main, main.model.as_deref(), true));
    m.cache_ttl = main.cache_ttl;
    m.cache_at = main.cache_at.clone();
    let mut agents = vec![m];
    let ended = c.ended_at.is_some();
    for e in &c.view.subs {
        let parent = e.meta.parent_agent_id.as_deref().filter(|p| !p.is_empty() && *p != e.agent_id && c.view.subs.iter().any(|s| s.agent_id == *p));
        let status = if ended && !e.resolved { AgentStatus::Unknown } else { e.status };
        let model = e.state.model.clone().or_else(|| e.meta.model.clone());
        let mut a = blank_agent(&e.agent_id, c.terminal_id, AgentRole::Subagent, status);
        a.parent_id = Some(parent.map_or_else(|| main_id.clone(), str::to_owned));
        a.name = e.meta.agent_type.clone();
        a.r#type = e.meta.agent_type.clone();
        a.description = e.meta.description.clone();
        a.tool_use_id = e.meta.tool_use_id.clone();
        a.spawn_depth = e.meta.spawn_depth.map(|d| d as u64);
        a.model = model.clone();
        a.effort = e.state.effort.clone();
        a.current_tool = if status == AgentStatus::Working { e.state.current_tool.clone() } else { None };
        a.started_at = e.started_at.clone();
        a.updated_at = e.state.last_at.clone();
        a.completed_at = if e.resolved { e.completed_at.clone() } else { None };
        apply_tokens(&mut a, &e.state.tokens);
        apply_context(&mut a, context_of(c, &e.state, model.as_deref(), false));
        a.msg_in = e.msg_in.filter(|n| *n != 0.0).map(|n| n as u64);
        a.msg_out = e.msg_out.filter(|n| *n != 0.0).map(|n| n as u64);
        agents.push(a);
    }
    agents
}

pub fn codex_calls_of(view: &TrackerView, main_id: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = view.main.codex_calls.iter().map(|at| (main_id.to_owned(), at.clone())).collect();
    for e in &view.subs {
        out.extend(e.state.codex_calls.iter().map(|at| (e.agent_id.clone(), at.clone())));
    }
    out
}

pub fn last_device_call_of(view: &TrackerView) -> Option<String> {
    let mut last = view.main.device_call_at.clone();
    for e in &view.subs {
        last = later_of(&[last.as_deref(), e.state.device_call_at.as_deref()]);
    }
    last
}

pub fn last_port_conflict_of(view: &TrackerView) -> Option<PortConflict> {
    let mut last = view.main.port_conflict.clone();
    for e in &view.subs {
        if let Some(c) = &e.state.port_conflict {
            if last.as_ref().is_none_or(|l| c.at.encode_utf16().cmp(l.at.encode_utf16()).is_gt()) {
                last = Some(c.clone());
            }
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basenames_and_later_like_node() {
        assert_eq!(basename("/a/b/"), "b");
        if cfg!(windows) {
            assert_eq!(basename("D:\\work\\demo"), "demo");
            assert_eq!(basename("C:\\"), "");
        }
        assert_eq!(later_of(&[None, Some("2026-01-02"), Some("2026-01-01"), Some("")]), Some("2026-01-02".into()));
        assert_eq!(to_iso(Some(0.0)), None);
        assert_eq!(to_iso(Some(1_791_366_405_292.7)), Some("2026-10-07T09:46:45.292Z".into()));
    }
}
