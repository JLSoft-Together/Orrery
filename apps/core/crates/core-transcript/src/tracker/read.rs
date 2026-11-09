use super::{signals_of, Links, Out, Run, Tracker};
use crate::jsstr::{num, put, put_opt};
use crate::signal::Signal;
use crate::spans::ToolSpanLog;
use crate::state::TranscriptState;
use crate::tail::{lite_filter, signal_line_filter, Item, JsonlTailer};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

fn is_agent_tool(name: &str) -> bool {
    name == "Agent" || name == "Task"
}

fn add_key(map: &mut HashMap<String, HashSet<String>>, dirty: &mut HashSet<String>, agent: &str, key: &str) {
    if map.entry(agent.to_owned()).or_default().insert(key.to_owned()) {
        dirty.insert(agent.to_owned());
    }
}

pub fn draft(kind: &str, terminal_id: &str, agent_id: Option<&str>, at: &Option<String>, data: Option<Map<String, Value>>) -> Map<String, Value> {
    let mut m = Map::new();
    put(&mut m, "kind", kind);
    put(&mut m, "terminalId", terminal_id);
    if let Some(a) = agent_id {
        put(&mut m, "agentId", a);
    }
    put_opt(&mut m, "at", at);
    if let Some(d) = data {
        put(&mut m, "data", d);
    }
    m
}

impl Links {
    fn record_links(&mut self, signals: &[Signal]) {
        for s in signals {
            match s {
                Signal::ToolUse { id, name, .. } if is_agent_tool(name) => {
                    self.agent_tool_uses.insert(id.clone());
                }
                Signal::ToolResult { tool_use_id, is_error, async_launched: false, at, .. } if self.agent_tool_uses.contains(tool_use_id) => {
                    self.foreground.insert(tool_use_id.clone(), (*is_error, at.clone()));
                }
                Signal::TaskNotification { tool_use_id: Some(t), status, at, .. } if !t.is_empty() => {
                    self.notifications.insert(t.clone(), (status.clone(), at.clone()));
                }
                _ => {}
            }
        }
    }

    fn record_messages(&mut self, signals: &[Signal], agent: Option<&str>, dirty: &mut HashSet<String>) {
        for s in signals {
            match (s, agent) {
                (Signal::Handback { from, key, .. }, _) => add_key(&mut self.msg_out, dirty, from, key),
                (Signal::CoordinatorMsg { key, .. }, Some(a)) => add_key(&mut self.msg_in, dirty, a, key),
                (Signal::SendMain { key, .. }, Some(a)) => add_key(&mut self.msg_out, dirty, a, key),
                _ => {}
            }
        }
    }
}

fn record_peers(out: &mut Out, terminal_id: &str, signals: &[Signal], agent_id: &str, catch_up: bool, main: bool) {
    for s in signals {
        match s {
            Signal::PeerSent { msg_id, at } => {
                let mut m = Map::new();
                put(&mut m, "terminalId", terminal_id);
                put(&mut m, "agentId", agent_id);
                put(&mut m, "msgId", msg_id.as_str());
                put_opt(&mut m, "at", at);
                out.sent.push(Value::Object(m));
            }
            Signal::PeerIn { from_key, msg_id, dedupe_key, at } if main => {
                let mut m = Map::new();
                put(&mut m, "receiver", terminal_id);
                put(&mut m, "fromKey", from_key.as_str());
                put_opt(&mut m, "msgId", msg_id);
                put(&mut m, "dedupeKey", dedupe_key.as_str());
                put_opt(&mut m, "at", at);
                put(&mut m, "live", !catch_up);
                out.inbound.push(Value::Object(m));
            }
            _ => {}
        }
    }
}

fn track_spans(out: &mut Out, spans: &mut ToolSpanLog, signals: &[Signal], agent_id: &str, now: i64) {
    for s in spans.apply(signals, agent_id, now) {
        out.spans.push(s.to_json());
    }
    for id in spans.take_dropped() {
        out.spans.push(serde_json::json!({ "drop": id }));
    }
}

