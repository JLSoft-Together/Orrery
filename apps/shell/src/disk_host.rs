use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use crate::state::lock;

const BOOT: Duration = Duration::from_secs(10);
pub const CALL: Duration = Duration::from_secs(120);
const STOP: Duration = Duration::from_secs(2);
const LINE_MAX: usize = 64 * 1024 * 1024;

pub type OnEvent = Arc<dyn Fn(&str, Value) + Send + Sync>;
pub type OnExit = Arc<dyn Fn(bool, String) + Send + Sync>;

type Reply = Result<Value, String>;

struct Proc {
    gen: u64,
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Child>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
}

struct Inner {
    bin: PathBuf,
    data_dir: PathBuf,
    proc: Mutex<Option<Arc<Proc>>>,
    boot: Mutex<()>,
    next_id: AtomicU64,
    next_gen: AtomicU64,
    scanning: AtomicBool,
    stopping: AtomicBool,
    last_error: Mutex<Option<String>>,
    on_event: Mutex<Option<OnEvent>>,
    on_exit: Mutex<Option<OnExit>>,
}

#[derive(Clone)]
pub struct DiskHost(Arc<Inner>);

pub enum Line {
    Ready,
    Event(String, Value),
    Reply(u64, Reply),
    Text,
}

pub fn parse_line(raw: &str) -> Line {
    let text = raw.trim_end_matches('\r');
    let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text) else { return Line::Text };
    if o.get("ready") == Some(&Value::Bool(true)) {
        return Line::Ready;
    }
    if let Some(event) = o.get("event").and_then(Value::as_str) {
        return Line::Event(event.to_owned(), o.get("payload").cloned().unwrap_or(Value::Null));
    }
    if let Some(id) = o.get("id").and_then(Value::as_u64) {
        let reply = if o.get("ok") == Some(&Value::Bool(true)) {
            Ok(o.get("value").cloned().unwrap_or(Value::Null))
        } else {
            Err(o.get("error").and_then(Value::as_str).unwrap_or("failed").to_owned())
        };
        return Line::Reply(id, reply);
    }
    Line::Text
}

impl DiskHost {
    pub fn new(bin: PathBuf, data_dir: PathBuf) -> Self {
        DiskHost(Arc::new(Inner {
            bin,
            data_dir,
            proc: Mutex::new(None),
            boot: Mutex::new(()),
            next_id: AtomicU64::new(1),
            next_gen: AtomicU64::new(1),
            scanning: AtomicBool::new(false),
            stopping: AtomicBool::new(false),
            last_error: Mutex::new(None),
            on_event: Mutex::new(None),
            on_exit: Mutex::new(None),
        }))
    }

    pub fn set_handlers(&self, on_event: OnEvent, on_exit: OnExit) {
        *lock(&self.0.on_event) = Some(on_event);
        *lock(&self.0.on_exit) = Some(on_exit);
    }

    pub fn running(&self) -> bool {
        lock(&self.0.proc).is_some()
    }

    #[cfg(test)]
    pub fn last_error(&self) -> Option<String> {
        lock(&self.0.last_error).clone()
    }

    pub fn mark_scanning(&self, on: bool) {
        self.0.scanning.store(on, Ordering::SeqCst);
    }

