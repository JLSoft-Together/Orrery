use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct RunOut {
    pub code: Option<i32>,
    pub timed_out: bool,
    pub ms: u128,
    pub stdout: String,
    pub stderr: String,
}

pub struct RunOpts<'a> {
    pub file: &'a str,
    pub args: &'a [String],
    pub cwd: &'a str,
    pub env: &'a BTreeMap<String, String>,
    pub stdin: Option<&'a [u8]>,
    pub timeout: Duration,
    pub tree: bool,
    pub cap: usize,
}

fn reader(mut r: impl Read + Send + 'static, cap: usize) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 65_536];
        loop {
            match r.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.len() > cap * 2 {
                        let cut = buf.len() - cap;
                        buf.drain(..cut);
                    }
                }
            }
        }
        buf
    })
}

pub fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let pid = child.id().to_string();
        let ok = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            let _ = child.kill();
        }
    }
    #[cfg(not(windows))]
    {
        let pid = child.id() as i32;
        #[cfg(target_os = "macos")]
        let done = unsafe { libc::kill(-pid, libc::SIGKILL) } == 0;
        #[cfg(not(target_os = "macos"))]
        let done = pid < 0;
        if !done {
            let _ = child.kill();
        }
    }
}

fn hide(cmd: &mut Command, tree: bool) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = tree;
        cmd.creation_flags(0x0800_0000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        if tree {
            cmd.process_group(0);
        }
    }
}

pub fn run(o: RunOpts) -> Option<RunOut> {
    let started = Instant::now();
    let mut cmd = Command::new(o.file);
    cmd.args(o.args).current_dir(o.cwd).env_clear().envs(o.env).stdin(if o.stdin.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    hide(&mut cmd, o.tree);
    let mut child = cmd.spawn().ok()?;
    let out = reader(child.stdout.take()?, o.cap);
    let err = reader(child.stderr.take()?, o.cap);
    if let (Some(mut stdin), Some(data)) = (child.stdin.take(), o.stdin) {
        let data = data.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
    }
    let deadline = started + o.timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if Instant::now() >= deadline && !timed_out => {
                timed_out = true;
                if o.tree {
                    kill_tree(&mut child);
                } else {
                    let _ = child.kill();
                }
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => break None,
        }
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let code = if timed_out && !o.tree { None } else { status.and_then(|s| s.code()) };
    Some(RunOut {
        code,
        timed_out,
        ms: started.elapsed().as_millis(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

pub fn env_vars() -> Vec<(String, String)> {
    std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect()
}
