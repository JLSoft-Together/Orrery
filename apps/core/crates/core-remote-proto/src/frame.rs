use std::collections::HashMap;

use crate::{Error, Result};

pub const HEADER_LEN: usize = 6;
pub const FLAG_MORE: u8 = 0b0000_0001;
pub const MAX_CHUNK: usize = 60 * 1024;
pub const MAX_MESSAGE: usize = 1 << 20;
pub const MAX_PENDING_STREAMS: usize = 32;
pub const MAX_PENDING_BYTES: usize = 4 * MAX_MESSAGE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kind {
    Json = 0,
    PtyOut = 1,
    PtyIn = 2,
    Ping = 3,
}

impl TryFrom<u8> for Kind {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Kind::Json),
            1 => Ok(Kind::PtyOut),
            2 => Ok(Kind::PtyIn),
            3 => Ok(Kind::Ping),
            other => Err(Error::UnknownKind(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: Kind,
    pub more: bool,
    pub stream: u32,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(kind: Kind, stream: u32, payload: Vec<u8>) -> Self {
        Self {
            kind,
            more: false,
            stream,
            payload,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.payload.len() > MAX_CHUNK {
            return Err(Error::TooLarge);
        }
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len());
        out.push(self.kind as u8);
        out.push(if self.more { FLAG_MORE } else { 0 });
        out.extend_from_slice(&self.stream.to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::BadFrame);
        }
        let kind = Kind::try_from(bytes[0])?;
        let flags = bytes[1];
        if flags & !FLAG_MORE != 0 {
            return Err(Error::BadFrame);
        }
        let payload = &bytes[HEADER_LEN..];
        if payload.len() > MAX_CHUNK {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            kind,
            more: flags & FLAG_MORE != 0,
            stream: u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            payload: payload.to_vec(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Chunker<'a> {
    kind: Kind,
    stream: u32,
    rest: &'a [u8],
    done: bool,
}

impl<'a> Chunker<'a> {
    pub fn new(kind: Kind, stream: u32, payload: &'a [u8]) -> Result<Self> {
        if payload.len() > MAX_MESSAGE {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            kind,
            stream,
            rest: payload,
            done: false,
        })
    }
}

impl Iterator for Chunker<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.done {
            return None;
        }
        let take = self.rest.len().min(MAX_CHUNK);
        let (head, tail) = self.rest.split_at(take);
        self.rest = tail;
        let more = !tail.is_empty();
        self.done = !more;
        Some(Frame {
            kind: self.kind,
            more,
            stream: self.stream,
            payload: head.to_vec(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: Kind,
    pub stream: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Reassembler {
    pending: HashMap<(Kind, u32), Vec<u8>>,
    pending_bytes: usize,
}

impl Reassembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_streams(&self) -> usize {
        self.pending.len()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.pending_bytes = 0;
    }

    fn drop_stream(&mut self, key: &(Kind, u32)) {
        if let Some(buf) = self.pending.remove(key) {
            self.pending_bytes -= buf.len();
        }
    }

    pub fn push(&mut self, frame: Frame) -> Result<Option<Message>> {
        if frame.payload.len() > MAX_CHUNK {
            return Err(Error::TooLarge);
        }
        let key = (frame.kind, frame.stream);
        let Some(buf) = self.pending.get(&key) else {
            if !frame.more {
                return Ok(Some(Message {
                    kind: frame.kind,
                    stream: frame.stream,
                    payload: frame.payload,
                }));
            }
            if self.pending.len() >= MAX_PENDING_STREAMS
                || self.pending_bytes + frame.payload.len() > MAX_PENDING_BYTES
            {
                return Err(Error::TooLarge);
            }
            self.pending_bytes += frame.payload.len();
            self.pending.insert(key, frame.payload);
            return Ok(None);
        };
        if buf.len() + frame.payload.len() > MAX_MESSAGE
            || self.pending_bytes + frame.payload.len() > MAX_PENDING_BYTES
        {
            self.drop_stream(&key);
            return Err(Error::TooLarge);
        }
        self.pending_bytes += frame.payload.len();
        let buf = self.pending.entry(key).or_default();
        buf.extend_from_slice(&frame.payload);
        if frame.more {
            return Ok(None);
        }
        let payload = self.pending.remove(&key).unwrap_or_default();
        self.pending_bytes -= payload.len();
        Ok(Some(Message {
            kind: frame.kind,
            stream: frame.stream,
            payload,
        }))
    }
}
