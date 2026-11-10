use super::exec::kill_tree;
use crate::extensions::status::child_env;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Spec {
    pub file: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub verbatim: bool,
}

pub struct Opts {
    pub input: Option<Vec<u8>>,
    pub timeout_ms: u64,
    pub max_stdout: usize,
    pub merged_cap: usize,
    pub cancel: Option<Arc<AtomicBool>>,
}

#[derive(Debug, PartialEq)]
pub enum End {
    Exit(Option<i32>),
    Timeout,
    Cancelled,
    TooLarge,
}

pub struct Out {
    pub end: End,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub merged: Vec<u8>,
}

pub fn run_env_raw() -> BTreeMap<String, String> {
    std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect()
}

pub fn run_env() -> BTreeMap<String, String> {
    let vars: Vec<(String, String)> = std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
    let mut env = child_env(&vars);
    for k in ["TERM", "COLORTERM", "FORCE_HYPERLINK"] {
        env.remove(k);
    }
    env
}

fn pump(mut r: impl Read + Send + 'static, keep: Option<usize>, over: Option<Arc<AtomicBool>>, merged: Arc<Mutex<Vec<u8>>>, merged_cap: usize) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut own: Vec<u8> = Vec::new();
        let mut buf = [0u8; 32 * 1024];
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            {
                let mut m = merged.lock().unwrap();
                m.extend_from_slice(&buf[..n]);
                if m.len() > merged_cap * 2 {
                    let cut = m.len() - merged_cap;
                    m.drain(..cut);
                }
            }
            match keep {
                Some(usize::MAX) => {}
                Some(max) => {
                    if over.as_ref().is_some_and(|o| o.load(Ordering::SeqCst)) {
                        continue;
                    }
                    if own.len() + n > max {
                        if let Some(o) = &over {
                            o.store(true, Ordering::SeqCst);
                        }
                        continue;
                    }
                    own.extend_from_slice(&buf[..n]);
                }
                None => {
                    own.extend_from_slice(&buf[..n]);
                    if own.len() > 16 * 1024 {
                        let cut = own.len() - 16 * 1024;
                        own.drain(..cut);
                    }
                }
            }
        }
        own
    })
}

pub fn run(spec: &Spec, opts: &Opts) -> std::io::Result<Out> {
    let mut cmd = Command::new(&spec.file);
    cmd.current_dir(&spec.cwd).env_clear().envs(run_env());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
        for a in &spec.args {
            if spec.verbatim {
                cmd.raw_arg(a);
            } else {
                cmd.arg(a);
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.args(&spec.args);
        cmd.process_group(0);
    }
    cmd.stdin(if opts.input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let over = Arc::new(AtomicBool::new(false));
    let merged = Arc::new(Mutex::new(Vec::new()));
    let out = pump(child.stdout.take().expect("stdout"), Some(opts.max_stdout), Some(over.clone()), merged.clone(), opts.merged_cap);
    let err = pump(child.stderr.take().expect("stderr"), None, None, merged.clone(), opts.merged_cap);
    if let (Some(mut stdin), Some(input)) = (child.stdin.take(), opts.input.clone()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let start = Instant::now();
    let deadline = Duration::from_millis(opts.timeout_ms);
    let mut wait = 1u64;
    let mut forced: Option<End> = None;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {}
            Err(_) => break None,
        }
        if over.load(Ordering::SeqCst) {
            forced = Some(End::TooLarge);
        } else if opts.cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)) {
            forced = Some(End::Cancelled);
        } else if start.elapsed() >= deadline {
            forced = Some(End::Timeout);
        }
        if forced.is_some() {
            kill_tree(&mut child);
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(wait));
        wait = (wait * 2).min(10);
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let merged = std::mem::take(&mut *merged.lock().unwrap());
    let end = forced.unwrap_or_else(|| End::Exit(status.and_then(|s| s.code())));
    Ok(Out { end, stdout, stderr, merged })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> Spec {
        Spec { file: "/bin/sh".into(), args: vec!["-c".into(), script.into()], cwd: std::env::temp_dir().to_string_lossy().to_string(), verbatim: false }
    }

    fn opts() -> Opts {
        Opts { input: None, timeout_ms: 5_000, max_stdout: 1024, merged_cap: 1024, cancel: None }
    }

    #[test]
    fn runs_captures_and_ends() {
        let o = run(&sh("echo out; echo err 1>&2; exit 3"), &opts()).unwrap();
        assert_eq!((o.end, o.stdout, o.stderr), (End::Exit(Some(3)), b"out\n".to_vec(), b"err\n".to_vec()));
        assert!(o.merged.len() == 8);
        let o = run(&sh("cat"), &Opts { input: Some(b"hello".to_vec()), ..opts() }).unwrap();
        assert_eq!(o.stdout, b"hello".to_vec());
        let o = run(&sh("sleep 5"), &Opts { timeout_ms: 80, ..opts() }).unwrap();
        assert_eq!(o.end, End::Timeout);
        let flag = Arc::new(AtomicBool::new(false));
        let f2 = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(60));
            f2.store(true, Ordering::SeqCst);
        });
        let o = run(&sh("sleep 5"), &Opts { cancel: Some(flag), ..opts() }).unwrap();
        assert_eq!(o.end, End::Cancelled);
        let o = run(&sh("head -c 5000 /dev/zero; sleep 5"), &Opts { max_stdout: 100, ..opts() }).unwrap();
        assert_eq!(o.end, End::TooLarge);
        assert!(run(&Spec { file: "/no/such/bin".into(), ..sh("") }, &opts()).is_err());
    }
}
