use super::extract::Meta;
use super::meta::read_session_meta;
use super::tail::{Records, RolloutTailer, CHUNK_BYTES, MAX_LINE};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const MAX_HANDLES: usize = 4096;
pub const READ_BUDGET: u64 = 4 * 1024 * 1024;

#[derive(Default)]
pub struct Rollouts {
    next: u64,
    open: HashMap<u64, RolloutTailer>,
}

fn pos(m: &Map<String, Value>, k: &str) -> Option<f64> {
    m.get(k).and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0)
}

fn handle_of(m: &Map<String, Value>) -> Option<u64> {
    m.get("handle").and_then(Value::as_u64)
}

fn file_of(m: &Map<String, Value>) -> Result<&str, &'static str> {
    m.get("file").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("file")
}

pub fn records_json(records: &Records) -> Value {
    Value::Array(records.iter().map(|r| Value::Array(r.iter().map(|s| s.to_json()).collect())).collect())
}

impl Rollouts {
    pub fn len(&self) -> usize {
        self.open.len()
    }

    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }

    fn open(&mut self, m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
        let file = file_of(m)?;
        if self.open.len() >= MAX_HANDLES {
            return Err("handles");
        }
        let from_end = pos(m, "fromEnd").map(|v| v as u64);
        let chunk = pos(m, "chunkBytes").map_or(CHUNK_BYTES, |v| (v as usize).clamp(1, 16 * CHUNK_BYTES));
        let max_line = pos(m, "maxLine").map_or(MAX_LINE, |v| v as usize);
        self.next += 1;
        self.open.insert(self.next, RolloutTailer::new(PathBuf::from(file), from_end, chunk, max_line));
        Ok([("handle".to_owned(), Value::from(self.next))].into_iter().collect())
    }

    fn read(&mut self, m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
        let t = handle_of(m).and_then(|h| self.open.get_mut(&h)).ok_or("handle")?;
        let budget = pos(m, "maxBytes").map_or(READ_BUDGET, |v| (v as u64).max(1));
        let mut records = Records::new();
        let more = t.read_some(budget, &mut records);
        let mut out = Map::new();
        out.insert("records".into(), records_json(&records));
        out.insert("offset".into(), Value::from(t.offset()));
        if let Some((at, skipping)) = t.resume_point() {
            out.insert("resumeAt".into(), Value::from(at));
            if skipping {
                out.insert("skipping".into(), Value::Bool(true));
            }
        }
        if more {
            out.insert("more".into(), Value::Bool(true));
        }
        Ok(out)
    }

    fn meta(m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
        let meta = read_session_meta(Path::new(file_of(m)?));
        Ok([("meta".to_owned(), meta.as_ref().map_or(Value::Null, Meta::to_json))].into_iter().collect())
    }

    pub fn handle(&mut self, op: &str, m: &Map<String, Value>) -> Option<Result<Map<String, Value>, &'static str>> {
        Some(match op {
            "codex.open" => self.open(m),
            "codex.read" => self.read(m),
            "codex.meta" => Self::meta(m),
            "codex.close" => match handle_of(m).and_then(|h| self.open.remove(&h)) {
                Some(_) => Ok(Map::new()),
                None => Err("handle"),
            },
            _ => return None,
        })
    }
}
