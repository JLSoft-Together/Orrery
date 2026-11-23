use super::{claude, codex, sorted, Agent, Builder, Clean, Item, REPLY_BYTES, SCAN_BYTES};
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const REPLY_MARGIN: usize = 8 * 1024;
const LAST_TAIL: u64 = 1024 * 1024;

pub struct Window {
    pub start: u64,
    pub end: u64,
    pub bytes: Vec<u8>,
    pub from_top: bool,
}

pub fn read_range(path: &Path, start: u64, end: u64) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::with_capacity((end - start) as usize);
    f.take(end - start).read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn window(path: &Path, before: Option<u64>, scan: u64) -> std::io::Result<Window> {
    let size = std::fs::metadata(path)?.len();
    let end = before.map_or(size, |b| b.min(size));
    let start = end.saturating_sub(scan);
    let lead = u64::from(start > 0);
    let bytes = read_range(path, start - lead, end)?;
    let mut skip = 0usize;
    if start > 0 {
        skip = if bytes.first() == Some(&b'\n') { 1 } else { memchr::memchr(b'\n', &bytes).map_or(bytes.len(), |i| i + 1) };
    }
    let mut cut = bytes.len();
    if before.is_none() || end == size {
        cut = memchr::memrchr(b'\n', &bytes).map_or(0, |i| i + 1).max(skip);
    }
    let real_start = start - lead + skip as u64;
    let real_end = start - lead + cut as u64;
    Ok(Window { start: real_start, end: real_end, bytes: bytes[skip..cut].to_vec(), from_top: start == 0 })
}

pub fn lines(w: &Window) -> Vec<(u64, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for i in memchr::memchr_iter(b'\n', &w.bytes) {
        let mut line = &w.bytes[at..i];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        if !line.is_empty() {
            out.push((w.start + at as u64, line));
        }
        at = i + 1;
    }
    out
}

fn feed_all(b: &mut Builder, lines: &[(u64, &[u8])]) {
    for (off, line) in lines {
        if b.wants(line) {
            b.feed(line, *off);
        }
    }
}

pub struct Page {
    pub items: Vec<Value>,
    pub older: Option<u64>,
    pub live: Option<Live>,
}

pub struct Live {
    pub path: PathBuf,
    pub builder: Builder,
    pub offset: u64,
}

pub enum Poll {
    Items(Vec<Item>),
    Reset,
}

fn size_of(v: &Value) -> usize {
    serde_json::to_vec(v).map_or(usize::MAX, |b| b.len() + 1)
}

pub fn page(path: &Path, agent: Agent, root: &str, before: Option<u64>, limit: usize, clean: &dyn Clean, live: bool) -> std::io::Result<Page> {
    let w = window(path, before, SCAN_BYTES)?;
    let all = lines(&w);
    let mut b = Builder::new(agent, root, true);
    feed_all(&mut b, &all);
    let mut segs: Vec<u64> = Vec::new();
    if b.items.iter().any(|i| i.turn.is_none()) || b.turns.is_empty() {
        segs.push(w.start);
    }
    segs.extend(b.turns.iter().copied());
    let items = sorted(std::mem::take(&mut b.items));
    let seg_of = |i: &Item| i.turn.unwrap_or(w.start);
    let budget = REPLY_BYTES - REPLY_MARGIN;
    let mut chosen: Vec<Value> = Vec::new();
    let mut bytes = 0usize;
    let mut older: Option<u64> = None;
    let mut done = false;
    for (k, seg) in segs.iter().enumerate().rev() {
        let group: Vec<&Item> = items.iter().filter(|i| seg_of(i) == *seg).collect();
        let vals: Vec<Value> = group.iter().map(|i| i.to_json(clean)).collect();
        let size: usize = vals.iter().map(size_of).sum();
        if bytes + size > budget {
            if chosen.is_empty() {
                let mut first_seq = None;
                for (v, i) in vals.iter().zip(group.iter()).rev() {
                    let n = size_of(v);
                    if bytes + n > budget {
                        break;
                    }
                    bytes += n;
                    first_seq = Some(i.seq);
                    chosen.push(v.clone());
                }
                chosen.reverse();
                older = Some(first_seq.unwrap_or(*seg));
            }
            done = true;
            break;
        }
        bytes += size;
        let mut next = vals;
        next.extend(chosen);
        chosen = next;
        older = Some(*seg);
        if chosen.len() >= limit {
            if k == 0 && w.from_top {
                older = None;
            }
            done = true;
            break;
        }
    }
    if !done {
        older = if w.from_top { None } else { Some(segs.first().copied().unwrap_or(w.start)) };
    }
    if older == Some(0) {
        older = None;
    }
    let live = if live && before.is_none() {
        let last = segs.last().copied().unwrap_or(w.start);
        let tail: Vec<(u64, &[u8])> = all.iter().filter(|(o, _)| *o >= last).copied().collect();
        let mut lb = Builder::new(agent, root, false);
        feed_all(&mut lb, &tail);
        lb.mark_clean();
        Some(Live { path: path.to_path_buf(), builder: lb, offset: w.end })
    } else {
        None
    };
    Ok(Page { items: chosen, older, live })
}

impl Live {
    pub fn poll(&mut self) -> std::io::Result<Poll> {
        let size = std::fs::metadata(&self.path)?.len();
        if size < self.offset {
            return Ok(Poll::Reset);
        }
        if size == self.offset {
            return Ok(Poll::Items(self.builder.take_dirty()));
        }
        let end = size.min(self.offset + SCAN_BYTES);
        let bytes = read_range(&self.path, self.offset, end)?;
        let Some(cut) = memchr::memrchr(b'\n', &bytes).map(|i| i + 1) else {
            if end - self.offset >= SCAN_BYTES {
                self.offset = end;
            }
            return Ok(Poll::Items(self.builder.take_dirty()));
        };
        let w = Window { start: self.offset, end: self.offset + cut as u64, bytes: bytes[..cut].to_vec(), from_top: false };
        let all = lines(&w);
        feed_all(&mut self.builder, &all);
        self.offset = w.end;
        Ok(Poll::Items(self.builder.take_dirty()))
    }
}

pub fn turn_bounds(path: &Path, agent: Agent, turn: u64) -> std::io::Result<Option<Option<u64>>> {
    let size = std::fs::metadata(path)?.len();
    if turn >= size || (turn > 0 && read_range(path, turn - 1, turn)? != b"\n") {
        return Ok(None);
    }
    let end = size.min(turn + SCAN_BYTES);
    let bytes = read_range(path, turn, end)?;
    let cut = if end == size { bytes.len() } else { memchr::memrchr(b'\n', &bytes).map_or(0, |i| i + 1) };
    let w = Window { start: turn, end: turn + cut as u64, bytes: bytes[..cut].to_vec(), from_top: turn == 0 };
    let all = lines(&w);
    let mut b = Builder::new(agent, "", true);
    feed_all(&mut b, &all);
    if b.turns.first() != Some(&turn) {
        return Ok(None);
    }
    Ok(Some(match b.turns.get(1) {
        Some(next) => Some(*next),
        None if end == size => None,
        None => Some(w.end.max(turn + 1)),
    }))
}

pub fn last_reply(path: &Path, agent: Agent) -> std::io::Result<Option<(String, Option<i64>)>> {
    let w = window(path, None, LAST_TAIL)?;
    let all = lines(&w);
    Ok(match agent {
        Agent::Claude => claude::last_reply(&all),
        Agent::Codex => codex::last_reply(&all),
    })
}
