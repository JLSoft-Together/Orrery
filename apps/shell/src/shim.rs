use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

const AGENTS: [&str; 2] = ["claude", "codex"];
const CLAUDE_SUBCOMMANDS: [&str; 23] = [
    "agents", "attach", "auth", "auto-mode", "config", "doctor", "gateway", "import", "install", "kill", "logs", "mcp", "migrate-installer", "plugin", "plugins", "purge", "respawn", "rm", "setup-token", "stop",
    "ultrareview", "update", "upgrade",
];
const KEEP_ID: [&str; 6] = ["-r", "--resume", "-c", "--continue", "--session-id", "--from-pr"];
const NO_INJECT: [&str; 4] = ["-v", "--version", "-h", "--help"];

pub const PS_SHIM_FIRST: &str = "if ($env:ORRERY_SHIM_DIR) { $env:Path = (@($env:ORRERY_SHIM_DIR) + @($env:Path -split ';' | Where-Object { $_ -and $_ -ne $env:ORRERY_SHIM_DIR })) -join ';' }";

const ZSH_SHIM_FIRST: &str = r#"[[ -n "$ORRERY_SHIM_DIR" ]] && path=("$ORRERY_SHIM_DIR" "${(@)path:#$ORRERY_SHIM_DIR}")"#;

fn not_found(agent: &str) -> String {
    format!("orrery: {agent} not found in PATH")
}

fn posix_find(agent: &str) -> Vec<String> {
    let mut v: Vec<String> = ["#!/bin/sh", "self=${0%/*}", "real=", "old_ifs=$IFS", "IFS=:", "set -f", "for d in $PATH; do", "  [ -n \"$d\" ] && [ \"$d\" != \"$self\" ] || continue"].map(String::from).to_vec();
    v.push(format!("  if [ -f \"$d/{agent}\" ] && [ -x \"$d/{agent}\" ]; then real=$d/{agent}; break; fi"));
    v.extend(["done", "set +f", "IFS=$old_ifs"].map(String::from));
    v.push(format!("if [ -z \"$real\" ]; then printf '%s\\n' '{}' >&2; exit 127; fi", not_found(agent)));
    v
}

