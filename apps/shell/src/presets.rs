use std::path::{Path, PathBuf};

use base64::Engine;

pub const PRESETS: [&str; 3] = ["claude", "codex", "shell"];

pub const PS_CWD_HOOK: &str = "if (-not $global:__AgentMonitorPrompt) {\n$global:__AgentMonitorPrompt = $function:prompt\nfunction global:prompt {\n$out = (& $global:__AgentMonitorPrompt) -join ''\n$loc = $executionContext.SessionState.Path.CurrentLocation\nif ($loc.Provider.Name -eq 'FileSystem') { $out += \"$([char]27)]9;9;`\"$($loc.ProviderPath)`\"$([char]27)\\\" }\n$out\n}\n}";

pub const BASH_PROMPT_COMMAND: &str = "printf '\\e]7;file://%s%s\\e\\\\' \"$HOSTNAME\" \"$PWD\"";

const AGENT_SESSION_VARS: [&str; 14] = [
    "CLAUDECODE",
    "CLAUDE_PID",
    "CLAUDE_EFFORT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_SSE_PORT",
    "CODEX_THREAD_ID",
    "CODEX_SANDBOX",
    "CODEX_SANDBOX_NETWORK_DISABLED",
];

#[derive(Debug, Clone, PartialEq)]
pub struct LaunchSpec {
    pub file: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn is_session_id(v: &str) -> bool {
    let groups: Vec<&str> = v.split('-').collect();
    groups.len() == 5 && groups.iter().zip([8, 4, 4, 4, 12]).all(|(g, n)| g.len() == n && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn encode_powershell(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn safe_shell(shell_env: Option<&str>, mac: bool) -> String {
    match shell_env {
        Some(s) if s.starts_with('/') && !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_./+-".contains(&b)) => s.to_owned(),
        _ if mac => "/bin/zsh".to_owned(),
        _ => "/bin/bash".to_owned(),
    }
}

fn agent_command(preset: &str, resume: Option<&str>, session: Option<&str>) -> String {
    let resume = resume.filter(|r| is_session_id(r));
    match (preset, resume) {
        ("codex", Some(r)) => format!("codex resume {r}"),
        ("codex", None) => "codex".to_owned(),
        (_, Some(r)) => format!("claude --resume {r}"),
        (_, None) => match session.filter(|s| is_session_id(s)) {
            Some(s) => format!("claude --session-id {s}"),
            None => "claude".to_owned(),
        },
    }
}

pub fn windows_powershell(system_root: Option<&str>) -> String {
    let root = system_root.filter(|r| !r.is_empty()).unwrap_or("C:\\Windows").trim_end_matches('\\');
    format!("{root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
}

pub fn preset_command(preset: &str, windows: bool, env: &dyn Fn(&str) -> Option<String>, resume: Option<&str>, session: Option<&str>) -> LaunchSpec {
    if windows {
        let file = windows_powershell(env("SystemRoot").or_else(|| env("SYSTEMROOT")).as_deref());
        let script = if preset == "shell" { PS_CWD_HOOK.to_owned() } else { format!("{PS_CWD_HOOK}; {}", agent_command(preset, resume, session)) };
        let args = vec!["-NoLogo".into(), "-NoExit".into(), "-EncodedCommand".into(), encode_powershell(&script)];
        return LaunchSpec { file, args, env: Vec::new() };
    }
    let shell = safe_shell(env("SHELL").as_deref(), cfg!(target_os = "macos"));
    let bash = Path::new(&shell).file_name().is_some_and(|n| n == "bash");
    let hook = if bash { vec![("PROMPT_COMMAND".to_owned(), BASH_PROMPT_COMMAND.to_owned())] } else { Vec::new() };
    if preset == "shell" {
        return LaunchSpec { file: shell, args: vec!["-l".into()], env: hook };
    }
    let command = format!("{}; exec {shell} -l", agent_command(preset, resume, session));
    LaunchSpec { file: shell, args: vec!["-l".into(), "-i".into(), "-c".into(), command], env: hook }
}

pub fn child_env(vars: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vars
        .filter(|(k, _)| {
            let u = k.to_ascii_uppercase();
            !(["CCM_", "ORRERY_", "GOLAIM_", "ELECTRON_", "WEBVIEW2_"].iter().any(|p| u.starts_with(p)) || u == "NODE_OPTIONS" || AGENT_SESSION_VARS.contains(&u.as_str()))
        })
        .filter(|(k, _)| !["TERM", "COLORTERM", "FORCE_HYPERLINK"].contains(&k.as_str()))
        .collect();
    out.push(("TERM".into(), "xterm-256color".into()));
    out.push(("COLORTERM".into(), "truecolor".into()));
    out.push(("FORCE_HYPERLINK".into(), "1".into()));
    out
}

fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix("\\\\?\\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
    }
}

fn unc(s: &str) -> bool {
    s.starts_with("\\\\") || s.starts_with("//")
}

pub fn resolve_cwd(cwd: Option<&str>, home: &Path) -> Option<PathBuf> {
    let Some(raw) = cwd.filter(|c| !c.is_empty()) else { return Some(home.to_path_buf()) };
    if unc(raw) || !Path::new(raw).is_absolute() {
        return None;
    }
    let real = strip_verbatim(std::fs::canonicalize(raw).ok()?);
    (!unc(&real.to_string_lossy()) && real.is_dir()).then_some(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_bare_uuids() {
        assert!(is_session_id("0f8fad5b-d9cb-469f-a165-70867728950e"));
        assert!(!is_session_id("0f8fad5b-d9cb-469f-a165-70867728950"));
        assert!(!is_session_id("0f8fad5b-d9cb-469f-a165-70867728950e; rm -rf /"));
        assert!(!is_session_id(""));
    }

    #[test]
    fn powershell_gets_utf16_base64() {
        assert_eq!(encode_powershell("ab"), "YQBiAA==");
    }

    #[test]
    fn windows_shell_and_agents_run_in_powershell_with_the_cwd_hook() {
        let env = |k: &str| (k == "SystemRoot").then(|| "D:\\Win".to_owned());
        let shell = preset_command("shell", true, &env, None, None);
        assert_eq!(shell.file, "D:\\Win\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        assert_eq!(shell.args[..3], ["-NoLogo", "-NoExit", "-EncodedCommand"]);
        assert_eq!(shell.args[3], encode_powershell(PS_CWD_HOOK));
        let id = "0f8fad5b-d9cb-469f-a165-70867728950e";
        let claude = preset_command("claude", true, &env, None, Some(id));
        assert_eq!(claude.args[3], encode_powershell(&format!("{PS_CWD_HOOK}; claude --session-id {id}")));
        let codex = preset_command("codex", true, &env, Some(id), None);
        assert_eq!(codex.args[3], encode_powershell(&format!("{PS_CWD_HOOK}; codex resume {id}")));
        let bad = preset_command("claude", true, &env, Some("x; calc"), None);
        assert_eq!(bad.args[3], encode_powershell(&format!("{PS_CWD_HOOK}; claude")));
    }

    #[test]
    fn posix_shells_are_checked_and_bash_reports_its_folder() {
        assert_eq!(safe_shell(Some("/usr/local/bin/fish"), true), "/usr/local/bin/fish");
        assert_eq!(safe_shell(Some("fish"), true), "/bin/zsh");
        assert_eq!(safe_shell(Some("/bin/sh; x"), false), "/bin/bash");
        let env = |k: &str| (k == "SHELL").then(|| "/bin/bash".to_owned());
        let shell = preset_command("shell", false, &env, None, None);
        assert_eq!((shell.file.as_str(), shell.args.clone()), ("/bin/bash", vec!["-l".to_owned()]));
        assert_eq!(shell.env, vec![("PROMPT_COMMAND".to_owned(), BASH_PROMPT_COMMAND.to_owned())]);
        let claude = preset_command("claude", false, &env, None, None);
        assert_eq!(claude.args, ["-l", "-i", "-c", "claude; exec /bin/bash -l"]);
    }

    #[test]
    fn child_env_drops_our_secrets_and_agent_vars() {
        let vars = [("CCM_APP_KEY", "k"), ("Path", "p"), ("ORRERY_TOKEN", "t"), ("CLAUDECODE", "1"), ("NODE_OPTIONS", "x"), ("TERM", "dumb"), ("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "y")];
        let out = child_env(vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())));
        let keys: Vec<&str> = out.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["Path", "TERM", "COLORTERM", "FORCE_HYPERLINK"]);
        assert_eq!(out[1].1, "xterm-256color");
    }

    #[test]
    fn cwd_must_be_an_absolute_existing_folder() {
        let home = std::env::temp_dir();
        assert_eq!(resolve_cwd(None, &home), Some(home.clone()));
        assert_eq!(resolve_cwd(Some(""), &home), Some(home.clone()));
        assert_eq!(resolve_cwd(Some("relative"), &home), None);
        assert_eq!(resolve_cwd(Some("\\\\server\\share"), &home), None);
        let dir = resolve_cwd(Some(&home.to_string_lossy()), &home).unwrap();
        assert!(dir.is_dir());
        assert!(!dir.to_string_lossy().starts_with("\\\\?\\"));
        let file = home.join(format!("orrery-shell-cwd-{}", std::process::id()));
        std::fs::write(&file, "x").unwrap();
        assert_eq!(resolve_cwd(Some(&file.to_string_lossy()), &home), None);
        let _ = std::fs::remove_file(file);
    }
}