    fn start(&self) -> Result<Arc<Proc>, String> {
        if let Some(p) = lock(&self.0.proc).clone() {
            return Ok(p);
        }
        let _boot = lock(&self.0.boot);
        if let Some(p) = lock(&self.0.proc).clone() {
            return Ok(p);
        }
        if self.0.stopping.load(Ordering::SeqCst) {
            return Err("stopping".into());
        }
        if !self.0.bin.is_file() {
            let e = format!("orrery-core not found at {}", self.0.bin.display());
            *lock(&self.0.last_error) = Some(e.clone());
            return Err(e);
        }
        let mut cmd = Command::new(&self.0.bin);
        cmd.arg("disk").env("CCM_DATA_DIR", &self.0.data_dir).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| format!("orrery-core disk: {e}"))?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();
        let gen = self.0.next_gen.fetch_add(1, Ordering::SeqCst);
        let proc = Arc::new(Proc { gen, stdin: Mutex::new(stdin), child: Mutex::new(child), pending: Mutex::new(HashMap::new()) });
        let (ready_tx, ready_rx) = mpsc::channel::<()>();
        let inner = self.0.clone();
        let reader_proc = proc.clone();
        std::thread::Builder::new()
            .name("disk-host-out".into())
            .spawn(move || read_loop(inner, reader_proc, stdout, ready_tx))
            .map_err(|e| e.to_string())?;
        if let Some(mut err) = stderr {
            let _ = std::thread::Builder::new().name("disk-host-err".into()).spawn(move || {
                let mut buf = [0u8; 4096];
                while matches!(err.read(&mut buf), Ok(n) if n > 0) {}
            });
        }
        match ready_rx.recv_timeout(BOOT) {
            Ok(()) => {
                *lock(&self.0.last_error) = None;
                *lock(&self.0.proc) = Some(proc.clone());
                Ok(proc)
            }
            Err(_) => {
                let _ = lock(&proc.child).kill();
                let e = "boot timeout".to_owned();
                *lock(&self.0.last_error) = Some(e.clone());
                Err(e)
            }
        }
    }

    pub fn call(&self, channel: &str, args: Value) -> Result<Value, String> {
        self.call_within(channel, args, CALL)
    }

    pub fn call_within(&self, channel: &str, args: Value, timeout: Duration) -> Result<Value, String> {
        if self.0.stopping.load(Ordering::SeqCst) {
            return Err("stopping".into());
        }
        let proc = self.start()?;
        let id = self.0.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        lock(&proc.pending).insert(id, tx);
        let mut line = serde_json::to_string(&json!({ "id": id, "channel": channel, "args": args })).map_err(|e| e.to_string())?;
        line.push('\n');
        let wrote = match lock(&proc.stdin).as_mut() {
            Some(w) => w.write_all(line.as_bytes()).and_then(|_| w.flush()).is_ok(),
            None => false,
        };
        if !wrote {
            lock(&proc.pending).remove(&id);
            return Err("not running".into());
        }
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                lock(&proc.pending).remove(&id);
                Err(format!("timeout {channel}"))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("orrery-core disk exited".into()),
        }
    }

    pub fn stop(&self) {
        self.0.stopping.store(true, Ordering::SeqCst);
        let Some(proc) = lock(&self.0.proc).take() else { return };
        if let Some(mut w) = lock(&proc.stdin).take() {
            let _ = w.write_all(b"{\"type\":\"shutdown\"}\n");
            let _ = w.flush();
        }
        let deadline = std::time::Instant::now() + STOP;
        loop {
            let done = lock(&proc.child).try_wait().map(|s| s.is_some()).unwrap_or(true);
            if done {
                return;
            }
            if std::time::Instant::now() >= deadline {
                let _ = lock(&proc.child).kill();
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

fn read_loop(inner: Arc<Inner>, proc: Arc<Proc>, stdout: std::process::ChildStdout, ready: mpsc::Sender<()>) {
    let mut reader = BufReader::new(stdout);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if buf.len() > LINE_MAX {
            continue;
        }
        match parse_line(&String::from_utf8_lossy(&buf)) {
            Line::Ready => {
                let _ = ready.send(());
            }
            Line::Event(event, payload) => {
                if event == "disk:done" || event == "disk:dupes-done" {
                    inner.scanning.store(false, Ordering::SeqCst);
                }
                let handler = lock(&inner.on_event).clone();
                if let Some(h) = handler {
                    h(&event, payload);
                }
            }
            Line::Reply(id, reply) => {
                if let Some(tx) = lock(&proc.pending).remove(&id) {
                    let _ = tx.send(reply);
                }
            }
            Line::Text => {}
        }
    }
    let code = lock(&proc.child).wait().ok().and_then(|s| s.code());
    let reason = format!("orrery-core disk exited (code {})", code.map_or("null".to_owned(), |c| c.to_string()));
    let mine = {
        let mut slot = lock(&inner.proc);
        if slot.as_ref().is_some_and(|p| p.gen == proc.gen) {
            *slot = None;
            true
        } else {
            false
        }
    };
    for (_, tx) in lock(&proc.pending).drain() {
        let _ = tx.send(Err(reason.clone()));
    }
    if !mine || inner.stopping.load(Ordering::SeqCst) {
        return;
    }
    *lock(&inner.last_error) = Some(reason.clone());
    let busy = inner.scanning.swap(false, Ordering::SeqCst);
    let handler = lock(&inner.on_exit).clone();
    if let Some(h) = handler {
        h(busy, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_lines_parse_into_ready_events_and_replies() {
        assert!(matches!(parse_line("{\"ready\":true,\"pid\":1,\"channels\":[]}\r"), Line::Ready));
        assert!(matches!(parse_line("{\"event\":\"disk:done\",\"payload\":{\"ok\":true}}"), Line::Event(e, p) if e == "disk:done" && p["ok"] == json!(true)));
        assert!(matches!(parse_line("{\"id\":3,\"ok\":true,\"value\":5}"), Line::Reply(3, Ok(v)) if v == json!(5)));
        assert!(matches!(parse_line("{\"id\":4,\"ok\":false,\"error\":\"busy\"}"), Line::Reply(4, Err(e)) if e == "busy"));
        assert!(matches!(parse_line("{\"id\":4,\"ok\":false}"), Line::Reply(4, Err(e)) if e == "failed"));
        assert!(matches!(parse_line("plain text"), Line::Text));
    }

    #[test]
    fn a_missing_core_binary_fails_the_call() {
        let host = DiskHost::new(PathBuf::from("Z:/definitely/missing/orrery-core.exe"), std::env::temp_dir());
        let err = host.call("disk.status", json!({})).unwrap_err();
        assert!(err.starts_with("orrery-core not found"), "{err}");
        assert_eq!(host.last_error(), Some(err));
        assert!(!host.running());
    }
}
