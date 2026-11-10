use crate::{js, Ctx, Desk, DeskEnv, DeskError, DeskIo};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

type Reply = Result<Value, String>;

pub struct Pipe {
    out: Mutex<Box<dyn Write + Send>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    next_call: AtomicU64,
    closed: AtomicBool,
}

impl Pipe {
    pub fn new(out: Box<dyn Write + Send>) -> Self {
        Pipe { out: Mutex::new(out), pending: Mutex::new(HashMap::new()), next_call: AtomicU64::new(1), closed: AtomicBool::new(false) }
    }

    pub fn send(&self, v: &Value) {
        let mut line = js::stringify(v);
        line.push('\n');
        let mut out = self.out.lock().unwrap();
        if out.write_all(line.as_bytes()).and_then(|_| out.flush()).is_err() {
            self.closed.store(true, Ordering::SeqCst);
        }
    }

    fn reply(&self, id: u64, reply: Reply) {
        if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
            let _ = tx.send(reply);
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.pending.lock().unwrap().clear();
    }
}

impl DeskIo for Pipe {
    fn emit(&self, event: &str, payload: Value) {
        self.send(&json!({"event": event, "payload": payload}));
    }

    fn host(&self, req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(DeskError::Host("closed".into()));
        }
        let id = self.next_call.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        self.send(&json!({"call": id, "for": req, "method": method, "params": params}));
        match rx.recv() {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(DeskError::Host(e)),
            Err(_) => Err(DeskError::Host("closed".into())),
        }
    }
}

fn error_text(e: &DeskError) -> String {
    e.to_string()
}

fn handle_line(desk: &Arc<Desk>, pipe: &Arc<Pipe>, line: &str) -> bool {
    let Ok(Value::Object(m)) = serde_json::from_str::<Value>(line) else { return true };
    if m.get("type").and_then(Value::as_str) == Some("shutdown") {
        return false;
    }
    if let Some(id) = m.get("reply").and_then(Value::as_u64) {
        let reply = if m.get("ok") == Some(&Value::Bool(true)) {
            Ok(m.get("value").cloned().unwrap_or(Value::Null))
        } else {
            Err(m.get("error").and_then(Value::as_str).unwrap_or("failed").to_string())
        };
        pipe.reply(id, reply);
        return true;
    }
    let (Some(id), Some(channel)) = (m.get("id").and_then(Value::as_u64), m.get("channel").and_then(Value::as_str)) else { return true };
    let args = m.get("args").cloned().unwrap_or(Value::Null);
    let (desk, pipe, channel) = (desk.clone(), pipe.clone(), channel.to_string());
    std::thread::spawn(move || {
        let r = desk.dispatch_with(&Ctx::new(&*pipe, id), &channel, args);
        let msg = match r {
            Ok(value) => json!({"id": id, "ok": true, "value": value}),
            Err(e) => {
                eprintln!("desk {channel}: {e}");
                json!({"id": id, "ok": false, "error": error_text(&e)})
            }
        };
        pipe.send(&msg);
    });
    true
}

pub fn serve(desk: Arc<Desk>, input: impl BufRead, pipe: Arc<Pipe>) -> i32 {
    pipe.send(&json!({"ready": true, "pid": std::process::id(), "channels": desk.channels()}));
    for line in input.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if !handle_line(&desk, &pipe, &line) {
            break;
        }
    }
    pipe.close();
    0
}

pub fn run() -> i32 {
    let desk = Arc::new(Desk::standard(&DeskEnv::from_env()));
    let pipe = Arc::new(Pipe::new(Box::new(std::io::stdout())));
    serve(desk, std::io::stdin().lock(), pipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);

    impl Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn answers_requests_and_unknown_channels() {
        let mut desk = Desk::new();
        desk.on("t", "t:echo", |ctx, args| {
            ctx.emit("t:event", json!(1));
            Ok(json!({"got": args.len()}))
        });
        let buf = Buf::default();
        let pipe = Arc::new(Pipe::new(Box::new(buf.clone())));
        let input = Cursor::new("{\"id\":1,\"channel\":\"t:echo\",\"args\":[1,2]}\n{\"id\":2,\"channel\":\"nope\",\"args\":[]}\n");
        serve(Arc::new(desk), input, pipe);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let text = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
            if text.contains("\"id\":1,") && text.contains("\"id\":2,") || std::time::Instant::now() > deadline {
                assert!(text.starts_with("{\"ready\":true"));
                assert!(text.contains("{\"id\":1,\"ok\":true,\"value\":{\"got\":2}}"), "{text}");
                assert!(text.contains("{\"event\":\"t:event\",\"payload\":1}"));
                assert!(text.contains("{\"id\":2,\"ok\":false,\"error\":\"unknown channel nope\"}"));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
