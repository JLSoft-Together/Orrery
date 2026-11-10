use crate::run::parse::{machine_request, parse_machine_line, MachineMessage};
use crate::run::proc::pump_lines;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MachineReply {
    pub result: Option<Value>,
    pub error: Option<Value>,
}

impl MachineReply {
    pub fn error(code: &str) -> Self {
        MachineReply { result: None, error: Some(json!(code)) }
    }

    pub fn failed(&self) -> bool {
        self.error.as_ref().is_some_and(|e| !e.is_null())
    }

    pub fn error_is(&self, code: &str) -> bool {
        self.error.as_ref().and_then(Value::as_str) == Some(code)
    }
}

pub type EventFn = Arc<dyn Fn(&str, &Map<String, Value>) + Send + Sync>;
pub type TextFn = Arc<dyn Fn(&str) + Send + Sync>;

pub struct MachineHandlers {
    pub on_event: EventFn,
    pub on_text: TextFn,
}

pub struct MachineClient {
    stdin: Mutex<Option<Box<dyn Write + Send>>>,
    pending: Mutex<HashMap<i64, Sender<MachineReply>>>,
    next_id: AtomicI64,
    disposed: AtomicBool,
}

impl MachineClient {
    pub fn new(stdin: Option<Box<dyn Write + Send>>) -> Arc<MachineClient> {
        Arc::new(MachineClient { stdin: Mutex::new(stdin), pending: Mutex::new(HashMap::new()), next_id: AtomicI64::new(1), disposed: AtomicBool::new(false) })
    }

    pub fn handle_line(&self, line: &str, handlers: &MachineHandlers) {
        match parse_machine_line(line) {
            MachineMessage::Event { event, params } => (handlers.on_event)(&event, &params),
            MachineMessage::Response { id, result, error } => {
                let tx = self.pending.lock().unwrap().remove(&id);
                if let Some(tx) = tx {
                    let _ = tx.send(MachineReply { result, error });
                }
            }
            MachineMessage::Text(_) => (handlers.on_text)(line),
        }
    }

    pub fn attach<R: Read + Send + 'static>(self: &Arc<Self>, stdout: R, handlers: MachineHandlers) -> JoinHandle<()> {
        let me = self.clone();
        pump_lines(stdout, move |line| me.handle_line(line, &handlers))
    }

    pub fn send(&self, method: &str, params: Value) -> Option<(i64, Receiver<MachineReply>)> {
        if self.disposed.load(Ordering::SeqCst) {
            return None;
        }
        let mut stdin = self.stdin.lock().unwrap();
        let w = stdin.as_mut()?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let wrote = w.write_all(machine_request(id, method, params).as_bytes()).and_then(|_| w.flush());
        if wrote.is_err() {
            self.pending.lock().unwrap().remove(&id);
            *stdin = None;
            return None;
        }
        Some((id, rx))
    }

    pub fn request(&self, method: &str, params: Value, timeout: Option<Duration>) -> MachineReply {
        let Some((id, rx)) = self.send(method, params) else {
            return MachineReply::error("not-running");
        };
        let got = match timeout {
            Some(t) => rx.recv_timeout(t).ok(),
            None => rx.recv().ok(),
        };
        match got {
            Some(r) => r,
            None => {
                self.pending.lock().unwrap().remove(&id);
                MachineReply::error(if timeout.is_some() { "timeout" } else { "exited" })
            }
        }
    }

    pub fn close_stdin(&self) {
        *self.stdin.lock().unwrap() = None;
    }

    pub fn fail_pending(&self, code: &str) {
        let drained: Vec<Sender<MachineReply>> = self.pending.lock().unwrap().drain().map(|(_, tx)| tx).collect();
        for tx in drained {
            let _ = tx.send(MachineReply::error(code));
        }
    }

    pub fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
        self.fail_pending("exited");
        self.close_stdin();
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn routes_replies_events_and_text() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let c = MachineClient::new(Some(Box::new(Sink(buf.clone()))));
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let texts = Arc::new(Mutex::new(Vec::<String>::new()));
        let (e, t) = (events.clone(), texts.clone());
        let h = MachineHandlers { on_event: Arc::new(move |ev, _| e.lock().unwrap().push(ev.into())), on_text: Arc::new(move |l| t.lock().unwrap().push(l.into())) };
        let (id, rx) = c.send("app.restart", json!({"appId": "a"})).unwrap();
        let (id2, _) = c.send("x", json!({})).unwrap();
        assert_eq!((id, id2), (1, 2));
        let sent = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(sent.starts_with(r#"[{"id":1,"method":"app.restart","params":{"appId":"a"}}]"#));
        c.handle_line(r#"[{"id":1,"result":{"code":0}}]"#, &h);
        assert_eq!(rx.recv().unwrap().result, Some(json!({"code": 0})));
        c.handle_line(r#"[{"event":"app.started","params":{}}]"#, &h);
        c.handle_line("plain", &h);
        assert_eq!(*events.lock().unwrap(), vec!["app.started"]);
        assert_eq!(*texts.lock().unwrap(), vec!["plain"]);
        let (_, rx3) = c.send("y", json!({})).unwrap();
        c.dispose();
        assert!(rx3.recv().unwrap().error_is("exited"));
        assert!(c.request("z", json!({}), None).error_is("not-running"));
        let t = MachineClient::new(Some(Box::new(Sink(buf))));
        assert!(t.request("slow", json!({}), Some(Duration::from_millis(10))).error_is("timeout"));
    }
}
