use crate::jsonline::{parse_line, Parsed};
use crate::jsstr::js_trim;
use memchr::memmem;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub const CHUNK_BYTES: usize = 1024 * 1024;
pub const LITE_TAIL_BYTES: u64 = 1024 * 1024;

pub enum Verdict {
    Parse,
    Drop,
    Branch(String),
}

pub enum Item {
    Record(serde_json::Value),
    Branch(String),
    Deep(String),
}

#[derive(Default, Clone, Copy)]
pub struct TailOptions {
    pub from_end: Option<u64>,
    pub max_line: Option<usize>,
}

pub struct JsonlTailer {
    pub file: PathBuf,
    chunk_bytes: usize,
    options: TailOptions,
    offset: u64,
    remainder: Vec<u8>,
    started: bool,
    skipping: bool,
    target: Option<u64>,
}

const TYPE_KEY: &[u8] = b"\"type\":\"";
const BRANCH_KEY: &[u8] = b"\"gitBranch\":\"";
const SIGNAL_TYPES: [&[u8]; 5] = [b"assistant\"", b"user\"", b"system\"", b"permission-mode\"", b"queue-operation\""];
const QUEUED_COMMAND_TYPE: [&[u8]; 1] = [b"queued_command\""];
const MESSAGE_KINDS: [&[u8]; 2] = [b"\"kind\":\"coordinator\"", b"\"kind\":\"peer\""];
const USER_TYPE: &[u8] = b"\"type\":\"user\"";
const NOTIFICATION_TAG: &[u8] = b"<task-notification>";
const TOOL_USE_ID_KEY: &[u8] = b"\"tool_use_id\":\"";

fn has(line: &[u8], needle: &[u8]) -> bool {
    memmem::find(line, needle).is_some()
}

fn has_type_value(line: &[u8], values: &[&[u8]]) -> bool {
    memmem::find_iter(line, TYPE_KEY).any(|at| {
        let from = at + TYPE_KEY.len();
        values.iter().any(|v| line.get(from..from + v.len()) == Some(*v))
    })
}

fn has_message_kind(line: &[u8]) -> bool {
    MESSAGE_KINDS.iter().any(|k| has(line, k))
}

fn branch_in(window: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(window);
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if c == '"' {
            return (n >= 1).then(|| text[..i].to_owned());
        }
        if c == '\\' {
            return None;
        }
        n += c.len_utf16();
        if n > 200 {
            return None;
        }
    }
    None
}

pub fn signal_line_filter(line: &[u8]) -> Verdict {
    if has_type_value(line, &SIGNAL_TYPES) || (has_message_kind(line) && has_type_value(line, &QUEUED_COMMAND_TYPE)) {
        return Verdict::Parse;
    }
    let Some(at) = memmem::rfind(line, BRANCH_KEY) else { return Verdict::Drop };
    let from = at + BRANCH_KEY.len();
    match branch_in(&line[from..(from + 202).min(line.len())]) {
        Some(b) => Verdict::Branch(b),
        None => Verdict::Parse,
    }
}

pub fn tool_use_ids_of(line: &[u8]) -> Vec<String> {
    let mut ids = Vec::new();
    let mut at = memmem::find(line, TOOL_USE_ID_KEY);
    while let Some(i) = at {
        let from = i + TOOL_USE_ID_KEY.len();
        let Some(len) = memchr::memchr(b'"', &line[from..]) else { break };
        if len > 200 {
            break;
        }
        ids.push(line[from..from + len].iter().map(|&b| b as char).collect());
        let end = from + len;
        at = memmem::find(&line[end..], TOOL_USE_ID_KEY).map(|j| end + j);
    }
    ids
}

pub fn lite_filter(line: &[u8], end: u64, size: u64, agent_tool_uses: &HashSet<String>) -> Verdict {
    if end as i128 > size as i128 - LITE_TAIL_BYTES as i128 || !has(line, USER_TYPE) {
        return signal_line_filter(line);
    }
    if has(line, NOTIFICATION_TAG) || has_message_kind(line) {
        return Verdict::Parse;
    }
    if tool_use_ids_of(line).iter().any(|id| agent_tool_uses.contains(id)) {
        return Verdict::Parse;
    }
    Verdict::Drop
}

