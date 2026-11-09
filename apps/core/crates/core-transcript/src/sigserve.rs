use crate::extract::{extract_signals, MAX_ENUM};
use crate::jsonline::deep_signals;
use crate::jsstr::js_slice;
use crate::signal::Signal;
use crate::spans::ToolSpanLog;
use crate::state::TranscriptState;
use crate::tail::{lite_filter, signal_line_filter, Item, JsonlTailer, TailOptions, CHUNK_BYTES};
use crate::time::now_ms;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const MAX_HANDLES: usize = 4096;
pub const MAX_IDS: usize = 20_000;
pub const READ_BUDGET: u64 = 4 * 1024 * 1024;
pub const MAX_LINE: usize = 64 * 1024 * 1024;

pub struct Transcript {
    pub tailer: JsonlTailer,
    pub state: TranscriptState,
    lite: Option<HashSet<String>>,
    spans: Option<(ToolSpanLog, String)>,
}

impl Transcript {
    pub fn new(file: PathBuf, lite: Option<HashSet<String>>, spans: Option<(String, String)>, chunk: usize, options: TailOptions) -> Self {
        Transcript { tailer: JsonlTailer::new(file, chunk, options), state: TranscriptState::default(), lite, spans: spans.map(|(t, a)| (ToolSpanLog::new(&t), a)) }
    }

    pub fn set_tool_use_ids(&mut self, ids: HashSet<String>) {
        if let Some(l) = self.lite.as_mut() {
            *l = ids;
        }
    }

    pub fn read(&mut self, now: i64) -> (Vec<Signal>, Vec<Value>) {
        let (signals, spans, _) = self.read_some(now, u64::MAX);
        (signals, spans)
    }

    pub fn read_some(&mut self, now: i64, budget: u64) -> (Vec<Signal>, Vec<Value>, bool) {
        let mut all = Vec::new();
        let mut changed = Vec::new();
        let Transcript { tailer, state, lite, spans } = self;
        let lite = lite.as_ref();
        let mut filter = |line: &[u8], end: u64, size: u64| match lite {
            Some(ids) => lite_filter(line, end, size, ids),
            None => signal_line_filter(line),
        };
        let mut on = |item: Item| {
            let signals = match item {
                Item::Record(v) => extract_signals(&v),
                Item::Deep(text) => deep_signals(text),
                Item::Branch(b) => vec![Signal::Branch { branch: js_slice(&b, MAX_ENUM).to_owned() }],
            };
            if let Some((log, agent)) = spans.as_mut() {
                changed.extend(log.apply(&signals, agent, now).iter().map(|s| s.to_json()));
            }
            state.apply(&signals);
            all.extend(signals);
        };
        let more = tailer.read_some(budget, &mut filter, &mut on);
        (all, changed, more)
    }
}

fn ids_of(v: Option<&Value>) -> Result<Option<HashSet<String>>, &'static str> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() <= MAX_IDS => Ok(Some(a.iter().filter_map(Value::as_str).map(str::to_owned).collect())),
        Some(_) => Err("toolUseIds"),
    }
}

fn pos(m: &Map<String, Value>, k: &str) -> Option<f64> {
    m.get(k).and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0)
}

#[derive(Default)]
pub struct Handles {
    next: u64,
    open: HashMap<u64, Transcript>,
}

fn handle_of(m: &Map<String, Value>) -> Option<u64> {
    m.get("handle").and_then(Value::as_u64)
}

impl Handles {
    fn open(&mut self, m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
        let file = m.get("file").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("file")?;
        if self.open.len() >= MAX_HANDLES {
            return Err("handles");
        }
        let lite = match m.get("lite") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => None,
            Some(Value::Bool(true)) => Some(ids_of(m.get("toolUseIds"))?.unwrap_or_default()),
            Some(_) => return Err("lite"),
        };
        let spans = match m.get("spans") {
            None | Some(Value::Null) => None,
            Some(Value::Object(s)) => {
                let t = s.get("terminalId").and_then(Value::as_str).ok_or("spans")?;
                let a = s.get("agentId").and_then(Value::as_str).ok_or("spans")?;
                Some((t.to_owned(), a.to_owned()))
            }
            Some(_) => return Err("spans"),
        };
        let chunk = pos(m, "chunkBytes").map_or(CHUNK_BYTES, |v| (v as usize).clamp(1, 64 * CHUNK_BYTES));
        let options = TailOptions { from_end: pos(m, "fromEnd").map(|v| v as u64), max_line: Some(pos(m, "maxLine").map_or(MAX_LINE, |v| v as usize)) };
        self.next += 1;
        self.open.insert(self.next, Transcript::new(PathBuf::from(file), lite, spans, chunk, options));
        Ok([("handle".to_owned(), Value::from(self.next))].into_iter().collect())
    }

    fn read(&mut self, m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
        let t = handle_of(m).and_then(|h| self.open.get_mut(&h)).ok_or("handle")?;
        if let Some(ids) = ids_of(m.get("toolUseIds"))? {
            t.set_tool_use_ids(ids);
        }
        let budget = pos(m, "maxBytes").map_or(READ_BUDGET, |v| (v as u64).max(1));
        let now = m.get("nowMs").and_then(Value::as_f64).filter(|v| v.is_finite()).map_or_else(now_ms, |v| v as i64);
        let (signals, spans, more) = t.read_some(now, budget);
        let mut out = Map::new();
        out.insert("signals".into(), Value::Array(signals.iter().map(Signal::to_json).collect()));
        out.insert("state".into(), t.state.to_json());
        if t.spans.is_some() {
            out.insert("spans".into(), Value::Array(spans));
        }
        out.insert("offset".into(), Value::from(t.tailer.offset()));
        if more {
            out.insert("more".into(), Value::Bool(true));
        }
        Ok(out)
    }

    pub fn handle(&mut self, op: &str, m: &Map<String, Value>) -> Option<Result<Map<String, Value>, &'static str>> {
        Some(match op {
            "signals.open" => self.open(m),
            "signals.read" => self.read(m),
            "signals.close" => match handle_of(m).and_then(|h| self.open.remove(&h)) {
                Some(_) => Ok(Map::new()),
                None => Err("handle"),
            },
            _ => return None,
        })
    }
}
