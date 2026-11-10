use super::outbox::outbox_line;
use crate::jsstr::js_trim;
use serde_json::{json, Map, Value};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const WINDOW: u64 = 1024 * 1024;
const MAX_LINE: usize = 256 * 1024;

fn pos(m: &Map<String, Value>, k: &str) -> Option<u64> {
    m.get(k)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| v as u64)
}

fn flag(m: &Map<String, Value>, k: &str) -> bool {
    m.get(k).and_then(Value::as_bool).unwrap_or(false)
}

fn missing() -> Map<String, Value> {
    [("missing".to_owned(), json!(true))].into_iter().collect()
}

fn reply(records: Vec<Value>, offset: u64, skipping: bool, more: bool) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("records".into(), Value::Array(records));
    out.insert("offset".into(), json!(offset));
    out.insert("skipping".into(), json!(skipping));
    out.insert("started".into(), json!(true));
    if more {
        out.insert("more".into(), json!(true));
    }
    out
}

fn line_record(line: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(line);
    let t = js_trim(&text);
    if t.is_empty() {
        return None;
    }
    outbox_line(&serde_json::from_str::<Value>(t).ok()?)
}

pub fn tail(m: &Map<String, Value>) -> Result<Map<String, Value>, &'static str> {
    let file = m
        .get("file")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("file")?;
    let max_line = pos(m, "maxLine").map_or(MAX_LINE, |v| (v as usize).clamp(1, 2 * MAX_LINE));
    let mut offset = pos(m, "offset").unwrap_or(0);
    let mut skipping = flag(m, "skipping");
    let Ok(mut f) = File::open(file) else {
        return Ok(missing());
    };
    let Ok(size) = f.metadata().map(|x| x.len()) else {
        return Ok(missing());
    };
    if !flag(m, "started") {
        if let Some(from) = pos(m, "fromEnd").filter(|from| size > *from) {
            offset = size - from;
            skipping = true;
        }
    } else if size < offset {
        offset = 0;
        skipping = false;
    }
    if size <= offset {
        return Ok(reply(Vec::new(), offset, skipping, false));
    }
    let want = (size - offset).min(WINDOW);
    let mut buf = Vec::with_capacity(want as usize);
    if f.seek(SeekFrom::Start(offset)).is_err() || f.take(want).read_to_end(&mut buf).is_err() {
        return Ok(reply(Vec::new(), offset, skipping, false));
    }
    let end = offset + buf.len() as u64;
    let more = end < size;
    let mut from = 0;
    if skipping {
        let Some(i) = memchr::memchr(b'\n', &buf) else {
            return Ok(reply(Vec::new(), end, true, more));
        };
        skipping = false;
        from = i + 1;
    }
    let mut records = Vec::new();
    while let Some(i) = memchr::memchr(b'\n', &buf[from..]) {
        let line = &buf[from..from + i];
        from += i + 1;
        if line.len() <= max_line {
            records.extend(line_record(line));
        }
    }
    let rest = buf.len() - from;
    if rest > max_line {
        return Ok(reply(records, end, true, more));
    }
    Ok(reply(records, end - rest as u64, skipping, more))
}
