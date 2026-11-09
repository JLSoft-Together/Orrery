use crate::jsstr::{js_slice, put, put_opt};
use crate::omap::OMap;
use crate::signal::Signal;
use crate::time::parse_iso;
use serde_json::{Map, Value};

pub const SPAN_LIMIT: usize = 2000;
pub const SPAN_RETENTION_MS: i64 = 7_200_000;
pub const MAX_TOOL_NAME: usize = 80;
const PRUNE_SLACK: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpan {
    pub id: String,
    pub terminal_id: String,
    pub agent_id: String,
    pub tool: String,
    pub start: String,
    pub end: Option<String>,
    pub outcome: Option<&'static str>,
}

impl ToolSpan {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        put(&mut m, "id", self.id.as_str());
        put(&mut m, "terminalId", self.terminal_id.as_str());
        put(&mut m, "agentId", self.agent_id.as_str());
        put(&mut m, "tool", self.tool.as_str());
        put(&mut m, "start", self.start.as_str());
        put_opt(&mut m, "end", &self.end);
        if let Some(o) = self.outcome {
            put(&mut m, "outcome", o);
        }
        Value::Object(m)
    }
}

struct Entry {
    span: ToolSpan,
    start_ms: i64,
    end_ms: Option<i64>,
}

pub struct ToolSpanLog {
    terminal_id: String,
    spans: OMap<Entry>,
}

fn close(entry: &mut Entry, at: &str, outcome: &'static str) -> bool {
    let Some(t) = parse_iso(at) else { return false };
    let early = t < entry.start_ms;
    entry.span.end = Some(if early { entry.span.start.clone() } else { at.to_owned() });
    entry.end_ms = Some(if early { entry.start_ms } else { t });
    entry.span.outcome = Some(outcome);
    true
}

