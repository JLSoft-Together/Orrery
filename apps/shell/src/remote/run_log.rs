use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use serde_json::{json, Value};

use super::super::push_content::mask_text;
use crate::state::lock;

pub const MAX_LINES: usize = 2000;
pub const MAX_SESSIONS: usize = 24;
pub const TEXT_MAX: usize = 1000;
pub const READ_MAX: usize = 300;
const BYTES_MAX: usize = 700_000;
const ID_MAX: usize = 64;
const TAG_MAX: usize = 64;
const STREAMS: [&str; 3] = ["build", "app", "logcat"];
const LEVELS: [&str; 6] = ["V", "D", "I", "W", "E", "F"];

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    n: u64,
    stream: &'static str,
    text: String,
    level: Option<&'static str>,
    tag: Option<String>,
    at: i64,
}

#[derive(Default)]
struct Buf {
    lines: VecDeque<Entry>,
    next: u64,
    touched: u64,
}

#[derive(Default)]
struct Store {
    map: HashMap<String, Buf>,
    tick: u64,
}

#[derive(Default)]
pub struct RunLog {
    store: Mutex<Store>,
}

static GLOBAL: LazyLock<RunLog> = LazyLock::new(RunLog::default);

pub fn global() -> &'static RunLog {
    &GLOBAL
}

pub fn mask_log(text: &str) -> String {
    mask_text(text)
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= ID_MAX && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn flat(text: &str, mask: &dyn Fn(&str) -> String) -> String {
    let kept: String = text.chars().filter(|c| !c.is_control() || *c == '\t').collect();
    mask(&kept).chars().take(TEXT_MAX).collect()
}

fn entry_of(line: &Value, mask: &dyn Fn(&str) -> String) -> Option<Entry> {
    let stream = STREAMS.into_iter().find(|s| line["stream"].as_str() == Some(s))?;
    let text = flat(line["text"].as_str()?, mask);
    let level = LEVELS.into_iter().find(|l| line["level"].as_str() == Some(l));
    let tag = line["tag"].as_str().map(|t| flat(t, mask).chars().take(TAG_MAX).collect::<String>()).filter(|t| !t.is_empty());
    Some(Entry { n: 0, stream, text, level, tag, at: line["at"].as_i64().unwrap_or(0) })
}

impl RunLog {
    pub fn record(&self, payload: &Value, mask: &dyn Fn(&str) -> String) -> usize {
        if payload["type"].as_str() != Some("log") {
            return 0;
        }
        let Some(id) = payload["sessionId"].as_str().filter(|i| valid_id(i)) else { return 0 };
        let entries: Vec<Entry> = payload["lines"].as_array().into_iter().flatten().filter_map(|l| entry_of(l, mask)).collect();
        if entries.is_empty() {
            return 0;
        }
        let mut st = lock(&self.store);
        st.tick += 1;
        let tick = st.tick;
        if st.map.len() >= MAX_SESSIONS && !st.map.contains_key(id) {
            let oldest = st.map.iter().min_by_key(|(_, b)| b.touched).map(|(k, _)| k.clone());
            if let Some(k) = oldest {
                st.map.remove(&k);
            }
        }
        let buf = st.map.entry(id.to_owned()).or_default();
        buf.touched = tick;
        let count = entries.len();
        for mut e in entries {
            buf.next += 1;
            e.n = buf.next;
            if buf.lines.len() >= MAX_LINES {
                buf.lines.pop_front();
            }
            buf.lines.push_back(e);
        }
        count
    }

    pub fn forget(&self, session: &str) {
        lock(&self.store).map.remove(session);
    }

    pub fn read(&self, session: &str, since: Option<u64>) -> Value {
        let st = lock(&self.store);
        let Some(buf) = st.map.get(session) else { return json!({ "known": false, "lines": [], "last": 0, "gap": false, "more": false }) };
        let first = buf.lines.front().map_or(0, |e| e.n);
        let newer: Vec<&Entry> = buf.lines.iter().filter(|e| since.is_none_or(|s| e.n > s)).collect();
        let (picked, more) = match since {
            None => (newer.iter().rev().take(READ_MAX).rev().copied().collect::<Vec<_>>(), false),
            Some(_) => (newer.iter().take(READ_MAX).copied().collect::<Vec<_>>(), newer.len() > READ_MAX),
        };
        let (picked, cut) = within_bytes(picked, since.is_none());
        let more = more || (since.is_some() && cut);
        let gap = match since {
            None => picked.first().is_some_and(|e| e.n > first),
            Some(s) => first > s.saturating_add(1) && first > 0,
        };
        let last = picked.last().map_or_else(|| since.unwrap_or(buf.next).min(buf.next), |e| e.n);
        let lines: Vec<Value> = picked.iter().map(|e| line_json(e)).collect();
        json!({ "known": true, "lines": lines, "last": last, "gap": gap, "more": more })
    }
}

fn within_bytes(picked: Vec<&Entry>, newest_first: bool) -> (Vec<&Entry>, bool) {
    let total = picked.len();
    let mut order = picked;
    if newest_first {
        order.reverse();
    }
    let mut used = 0usize;
    let mut take = 0usize;
    for e in &order {
        used += line_json(e).to_string().len() + 1;
        if used > BYTES_MAX && take > 0 {
            break;
        }
        take += 1;
    }
    order.truncate(take);
    if newest_first {
        order.reverse();
    }
    (order, take < total)
}

fn line_json(e: &Entry) -> Value {
    let mut out = json!({ "n": e.n, "stream": e.stream, "text": e.text, "at": e.at });
    if let Some(l) = e.level {
        out["level"] = Value::from(l);
    }
    if let Some(t) = &e.tag {
        out["tag"] = Value::from(t.clone());
    }
    out
}

pub fn record_event(payload: &Value) {
    global().record(payload, &mask_log);
}

pub fn reply(args: &Value) -> Value {
    let req = args.get(0).unwrap_or(&Value::Null);
    let Some(session) = req["session"].as_str().filter(|s| valid_id(s)) else { return json!({ "error": "internal" }) };
    json!({ "raw": global().read(session, req["since"].as_u64()) })
}

#[cfg(test)]
#[path = "run_log_tests.rs"]
mod tests;
