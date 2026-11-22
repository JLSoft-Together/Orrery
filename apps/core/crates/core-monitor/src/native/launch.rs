use std::path::{Path, PathBuf};

use crate::monitor::{LaunchError, LaunchMode, TerminalApp};

const SHELL_LINE: &str = "set cmdLine to \"cd \" & quoted form of (item 1 of argv) & \" && \" & (item 2 of argv)";

pub fn terminal_launch_script() -> String {
    format!("on run argv\n  {SHELL_LINE}\n  tell application id \"com.apple.Terminal\"\n    do script cmdLine\n    activate\n  end tell\nend run")
}

pub fn iterm_launch_script() -> String {
    format!("on run argv\n  {SHELL_LINE}\n  tell application id \"com.googlecode.iterm2\"\n    set w to (create window with default profile)\n    tell current session of w to write text cmdLine\n    activate\n  end tell\nend run")
}

pub fn claude_command(mode: LaunchMode) -> &'static str {
    match mode {
        LaunchMode::Continue => "claude --continue",
        LaunchMode::New => "claude",
    }
}

pub fn mac_launch_args(app: TerminalApp, dir: &str, mode: LaunchMode) -> Vec<String> {
    let script = if app == TerminalApp::Iterm { iterm_launch_script() } else { terminal_launch_script() };
    vec!["-e".into(), script, dir.into(), claude_command(mode).into()]
}

pub fn windows_powershell(system_root: Option<&str>) -> String {
    format!("{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe", system_root.filter(|s| !s.is_empty()).unwrap_or("C:\\Windows").trim_end_matches('\\'))
}

pub fn wt_safe(dir: &str) -> bool {
    !dir.contains(';') && !dir.contains('"')
}

pub fn find_on_path(name: &str, path: Option<&str>, sep: char) -> Option<PathBuf> {
    path?.split(sep).filter(|d| !d.is_empty() && Path::new(d).is_absolute()).map(|d| Path::new(d).join(name)).find(|f| f.is_file())
}

pub fn windows_terminal(local_app_data: Option<&str>, path: Option<&str>) -> Option<PathBuf> {
    if let Some(root) = local_app_data.filter(|s| !s.is_empty()) {
        let alias = Path::new(root).join("Microsoft").join("WindowsApps").join("wt.exe");
        if std::fs::symlink_metadata(&alias).is_ok() {
            return Some(alias);
        }
    }
    find_on_path("wt.exe", path, ';')
}

fn claude_args(mode: LaunchMode) -> Vec<String> {
    vec!["-NoLogo".into(), "-NoExit".into(), "-Command".into(), claude_command(mode).into()]
}

pub fn windows_plan(dir: &str, mode: LaunchMode, wt: Option<&Path>, powershell: &str) -> Vec<(String, Vec<String>)> {
    let mut plan = Vec::new();
    if let Some(wt) = wt.filter(|_| wt_safe(dir)) {
        let mut args = vec!["-w".into(), "0".into(), "nt".into(), "-d".into(), dir.into(), powershell.into()];
        args.extend(claude_args(mode));
        plan.push((wt.to_string_lossy().into_owned(), args));
    }
    plan.push((powershell.into(), claude_args(mode)));
    plan
}

pub fn mac_launch_error(stderr: &str) -> LaunchError {
    use super::osa::{parse_osa_error, OsaError};
    match parse_osa_error(stderr) {
        OsaError::Denied => LaunchError::Denied,
        OsaError::NotInstalled => LaunchError::NotInstalled,
        _ => LaunchError::Failed,
    }
}