fn tool_activity(out: &mut Out, terminal_id: &str, signals: &[Signal], state: &TranscriptState, agent_id: &str) {
    for s in signals {
        match s {
            Signal::ToolUse { name, at, touch, .. } => {
                let mut data = Map::new();
                put(&mut data, "tool", name.as_str());
                let mut m = draft("tool.started", terminal_id, Some(agent_id), at, Some(data));
                if let Some(t) = touch {
                    put(&mut m, "touch", t.to_json());
                }
                out.activity.push(Value::Object(m));
            }
            Signal::ToolResult { tool_use_id, is_error: true, at, .. } => {
                let mut data = Map::new();
                if let Some(tool) = state.open_tools.get(tool_use_id) {
                    put(&mut data, "tool", tool.as_str());
                }
                out.activity.push(Value::Object(draft("tool.failed", terminal_id, Some(agent_id), at, Some(data))));
            }
            _ => {}
        }
    }
}

fn session_activity(out: &mut Out, terminal_id: &str, signals: &[Signal], state: &TranscriptState) {
    let mut mode = state.permission_mode.clone();
    for s in signals {
        match s {
            Signal::Compact { trigger, pre_tokens, post_tokens, at } => {
                let mut data = Map::new();
                put_opt(&mut data, "trigger", trigger);
                if let Some(p) = pre_tokens {
                    put(&mut data, "preTokens", num(*p));
                }
                if let Some(p) = post_tokens {
                    put(&mut data, "postTokens", num(*p));
                }
                out.activity.push(Value::Object(draft("terminal.compacted", terminal_id, None, at, Some(data))));
            }
            Signal::PermissionMode { mode: m } if mode.as_deref() != Some(m.as_str()) => {
                if mode.is_some() {
                    let mut data = Map::new();
                    put(&mut data, "mode", m.as_str());
                    out.activity.push(Value::Object(draft("terminal.mode", terminal_id, None, &None, Some(data))));
                }
                mode = Some(m.clone());
            }
            _ => {}
        }
    }
}

fn used(tailer: &JsonlTailer, before: u64) -> u64 {
    let after = tailer.offset();
    if after >= before {
        after - before
    } else {
        after
    }
}

impl Tracker {
    pub(super) fn read_main(&mut self, run: &Run, out: &mut Out, left: &mut u64) -> bool {
        let Tracker { terminal_id, main_agent_id, main_tailer, main, links, spans, dirty_main, dirty, .. } = self;
        let Some(tailer) = main_tailer.as_mut() else { return false };
        let (catch_up, now) = (run.catch_up, run.now);
        let before = tailer.offset();
        let mut filter = |line: &[u8], _end: u64, _size: u64| signal_line_filter(line);
        let mut on = |item: Item| {
            let signals = signals_of(item);
            if !signals.is_empty() {
                *dirty_main = true;
            }
            links.record_links(&signals);
            links.record_messages(&signals, None, dirty);
            record_peers(out, terminal_id, &signals, main_agent_id, catch_up, true);
            track_spans(out, spans, &signals, main_agent_id, now);
            if !catch_up {
                tool_activity(out, terminal_id, &signals, main, main_agent_id);
                session_activity(out, terminal_id, &signals, main);
            }
            main.apply(&signals);
        };
        let more = tailer.read_some(*left, &mut filter, &mut on);
        *left = left.saturating_sub(used(tailer, before));
        more
    }

    pub(super) fn read_sub(&mut self, id: &str, lite: bool, catch_up: bool, now: i64, out: &mut Out, left: &mut u64) -> bool {
        let Tracker { terminal_id, subs, links, spans, dirty, .. } = self;
        let Some(entry) = subs.get_mut(id) else { return false };
        let agent_id = entry.agent_id.clone();
        let links = RefCell::new(links);
        let before = entry.tailer.offset();
        let state = &mut entry.state;
        let mut filter = |line: &[u8], end: u64, size: u64| {
            if lite {
                lite_filter(line, end, size, &links.borrow().agent_tool_uses)
            } else {
                signal_line_filter(line)
            }
        };
        let mut on = |item: Item| {
            let signals = signals_of(item);
            if !signals.is_empty() {
                dirty.insert(agent_id.clone());
            }
            let mut l = links.borrow_mut();
            l.record_links(&signals);
            l.record_messages(&signals, Some(&agent_id), dirty);
            record_peers(out, terminal_id, &signals, &agent_id, catch_up, false);
            if !lite {
                track_spans(out, spans, &signals, &agent_id, now);
            }
            if !catch_up {
                tool_activity(out, terminal_id, &signals, state, &agent_id);
            }
            state.apply(&signals);
        };
        let more = entry.tailer.read_some(*left, &mut filter, &mut on);
        *left = left.saturating_sub(used(&entry.tailer, before));
        more
    }
}
