use super::classify::{classify_line, timestamp_in, LineClass, HEAD_BYTES};
use super::extract::{compacted_signals, extract_codex_signals, CodexSignal};
use super::value::parse_record;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub const CHUNK_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_LINE: usize = 32 * 1024 * 1024;

pub type Records = Vec<Vec<CodexSignal>>;

pub struct RolloutTailer {
    pub file: PathBuf,
    from_end: Option<u64>,
    chunk_bytes: usize,
    max_line: usize,
    offset: u64,
    started: bool,
    remainder: Vec<u8>,
    skipping: bool,
    target: Option<u64>,
}

fn head(line: &[u8]) -> &[u8] {
    &line[..line.len().min(HEAD_BYTES)]
}

fn take(line: &[u8], out: &mut Records) {
    if line.is_empty() {
        return;
    }
    match classify_line(head(line)) {
        LineClass::Skip => {}
        LineClass::Compacted => out.push(compacted_signals(timestamp_in(head(line)))),
        LineClass::Parse => {
            if let Some(rec) = parse_record(&String::from_utf8_lossy(line)) {
                out.push(extract_codex_signals(&rec));
            }
        }
    }
}

impl RolloutTailer {
    pub fn new(file: PathBuf, from_end: Option<u64>, chunk_bytes: usize, max_line: usize) -> Self {
        RolloutTailer { file, from_end, chunk_bytes: chunk_bytes.max(1), max_line, offset: 0, started: false, remainder: Vec::new(), skipping: false, target: None }
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn resume_point(&self) -> Option<(u64, bool)> {
        if !self.started {
            return None;
        }
        Some(if self.skipping { (self.offset, true) } else { (self.offset - self.remainder.len() as u64, false) })
    }

    pub fn read_all(&mut self, out: &mut Records) {
        while self.read_some(u64::MAX, out) {}
    }

    pub fn read_some(&mut self, budget: u64, out: &mut Records) -> bool {
        let Some((mut f, now)) = File::open(&self.file).ok().and_then(|f| f.metadata().ok().map(|m| (f, m.len()))) else {
            self.target = None;
            return false;
        };
        let size = match self.target.take() {
            Some(t) if now >= self.offset => t,
            _ => now,
        };
        if !self.started {
            self.started = true;
            self.offset = self.from_end.map_or(0, |from| size.saturating_sub(from));
            self.skipping = self.offset > 0;
        } else if size < self.offset {
            self.offset = 0;
            self.remainder.clear();
            self.skipping = false;
        }
        if size <= self.offset {
            return false;
        }
        let mut used = 0u64;
        let mut chunk = vec![0u8; (size - self.offset).min(self.chunk_bytes as u64).min(budget) as usize];
        while self.offset < size && used < budget {
            let want = (size - self.offset).min(chunk.len() as u64).min(budget - used) as usize;
            if f.seek(SeekFrom::Start(self.offset)).is_err() {
                break;
            }
            let n = match f.read(&mut chunk[..want]) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            self.offset += n as u64;
            used += n as u64;
            self.consume(&chunk[..n], out);
        }
        let more = self.offset < size && used >= budget;
        if more {
            self.target = Some(size);
        }
        more
    }

    fn consume(&mut self, chunk: &[u8], out: &mut Records) {
        let mut start = 0;
        if self.skipping {
            let Some(nl) = memchr::memchr(b'\n', chunk) else { return };
            self.skipping = false;
            start = nl + 1;
        }
        let joined;
        let data: &[u8] = if self.remainder.is_empty() {
            &chunk[start..]
        } else {
            let mut j = std::mem::take(&mut self.remainder);
            j.extend_from_slice(&chunk[start..]);
            joined = j;
            &joined
        };
        let mut from = 0;
        while let Some(i) = memchr::memchr(b'\n', &data[from..]) {
            take(&data[from..from + i], out);
            from += i + 1;
        }
        let rest = &data[from..];
        let kind = if rest.len() >= HEAD_BYTES { classify_line(&rest[..HEAD_BYTES]) } else { LineClass::Parse };
        match kind {
            LineClass::Compacted => {
                take(&rest[..HEAD_BYTES], out);
                self.skipping = true;
            }
            LineClass::Skip => self.skipping = true,
            LineClass::Parse if rest.len() > self.max_line => self.skipping = true,
            LineClass::Parse => self.remainder = rest.to_vec(),
        }
    }
}
