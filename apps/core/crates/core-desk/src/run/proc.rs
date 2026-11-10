use super::args::SubstituteVars;
use super::types::Env;
use crate::extensions::status::child_env;
use std::io::Read;
use std::process::{Child, Command, Stdio};

#[derive(Debug, Clone, Default)]
pub struct SpawnSpec {
    pub file: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: Env,
    pub verbatim: bool,
    pub detached: bool,
    pub stdin_pipe: bool,
}

pub fn process_env() -> Env {
    std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect()
}

pub fn run_env(base: &Env, extra: &Env) -> Env {
    let list: Vec<(String, String)> = base.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut env = child_env(&list);
    for (k, v) in extra {
        env.insert(k.clone(), v.clone());
    }
    for k in ["TERM", "COLORTERM", "FORCE_HYPERLINK"] {
        env.remove(k);
    }
    env.insert("NO_COLOR".into(), "1".into());
    env
}

pub fn subst_vars(project_dir: &str, workspace_dir: &str, base: &Env, cfg_env: &Env) -> SubstituteVars {
    SubstituteVars { project_dir: project_dir.to_string(), workspace_dir: workspace_dir.to_string(), env: run_env(base, cfg_env), mask_env: false }
}

pub fn spawn(spec: &SpawnSpec, base: &Env) -> std::io::Result<Child> {
    let mut cmd = Command::new(&spec.file);
    cmd.current_dir(&spec.cwd).env_clear().envs(run_env(base, &spec.env));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(if spec.detached { 0x0800_0000 | 0x0000_0200 } else { 0x0800_0000 });
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
    if spec.detached {
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    } else {
        cmd.stdin(if spec.stdin_pipe { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    cmd.spawn()
}

pub fn kill_pid(pid: u32, sync: bool) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("taskkill");
        c.args(["/T", "/F", "/PID", &pid.to_string()]).creation_flags(0x0800_0000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if sync {
            let _ = c.status();
        } else {
            let _ = c.spawn();
        }
    }
    #[cfg(unix)]
    {
        let _ = sync;
        let r = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if r != 0 {
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    }
}

pub fn kill_pid_only(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).creation_flags(0x0800_0000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

pub fn is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

const MAX_LINE: usize = 64 * 1024;

pub struct LineSplitter {
    rest: String,
    on_line: Box<dyn FnMut(&str) + Send>,
}

impl LineSplitter {
    pub fn new(on_line: impl FnMut(&str) + Send + 'static) -> Self {
        LineSplitter { rest: String::new(), on_line: Box::new(on_line) }
    }

    pub fn push(&mut self, chunk: &str) {
        self.rest.push_str(chunk);
        let mut parts: Vec<String> = self.rest.split('\n').map(String::from).collect();
        self.rest = parts.pop().unwrap_or_default();
        for p in parts {
            (self.on_line)(p.strip_suffix('\r').unwrap_or(&p));
        }
        while crate::js::len(&self.rest) > MAX_LINE {
            let head = crate::js::slice(&self.rest, MAX_LINE);
            self.rest = self.rest[head.len()..].to_string();
            (self.on_line)(&head);
        }
    }

    pub fn flush(&mut self) {
        if !self.rest.is_empty() {
            let r = std::mem::take(&mut self.rest);
            (self.on_line)(&r);
        }
    }
}

pub fn pump_lines<R: Read + Send + 'static>(mut r: R, on_line: impl FnMut(&str) + Send + 'static) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut sp = LineSplitter::new(on_line);
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
            sp.push(&String::from_utf8_lossy(&chunk));
        }
        if !pending.is_empty() {
            sp.push(&String::from_utf8_lossy(&pending));
        }
        sp.flush();
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn splits_lines_and_flushes() {
        let got = Arc::new(Mutex::new(Vec::new()));
        let g = got.clone();
        let mut sp = LineSplitter::new(move |l| g.lock().unwrap().push(l.to_string()));
        sp.push("a\r\nb");
        sp.push("c\nd");
        sp.flush();
        assert_eq!(*got.lock().unwrap(), vec!["a", "bc", "d"]);
    }

    #[test]
    fn env_is_filtered() {
        let mut base = Env::new();
        base.insert("CCM_X".into(), "1".into());
        base.insert("KEEP".into(), "2".into());
        let mut extra = Env::new();
        extra.insert("TERM".into(), "x".into());
        let e = run_env(&base, &extra);
        assert!(!e.contains_key("CCM_X") && !e.contains_key("TERM"));
        assert_eq!(e.get("KEEP").map(String::as_str), Some("2"));
        assert_eq!(e.get("NO_COLOR").map(String::as_str), Some("1"));
    }
}