fn posix_shim(agent: &str) -> String {
    let mut v = posix_find(agent);
    if agent == "codex" {
        v.extend(
            [
                "if [ -n \"${ORRERY_API-}\" ] && [ -n \"${ORRERY_TOKEN-}\" ]; then",
                "  exec \"$real\" -c \"mcp_servers.orrery.url=$ORRERY_API/mcp\" -c mcp_servers.orrery.bearer_token_env_var=ORRERY_TOKEN \"$@\"",
                "fi",
                "exec \"$real\" \"$@\"",
                "",
            ]
            .map(String::from),
        );
        return v.join("\n");
    }
    let keep: Vec<String> = KEEP_ID.iter().map(|f| if f.starts_with("--") { format!("{f}|{f}=*") } else { (*f).to_owned() }).collect();
    v.extend(["inject=1".to_owned(), "keep=".to_owned(), format!("case \"${{1-}}\" in {}) inject= ;; esac", CLAUDE_SUBCOMMANDS.join("|"))]);
    v.extend(["for a in \"$@\"; do", "  case \"$a\" in", "    --) inject=; break ;;"].map(String::from));
    v.push(format!("    {}) inject= ;;", NO_INJECT.join("|")));
    v.push(format!("    {}) keep=1 ;;", keep.join("|")));
    v.extend(["  esac", "done", "[ -n \"$inject\" ] || exec \"$real\" \"$@\"", "if [ -z \"$keep\" ]; then"].map(String::from));
    v.push(r#"  sid=$(od -An -N16 -tx1 /dev/urandom 2>/dev/null | tr -d ' \n' | sed -n 's/^\(.\{8\}\)\(.\{4\}\).\(.\{3\}\).\(.\{3\}\)\(.\{12\}\)$/\1-\2-4\3-a\4-\5/p')"#.to_owned());
    v.extend(
        [
            "  if [ -n \"$sid\" ]; then set -- \"$@\" --session-id \"$sid\"; fi",
            "fi",
            "if [ -n \"${ORRERY_MCP_CONFIG-}\" ] && [ -f \"$ORRERY_MCP_CONFIG\" ]; then set -- \"$@\" --mcp-config \"$ORRERY_MCP_CONFIG\"; fi",
            "exec \"$real\" \"$@\"",
            "",
        ]
        .map(String::from),
    );
    v.join("\n")
}

fn windows_head(agent: &str) -> Vec<String> {
    let mut v = vec!["@echo off".to_owned(), "setlocal EnableExtensions DisableDelayedExpansion".to_owned(), "set \"__orr_real=\"".to_owned()];
    v.push(format!("for /f \"delims=\" %%P in ('where \"$PATH:{agent}\" 2^>nul') do if not defined __orr_real if /i not \"%%~dpP\"==\"%~dp0\" ("));
    v.extend(["  if /i \"%%~xP\"==\".exe\" set \"__orr_real=%%P\"", "  if /i \"%%~xP\"==\".cmd\" set \"__orr_real=%%P\"", "  if /i \"%%~xP\"==\".bat\" set \"__orr_real=%%P\"", ")"].map(String::from));
    v.push(format!("if not defined __orr_real (echo {}>&2& exit /b 9009)", not_found(agent)));
    v
}

fn windows_shim(agent: &str) -> String {
    let mut v = windows_head(agent);
    if agent == "codex" {
        v.extend(
            [
                "if not defined ORRERY_API goto plain",
                "if not defined ORRERY_TOKEN goto plain",
                "\"%__orr_real%\" -c \"mcp_servers.orrery.url=%ORRERY_API%/mcp\" -c mcp_servers.orrery.bearer_token_env_var=ORRERY_TOKEN %*",
                "exit /b %ERRORLEVEL%",
                ":plain",
                "\"%__orr_real%\" %*",
                "exit /b %ERRORLEVEL%",
                "",
            ]
            .map(String::from),
        );
        return v.join("\r\n");
    }
    v.extend(["set \"__orr_inject=1\"", "set \"__orr_keep=\"", "set \"__orr_first=1\"", ":scan", "if \"%~1\"==\"\" goto run", "set \"__orr_a=%~1\""].map(String::from));
    v.push(format!("if defined __orr_first for %%S in ({}) do if \"%__orr_a%\"==\"%%S\" set \"__orr_inject=\"", CLAUDE_SUBCOMMANDS.join(" ")));
    v.extend(["set \"__orr_first=\"", "if \"%__orr_a%\"==\"--\" (set \"__orr_inject=\" & goto run)"].map(String::from));
    v.push(format!("for %%F in ({}) do if \"%__orr_a%\"==\"%%F\" set \"__orr_inject=\"", NO_INJECT.join(" ")));
    v.push(format!("for %%F in ({}) do if \"%__orr_a%\"==\"%%F\" set \"__orr_keep=1\"", KEEP_ID.join(" ")));
    for f in KEEP_ID.iter().filter(|f| f.starts_with("--")) {
        v.push(format!("if \"%__orr_a:~0,{}%\"==\"{f}=\" set \"__orr_keep=1\"", f.len() + 1));
    }
    v.extend(["shift", "goto scan", ":run", "if not defined __orr_inject (", "  \"%__orr_real%\" %*", "  exit /b %ERRORLEVEL%", ")", "set \"__orr_tail=\"", "if defined __orr_keep goto mcp"].map(String::from));
    v.push(r#"for /f "delims=" %%G in ('"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command [guid]::NewGuid^(^).Guid') do set "__orr_tail=--session-id %%G""#.to_owned());
    v.extend([":mcp", "if defined ORRERY_MCP_CONFIG if exist \"%ORRERY_MCP_CONFIG%\" set \"__orr_tail=%__orr_tail% --mcp-config \"%ORRERY_MCP_CONFIG%\"\"", "\"%__orr_real%\" %* %__orr_tail%", "exit /b %ERRORLEVEL%", ""].map(String::from));
    v.join("\r\n")
}

pub fn shim_files(windows: bool) -> Vec<(String, String)> {
    AGENTS.iter().map(|a| (if windows { format!("{a}.cmd") } else { (*a).to_owned() }, if windows { windows_shim(a) } else { posix_shim(a) })).collect()
}

fn write_if_changed(file: &Path, body: &str, mode: u32) -> std::io::Result<()> {
    if std::fs::read_to_string(file).is_ok_and(|c| c == body) {
        return Ok(());
    }
    std::fs::write(file, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode))?;
    }
    let _ = mode;
    Ok(())
}

