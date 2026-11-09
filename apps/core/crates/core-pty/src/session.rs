use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

#[cfg(unix)]
use crate::pty_unix as sys;
#[cfg(windows)]
use crate::pty_win as sys;

pub use sys::Force;

pub const WRITE_QUEUE_MAX: usize = 4 * 1024 * 1024;
pub const READ_CHUNK: usize = 64 * 1024;
pub const EXIT_DRAIN: Duration = Duration::from_millis(200);

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub file: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PtyEvent {
    Data(Vec<u8>),
    Draining,
    Exit { code: i64, signal: Option<String> },
}

pub type Sink = Arc<dyn Fn(PtyEvent) + Send + Sync>;

pub fn dedupe_env(env: &[(String, String)], case_insensitive: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::with_capacity(env.len());
    let mut index = std::collections::HashMap::new();
    for (k, v) in env {
        if k.is_empty() || k.contains('\0') || v.contains('\0') || k.chars().skip(1).any(|c| c == '=') {
            continue;
        }
        let key = if case_insensitive { k.to_uppercase() } else { k.clone() };
        match index.get(&key) {
            Some(&i) => out[i] = (k.clone(), v.clone()),
            None => {
                index.insert(key, out.len());
                out.push((k.clone(), v.clone()));
            }
        }
    }
    out
}

pub struct WriteQueue {
    tx: mpsc::Sender<Vec<u8>>,
    pending: Arc<AtomicUsize>,
}

impl WriteQueue {
    pub fn start(mut w: Box<dyn Write + Send>) -> WriteQueue {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let pending = Arc::new(AtomicUsize::new(0));
        let left = pending.clone();
        let _ = std::thread::Builder::new().name("pty-write".into()).spawn(move || {
            for buf in rx {
                let ok = w.write_all(&buf).and_then(|_| w.flush()).is_ok();
                left.fetch_sub(buf.len(), Ordering::AcqRel);
                if !ok {
                    break;
                }
            }
        });
        WriteQueue { tx, pending }
    }

    pub fn push(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return true;
        }
        if self.pending.load(Ordering::Acquire) + data.len() > WRITE_QUEUE_MAX {
            return false;
        }
        self.pending.fetch_add(data.len(), Ordering::AcqRel);
        self.tx.send(data.to_vec()).is_ok()
    }
}

pub struct Reader {
    pub done: mpsc::Receiver<()>,
    pub last_data: Arc<AtomicU64>,
    pub epoch: Instant,
}

impl Reader {
    pub fn quiet_for(&self) -> Duration {
        let last = Duration::from_micros(self.last_data.load(Ordering::Acquire));
        self.epoch.elapsed().saturating_sub(last)
    }

    pub fn wait_done(&self, max: Duration) -> bool {
        matches!(self.done.recv_timeout(max), Err(mpsc::RecvTimeoutError::Disconnected) | Ok(()))
    }
}

pub fn start_reader(mut r: Box<dyn Read + Send>, sink: Sink) -> Reader {
    let (done_tx, done) = mpsc::channel::<()>();
    let epoch = Instant::now();
    let last_data = Arc::new(AtomicU64::new(0));
    let stamp = last_data.clone();
    let _ = std::thread::Builder::new().name("pty-read".into()).spawn(move || {
        let mut buf = vec![0u8; READ_CHUNK];
        loop {
            match r.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    stamp.store(epoch.elapsed().as_micros() as u64, Ordering::Release);
                    sink(PtyEvent::Data(buf[..n].to_vec()));
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        drop(done_tx);
    });
    Reader { done, last_data, epoch }
}

pub struct PtyProc {
    pub pid: u32,
    writer: WriteQueue,
    sys: sys::Sys,
}

impl PtyProc {
    pub fn spawn(spec: &SpawnSpec, sink: Sink) -> io::Result<PtyProc> {
        if !std::path::Path::new(&spec.cwd).is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "cwd is not a directory"));
        }
        let (sys, pid, writer) = sys::spawn(spec, sink)?;
        Ok(PtyProc { pid, writer: WriteQueue::start(writer), sys })
    }

    pub fn write(&self, data: &[u8]) -> bool {
        self.writer.push(data)
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        self.sys.resize(cols, rows);
    }

    pub fn kill(&self) {
        self.sys.kill();
    }

    pub fn hangup(&self) -> Force {
        self.sys.hangup()
    }
}

pub fn clamp_dim(v: Option<&serde_json::Value>, fallback: u16) -> u16 {
    use serde_json::Value;
    let n = match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                Some(0.0)
            } else {
                t.parse::<f64>().ok()
            }
        }
        Some(Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    };
    match n {
        Some(n) if n.fract() == 0.0 && n > 0.0 && n <= 1000.0 => n as u16,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clamp_dim_mirrors_node() {
        assert_eq!(clamp_dim(Some(&json!(120)), 80), 120);
        assert_eq!(clamp_dim(Some(&json!(0)), 80), 80);
        assert_eq!(clamp_dim(Some(&json!(1001)), 80), 80);
        assert_eq!(clamp_dim(Some(&json!(12.5)), 80), 80);
        assert_eq!(clamp_dim(Some(&json!("40")), 80), 40);
        assert_eq!(clamp_dim(Some(&json!(null)), 24), 24);
        assert_eq!(clamp_dim(None, 24), 24);
    }

    #[test]
    fn dedupe_env_last_wins_and_keeps_first_position() {
        let env = vec![("Path".to_owned(), "a".to_owned()), ("X".to_owned(), "1".to_owned()), ("PATH".to_owned(), "b".to_owned()), ("".to_owned(), "z".to_owned())];
        assert_eq!(dedupe_env(&env, true), vec![("PATH".to_owned(), "b".to_owned()), ("X".to_owned(), "1".to_owned())]);
        assert_eq!(dedupe_env(&env, false).len(), 3);
    }
}
