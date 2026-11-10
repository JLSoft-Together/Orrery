use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecResult {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub timed_out: bool,
    pub bytes: Option<Vec<u8>>,
}

impl ExecResult {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }

    pub fn no_git(&self) -> bool {
        self.code.is_none() && self.stderr == "git not found"
    }
}

#[derive(Clone)]
pub struct ExecOpts {
    pub input: Option<String>,
    pub max_bytes: usize,
    pub timeout_ms: u64,
    pub kill_tree: bool,
    pub env: Vec<(String, String)>,
    pub signal: Option<Arc<AtomicBool>>,
    pub binary: bool,
}

impl Default for ExecOpts {
    fn default() -> Self {
        ExecOpts { input: None, max_bytes: 8 * 1024 * 1024, timeout_ms: 30_000, kill_tree: false, env: Vec::new(), signal: None, binary: false }
    }
}

impl ExecOpts {
    pub fn timeout(ms: u64) -> Self {
        ExecOpts { timeout_ms: ms, ..Default::default() }
    }

    pub fn max(bytes: usize) -> Self {
        ExecOpts { max_bytes: bytes, ..Default::default() }
    }

    pub fn with_max(mut self, bytes: usize) -> Self {
        self.max_bytes = bytes;
        self
    }

    pub fn with_input(mut self, input: &str) -> Self {
        self.input = Some(input.to_string());
        self
    }
}

const GIT_ENV: [(&str, &str); 4] = [("GIT_TERMINAL_PROMPT", "0"), ("GIT_OPTIONAL_LOCKS", "0"), ("LC_ALL", "C"), ("GIT_LITERAL_PATHSPECS", "1")];
const GIT_CONFIG: [&str; 10] = ["-c", "core.quotePath=false", "-c", "diff.noprefix=false", "-c", "diff.mnemonicPrefix=false", "-c", "color.ui=false", "-c", "log.showSignature=false"];

fn hide(cmd: &mut Command, group: bool) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = group;
        cmd.creation_flags(0x0800_0000);
    }
    #[cfg(unix)]
    if group {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(not(any(windows, unix)))]
    let _ = (cmd, group);
}

pub fn kill_tree(child: &mut Child) {
    let pid = child.id();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let ok = Command::new("taskkill")
            .args(["/pid", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            let _ = child.kill();
        }
    }
    #[cfg(unix)]
    {
        let r = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if r != 0 {
            let _ = child.kill();
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        let _ = child.kill();
    }
}

fn read_capped(mut r: impl Read + Send + 'static, max: usize, over: Arc<AtomicBool>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let mut buf = [0u8; 32 * 1024];
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            if over.load(Ordering::SeqCst) {
                continue;
            }
            if out.len() + n > max {
                out.extend_from_slice(&buf[..max - out.len()]);
                over.store(true, Ordering::SeqCst);
                continue;
            }
            out.extend_from_slice(&buf[..n]);
        }
        out
    })
}

fn read_err(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut out = String::new();
        let mut buf = [0u8; 16 * 1024];
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            if crate::js::len(&out) < 16_384 {
                out.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        }
        out
    })
}

pub fn exec_git(cwd: &Path, args: &[&str], opts: &ExecOpts) -> ExecResult {
    let Some(git) = crate::git::git_path() else {
        return ExecResult { stderr: "git not found".into(), ..Default::default() };
    };
    let mut full: Vec<&str> = GIT_CONFIG.to_vec();
    full.push("-C");
    let cwd_s = cwd.to_string_lossy().to_string();
    full.push(&cwd_s);
    full.extend_from_slice(args);
    exec_file(git, &full, opts, true)
}

pub fn exec_file(file: &Path, args: &[&str], opts: &ExecOpts, git_env: bool) -> ExecResult {
    let mut cmd = Command::new(file);
    cmd.args(args);
    if git_env {
        for (k, v) in GIT_ENV {
            cmd.env(k, v);
        }
    }
    for (k, v) in &opts.env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    hide(&mut cmd, opts.kill_tree);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return ExecResult { stderr: e.to_string(), ..Default::default() },
    };
    let over = Arc::new(AtomicBool::new(false));
    let out = read_capped(child.stdout.take().expect("stdout"), opts.max_bytes, over.clone());
    let err = read_err(child.stderr.take().expect("stderr"));
    if let Some(mut stdin) = child.stdin.take() {
        let input = opts.input.clone().unwrap_or_default();
        std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
    }
    let start = Instant::now();
    let timeout = Duration::from_millis(opts.timeout_ms);
    let mut timed_out = false;
    let mut aborted = false;
    let mut wait = 1u64;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {}
            Err(_) => break None,
        }
        if over.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        if opts.signal.as_ref().map(|s| s.load(Ordering::SeqCst)).unwrap_or(false) {
            aborted = true;
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            if opts.kill_tree {
                kill_tree(&mut child);
            } else {
                let _ = child.kill();
            }
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(wait));
        wait = (wait * 2).min(10);
    };
    let truncated = over.load(Ordering::SeqCst);
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let code = if truncated && !timed_out && !aborted {
        Some(0)
    } else if timed_out || aborted {
        None
    } else {
        status.and_then(|s| s.code())
    };
    let (text, bytes) = if opts.binary { (String::new(), Some(stdout)) } else { (String::from_utf8_lossy(&stdout).to_string(), None) };
    ExecResult { code, stdout: text, stderr, truncated, timed_out, bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_git_and_caps_output() {
        if crate::git::git_path().is_none() {
            return;
        }
        let dir = std::env::temp_dir();
        let r = exec_git(&dir, &["--version"], &ExecOpts::default());
        assert!(r.ok() && r.stdout.starts_with("git version"), "{r:?}");
        let r = exec_git(&dir, &["--version"], &ExecOpts::max(4));
        assert!(r.truncated && r.ok() && r.stdout == "git ", "{r:?}");
        let r = exec_git(&dir, &["hash-object", "-t", "tree", "--stdin"], &ExecOpts::default().with_input(""));
        assert!(r.ok() && r.stdout.trim().len() >= 40, "{r:?}");
    }
}