#[cfg(windows)]
pub fn launch(dir: &str, mode: LaunchMode, _app: TerminalApp) -> Option<Result<(), LaunchError>> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    if !Path::new(dir).is_dir() {
        return Some(Err(LaunchError::NotFound));
    }
    let var = |k: &str| std::env::var(k).ok();
    let powershell = windows_powershell(var("SystemRoot").as_deref());
    let wt = windows_terminal(var("LOCALAPPDATA").as_deref(), var("PATH").as_deref());
    for (file, args) in windows_plan(dir, mode, wt.as_deref(), &powershell) {
        let spawned = Command::new(&file)
            .args(&args)
            .current_dir(dir)
            .env_remove("ELECTRON_RUN_AS_NODE")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0000_0010 | 0x0000_0200)
            .spawn();
        if spawned.is_ok() {
            return Some(Ok(()));
        }
    }
    Some(Err(LaunchError::Failed))
}

#[cfg(target_os = "macos")]
pub fn launch(dir: &str, mode: LaunchMode, app: TerminalApp) -> Option<Result<(), LaunchError>> {
    use std::time::Duration;
    if !Path::new(dir).is_dir() {
        return Some(Err(LaunchError::NotFound));
    }
    if super::osa::has_control_chars(dir) {
        return Some(Err(LaunchError::Failed));
    }
    let args = mac_launch_args(app, dir, mode);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(r) = crate::job_stop::run_capped(Path::new("/usr/bin/osascript"), &refs, &[], Duration::from_secs(60)) else { return Some(Err(LaunchError::Failed)) };
    Some(if r.code == Some(0) { Ok(()) } else { Err(mac_launch_error(&r.stderr)) })
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn launch(_dir: &str, _mode: LaunchMode, _app: TerminalApp) -> Option<Result<(), LaunchError>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_tries_windows_terminal_then_a_console() {
        let ps = windows_powershell(Some("C:\\Windows\\"));
        assert_eq!(ps, "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        assert_eq!(windows_powershell(None), ps);
        let wt = PathBuf::from("C:\\Users\\me\\AppData\\Local\\Microsoft\\WindowsApps\\wt.exe");
        let plan = windows_plan("C:\\work\\app", LaunchMode::Continue, Some(&wt), &ps);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].1, vec!["-w", "0", "nt", "-d", "C:\\work\\app", ps.as_str(), "-NoLogo", "-NoExit", "-Command", "claude --continue"]);
        assert_eq!(plan[1], (ps.clone(), vec!["-NoLogo".to_owned(), "-NoExit".into(), "-Command".into(), "claude --continue".into()]));
        assert_eq!(windows_plan("C:\\a;b", LaunchMode::New, Some(&wt), &ps).len(), 1);
        assert_eq!(windows_plan("C:\\work", LaunchMode::New, None, &ps)[0].1[3], "claude");
    }

    #[test]
    fn mac_passes_the_folder_as_an_argument_never_in_the_script() {
        let args = mac_launch_args(TerminalApp::Iterm, "/Users/me/my \"app\"", LaunchMode::New);
        assert_eq!(args[0], "-e");
        assert!(args[1].contains("com.googlecode.iterm2"));
        assert!(!args[1].contains("my "));
        assert_eq!(args[2], "/Users/me/my \"app\"");
        assert_eq!(args[3], "claude");
        assert!(mac_launch_args(TerminalApp::Terminal, "/a", LaunchMode::Continue)[1].contains("com.apple.Terminal"));
        assert_eq!(mac_launch_error("(-1743)"), LaunchError::Denied);
        assert_eq!(mac_launch_error("(-1728)"), LaunchError::NotInstalled);
        assert_eq!(mac_launch_error("(-128)"), LaunchError::Failed);
    }

    #[test]
    fn path_lookup_skips_relative_entries() {
        let dir = std::env::temp_dir().join("orrery-native-path-x7");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("tool.bin"), b"x");
        let sep = if cfg!(windows) { ';' } else { ':' };
        let path = format!(".{sep}{}", dir.display());
        assert_eq!(find_on_path("tool.bin", Some(&path), sep), Some(dir.join("tool.bin")));
        assert_eq!(find_on_path("tool.bin", Some("."), sep), None);
        assert_eq!(find_on_path("missing.bin", Some(&path), sep), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
