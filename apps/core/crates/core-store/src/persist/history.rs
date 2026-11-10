use std::path::PathBuf;

use serde_json::{Map, Value};

use super::json_file::JsonFile;
use super::localtime::Zone;
use super::{system_clock, Clock};
use crate::extra::SessionRecord;
use crate::num::JsNum;

pub const HISTORY_LIMIT: usize = 1000;

fn num(o: &Map<String, Value>, key: &str) -> Option<f64> {
    o.get(key).and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0)
}

fn text(o: &Map<String, Value>, key: &str) -> Option<String> {
    match o.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn parse_record(v: &Value, zone: Zone) -> Option<SessionRecord> {
    let Value::Object(r) = v else { return None };
    let (id, title, ended_at) = (text(r, "id")?, text(r, "title")?, text(r, "endedAt")?);
    zone.parse_iso(&ended_at)?;
    Some(SessionRecord {
        id,
        title,
        ended_at,
        input_tokens: num(r, "inputTokens").unwrap_or(0.0) as u64,
        output_tokens: num(r, "outputTokens").unwrap_or(0.0) as u64,
        cache_read_tokens: num(r, "cacheReadTokens").unwrap_or(0.0) as u64,
        subagents: num(r, "subagents").unwrap_or(0.0) as u64,
        session_id: text(r, "sessionId"),
        cwd: text(r, "cwd"),
        git_branch: text(r, "gitBranch"),
        model: text(r, "model"),
        started_at: text(r, "startedAt"),
        compactions: num(r, "compactions").map(|n| n as u64),
        cost_usd: num(r, "costUsd").map(JsNum),
        working_ms: num(r, "workingMs").map(JsNum),
        waiting_ms: num(r, "waitingMs").map(JsNum),
    })
}

pub struct SessionHistory {
    records: Vec<SessionRecord>,
    file: Option<JsonFile>,
    zone: Zone,
    clock: Clock,
}

impl SessionHistory {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self::with_clock(file, Zone::Local, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, zone: Zone, clock: Clock) -> Self {
        Self { records: Vec::new(), file: file.map(|f| JsonFile::new(f, 10_000)), zone, clock }
    }

    pub fn load(&mut self) {
        let Some(Value::Array(raw)) = self.file.as_ref().and_then(JsonFile::read) else { return };
        self.records = raw.iter().filter_map(|v| parse_record(v, self.zone)).take(HISTORY_LIMIT).collect();
    }

    pub fn add(&mut self, record: SessionRecord) {
        self.records.retain(|r| r.id != record.id);
        self.records.insert(0, record);
        self.records.truncate(HISTORY_LIMIT);
        let now = (self.clock)();
        if let Some(f) = &mut self.file {
            f.schedule(now);
        }
    }

    pub fn list(&self) -> Vec<SessionRecord> {
        self.records.clone()
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let records = &self.records;
        self.file.as_mut().is_some_and(|f| f.poll(now, || serde_json::to_value(records).unwrap_or(Value::Null)))
    }

    pub fn save(&mut self) -> bool {
        let records = &self.records;
        self.file.as_mut().is_some_and(|f| f.flush(|| serde_json::to_value(records).unwrap_or(Value::Null)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use serde_json::json;
    use std::fs;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    fn record(id: &str) -> SessionRecord {
        serde_json::from_value(json!({ "id": id, "title": id, "endedAt": "2026-10-08T08:00:00.000Z", "inputTokens": 1, "outputTokens": 2, "cacheReadTokens": 3, "subagents": 0 })).unwrap()
    }

    #[test]
    fn newest_first_deduped_capped_and_saved_after_the_delay() {
        let dir = temp_dir("history");
        let file = dir.join("history.json");
        let t = Arc::new(AtomicI64::new(1000));
        let c = t.clone();
        let mut h = SessionHistory::with_clock(Some(file.clone()), Zone::Fixed(0), Box::new(move || c.load(Ordering::SeqCst)));
        h.add(record("a"));
        h.add(record("b"));
        h.add(record("a"));
        assert_eq!(h.list().iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(h.next_flush_at(), Some(11_000));
        assert!(!h.poll(10_999));
        assert!(h.poll(11_000));
        let saved: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(saved.as_array().unwrap().len(), 2);
        for i in 0..HISTORY_LIMIT + 5 {
            h.add(record(&format!("r{i}")));
        }
        assert_eq!(h.list().len(), HISTORY_LIMIT);
        assert!(h.save());
        let mut again = SessionHistory::with_clock(Some(file), Zone::Fixed(0), Box::new(|| 0));
        again.load();
        assert_eq!(again.list(), h.list());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_validates_rows_like_the_node_parser() {
        let dir = temp_dir("history-bad");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("history.json");
        fs::write(
            &file,
            json!([
                { "id": "ok", "title": "t", "endedAt": "2026-10-08T08:00:00.000Z", "inputTokens": -5, "outputTokens": "x", "costUsd": 1.5, "model": 4 },
                { "id": "bad-date", "title": "t", "endedAt": "soon" },
                { "id": 1, "title": "t", "endedAt": "2026-10-08" },
                null
            ])
            .to_string(),
        )
        .unwrap();
        let mut h = SessionHistory::with_clock(Some(file), Zone::Fixed(0), Box::new(|| 0));
        h.load();
        let list = h.list();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].input_tokens, list[0].output_tokens, list[0].model.clone()), (0, 0, None));
        assert_eq!(list[0].cost_usd, Some(JsNum(1.5)));
        let _ = fs::remove_dir_all(&dir);
    }
}
