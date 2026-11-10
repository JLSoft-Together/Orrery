use super::cds::Record;
use super::download::{jar_name, join_native};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type Spawn = Arc<dyn Fn(&str, &[String], &str) -> std::io::Result<Child> + Send + Sync>;

pub fn daemon_args(daemon_jar: &str, layoutlib_dir: &str, win: bool, jvm_extra: &[String]) -> Vec<String> {
    let sep = if win { ";" } else { ":" };
    let mut a: Vec<String> = ["-Xmx1g", "-XX:+EnableDynamicAgentLoading", "-Djdk.attach.allowAttachSelf=true"].iter().map(|s| s.to_string()).collect();
    a.extend(jvm_extra.iter().cloned());
    a.extend(["-cp".to_string(), format!("{daemon_jar}{sep}{}", join_native(win, layoutlib_dir, &jar_name())), "dev.ccm.layoutdaemon.MainKt".to_string(), "--layoutlib".to_string(), layoutlib_dir.to_string()]);
    a
}

pub fn default_spawn() -> Spawn {
    Arc::new(|java, args, cwd| {
        let base = crate::run::proc::process_env();
        let mut cmd = Command::new(java);
        cmd.args(args).current_dir(cwd).env_clear().envs(crate::run::proc::run_env(&base, &Default::default())).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.spawn()
    })
}

pub fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill").args(["/pid", &pid.to_string(), "/T", "/F"]).creation_flags(0x0800_0000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(unix)]
    unsafe {
        if libc::kill(-(pid as i32), libc::SIGKILL) != 0 {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
}

fn drain(mut r: impl Read + Send + 'static) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while matches!(r.read(&mut buf), Ok(n) if n > 0) {}
    });
}

pub fn run_dump(spawn: &Spawn, java: &str, cache_dir: &str, plan: &Record, deadline: Duration, spawned: &dyn Fn(u32)) -> Option<i32> {
    let mut child = spawn(java, &plan.dump_args, cache_dir).ok()?;
    spawned(child.id());
    drop(child.stdin.take());
    if let Some(o) = child.stdout.take() {
        drain(o);
    }
    if let Some(e) = child.stderr.take() {
        drain(e);
    }
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(s)) => return s.code(),
            Ok(None) => {}
            Err(_) => return None,
        }
        if start.elapsed() > deadline {
            kill_pid(child.id());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
