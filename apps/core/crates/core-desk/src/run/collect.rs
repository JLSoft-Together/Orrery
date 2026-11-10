use super::proc::{kill_pid, spawn, SpawnSpec};
use super::types::Env;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Collected {
    pub code: Option<i32>,
    pub out: String,
    pub timed_out: bool,
}

#[derive(Default)]
pub struct CollectOpts {
    pub timeout_ms: Option<u64>,
    pub env: Env,
    pub verbatim: bool,
    pub track: Option<Arc<dyn Fn(Option<u32>) + Send + Sync>>,
    pub kill: Option<fn(u32)>,
}

pub fn collect(file: &str, args: &[String], cwd: &str, base: &Env, opts: &CollectOpts) -> Collected {
    let spec = SpawnSpec { file: file.into(), args: args.to_vec(), cwd: cwd.into(), env: opts.env.clone(), verbatim: opts.verbatim, ..Default::default() };
    let Ok(mut child) = spawn(&spec, base) else {
        return Collected::default();
    };
    let pid = child.id();
    if let Some(t) = &opts.track {
        t(Some(pid));
    }
    let out = Arc::new(Mutex::new(String::new()));
    let (tx, rx) = mpsc::channel::<()>();
    for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
        let (o, t) = (out.clone(), tx.clone());
        pump_lines_raw(pipe, o, t);
    }
    drop(tx);
    let (done_tx, done_rx) = mpsc::channel::<Option<i32>>();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|s| s.code());
        let _ = done_tx.send(code);
    });
    let result = match opts.timeout_ms {
        Some(ms) => match done_rx.recv_timeout(Duration::from_millis(ms)) {
            Ok(code) => Ok(code),
            Err(_) => Err(()),
        },
        None => Ok(done_rx.recv().unwrap_or(None)),
    };
    let (code, timed_out) = match result {
        Ok(code) => (code, false),
        Err(()) => {
            match opts.kill {
                Some(k) => k(pid),
                None => kill_pid(pid, false),
            }
            (None, true)
        }
    };
    if !timed_out {
        while rx.recv_timeout(Duration::from_millis(1500)).is_ok() {}
    }
    if let Some(t) = &opts.track {
        t(None);
    }
    let text = out.lock().unwrap().clone();
    Collected { code, out: text, timed_out }
}

fn pump_lines_raw(mut r: Box<dyn std::io::Read + Send>, out: Arc<Mutex<String>>, done: mpsc::Sender<()>) {
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 16 * 1024];
        let mut pending: Vec<u8> = Vec::new();
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            pending.extend_from_slice(&buf[..n]);
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                Err(_) => pending.len(),
            };
            let chunk: Vec<u8> = pending.drain(..valid).collect();
            out.lock().unwrap().push_str(&String::from_utf8_lossy(&chunk));
        }
        if !pending.is_empty() {
            out.lock().unwrap().push_str(&String::from_utf8_lossy(&pending));
        }
        let _ = done.send(());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn collects_both_pipes_and_times_out() {
        let base = super::super::proc::process_env();
        let r = collect("/bin/sh", &["-c".into(), "echo out; echo err 1>&2; exit 3".into()], "/", &base, &CollectOpts::default());
        assert_eq!(r.code, Some(3));
        assert!(r.out.contains("out") && r.out.contains("err"));
        let t = collect("/bin/sh", &["-c".into(), "sleep 5".into()], "/", &base, &CollectOpts { timeout_ms: Some(100), ..Default::default() });
        assert!(t.timed_out && t.code.is_none());
        let missing = collect("/nope/x", &[], "/", &base, &CollectOpts::default());
        assert_eq!(missing, Collected::default());
    }
}
