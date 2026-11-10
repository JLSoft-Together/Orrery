use super::adb::Adb;
use crate::run::collect::Collected;
use crate::run::proc::{kill_pid_only, spawn, SpawnSpec};
use std::io::{Read, Write};
use std::process::Child;
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Streamed {
    pub code: Option<i32>,
    pub out: Vec<u8>,
    pub err: String,
    pub timed_out: bool,
    pub overflow: bool,
}

impl Streamed {
    fn empty() -> Self {
        Streamed { code: None, out: Vec::new(), err: String::new(), timed_out: false, overflow: false }
    }
}

enum Ended {
    Exit(Option<i32>),
    Overflow,
}

impl Adb {
    pub fn spawn_shell(&self, id: &str, args: &[String]) -> Option<Child> {
        self.used(id);
        let adb = self.path.lock().unwrap().clone()?;
        let mut list = vec!["-s".to_string(), id.to_string(), "shell".to_string()];
        list.extend(args.iter().cloned());
        spawn(&SpawnSpec { file: adb, args: list, cwd: self.home.clone(), ..Default::default() }, &self.env).ok()
    }

    pub fn spawn_adb(&self, id: &str, args: &[String], stdin: bool) -> Option<Child> {
        self.used(id);
        let adb = self.path.lock().unwrap().clone()?;
        let mut list = vec!["-s".to_string(), id.to_string()];
        list.extend(args.iter().cloned());
        spawn(&SpawnSpec { file: adb, args: list, cwd: self.home.clone(), stdin_pipe: stdin, ..Default::default() }, &self.env).ok()
    }

    pub fn pull(&self, id: &str, remote: &str, local: &str, timeout_ms: u64) -> Option<Collected> {
        self.run(&["-s", id, "pull", remote, local], timeout_ms)
    }

    pub fn exec_out(&self, id: &str, args: &[String], timeout_ms: u64, max: usize) -> Option<Streamed> {
        self.used(id);
        let mut list = vec!["-s".to_string(), id.to_string(), "exec-out".to_string()];
        list.extend(args.iter().cloned());
        self.stream(list, timeout_ms, max, None)
    }

    pub fn shell_input(&self, id: &str, args: &[String], input: &[u8], timeout_ms: u64) -> Option<Streamed> {
        self.used(id);
        let mut list = vec!["-s".to_string(), id.to_string(), "shell".to_string()];
        list.extend(args.iter().cloned());
        self.stream(list, timeout_ms, 64 * 1024, Some(input.to_vec()))
    }

    fn stream(&self, args: Vec<String>, timeout_ms: u64, max: usize, input: Option<Vec<u8>>) -> Option<Streamed> {
        let adb = self.path.lock().unwrap().clone()?;
        let spec = SpawnSpec { file: adb, args, cwd: self.home.clone(), stdin_pipe: input.is_some(), ..Default::default() };
        let Ok(mut child) = spawn(&spec, &self.env) else { return Some(Streamed::empty()) };
        let pid = child.id();
        let out_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let err_buf = Arc::new(Mutex::new(String::new()));
        let (readers_tx, readers_rx) = mpsc::channel::<()>();
        let (end_tx, end_rx) = mpsc::channel::<Ended>();
        let over = Arc::new(AtomicBool::new(false));
        if let Some(mut so) = child.stdout.take() {
            let (buf, done, end, over) = (out_buf.clone(), readers_tx.clone(), end_tx.clone(), over.clone());
            std::thread::spawn(move || {
                let mut chunk = [0u8; 32 * 1024];
                while let Ok(n) = so.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let mut b = buf.lock().unwrap();
                    if b.len() + n > max {
                        drop(b);
                        over.store(true, Ordering::SeqCst);
                        kill_pid_only(pid);
                        let _ = end.send(Ended::Overflow);
                        break;
                    }
                    b.extend_from_slice(&chunk[..n]);
                }
                let _ = done.send(());
            });
        }
        if let Some(mut se) = child.stderr.take() {
            let (buf, done) = (err_buf.clone(), readers_tx.clone());
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = se.read_to_end(&mut bytes);
                let text = String::from_utf8_lossy(&bytes).to_string();
                let chars: Vec<char> = text.chars().collect();
                *buf.lock().unwrap() = chars[chars.len().saturating_sub(4096)..].iter().collect();
                let _ = done.send(());
            });
        }
        if let (Some(data), Some(mut si)) = (input, child.stdin.take()) {
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
        drop(readers_tx);
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            let _ = end_tx.send(Ended::Exit(code));
        });
        let first = end_rx.recv_timeout(Duration::from_millis(timeout_ms));
        let mut r = Streamed::empty();
        match first {
            Ok(Ended::Exit(code)) => {
                r.code = code;
                while readers_rx.recv_timeout(Duration::from_millis(1500)).is_ok() {}
            }
            Ok(Ended::Overflow) => r.overflow = true,
            Err(_) => {
                kill_pid_only(pid);
                r.timed_out = true;
            }
        }
        if over.load(Ordering::SeqCst) && !r.timed_out {
            r.overflow = true;
            r.code = None;
        }
        r.out = out_buf.lock().unwrap().clone();
        r.err = err_buf.lock().unwrap().clone();
        Some(r)
    }
}
