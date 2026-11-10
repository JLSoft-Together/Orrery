use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const MAX_FILE: u64 = 256 * 1024;
const MAX_EXEC_OUT: usize = 256 * 1024;

pub trait EnvDeps: Send + Sync {
    fn is_file(&self, file: &str) -> bool;
    fn read_text(&self, file: &str) -> Option<String>;
    fn mode(&self, file: &str) -> Option<u32>;
    fn exec(&self, file: &str, args: &[&str], timeout_ms: u64) -> Option<(String, String)>;
    fn realpath(&self, file: &str) -> String {
        file.to_string()
    }
    fn list_dirs(&self, _dir: &str) -> Option<Vec<String>> {
        None
    }
    fn cache_key(&self) -> Option<u64> {
        None
    }
}

pub struct FsEnvDeps;

impl EnvDeps for FsEnvDeps {
    fn is_file(&self, file: &str) -> bool {
        std::fs::metadata(file).map(|m| m.is_file()).unwrap_or(false)
    }

    fn read_text(&self, file: &str) -> Option<String> {
        let meta = std::fs::metadata(file).ok()?;
        if !meta.is_file() || meta.len() > MAX_FILE {
            return None;
        }
        std::fs::read_to_string(file).ok()
    }

    fn mode(&self, file: &str) -> Option<u32> {
        let meta = std::fs::metadata(file).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            Some(meta.permissions().mode())
        }
        #[cfg(not(unix))]
        {
            Some(if meta.permissions().readonly() { 0o444 } else { 0o666 })
        }
    }

    fn exec(&self, file: &str, args: &[&str], timeout_ms: u64) -> Option<(String, String)> {
        exec_file(file, args, timeout_ms)
    }

    fn realpath(&self, file: &str) -> String {
        crate::preview::paths::real_or_null(file).unwrap_or_else(|| file.to_string())
    }

    fn list_dirs(&self, dir: &str) -> Option<Vec<String>> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Some(Vec::new()) };
        Some(rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir() || t.is_symlink()).unwrap_or(false)).map(|e| e.file_name().to_string_lossy().to_string()).collect())
    }

    fn cache_key(&self) -> Option<u64> {
        Some(0)
    }
}

fn read_capped(mut r: impl Read) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    while let Ok(n) = r.read(&mut chunk) {
        if n == 0 {
            break;
        }
        if buf.len() < MAX_EXEC_OUT {
            buf.extend_from_slice(&chunk[..n.min(MAX_EXEC_OUT - buf.len())]);
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

pub fn exec_file(file: &str, args: &[&str], timeout_ms: u64) -> Option<(String, String)> {
    let mut cmd = Command::new(file);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().ok()?;
    let out = child.stdout.take().map(|p| std::thread::spawn(move || read_capped(p)));
    let err = child.stderr.take().map(|p| std::thread::spawn(move || read_capped(p)));
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait().ok().map(|s| s.success()));
    });
    let ok = match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(status) => status.unwrap_or(false),
        Err(_) => {
            crate::run::proc::kill_pid_only(pid);
            false
        }
    };
    let stdout = out.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err.and_then(|h| h.join().ok()).unwrap_or_default();
    if !ok && stdout.is_empty() && stderr.is_empty() {
        return None;
    }
    Some((stdout, stderr))
}
