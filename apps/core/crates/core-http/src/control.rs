use std::io::{Read, Write};

use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub const LINE_MAX: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlIn {
    Shutdown,
    Eof,
    Message(Value),
}

pub fn parse_line(line: &str) -> Option<ControlIn> {
    let value: Value = serde_json::from_str(line).ok()?;
    let kind = value.as_object()?.get("type")?.as_str()?;
    if kind == "shutdown" {
        Some(ControlIn::Shutdown)
    } else {
        Some(ControlIn::Message(value))
    }
}

pub fn encode_line(value: &Value) -> String {
    let mut line = value.to_string();
    line.push('\n');
    line
}

#[derive(Default)]
pub struct LineSplitter {
    buf: Vec<u8>,
    skipping: bool,
}

impl LineSplitter {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Option<ControlIn>> {
        let mut out = Vec::new();
        let mut rest = chunk;
        loop {
            let Some(nl) = rest.iter().position(|b| *b == b'\n') else {
                if !self.skipping {
                    if self.buf.len() + rest.len() > LINE_MAX {
                        self.buf.clear();
                        self.skipping = true;
                    } else {
                        self.buf.extend_from_slice(rest);
                    }
                }
                return out;
            };
            let part = &rest[..nl];
            rest = &rest[nl + 1..];
            if self.skipping || self.buf.len() + part.len() > LINE_MAX {
                self.buf.clear();
                self.skipping = false;
                out.push(None);
                continue;
            }
            self.buf.extend_from_slice(part);
            let line = std::mem::take(&mut self.buf);
            match std::str::from_utf8(&line) {
                Ok(s) if s.trim().is_empty() => {}
                Ok(s) => out.push(parse_line(s.trim_end_matches('\r'))),
                Err(_) => out.push(None),
            }
        }
    }
}

pub fn spawn_stdin(tx: UnboundedSender<ControlIn>) {
    let _ = std::thread::Builder::new().name("control-in".into()).spawn(move || {
        let mut input = std::io::stdin();
        let mut splitter = LineSplitter::default();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match input.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for message in splitter.push(&buf[..n]).into_iter().flatten() {
                        if tx.send(message).is_err() {
                            return;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = tx.send(ControlIn::Eof);
    });
}

pub fn write_line(value: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(encode_line(value).as_bytes());
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn lines_become_shutdown_or_messages_and_garbage_is_dropped() {
        let mut s = LineSplitter::default();
        let out = s.push(b"{\"type\":\"shutdown\"}\n{\"type\":\"deviceUsed\",\"id\":1}\r\nnot json\n\n[1]\n{\"x\":1}\n");
        assert_eq!(out, vec![Some(ControlIn::Shutdown), Some(ControlIn::Message(json!({"type": "deviceUsed", "id": 1}))), None, None, None]);
    }

    #[test]
    fn a_line_split_across_chunks_is_joined() {
        let mut s = LineSplitter::default();
        assert!(s.push(b"{\"type\":\"shu").is_empty());
        assert_eq!(s.push(b"tdown\"}\n"), vec![Some(ControlIn::Shutdown)]);
    }

    #[test]
    fn an_oversize_line_is_skipped_up_to_its_newline() {
        let mut s = LineSplitter::default();
        assert!(s.push(&vec![b'a'; LINE_MAX]).is_empty());
        assert!(s.push(b"bbbb").is_empty());
        assert_eq!(s.push(b"cc\n{\"type\":\"shutdown\"}\n"), vec![None, Some(ControlIn::Shutdown)]);
    }

    #[test]
    fn encodes_one_line() {
        assert_eq!(encode_line(&json!({"type": "listening", "port": 5})), "{\"type\":\"listening\",\"port\":5}\n");
    }
}
