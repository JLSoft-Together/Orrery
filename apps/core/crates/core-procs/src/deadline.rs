use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Output {
    pub success: bool,
    pub stdout: String,
}

pub fn run_with_deadline(file: &str, args: &[&str], deadline: Duration) -> Option<Output> {
    let mut child = Command::new(file).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new().name("procs-cmd".into()).spawn(move || {
        let mut buf = Vec::new();
        let ok = stdout.read_to_end(&mut buf).is_ok();
        let _ = tx.send(ok.then_some(buf));
    });
    if reader.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    let end = Instant::now() + deadline;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let out = rx.recv_timeout(end.saturating_duration_since(Instant::now()).max(Duration::from_millis(200))).ok()??;
    Some(Output { success: status.success(), stdout: String::from_utf8_lossy(&out).into_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_killed_at_its_deadline() {
        #[cfg(windows)]
        let slow: &[&str] = &["powershell.exe", "-NoProfile", "-Command", "Start-Sleep -Seconds 20"];
        #[cfg(windows)]
        let quick: &[&str] = &["cmd.exe", "/d", "/c", "echo 12 1"];
        #[cfg(windows)]
        let failing: &[&str] = &["cmd.exe", "/d", "/c", "echo partial& exit 1"];
        #[cfg(unix)]
        let slow: &[&str] = &["/bin/sleep", "20"];
        #[cfg(unix)]
        let quick: &[&str] = &["/bin/echo", "12 1"];
        #[cfg(unix)]
        let failing: &[&str] = &["/bin/sh", "-c", "echo partial; exit 1"];
        let t = Instant::now();
        assert!(run_with_deadline(slow[0], &slow[1..], Duration::from_millis(500)).is_none());
        assert!(t.elapsed() < Duration::from_secs(5), "deadline not honoured: {:?}", t.elapsed());
        let ok = run_with_deadline(quick[0], &quick[1..], Duration::from_secs(10)).unwrap();
        assert!(ok.success);
        assert_eq!(ok.stdout.trim(), "12 1");
        let bad = run_with_deadline(failing[0], &failing[1..], Duration::from_secs(10)).unwrap();
        assert!(!bad.success);
        assert_eq!(bad.stdout.trim(), "partial");
        assert!(run_with_deadline("/definitely/missing/ps", &[], Duration::from_secs(1)).is_none());
    }
}
