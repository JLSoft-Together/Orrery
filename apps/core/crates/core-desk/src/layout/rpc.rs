use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

const MAX_FRAME: usize = 64 * 1024 * 1024;
const MAX_HEADER: usize = 8192;

pub fn encode_message(msg: &Value) -> Vec<u8> {
    let body = crate::js::stringify(msg).into_bytes();
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(&body);
    out
}

#[derive(Default)]
pub struct FrameParser {
    buf: Vec<u8>,
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn content_length(header: &str) -> Option<usize> {
    let lower = header.to_ascii_lowercase();
    let at = lower.find("content-length:")?;
    let rest = lower[at + 15..].trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

impl FrameParser {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Value>, String> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        loop {
            let Some(sep) = find(&self.buf, b"\r\n\r\n") else {
                if self.buf.len() > MAX_HEADER {
                    return Err("header too long".into());
                }
                return Ok(out);
            };
            let len = content_length(&String::from_utf8_lossy(&self.buf[..sep])).ok_or("missing Content-Length")?;
            if len > MAX_FRAME {
                return Err("frame too large".into());
            }
            if self.buf.len() < sep + 4 + len {
                return Ok(out);
            }
            let body: Vec<u8> = self.buf[sep + 4..sep + 4 + len].to_vec();
            self.buf.drain(..sep + 4 + len);
            out.push(serde_json::from_slice(&body).map_err(|_| "bad json".to_string())?);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RpcFail {
    Rpc { code: i64, message: String },
    Closed(String),
    Timeout,
}

type Reply = Result<Value, RpcFail>;
type Notify = Box<dyn Fn(&str, &Value) + Send + Sync>;

pub struct RpcClient {
    write: Mutex<Box<dyn Write + Send>>,
    pending: Mutex<HashMap<u64, Sender<Reply>>>,
    next: Mutex<u64>,
    notify: Notify,
}

impl RpcClient {
    pub fn new(write: Box<dyn Write + Send>, notify: Notify) -> Self {
        RpcClient { write: Mutex::new(write), pending: Mutex::new(HashMap::new()), next: Mutex::new(1), notify }
    }

    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Reply {
        let (tx, rx) = mpsc::channel();
        let id = {
            let mut n = self.next.lock().unwrap();
            let id = *n;
            *n += 1;
            id
        };
        self.pending.lock().unwrap().insert(id, tx);
        let frame = encode_message(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let wrote = {
            let mut w = self.write.lock().unwrap();
            w.write_all(&frame).and_then(|_| w.flush())
        };
        if let Err(e) = wrote {
            self.pending.lock().unwrap().remove(&id);
            return Err(RpcFail::Closed(e.to_string()));
        }
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(RecvTimeoutError::Timeout) => {
                self.pending.lock().unwrap().remove(&id);
                Err(RpcFail::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => Err(RpcFail::Closed("closed".into())),
        }
    }

    pub fn handle(&self, msg: &Value) {
        let Some(m) = msg.as_object() else { return };
        let id = m.get("id");
        if let (Some(method), None) = (m.get("method").and_then(Value::as_str), id) {
            (self.notify)(method, m.get("params").unwrap_or(&Value::Null));
            return;
        }
        let Some(id) = id.and_then(Value::as_u64) else { return };
        let Some(tx) = self.pending.lock().unwrap().remove(&id) else { return };
        let reply = match m.get("error") {
            Some(e) if !e.is_null() => {
                let code = e.get("code").and_then(Value::as_i64).unwrap_or(-32603);
                let message = match e.get("message") {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Null) | None => "error".into(),
                    Some(other) => crate::js::stringify(other),
                };
                Err(RpcFail::Rpc { code, message })
            }
            _ => Ok(m.get("result").cloned().unwrap_or(Value::Null)),
        };
        let _ = tx.send(reply);
    }

    pub fn fail_all(&self, why: &str) {
        let all: Vec<_> = self.pending.lock().unwrap().drain().collect();
        for (_, tx) in all {
            let _ = tx.send(Err(RpcFail::Closed(why.to_string())));
        }
    }

    pub fn close_write(&self) {
        let mut w = self.write.lock().unwrap();
        let _ = w.flush();
        *w = Box::new(std::io::sink());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_split_across_chunks() {
        let a = encode_message(&json!({"a": 1}));
        let b = encode_message(&json!({"b": "é"}));
        let all = [a, b].concat();
        let mut p = FrameParser::default();
        let mut got = p.push(&all[..7]).unwrap();
        got.extend(p.push(&all[7..20]).unwrap());
        got.extend(p.push(&all[20..]).unwrap());
        assert_eq!(got, vec![json!({"a": 1}), json!({"b": "é"})]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(FrameParser::default().push(b"X: 1\r\n\r\n{}").is_err());
        assert!(FrameParser::default().push(b"Content-Length: 3\r\n\r\n{xx").is_err());
        assert!(FrameParser::default().push(&vec![b'a'; 9000]).is_err());
    }
}
