pub mod record;
pub mod sample;
pub mod tree;
pub mod wire;
#[cfg(windows)]
pub mod win;

use crate::index::Index;
use crate::store;
use crate::walk::Control;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tree::Table;
use wire::{write_frame, Frame, FrameError, KIND_ERROR, KIND_INDEX, KIND_PROGRESS, MAX_INDEX};

pub const TIMEOUT: Duration = Duration::from_secs(600);
pub const TICK: Duration = Duration::from_millis(100);
pub const MAX_FIXTURE: u64 = 256 << 20;
const CHUNK_BYTES: usize = 4 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MftError {
    Unsupported,
    Declined,
    Cancelled,
    Timeout,
    Helper(String),
}

impl std::fmt::Display for MftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MftError::Unsupported => f.write_str("unsupported"),
            MftError::Declined => f.write_str("elevation-declined"),
            MftError::Cancelled => f.write_str("cancelled"),
            MftError::Timeout => f.write_str("timeout"),
            MftError::Helper(s) => f.write_str(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub drive: char,
    pub fixture: Option<PathBuf>,
}

pub fn drive_letter(s: &str) -> Option<char> {
    let t = s.trim();
    let t = t.strip_suffix('\\').or_else(|| t.strip_suffix('/')).unwrap_or(t);
    let t = t.strip_suffix(':').unwrap_or(t);
    let mut it = t.chars();
    match (it.next(), it.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Some(c.to_ascii_uppercase()),
        _ => None,
    }
}

pub fn root_name(drive: char) -> String {
    format!("{drive}:\\")
}

pub trait Source {
    fn record_size(&self) -> usize;
    fn total_records(&self) -> u64;
    fn next(&mut self, buf: &mut Vec<u8>) -> std::io::Result<Option<u64>>;
}

pub struct MemorySource {
    bytes: Vec<u8>,
    size: usize,
    at: usize,
}

pub fn record_size_of(first: &[u8]) -> Option<usize> {
    if first.len() < 0x20 || &first[..4] != b"FILE" {
        return None;
    }
    let size = u32::from_le_bytes(first[0x1C..0x20].try_into().ok()?) as usize;
    (size.is_power_of_two() && (1024..=65536).contains(&size)).then_some(size)
}

impl MemorySource {
    pub fn new(bytes: Vec<u8>) -> std::io::Result<Self> {
        let size = record_size_of(&bytes).ok_or_else(|| std::io::Error::other("not an MFT image"))?;
        Ok(MemorySource { bytes, size, at: 0 })
    }
}

impl Source for MemorySource {
    fn record_size(&self) -> usize {
        self.size
    }

    fn total_records(&self) -> u64 {
        (self.bytes.len() / self.size) as u64
    }

    fn next(&mut self, buf: &mut Vec<u8>) -> std::io::Result<Option<u64>> {
        let usable = self.bytes.len() / self.size * self.size;
        if self.at >= usable {
            return Ok(None);
        }
        let end = (self.at + CHUNK_BYTES / self.size * self.size).min(usable);
        buf.clear();
        buf.extend_from_slice(&self.bytes[self.at..end]);
        let first = (self.at / self.size) as u64;
        self.at = end;
        Ok(Some(first))
    }
}

pub fn collect(src: &mut dyn Source, mut tick: impl FnMut(u64, u64) -> std::io::Result<()>) -> std::io::Result<Table> {
    let total = src.total_records();
    let size = src.record_size();
    let mut table = Table::new(total as usize);
    let mut buf = Vec::new();
    let mut last = Instant::now();
    while let Some(first) = src.next(&mut buf)? {
        table.add_chunk(&mut buf, first, size);
        if last.elapsed() >= TICK {
            tick(table.records, total)?;
            last = Instant::now();
        }
    }
    tick(table.records, total)?;
    Ok(table)
}

pub fn produce(src: &mut dyn Source, drive: char, out: &mut impl Write) -> std::io::Result<()> {
    let table = {
        let mut send = |records: u64, total: u64| write_frame(out, KIND_PROGRESS, &wire::progress_payload(records, total));
        collect(src, &mut send)?
    };
    match table.build(&root_name(drive)) {
        Ok(ix) => {
            let body = zstd::stream::encode_all(&store::encode(&ix)[..], 1)?;
            drop(ix);
            if body.len() as u64 > MAX_INDEX as u64 {
                return write_frame(out, KIND_ERROR, b"index too large");
            }
            write_frame(out, KIND_INDEX, &body)
        }
        Err(e) => write_frame(out, KIND_ERROR, e.to_string().as_bytes()),
    }
}

pub fn consume(r: &mut impl Read, drive: char, mut progress: impl FnMut(u64, u64)) -> Result<Index, MftError> {
    loop {
        match wire::read_frame(r) {
            Ok(Frame::Progress { records, total }) => progress(records, total),
            Ok(Frame::Index(body)) => return accept(&body, drive),
            Ok(Frame::Error(e)) => return Err(MftError::Helper(if e.is_empty() { "helper failed".into() } else { e })),
            Err(FrameError::Eof) => return Err(MftError::Helper("helper exited".into())),
            Err(FrameError::Io(e)) => return Err(MftError::Helper(format!("pipe: {e}"))),
            Err(FrameError::Bad(e)) => return Err(MftError::Helper(format!("bad {e}"))),
        }
    }
}

pub fn accept(body: &[u8], drive: char) -> Result<Index, MftError> {
    let raw = store::inflate(body, store::MAX_DECODED).map_err(|e| MftError::Helper(format!("index: {e}")))?;
    let ix = store::decode(&raw).map_err(|e| MftError::Helper(format!("index: {e}")))?;
    drop(raw);
    if ix.roots.len() != 1 || ix.name(ix.roots[0]) != root_name(drive) {
        return Err(MftError::Helper("index root mismatch".into()));
    }
    Ok(ix)
}

pub fn supported(drive: char) -> bool {
    #[cfg(windows)]
    {
        win::supported(drive)
    }
    #[cfg(not(windows))]
    {
        let _ = drive;
        false
    }
}

pub fn run(job: &Job, control: &Control, progress: &dyn Fn(u64, u64)) -> Result<Index, MftError> {
    #[cfg(windows)]
    {
        win::run(job, control, progress)
    }
    #[cfg(not(windows))]
    {
        let _ = (job, control, progress);
        Err(MftError::Unsupported)
    }
}

pub fn helper_main(args: &[String]) -> i32 {
    #[cfg(windows)]
    {
        win::helper_main(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        2
    }
}

pub fn fixture_main(args: &[String]) -> i32 {
    #[cfg(windows)]
    {
        win::fixture_main(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        2
    }
}