pub fn write_shims(dir: &Path, windows: bool) -> bool {
    let run = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        shim_files(windows).iter().try_for_each(|(name, body)| write_if_changed(&dir.join(name), body, 0o755))
    };
    run().is_ok()
}

pub fn shim_dir_for(mcp_config: &Path) -> Option<PathBuf> {
    mcp_config.parent().filter(|p| p.is_absolute()).map(|p| p.join("shell-shim"))
}

pub fn init_dir_for(mcp_config: &Path) -> Option<PathBuf> {
    mcp_config.parent().filter(|p| p.is_absolute()).map(|p| p.join("shell-init-tauri"))
}

pub fn zsh_files() -> Vec<(&'static str, String)> {
    let pass = |file: &str| {
        [
            "__agent_monitor_dir=\"$ZDOTDIR\"".to_owned(),
            "ZDOTDIR=\"${AGENT_MONITOR_ZDOTDIR:-$HOME}\"".to_owned(),
            format!("[[ -r \"$ZDOTDIR/{file}\" ]] && builtin source \"$ZDOTDIR/{file}\""),
            "AGENT_MONITOR_ZDOTDIR=\"$ZDOTDIR\"".to_owned(),
            "ZDOTDIR=\"$__agent_monitor_dir\"".to_owned(),
            "unset __agent_monitor_dir".to_owned(),
            String::new(),
        ]
        .join("\n")
    };
    let rc = [
        "ZDOTDIR=\"${AGENT_MONITOR_ZDOTDIR:-$HOME}\"",
        "unset AGENT_MONITOR_ZDOTDIR",
        "[[ -r \"$ZDOTDIR/.zshrc\" ]] && builtin source \"$ZDOTDIR/.zshrc\"",
        ZSH_SHIM_FIRST,
        r#"__agent_monitor_cwd() { builtin printf '\e]7;file://%s%s\e\\' "$HOST" "$PWD" }"#,
        "autoload -Uz add-zsh-hook && add-zsh-hook precmd __agent_monitor_cwd",
        "",
    ]
    .join("\n");
    let login = ["ZDOTDIR=\"${AGENT_MONITOR_ZDOTDIR:-$HOME}\"", "unset AGENT_MONITOR_ZDOTDIR", "[[ -r \"$ZDOTDIR/.zlogin\" ]] && builtin source \"$ZDOTDIR/.zlogin\"", ""].join("\n");
    vec![(".zshenv", pass(".zshenv")), (".zprofile", pass(".zprofile")), (".zshrc", rc), (".zlogin", login)]
}

pub fn write_zsh_init(dir: &Path) -> bool {
    let run = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        zsh_files().iter().try_for_each(|(name, body)| write_if_changed(&dir.join(name), body, 0o644))
    };
    run().is_ok()
}

pub fn apply_shim_env(env: &mut Map<String, Value>, windows: bool, dir: &str) {
    let key = env.keys().find(|k| k.eq_ignore_ascii_case("PATH")).cloned().unwrap_or_else(|| if windows { "Path".into() } else { "PATH".into() });
    let sep = if windows { ';' } else { ':' };
    let same = |d: &str| if windows { d.eq_ignore_ascii_case(dir) } else { d == dir };
    let current = env.get(&key).and_then(Value::as_str).unwrap_or("").to_owned();
    let rest: Vec<&str> = current.split(sep).filter(|d| !d.is_empty() && !same(d)).collect();
    let joined = std::iter::once(dir).chain(rest).collect::<Vec<_>>().join(&sep.to_string());
    env.insert(key, Value::String(joined));
    env.insert("ORRERY_SHIM_DIR".into(), Value::String(dir.to_owned()));
}

#[cfg(test)]
mod tests;
