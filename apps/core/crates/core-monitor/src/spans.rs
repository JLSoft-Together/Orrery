use core_store::extra::{ToolOutcome, ToolSpan};
use core_transcript::spans::{SPAN_LIMIT, SPAN_RETENTION_MS};
use core_transcript::time::parse_iso;
use indexmap::IndexMap;
use serde_json::Value;

struct Entry {
    span: ToolSpan,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    ended: bool,
}

pub enum SpanEvent {
    Span(ToolSpan),
    Drop(String),
}

pub fn span_event(v: Value) -> Option<SpanEvent> {
    if let Some(id) = v.get("drop").and_then(Value::as_str) {
        return Some(SpanEvent::Drop(id.to_owned()));
    }
    serde_json::from_value(v).ok().map(SpanEvent::Span)
}

#[derive(Default)]
pub struct SpanMirror {
    spans: IndexMap<String, Entry>,
}

impl SpanMirror {
    pub fn clear(&mut self) {
        self.spans.clear();
    }

    pub fn apply(&mut self, e: &SpanEvent) {
        match e {
            SpanEvent::Drop(id) => {
                self.spans.shift_remove(id);
            }
            SpanEvent::Span(s) => {
                let start_ms = parse_iso(&s.start);
                let end_ms = s.end.as_deref().and_then(parse_iso);
                self.spans.insert(s.id.clone(), Entry { span: s.clone(), start_ms, end_ms, ended: s.end.is_some() });
            }
        }
    }

    pub fn all(&self) -> Vec<ToolSpan> {
        self.spans.values().map(|e| e.span.clone()).collect()
    }

    pub fn close_open(&mut self, at: &str) -> Vec<ToolSpan> {
        let Some(t) = parse_iso(at) else { return Vec::new() };
        let mut changed = Vec::new();
        for entry in self.spans.values_mut() {
            if entry.ended {
                continue;
            }
            let early = entry.start_ms.is_some_and(|s| t < s);
            entry.span.end = Some(if early { entry.span.start.clone() } else { at.to_owned() });
            entry.end_ms = if early { entry.start_ms } else { Some(t) };
            entry.ended = true;
            entry.span.outcome = Some(ToolOutcome::Interrupted);
            changed.push(entry.span.clone());
        }
        changed
    }

    pub fn list(&mut self, now: i64, since_ms: Option<f64>) -> Vec<ToolSpan> {
        self.prune(now);
        let mut out: Vec<&Entry> = self.spans.values().filter(|e| !(e.ended && since_ms.is_some_and(|s| e.end_ms.is_some_and(|m| (m as f64) < s)))).collect();
        out.sort_by(|a, b| match (a.start_ms, b.start_ms) {
            (Some(x), Some(y)) => x.cmp(&y),
            _ => std::cmp::Ordering::Equal,
        });
        out.into_iter().map(|e| e.span.clone()).collect()
    }

    fn prune(&mut self, now: i64) {
        let cutoff = now - SPAN_RETENTION_MS;
        self.spans.retain(|_, e| !(e.ended && e.end_ms.is_some_and(|m| m < cutoff)));
        if self.spans.len() <= SPAN_LIMIT {
            return;
        }
        let mut excess = self.spans.len() - SPAN_LIMIT;
        self.spans.retain(|_, e| {
            if excess > 0 && e.ended {
                excess -= 1;
                return false;
            }
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(id: &str, start: &str, end: Option<&str>) -> SpanEvent {
        SpanEvent::Span(ToolSpan { id: id.into(), terminal_id: "t".into(), agent_id: "t:main".into(), tool: "Bash".into(), start: start.into(), end: end.map(Into::into), outcome: end.map(|_| ToolOutcome::Ok) })
    }

    #[test]
    fn mirrors_closes_lists_and_drops() {
        let mut m = SpanMirror::default();
        m.apply(&span("b", "2026-10-07T10:00:02.000Z", None));
        m.apply(&span("a", "2026-10-07T10:00:00.000Z", Some("2026-10-07T10:00:01.000Z")));
        let now = parse_iso("2026-10-07T10:01:00.000Z").unwrap();
        assert_eq!(m.list(now, None).iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        let since = parse_iso("2026-10-07T10:00:01.500Z").unwrap() as f64;
        assert_eq!(m.list(now, Some(since)).len(), 1);
        let closed = m.close_open("2026-10-07T10:00:01.000Z");
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].end.as_deref(), Some("2026-10-07T10:00:02.000Z"));
        assert_eq!(closed[0].outcome, Some(ToolOutcome::Interrupted));
        m.apply(&SpanEvent::Drop("a".into()));
        assert_eq!(m.all().len(), 1);
        assert!(m.list(now + SPAN_RETENTION_MS + 10_000, None).is_empty());
    }
}
