use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const RUN_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_OUTPUT: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStopResult {
    Ok,
    NotFound,
    NotRunning,
    Unconfirmed,
    NoCli,
    Failed,
}

impl JobStopResult {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStopResult::Ok => "ok",
            JobStopResult::NotFound => "notFound",
            JobStopResult::NotRunning => "notRunning",
            JobStopResult::Unconfirmed => "unconfirmed",
            JobStopResult::NoCli => "noCli",
            JobStopResult::Failed => "failed",
        }
    }
}

pub struct CliRun {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn is_stoppable_job_id(id: &str) -> bool {
    let b = id.as_bytes();
    (4..=64).contains(&b.len()) && b[0].is_ascii_alphanumeric() && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
}

pub fn claude_cli_candidates(path_var: Option<&str>, home: Option<&str>, win: bool) -> Vec<String> {
    let (sep, delim, exe) = if win { ('\\', ';', "claude.exe") } else { ('/', ':', "claude") };
    let join = |parts: &[&str]| {
        let mut out = String::new();
        for p in parts {
            if !out.is_empty() && !out.ends_with(['\\', '/']) {
                out.push(sep);
            }
            out.push_str(p);
        }
        out
    };
    let mut out = Vec::new();
    for dir in path_var.unwrap_or("").split(delim).filter(|d| !d.is_empty()) {
        out.push(join(&[dir, exe]));
        out.push(join(&[dir, "node_modules", "@anthropic-ai", "claude-code", "bin", exe]));
    }
    if let Some(h) = home.filter(|h| !h.is_empty()) {
        out.push(join(&[h, ".local", "bin", exe]));
    }
    out
}

pub fn find_claude_cli() -> Option<PathBuf> {
    let path = std::env::var("PATH").ok().or_else(|| std::env::var("Path").ok());
    let home = std::env::var("USERPROFILE").ok().or_else(|| std::env::var("HOME").ok());
    claude_cli_candidates(path.as_deref(), home.as_deref(), cfg!(windows)).into_iter().map(PathBuf::from).find(|p| p.is_file())
}

fn read_capped(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.by_ref().take(MAX_OUTPUT as u64).read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

pub fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub fn run_capped(file: &Path, args: &[&str], envs: &[(&str, &str)], timeout: Duration) -> Option<CliRun> {
    let mut cmd = Command::new(file);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = hidden(&mut cmd).spawn().ok()?;
    let out = read_capped(child.stdout.take()?);
    let err = read_capped(child.stderr.take()?);
    let started = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if started.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    Some(CliRun { code, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default() })
}

pub fn stop_outcome(run: &CliRun) -> JobStopResult {
    if run.code == Some(0) {
        return JobStopResult::Ok;
    }
    let output = format!("{}\n{}", run.stdout, run.stderr).to_lowercase();
    if output.contains("no job matching") {
        return JobStopResult::NotFound;
    }
    if output.contains("couldn't confirm") || output.contains("could not confirm") {
        return JobStopResult::Unconfirmed;
    }
    JobStopResult::Failed
}

pub fn stop_background_job(job_id: &str, cli: Option<PathBuf>, run: &dyn Fn(&Path, &[&str]) -> Option<CliRun>) -> JobStopResult {
    if !is_stoppable_job_id(job_id) {
        return JobStopResult::Failed;
    }
    let Some(cli) = cli else { return JobStopResult::NoCli };
    match run(&cli, &["stop", job_id]) {
        Some(r) => stop_outcome(&r),
        None => JobStopResult::Failed,
    }
}

pub fn stop_with_claude(job_id: &str) -> JobStopResult {
    let cli = if is_stoppable_job_id(job_id) { find_claude_cli() } else { None };
    stop_background_job(job_id, cli, &|f, a| run_capped(f, a, &[], RUN_TIMEOUT))
}

pub fn is_daemon_running(claude_root: &Path, alive: &dyn Fn(u32) -> bool) -> bool {
    let Ok(text) = std::fs::read(claude_root.join("daemon").join("roster.json")) else { return true };
    let Ok(v) = serde_json::from_slice::<Value>(&text) else { return true };
    match v.get("supervisorPid").and_then(Value::as_f64) {
        Some(p) if p.fract() == 0.0 && p > 0.0 && p <= f64::from(u32::MAX) => alive(p as u32),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_for_the_real_executable_next_to_path_entries_and_the_npm_package() {
        let list = claude_cli_candidates(Some("C:\\npm;C:\\bin"), Some("C:\\Users\\me"), true);
        assert!(list.contains(&"C:\\npm\\claude.exe".to_owned()));
        assert!(list.contains(&"C:\\npm\\node_modules\\@anthropic-ai\\claude-code\\bin\\claude.exe".to_owned()));
        assert!(list.contains(&"C:\\Users\\me\\.local\\bin\\claude.exe".to_owned()));
        assert!(!list.iter().any(|f| [".cmd", ".ps1", ".bat"].iter().any(|e| f.to_lowercase().ends_with(e))));
        assert_eq!(claude_cli_candidates(Some("/usr/bin"), Some("/home/me"), false), ["/usr/bin/claude", "/usr/bin/node_modules/@anthropic-ai/claude-code/bin/claude", "/home/me/.local/bin/claude"]);
    }

    #[test]
    fn runs_claude_stop_without_a_shell_and_maps_the_outcome() {
        let calls = std::cell::RefCell::new(Vec::new());
        let ok = |f: &Path, a: &[&str]| {
            calls.borrow_mut().push(format!("{} {}", f.display(), a.join(" ")));
            Some(CliRun { code: Some(0), stdout: String::new(), stderr: String::new() })
        };
        assert_eq!(stop_background_job("9261d07b", Some("C:\\claude.exe".into()), &ok), JobStopResult::Ok);
        assert_eq!(calls.borrow().as_slice(), ["C:\\claude.exe stop 9261d07b"]);
        let reply = |code: i32, stdout: &'static str, stderr: &'static str| move |_: &Path, _: &[&str]| Some(CliRun { code: Some(code), stdout: stdout.into(), stderr: stderr.into() });
        assert_eq!(stop_background_job("9261d07b", Some("c".into()), &reply(1, "No job matching 'x'. Run 'claude agents'", "")), JobStopResult::NotFound);
        assert_eq!(stop_background_job("9261d07b", Some("c".into()), &reply(2, "", "boom")), JobStopResult::Failed);
        assert_eq!(stop_background_job("9261d07b", Some("c".into()), &reply(1, "couldn't confirm 9261d07b was stopped", "")), JobStopResult::Unconfirmed);
        assert_eq!(stop_background_job("9261d07b", None, &ok), JobStopResult::NoCli);
        assert_eq!(stop_background_job("--all", Some("c".into()), &ok), JobStopResult::Failed);
        assert_eq!(stop_background_job("-x123", Some("c".into()), &ok), JobStopResult::Failed);
    }

    #[test]
    fn daemon_roster_decides_running() {
        let dir = std::env::temp_dir().join(format!("ccm-roster-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("daemon")).unwrap();
        assert!(is_daemon_running(&dir.join("none"), &|_| false));
        std::fs::write(dir.join("daemon").join("roster.json"), r#"{"supervisorPid":4242}"#).unwrap();
        assert!(!is_daemon_running(&dir, &|p| p != 4242));
        assert!(is_daemon_running(&dir, &|_| true));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
