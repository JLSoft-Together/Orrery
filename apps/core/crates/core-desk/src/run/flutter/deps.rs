use crate::run::types::Env;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const MAX_TEXT: u64 = 256 * 1024;
const MAX_OUT: usize = 1024 * 1024;
const MAX_DIR_ENTRIES: usize = 200;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait EnvDeps {
    fn is_file(&self, file: &str) -> bool;
    fn read_text(&self, file: &str) -> Option<String>;
    fn readdir(&self, dir: &str) -> Vec<String>;
    fn realpath(&self, file: &str) -> String;
    fn exec(&self, file: &str, args: &[String], timeout_ms: u64, env: &Env, verbatim: bool) -> ExecResult;
}

pub struct FsEnvDeps;

impl EnvDeps for FsEnvDeps {
    fn is_file(&self, file: &str) -> bool {
        std::fs::metadata(file).map(|m| m.is_file()).unwrap_or(false)
    }

    fn read_text(&self, file: &str) -> Option<String> {
        let m = std::fs::metadata(file).ok()?;
        if !m.is_file() || m.len() > MAX_TEXT {
            return None;
        }
        std::fs::read_to_string(file).ok()
    }

    fn readdir(&self, dir: &str) -> Vec<String> {
        std::fs::read_dir(dir).map(|rd| rd.flatten().take(MAX_DIR_ENTRIES).map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default()
    }

    fn realpath(&self, file: &str) -> String {
        crate::preview::paths::real_or_null(file).unwrap_or_else(|| file.to_string())
    }

    fn exec(&self, file: &str, args: &[String], timeout_ms: u64, env: &Env, verbatim: bool) -> ExecResult {
        exec_capture(file, args, timeout_ms, env, verbatim)
    }
}

fn read_capped(mut r: impl Read) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    while let Ok(n) = r.read(&mut chunk) {
        if n == 0 {
            break;
        }
        if buf.len() < MAX_OUT {
            buf.extend_from_slice(&chunk[..n.min(MAX_OUT - buf.len())]);
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

pub fn exec_capture(file: &str, args: &[String], timeout_ms: u64, env: &Env, verbatim: bool) -> ExecResult {
    let mut cmd = Command::new(file);
    cmd.env_clear().envs(env).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
        for a in args {
            if verbatim {
                cmd.raw_arg(a);
            } else {
                cmd.arg(a);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = verbatim;
        cmd.args(args);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let Ok(mut child) = cmd.spawn() else {
        return ExecResult::default();
    };
    let pid = child.id();
    let out = child.stdout.take().map(|s| std::thread::spawn(move || read_capped(s)));
    let err = child.stderr.take().map(|s| std::thread::spawn(move || read_capped(s)));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait().ok().map(|s| s.success()));
    });
    let ok = match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(status) => status.unwrap_or(false),
        Err(_) => {
            crate::run::proc::kill_pid(pid, false);
            false
        }
    };
    let stdout = out.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err.and_then(|h| h.join().ok()).unwrap_or_default();
    ExecResult { ok, stdout, stderr }
}
