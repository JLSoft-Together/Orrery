use std::path::PathBuf;

use core_transcript::sidefiles::SideFiles;
use serde_json::{Map, Value};

use super::state::OutboxEvent;

pub struct OutboxTail {
    file: PathBuf,
    from_end: Option<u64>,
    max_line: u64,
    started: bool,
    offset: u64,
    skipping: bool,
}

impl OutboxTail {
    pub fn new(file: PathBuf, from_end: Option<u64>, max_line: u64) -> Self {
        OutboxTail { file, from_end, max_line, started: false, offset: 0, skipping: false }
    }

    pub fn read_each(&mut self, mut on_event: impl FnMut(OutboxEvent)) {
        loop {
            let mut m = Map::new();
            m.insert("file".into(), Value::from(self.file.to_string_lossy().into_owned()));
            m.insert("started".into(), Value::from(self.started));
            m.insert("offset".into(), Value::from(self.offset));
            m.insert("skipping".into(), Value::from(self.skipping));
            if let Some(f) = self.from_end {
                m.insert("fromEnd".into(), Value::from(f));
            }
            m.insert("maxLine".into(), Value::from(self.max_line));
            let Some(Ok(reply)) = SideFiles.handle("sidefiles.outbox", &m) else { return };
            if reply.get("missing") == Some(&Value::Bool(true)) {
                return;
            }
            let (Some(Value::Array(records)), Some(offset)) = (reply.get("records"), reply.get("offset").and_then(Value::as_u64)) else { return };
            self.started = true;
            self.offset = offset;
            self.skipping = reply.get("skipping") == Some(&Value::Bool(true));
            for r in records {
                if let Some(e) = OutboxEvent::from_value(r) {
                    on_event(e);
                }
            }
            if reply.get("more") != Some(&Value::Bool(true)) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tails_new_lines_and_drops_foreign_ones() {
        let dir = std::env::temp_dir().join(format!("ccm-outbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("out.jsonl");
        let line = |v: Value| format!("{v}\n");
        std::fs::write(&file, line(serde_json::json!({ "event": "session_start", "thread_id": "t1", "timestamp": "2026-11-01T10:00:00.000Z", "payload": { "workspace": "/tmp/x" } })) + "garbage\n" + &line(serde_json::json!({ "event": "nope", "thread_id": "t1" }))).unwrap();
        let mut tail = OutboxTail::new(file.clone(), None, 262_144);
        let mut seen = Vec::new();
        tail.read_each(|e| seen.push(e));
        assert_eq!(seen.len(), 1);
        assert_eq!((seen[0].event.as_str(), seen[0].thread_id.as_str(), seen[0].workspace.as_deref()), ("session_start", "t1", Some("/tmp/x")));
        let mut more = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
        std::io::Write::write_all(&mut more, line(serde_json::json!({ "event": "turn_start", "thread_id": "t1" })).as_bytes()).unwrap();
        tail.read_each(|e| seen.push(e));
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].event, "turn_start");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
