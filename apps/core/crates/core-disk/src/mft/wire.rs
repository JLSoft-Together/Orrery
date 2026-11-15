use std::io::{Read, Write};

pub const HELLO: &[u8; 8] = b"ORRMFT1\0";
pub const ACK: u8 = 0x06;
pub const TOKEN_BYTES: usize = 32;
pub const PIPE_PREFIX: &str = r"\\.\pipe\orrery-mft-";
pub const KIND_PROGRESS: u8 = 1;
pub const KIND_INDEX: u8 = 2;
pub const KIND_ERROR: u8 = 3;
pub const MAX_SMALL: u32 = 4096;
pub const MAX_INDEX: u32 = 3 << 30;
pub const CHUNK: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperArgs {
    pub drive: char,
    pub pipe: String,
    pub token: [u8; TOKEN_BYTES],
}

pub fn valid_drive(s: &str) -> Option<char> {
    let mut it = s.chars();
    match (it.next(), it.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => Some(c),
        _ => None,
    }
}

fn is_lower_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn valid_pipe(s: &str) -> bool {
    s.strip_prefix(PIPE_PREFIX).is_some_and(|id| is_lower_hex(id, 32))
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn parse_token(s: &str) -> Option<[u8; TOKEN_BYTES]> {
    if !is_lower_hex(s, TOKEN_BYTES * 2) {
        return None;
    }
    let mut out = [0u8; TOKEN_BYTES];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

pub fn parse_helper_args(args: &[String]) -> Option<HelperArgs> {
    let [drive, pipe, token] = args else { return None };
    let drive = valid_drive(drive)?;
    if !valid_pipe(pipe) {
        return None;
    }
    Some(HelperArgs { drive, pipe: pipe.clone(), token: parse_token(token)? })
}

pub fn same_token(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn hello(token: &[u8; TOKEN_BYTES]) -> Vec<u8> {
    let mut v = HELLO.to_vec();
    v.extend_from_slice(token);
    v
}

pub fn check_hello(got: &[u8], token: &[u8; TOKEN_BYTES]) -> bool {
    got.len() == HELLO.len() + TOKEN_BYTES && same_token(&got[..HELLO.len()], HELLO) & same_token(&got[HELLO.len()..], token)
}

pub fn write_frame(w: &mut impl Write, kind: u8, payload: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(payload.len()).map_err(|_| std::io::Error::other("frame too large"))?;
    let mut head = [0u8; 5];
    head[0] = kind;
    head[1..].copy_from_slice(&len.to_le_bytes());
    w.write_all(&head)?;
    for c in payload.chunks(CHUNK) {
        w.write_all(c)?;
    }
    w.flush()
}

pub fn progress_payload(records: u64, total: u64) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[..8].copy_from_slice(&records.to_le_bytes());
    p[8..].copy_from_slice(&total.to_le_bytes());
    p
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Progress { records: u64, total: u64 },
    Index(Vec<u8>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    Eof,
    Io(String),
    Bad(&'static str),
}

pub fn max_len(kind: u8) -> Option<u32> {
    match kind {
        KIND_PROGRESS => Some(16),
        KIND_ERROR => Some(MAX_SMALL),
        KIND_INDEX => Some(MAX_INDEX),
        _ => None,
    }
}

pub fn decode_frame(kind: u8, payload: Vec<u8>) -> Result<Frame, FrameError> {
    match kind {
        KIND_PROGRESS if payload.len() == 16 => Ok(Frame::Progress { records: u64::from_le_bytes(payload[..8].try_into().unwrap()), total: u64::from_le_bytes(payload[8..].try_into().unwrap()) }),
        KIND_INDEX => Ok(Frame::Index(payload)),
        KIND_ERROR => Ok(Frame::Error(String::from_utf8_lossy(&payload).chars().take(300).collect())),
        _ => Err(FrameError::Bad("frame")),
    }
}

pub fn read_frame(r: &mut impl Read) -> Result<Frame, FrameError> {
    let mut head = [0u8; 5];
    r.read_exact(&mut head).map_err(|e| if e.kind() == std::io::ErrorKind::UnexpectedEof { FrameError::Eof } else { FrameError::Io(e.to_string()) })?;
    let len = u32::from_le_bytes(head[1..].try_into().unwrap());
    let max = max_len(head[0]).ok_or(FrameError::Bad("kind"))?;
    if len > max {
        return Err(FrameError::Bad("length"));
    }
    let mut payload = Vec::new();
    let mut left = len as usize;
    let mut buf = vec![0u8; CHUNK.min(left.max(1))];
    while left > 0 {
        let n = left.min(buf.len());
        r.read_exact(&mut buf[..n]).map_err(|e| FrameError::Io(e.to_string()))?;
        payload.extend_from_slice(&buf[..n]);
        left -= n;
    }
    decode_frame(head[0], payload)
}
