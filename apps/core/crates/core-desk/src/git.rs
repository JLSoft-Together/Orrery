use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

const MAX_BUFFER: usize = 4 * 1024 * 1024;

pub fn find_git(path_var: &str) -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut dirs: Vec<PathBuf> = path_var.split(sep).filter(|d| !d.is_empty()).map(PathBuf::from).filter(|d| d.is_absolute()).collect();
    if !cfg!(windows) {
        dirs.extend(["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"].map(PathBuf::from));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|f| f.is_file())
}

pub fn git_path() -> Option<&'static PathBuf> {
    static GIT: OnceLock<Option<PathBuf>> = OnceLock::new();
    GIT.get_or_init(|| {
        let path = std::env::var("PATH").or_else(|_| std::env::var("Path")).unwrap_or_default();
        find_git(&path)
    })
    .as_ref()
}

fn hide(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

fn drain(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<(Vec<u8>, bool)> {
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let mut buf = [0u8; 16 * 1024];
        let mut over = false;
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            if out.len() + n > MAX_BUFFER {
                over = true;
            } else {
                out.extend_from_slice(&buf[..n]);
            }
        }
        (out, over)
    })
}

pub fn run_git(cwd: &Path, args: &[&str], timeout: Duration) -> GitResult {
    let Some(git) = git_path() else {
        return GitResult { ok: false, stdout: String::new(), stderr: "git not found".into() };
    };
    let mut cmd = Command::new(git);
    cmd.arg("-C").arg(cwd).args(args);
    cmd.env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0").env("LC_ALL", "C");
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    hide(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return GitResult { ok: false, stdout: String::new(), stderr: e.to_string() },
    };
    let out = drain(child.stdout.take().expect("piped stdout"));
    let err = drain(child.stderr.take().expect("piped stderr"));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };
    let (stdout, over_out) = out.join().unwrap_or_default();
    let (stderr, over_err) = err.join().unwrap_or_default();
    GitResult {
        ok: status.map(|s| s.success()).unwrap_or(false) && !over_out && !over_err,
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nothing_in_empty_path() {
        if cfg!(windows) {
            assert!(find_git("").is_none());
        }
        let r = run_git(&std::env::temp_dir(), &["--version"], Duration::from_secs(20));
        if git_path().is_some() {
            assert!(r.ok && r.stdout.starts_with("git version"), "{r:?}");
        }
    }
}
