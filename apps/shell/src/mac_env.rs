#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

pub const MARK: &str = "__CCM_PATH__";

pub fn login_shell(shell_env: Option<&str>) -> String {
    shell_env.filter(|s| s.starts_with('/')).unwrap_or("/bin/zsh").to_owned()
}

pub fn login_args() -> Vec<String> {
    vec!["-ilc".into(), format!("printf '{MARK}%s{MARK}' \"$PATH\"")]
}

pub fn parse_marked(stdout: &str) -> Option<&str> {
    let start = stdout.find(MARK)? + MARK.len();
    let len = stdout[start..].find(MARK)?;
    Some(&stdout[start..start + len]).filter(|s| !s.is_empty())
}

pub fn merge_path(found: &str, current: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    found.split(':').chain(current.split(':')).filter(|d| !d.is_empty() && seen.insert(*d)).collect::<Vec<_>>().join(":")
}

#[cfg(target_os = "macos")]
pub fn adopt_login_shell_path() {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let shell = login_shell(std::env::var("SHELL").ok().as_deref());
    let Ok(mut child) = Command::new(&shell).args(login_args()).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return };
    let Some(mut out) = child.stdout.take() else { return };
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out.by_ref().take(256 * 1024).read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    });
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
    let Ok(stdout) = reader.join() else { return };
    if let Some(found) = parse_marked(&stdout) {
        let merged = merge_path(found, &std::env::var("PATH").unwrap_or_default());
        std::env::set_var("PATH", merged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_between_the_marks_wins_over_rc_noise() {
        let out = format!("Last login: today\nwelcome\n{MARK}/opt/homebrew/bin:/usr/bin{MARK}trailing");
        assert_eq!(parse_marked(&out), Some("/opt/homebrew/bin:/usr/bin"));
        assert_eq!(parse_marked("no marks"), None);
        assert_eq!(parse_marked(&format!("{MARK}{MARK}")), None);
        assert_eq!(parse_marked(&format!("{MARK}/a")), None);
    }

    #[test]
    fn login_entries_come_first_without_duplicates() {
        assert_eq!(merge_path("/opt/homebrew/bin:/usr/bin", "/usr/bin:/bin::/usr/sbin"), "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin");
        assert_eq!(merge_path("", "/usr/bin"), "/usr/bin");
    }

    #[test]
    fn the_shell_falls_back_to_zsh() {
        assert_eq!(login_shell(Some("/bin/bash")), "/bin/bash");
        assert_eq!(login_shell(Some("bash")), "/bin/zsh");
        assert_eq!(login_shell(None), "/bin/zsh");
        assert_eq!(login_args()[0], "-ilc");
        assert!(login_args()[1].contains("\"$PATH\""));
    }
}
