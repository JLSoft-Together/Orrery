use std::path::Path;

pub const BASH_INIT_FILE: &str = "orrery-bash.sh";

const POSIX_ESCAPE: &str = r#"  __orrery_esc() {
    local s="$1" x=$'\\x' bs=$'\\' nl=$'\n' cr=$'\r' tab=$'\t'
    s=${s//"$bs"/${x}5c}
    s=${s//";"/${x}3b}
    s=${s//"$nl"/${x}0a}
    s=${s//"$cr"/${x}0d}
    s=${s//"$tab"/${x}09}
    s=${s//[[:cntrl:]]/}
    builtin printf '%s' "$s"
  }"#;

pub fn zsh_integration() -> String {
    [
        r#"if [[ -n "$ORRERY_SI_NONCE" ]]; then"#,
        r#"  typeset -g __orrery_nonce="$ORRERY_SI_NONCE" __orrery_ran=''"#,
        "  unset ORRERY_SI_NONCE",
        POSIX_ESCAPE,
        "  __orrery_precmd() {",
        "    local s=$?",
        r#"    [[ -n $__orrery_ran ]] && builtin printf '\e]633;D;%s\a' "$s""#,
        "    __orrery_ran=''",
        r#"    builtin printf '\e]633;P;Cwd=%s\a' "$(__orrery_esc "$PWD")""#,
        "  }",
        "  __orrery_prompt() {",
        r#"    [[ $PS1 == *$'\e]633;B'* ]] || PS1=$'%{\e]633;A\a%}'"$PS1"$'%{\e]633;B\a%}'"#,
        "  }",
        "  __orrery_preexec() {",
        "    __orrery_ran=1",
        r#"    builtin printf '\e]633;E;%s;%s\a\e]633;C\a' "$(__orrery_esc "${1[1,2000]}")" "$__orrery_nonce""#,
        "  }",
        "  precmd_functions=(__orrery_precmd $precmd_functions __orrery_prompt)",
        "  autoload -Uz add-zsh-hook && add-zsh-hook preexec __orrery_preexec",
        "fi",
    ]
    .join("\n")
}

const BASH_SHIM_FIRST: &str = r#"if [ -n "${ORRERY_SHIM_DIR:-}" ]; then
  __orrery_p=":$PATH:"
  __orrery_p="${__orrery_p//":$ORRERY_SHIM_DIR:"/:}"
  __orrery_p="${__orrery_p#:}"
  PATH="$ORRERY_SHIM_DIR:${__orrery_p%:}"
  unset __orrery_p
fi"#;

pub fn bash_init() -> String {
    [
        "if [ -r /etc/profile ]; then . /etc/profile; fi",
        r#"for __orrery_f in "$HOME/.bash_profile" "$HOME/.bash_login" "$HOME/.profile"; do"#,
        r#"  if [ -r "$__orrery_f" ]; then . "$__orrery_f"; break; fi"#,
        "done",
        "unset __orrery_f",
        BASH_SHIM_FIRST,
        r#"if [[ -n "${ORRERY_SI_NONCE:-}" ]] && (( BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4) )); then"#,
        r#"  __orrery_nonce="$ORRERY_SI_NONCE""#,
        r#"  __orrery_orig=("${PROMPT_COMMAND[@]}")"#,
        "  __orrery_hist=''",
        POSIX_ESCAPE,
        "  __orrery_histline() { HISTTIMEFORMAT='' builtin history 1 2>/dev/null; }",
        "  __orrery_preexec() {",
        "    local h",
        r#"    h="$(__orrery_histline)""#,
        r#"    if [[ -n "$h" && "$h" != "$__orrery_hist" && "$h" =~ ^[[:space:]]*[0-9]+[*]?[[:space:]]+(.*)$ ]]; then"#,
        r#"      builtin printf '\e]633;E;%s;%s\a' "$(__orrery_esc "${BASH_REMATCH[1]:0:2000}")" "$__orrery_nonce""#,
        "    fi",
        r#"    builtin printf '\e]633;C\a'"#,
        "  }",
        "  __orrery_precmd() {",
        "    local s=$? c",
        r#"    builtin printf '\e]633;D;%s\a' "$s""#,
        r#"    builtin printf '\e]633;P;Cwd=%s\a' "$(__orrery_esc "$PWD")""#,
        r#"    for c in "${__orrery_orig[@]}"; do (exit "$s"); eval "$c"; done"#,
        r#"    __orrery_hist="$(__orrery_histline)""#,
        r#"    [[ "$PS1" == *'633;B'* ]] || PS1='\[\e]633;A\a\]'"$PS1"'\[\e]633;B\a\]'"#,
        "  }",
        r#"  PS0="${PS0:-}"'$(__orrery_preexec)'"#,
        "  unset PROMPT_COMMAND",
        "  PROMPT_COMMAND=__orrery_precmd",
        "fi",
        "unset ORRERY_SI_NONCE",
        "",
    ]
    .join("\n")
}

pub const PS_SHELL_INTEGRATION: &str = r#"try {
if ($env:ORRERY_SI_NONCE -and -not $global:__OrrerySI) {
$global:__OrrerySI = @{ Nonce = $env:ORRERY_SI_NONCE; Prompt = $function:prompt; ReadLine = $null; LastId = -1; Running = $false }
Remove-Item Env:ORRERY_SI_NONCE -ErrorAction SilentlyContinue
function global:__Orrery-Escape([string]$value) { [regex]::Replace($value, '[\x00-\x1f\x7f\\;]', { param($m) '\x{0:x2}' -f [int][char]$m.Value }) }
if (Get-Command PSConsoleHostReadLine -CommandType Function -ErrorAction SilentlyContinue) {
$global:__OrrerySI.ReadLine = $function:PSConsoleHostReadLine
function global:PSConsoleHostReadLine {
$si = $global:__OrrerySI
$line = & $si.ReadLine
$si.Running = $true
$cmd = __Orrery-Escape $line
[Console]::Write("$([char]27)]633;E;$cmd;$($si.Nonce)$([char]7)$([char]27)]633;C$([char]7)")
$line
}
}
function global:prompt {
$failed = -not $global:?
$code = $global:LASTEXITCODE
$si = $global:__OrrerySI
$h = Get-History -Count 1
$id = if ($h) { $h.Id } else { 0 }
$out = ''
if ($si.Running) {
$si.Running = $false
if ($id -eq $si.LastId) { $out += "$([char]27)]633;D$([char]7)" }
else {
$n = if (-not $failed) { 0 } elseif ($code -is [int] -and $code -ne 0) { $code } else { 1 }
$out += "$([char]27)]633;D;$n$([char]7)"
}
}
$si.LastId = $id
$loc = $executionContext.SessionState.Path.CurrentLocation
if ($loc.Provider.Name -eq 'FileSystem') { $out += "$([char]27)]633;P;Cwd=$(__Orrery-Escape $loc.ProviderPath)$([char]7)" }
$out += "$([char]27)]633;A$([char]7)"
if ($failed) { Write-Error 'failure' -ErrorAction Ignore }
$out += (& $si.Prompt) -join ''
$out + "$([char]27)]633;B$([char]7)"
}
}
} catch { }"#;

pub fn is_nonce(v: &str) -> bool {
    (16..=64).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn write_bash_init(dir: &Path) -> bool {
    let run = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let file = dir.join(BASH_INIT_FILE);
        let body = bash_init();
        if std::fs::read_to_string(&file).is_ok_and(|c| c == body) {
            return Ok(());
        }
        std::fs::write(file, body)
    };
    run().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_mark_prompt_command_exit_and_cwd() {
        for s in [zsh_integration(), bash_init(), PS_SHELL_INTEGRATION.to_owned()] {
            for m in ["633;A", "633;B", "633;C", "633;D", "633;E;", "633;P;Cwd="] {
                assert!(s.contains(m), "{m} missing");
            }
            assert!(s.contains("ORRERY_SI_NONCE"));
            assert!(!s.lines().any(|l| l.trim_start().starts_with('#')), "no comment lines");
        }
    }

    #[test]
    fn bash_keeps_the_login_files_and_needs_bash_4_4() {
        let b = bash_init();
        assert!(b.starts_with("if [ -r /etc/profile ]"));
        assert!(b.contains("BASH_VERSINFO[1] >= 4") && b.contains("PS0=") && b.ends_with("unset ORRERY_SI_NONCE\n"));
    }

    #[test]
    fn nonces_are_lowercase_hex_of_16_to_64() {
        assert!(is_nonce("0123456789abcdef"));
        assert!(!is_nonce("0123456789ABCDEF") && !is_nonce("abc") && !is_nonce("0123456789abcdeg"));
    }

    #[cfg(unix)]
    #[test]
    fn bash_integration_prints_marks_with_the_exit_code() {
        let ver = std::process::Command::new("bash").args(["-c", "echo $((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1]))"]).output();
        if !ver.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().is_ok_and(|v| v >= 404)) {
            return;
        }
        let dir = crate::config::test_temp_dir().join(format!("si-bash-{}", std::process::id()));
        assert!(write_bash_init(&dir));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let out = std::process::Command::new("bash")
            .args(["--init-file", &dir.join(BASH_INIT_FILE).to_string_lossy(), "-i"])
            .env("HOME", &home)
            .env("ORRERY_SI_NONCE", "0123456789abcdef")
            .env("HISTFILE", dir.join("hist"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                use std::io::Write;
                c.stdin.take().unwrap().write_all(b"sh -c 'exit 4'\nexit\n")?;
                c.wait_with_output()
            });
        let _ = std::fs::remove_dir_all(&dir);
        let Ok(out) = out else { return };
        let all = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert!(all.contains("\x1b]633;D;4\x07"), "{all:?}");
    }
}
