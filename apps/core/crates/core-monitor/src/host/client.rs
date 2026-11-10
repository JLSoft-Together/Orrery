use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use super::protocol::{request, response_body, response_id};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    Closed,
    Timeout,
    Remote(String),
}

type Pending = Sender<Result<Value, String>>;

pub struct HostClient {
    out: Mutex<Box<dyn Write + Send>>,
    pending: Mutex<HashMap<u64, Pending>>,
    next: AtomicU64,
    closed: Mutex<bool>,
}

impl HostClient {
    pub fn new(out: Box<dyn Write + Send>) -> Arc<HostClient> {
        Arc::new(HostClient { out: Mutex::new(out), pending: Mutex::new(HashMap::new()), next: AtomicU64::new(1), closed: Mutex::new(false) })
    }

    pub fn call(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, HostError> {
        if *self.closed.lock().map_err(|_| HostError::Closed)? {
            return Err(HostError::Closed);
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        self.pending.lock().map_err(|_| HostError::Closed)?.insert(id, tx);
        let mut line = request(id, method, params).to_string();
        line.push('\n');
        let wrote = match self.out.lock() {
            Ok(mut o) => o.write_all(line.as_bytes()).and_then(|()| o.flush()).is_ok(),
            Err(_) => false,
        };
        if !wrote {
            self.forget(id);
            return Err(HostError::Closed);
        }
        match rx.recv_timeout(timeout) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(HostError::Remote(e)),
            Err(RecvTimeoutError::Timeout) => {
                self.forget(id);
                Err(HostError::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => Err(HostError::Closed),
        }
    }

    fn forget(&self, id: u64) {
        if let Ok(mut p) = self.pending.lock() {
            p.remove(&id);
        }
    }

    pub fn on_message(&self, v: &Value) -> bool {
        let Some(id) = response_id(v) else { return false };
        let tx = self.pending.lock().ok().and_then(|mut p| p.remove(&id));
        if let Some(tx) = tx {
            let _ = tx.send(response_body(v));
        }
        true
    }

    pub fn close(&self) {
        if let Ok(mut c) = self.closed.lock() {
            *c = true;
        }
        if let Ok(mut p) = self.pending.lock() {
            p.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::json;

    use super::*;

    #[derive(Clone, Default)]
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
    fn a_call_waits_for_the_response_with_its_id() {
        let sink = Sink::default();
        let client = HostClient::new(Box::new(sink.clone()));
        let c2 = client.clone();
        let t = std::thread::spawn(move || c2.call("focus", json!({"pid": 9}), Duration::from_secs(5)));
        let line = loop {
            let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
            if text.ends_with('\n') {
                break text;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let req: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(req["type"], "host.req");
        assert_eq!(req["method"], "focus");
        assert_eq!(req["params"]["pid"], 9);
        assert!(!client.on_message(&json!({"type": "shutdown"})));
        assert!(client.on_message(&json!({"type": "host.res", "id": req["id"], "result": {"result": "ok"}})));
        assert_eq!(t.join().unwrap(), Ok(json!({"result": "ok"})));
    }

    #[test]
    fn timeout_error_and_close_are_reported() {
        let client = HostClient::new(Box::new(Sink::default()));
        assert_eq!(client.call("x", json!({}), Duration::from_millis(30)), Err(HostError::Timeout));
        assert!(client.pending.lock().unwrap().is_empty());
        let c2 = client.clone();
        let t = std::thread::spawn(move || c2.call("x", json!({}), Duration::from_secs(5)));
        while client.pending.lock().unwrap().is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let id = *client.pending.lock().unwrap().keys().next().unwrap();
        client.on_message(&json!({"type": "host.res", "id": id, "error": "boom"}));
        assert_eq!(t.join().unwrap(), Err(HostError::Remote("boom".into())));
        client.close();
        assert_eq!(client.call("x", json!({}), Duration::from_millis(30)), Err(HostError::Closed));
    }
}