impl ToolSpanLog {
    pub fn new(terminal_id: &str) -> Self {
        ToolSpanLog { terminal_id: terminal_id.to_owned(), spans: OMap::new() }
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    fn slots(&self) -> Vec<usize> {
        self.spans.iter_slots().map(|(i, _, _)| i).collect()
    }

    pub fn apply(&mut self, signals: &[Signal], agent_id: &str, now_ms: i64) -> Vec<ToolSpan> {
        let mut changed = Vec::new();
        for s in signals {
            match s {
                Signal::ToolUse { id, name, at: Some(at), .. } if !at.is_empty() => {
                    if self.spans.contains(id) {
                        continue;
                    }
                    let Some(start_ms) = parse_iso(at) else { continue };
                    let span = ToolSpan { id: id.clone(), terminal_id: self.terminal_id.clone(), agent_id: agent_id.to_owned(), tool: js_slice(name, MAX_TOOL_NAME).to_owned(), start: at.clone(), end: None, outcome: None };
                    changed.push(span.clone());
                    self.spans.insert(id, Entry { span, start_ms, end_ms: None });
                }
                Signal::ToolResult { tool_use_id, is_error, at: Some(at), .. } if !at.is_empty() => {
                    if let Some(e) = self.spans.get_mut(tool_use_id) {
                        if e.end_ms.is_none() && close(e, at, if *is_error { "error" } else { "ok" }) {
                            changed.push(e.span.clone());
                        }
                    }
                }
                Signal::TurnEnd { at: Some(at) } if !at.is_empty() => {
                    for i in self.slots() {
                        let Some(e) = self.spans.at_mut(i) else { continue };
                        if e.span.agent_id == agent_id && e.end_ms.is_none() && close(e, at, "interrupted") {
                            changed.push(e.span.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        if self.spans.len() > SPAN_LIMIT + PRUNE_SLACK {
            self.prune(now_ms);
        }
        changed
    }

    pub fn close_open(&mut self, at: &str, agent_id: Option<&str>) -> Vec<ToolSpan> {
        let mut changed = Vec::new();
        for i in self.slots() {
            let Some(e) = self.spans.at_mut(i) else { continue };
            if e.end_ms.is_some() || agent_id.is_some_and(|a| e.span.agent_id != a) {
                continue;
            }
            if close(e, at, "interrupted") {
                changed.push(e.span.clone());
            }
        }
        changed
    }

    pub fn list(&mut self, since_ms: Option<i64>, now_ms: i64) -> Vec<ToolSpan> {
        self.prune(now_ms);
        let mut out: Vec<&Entry> = self.spans.iter().map(|(_, e)| e).filter(|e| !(since_ms.is_some() && e.end_ms.is_some_and(|end| end < since_ms.unwrap_or(0)))).collect();
        out.sort_by_key(|e| e.start_ms);
        out.into_iter().map(|e| e.span.clone()).collect()
    }

    pub fn prune(&mut self, now_ms: i64) {
        let cutoff = now_ms - SPAN_RETENTION_MS;
        let old: Vec<String> = self.spans.iter().filter(|(_, e)| e.end_ms.is_some_and(|t| t < cutoff)).map(|(k, _)| k.to_owned()).collect();
        for k in old {
            self.spans.remove(&k);
        }
        let ended: Vec<String> = self.spans.iter().filter(|(_, e)| e.end_ms.is_some()).map(|(k, _)| k.to_owned()).collect();
        for k in ended {
            if self.spans.len() <= SPAN_LIMIT {
                break;
            }
            self.spans.remove(&k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_791_400_000_000;

    fn use_(id: &str, at: &str) -> Signal {
        Signal::ToolUse { id: id.into(), name: "Bash".into(), at: Some(at.into()), codex: false, device: false, touch: None, ask: None }
    }

    fn result(id: &str, at: &str, err: bool) -> Signal {
        Signal::ToolResult { tool_use_id: id.into(), is_error: err, async_launched: false, at: Some(at.into()), port_conflict: None }
    }

    #[test]
    fn opens_closes_and_interrupts_like_node() {
        let mut log = ToolSpanLog::new("t1");
        let a = log.apply(&[use_("u1", "2026-10-07T10:00:00.000Z"), use_("u2", "2026-10-07T10:00:01.000Z")], "main", T0);
        assert_eq!(a.len(), 2);
        let b = log.apply(&[result("u1", "2026-10-07T09:59:00.000Z", true)], "main", T0);
        assert_eq!(b[0].end.as_deref(), Some("2026-10-07T10:00:00.000Z"));
        assert_eq!(b[0].outcome, Some("error"));
        assert!(log.apply(&[result("u1", "2026-10-07T10:01:00.000Z", false)], "main", T0).is_empty());
        assert!(log.apply(&[Signal::TurnEnd { at: Some("2026-10-07T10:02:00.000Z".into()) }], "other", T0).is_empty());
        let c = log.apply(&[Signal::TurnEnd { at: Some("2026-10-07T10:02:00.000Z".into()) }], "main", T0);
        assert_eq!(c[0].outcome, Some("interrupted"));
        assert!(log.apply(&[use_("bad", "not a date")], "main", T0).is_empty());
    }

    #[test]
    fn prunes_old_and_overflowing_spans() {
        let mut log = ToolSpanLog::new("t1");
        let now = parse_iso("2026-10-07T12:00:00.000Z").unwrap();
        log.apply(&[use_("old", "2026-10-07T09:00:00.000Z"), result("old", "2026-10-07T09:00:01.000Z", false)], "m", now);
        log.apply(&[use_("open", "2026-10-07T09:00:00.000Z")], "m", now);
        assert_eq!(log.list(None, now).iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["open"]);
        let mut sig = Vec::new();
        for i in 0..(SPAN_LIMIT + PRUNE_SLACK + 1) {
            sig.push(use_(&format!("s{i}"), "2026-10-07T11:00:00.000Z"));
            sig.push(result(&format!("s{i}"), "2026-10-07T11:00:01.000Z", false));
        }
        log.apply(&sig, "m", now);
        assert_eq!(log.len(), SPAN_LIMIT);
        assert_eq!(log.close_open("2026-10-07T12:00:00.000Z", Some("m")).len(), 1);
    }
}