impl JsonlTailer {
    pub fn new(file: PathBuf, chunk_bytes: usize, options: TailOptions) -> Self {
        JsonlTailer { file, chunk_bytes: chunk_bytes.max(1), options, offset: 0, remainder: Vec::new(), started: false, skipping: false, target: None }
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn read_each(&mut self, filter: &mut dyn FnMut(&[u8], u64, u64) -> Verdict, on: &mut dyn FnMut(Item)) {
        while self.read_some(u64::MAX, filter, on) {}
    }

    pub fn read_some(&mut self, budget: u64, filter: &mut dyn FnMut(&[u8], u64, u64) -> Verdict, on: &mut dyn FnMut(Item)) -> bool {
        let Ok(now) = std::fs::metadata(&self.file).map(|m| m.len()) else {
            self.target = None;
            return false;
        };
        let size = match self.target.take() {
            Some(t) if now >= self.offset => t,
            _ => now,
        };
        if !self.started {
            self.started = true;
            if let Some(from) = self.options.from_end.filter(|f| size > *f) {
                self.offset = size - from;
                self.skipping = true;
            }
        } else if size < self.offset {
            self.offset = 0;
            self.remainder.clear();
            self.skipping = false;
        }
        if size == self.offset {
            return false;
        }
        let Ok(mut f) = File::open(&self.file) else { return false };
        let mut used = 0u64;
        let mut chunk = vec![0u8; (size - self.offset).min(self.chunk_bytes as u64) as usize];
        while self.offset < size && used < budget {
            let want = (size - self.offset).min(chunk.len() as u64).min(budget - used) as usize;
            if f.seek(SeekFrom::Start(self.offset)).is_err() {
                break;
            }
            let n = match f.read(&mut chunk[..want]) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let base = self.offset - self.remainder.len() as u64;
            self.offset += n as u64;
            used += n as u64;
            self.consume(&chunk[..n], base, size, filter, on);
        }
        let more = self.offset < size && used >= budget;
        if more {
            self.target = Some(size);
        }
        more
    }

    fn consume(&mut self, chunk: &[u8], base: u64, size: u64, filter: &mut dyn FnMut(&[u8], u64, u64) -> Verdict, on: &mut dyn FnMut(Item)) {
        let mut start = 0;
        if self.skipping {
            let Some(nl) = memchr::memchr(b'\n', chunk) else { return };
            self.skipping = false;
            start = nl + 1;
        }
        let mut joined;
        let data: &[u8] = if self.remainder.is_empty() {
            chunk
        } else {
            joined = std::mem::take(&mut self.remainder);
            joined.extend_from_slice(chunk);
            &joined
        };
        let max_line = self.options.max_line.unwrap_or(usize::MAX);
        while let Some(i) = memchr::memchr(b'\n', &data[start..]) {
            let idx = start + i;
            let raw = &data[start..idx];
            start = idx + 1;
            if raw.len() > max_line {
                continue;
            }
            match filter(raw, base + start as u64, size) {
                Verdict::Drop => continue,
                Verdict::Branch(b) => {
                    on(Item::Branch(b));
                    continue;
                }
                Verdict::Parse => {}
            }
            let text = String::from_utf8_lossy(raw);
            let line = js_trim(&text);
            if line.is_empty() {
                continue;
            }
            match parse_line(line) {
                Parsed::Value(v) => on(Item::Record(v)),
                Parsed::Deep(text) => on(Item::Deep(text)),
                Parsed::Bad => {}
            }
        }
        let rest = &data[start..];
        if rest.len() > max_line {
            self.remainder = Vec::new();
            self.skipping = true;
        } else {
            self.remainder = rest.to_vec();
        }
    }
}
