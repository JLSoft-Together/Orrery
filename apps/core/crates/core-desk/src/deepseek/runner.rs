use crate::claude::proc::kill_tree;
use crate::preview::format::{with_args, Resolved};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const OUT_CAP: usize = 256 * 1024;

#[derive(Debug, Default, Clone)]
pub struct RunResult {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub cancelled: bool,
    pub spawn_error: bool,
}

pub struct RunOpts<'a> {
    pub cwd: &'a str,
    pub timeout: Duration,
    pub stdin: Option<String>,
    pub on_line: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel: Option<Arc<AtomicBool>>,
}

pub type Runner = Arc<dyn Fn(&Resolved, &[String], &RunOpts) -> RunResult + Send + Sync>;

struct Feed<'a> {
    partial: Mutex<String>,
    on_line: Option<&'a (dyn Fn(&str) + Send + Sync)>,
}

impl Feed<'_> {
    fn push(&self, chunk: &str) {
        let Some(cb) = self.on_line else { return };
        let mut partial = self.partial.lock().unwrap();
        partial.push_str(chunk);
        let text = std::mem::take(&mut *partial);
        let mut rest = text.as_str();
        while let Some(i) = rest.find(['\r', '\n']) {
            let line = &rest[..i];
            if !line.trim().is_empty() {
                cb(line);
            }
            let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
            rest = &rest[i + skip..];
        }
        partial.push_str(rest);
    }

    fn flush(&self) {
        if let Some(cb) = self.on_line {
            let p = self.partial.lock().unwrap();
            if !p.trim().is_empty() {
                cb(&p);
            }
        }
    }
}

fn pump<'s, 'a: 's>(scope: &'s std::thread::Scope<'s, 'a>, mut r: impl Read + Send + 's, sink: &'s Mutex<String>, feed: &'s Feed<'a>) {
    scope.spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        let mut pending: Vec<u8> = Vec::new();
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            pending.extend_from_slice(&buf[..n]);
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(e) => e.valid_up_to(),
            };
            let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
            pending.drain(..valid);
            if pending.len() > 4 {
                pending.clear();
            }
            {
                let mut s = sink.lock().unwrap();
                if s.len() < OUT_CAP {
                    s.push_str(&text);
                }
            }
            feed.push(&text);
        }
    });
}

pub fn default_runner(env: BTreeMap<String, String>) -> Runner {
    Arc::new(move |base, extra, opts| run(&env, base, extra, opts))
}

fn run(env: &BTreeMap<String, String>, base: &Resolved, extra: &[String], opts: &RunOpts) -> RunResult {
    let cmd_spec = if extra.is_empty() {
        base.clone()
    } else {
        match with_args(base, extra) {
            Some(c) => c,
            None => return RunResult { spawn_error: true, ..Default::default() },
        }
    };
    let mut cmd = Command::new(&cmd_spec.file);
    cmd.current_dir(opts.cwd).env_clear().envs(env);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
        for a in &cmd_spec.args {
            if cmd_spec.verbatim {
                cmd.raw_arg(a);
            } else {
                cmd.arg(a);
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.args(&cmd_spec.args);
        cmd.process_group(0);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let Ok(mut child) = cmd.spawn() else { return RunResult { spawn_error: true, ..Default::default() } };
    let stdin = child.stdin.take();
    let data = opts.stdin.clone();
    let (out_buf, err_buf) = (Mutex::new(String::new()), Mutex::new(String::new()));
    let feed = Feed { partial: Mutex::new(String::new()), on_line: opts.on_line };
    let mut timed_out = false;
    let mut cancelled = false;
    let mut code = None;
    std::thread::scope(|scope| {
        pump(scope, child.stdout.take().expect("stdout"), &out_buf, &feed);
        pump(scope, child.stderr.take().expect("stderr"), &err_buf, &feed);
        if let Some(mut stdin) = stdin {
            scope.spawn(move || {
                if let Some(d) = data {
                    let _ = stdin.write_all(d.as_bytes());
                }
            });
        }
        let deadline = Instant::now() + opts.timeout;
        let mut killed = false;
        loop {
            match child.try_wait() {
                Ok(Some(s)) => {
                    code = s.code();
                    break;
                }
                Ok(None) => {}
                Err(_) => break,
            }
            if !killed {
                if opts.cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)) {
                    cancelled = true;
                    killed = true;
                    kill_tree(&mut child);
                } else if Instant::now() >= deadline {
                    timed_out = true;
                    killed = true;
                    kill_tree(&mut child);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    feed.flush();
    let stdout = out_buf.into_inner().unwrap_or_default();
    let stderr = err_buf.into_inner().unwrap_or_default();
    RunResult { code, stdout, stderr, timed_out, cancelled, spawn_error: false }
}
