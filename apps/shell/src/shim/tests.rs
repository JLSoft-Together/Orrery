use serde_json::json;

use super::*;

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("orrery-shim-{tag}-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(&d).unwrap()
}

#[test]
fn path_gets_the_shim_dir_first_once_under_the_existing_key() {
    let mut env = Map::new();
    env.insert("Path".into(), json!("C:\\a;c:\\SHIM;C:\\b;"));
    apply_shim_env(&mut env, true, "C:\\shim");
    assert_eq!(env["Path"], "C:\\shim;C:\\a;C:\\b");
    assert_eq!(env["ORRERY_SHIM_DIR"], "C:\\shim");
    assert!(!env.contains_key("PATH"));
    let mut unix = Map::new();
    apply_shim_env(&mut unix, false, "/s");
    assert_eq!(unix["PATH"], "/s");
    unix.insert("PATH".into(), json!("/a::/s:/b"));
    apply_shim_env(&mut unix, false, "/s");
    assert_eq!(unix["PATH"], "/s:/a:/b");
}

#[test]
fn the_files_are_named_per_platform_and_windows_ones_use_crlf() {
    let names: Vec<String> = shim_files(false).into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["claude", "codex"]);
    let win = shim_files(true);
    assert_eq!(win[0].0, "claude.cmd");
    assert!(win[0].1.contains("where \"$PATH:claude\""));
    assert!(win[0].1.contains("\r\n") && !win[0].1.replace("\r\n", "").contains('\n'));
    assert!(win[0].1.contains("if \"%__orr_a:~0,9%\"==\"--resume=\" set \"__orr_keep=1\""));
    assert!(win[1].1.contains("mcp_servers.orrery.bearer_token_env_var=ORRERY_TOKEN %*"));
    assert!(win[0].1.contains("exit /b 9009"));
}

#[test]
fn shims_are_written_once_and_the_zsh_wrappers_hand_zdotdir_back() {
    let base = temp("write");
    let dir = base.join("shell-shim");
    assert!(write_shims(&dir, false));
    assert!(write_shims(&dir, false));
    assert!(dir.join("claude").is_file() && dir.join("codex").is_file());
    let init = base.join("shell-init-tauri");
    assert!(write_zsh_init(&init));
    let rc = std::fs::read_to_string(init.join(".zshrc")).unwrap();
    assert!(rc.contains("ORRERY_SHIM_DIR") && rc.contains("add-zsh-hook precmd __agent_monitor_cwd"));
    assert!(std::fs::read_to_string(init.join(".zshenv")).unwrap().contains("builtin source \"$ZDOTDIR/.zshenv\""));
    assert_eq!(shim_dir_for(&base.join("orrery-mcp.json")), Some(base.join("shell-shim")));
    assert_eq!(shim_dir_for(Path::new("relative.json")), None);
    let _ = std::fs::remove_dir_all(&base);
}

#[cfg(unix)]
mod run {
    use std::process::Command;

    use super::*;

    fn rig(tag: &str) -> (PathBuf, PathBuf) {
        let base = temp(tag);
        let real = base.join("real");
        std::fs::create_dir_all(&real).unwrap();
        for agent in ["claude", "codex"] {
            let f = real.join(agent);
            std::fs::write(&f, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let shim = base.join("shell-shim");
        assert!(write_shims(&shim, false));
        (base, shim)
    }

    fn run(shim: &Path, real: Option<&Path>, agent: &str, args: &[&str], envs: &[(&str, &str)]) -> (i32, Vec<String>) {
        let mut path = shim.to_string_lossy().into_owned();
        if let Some(r) = real {
            path = format!("{path}:{}", r.display());
        }
        let out = Command::new(shim.join(agent)).args(args).env_clear().env("PATH", format!("{path}:/usr/bin:/bin")).envs(envs.iter().copied()).output().unwrap();
        (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).lines().map(str::to_owned).collect())
    }

    #[test]
    fn claude_gets_a_session_id_and_the_mcp_config_unless_the_user_chose() {
        let (base, shim) = rig("claude");
        let real = base.join("real");
        let cfg = base.join("orrery-mcp.json");
        std::fs::write(&cfg, "{}").unwrap();
        let envs = [("ORRERY_MCP_CONFIG", cfg.to_str().unwrap())];
        let (code, out) = run(&shim, Some(&real), "claude", &["-p", "hi"], &envs);
        assert_eq!(code, 0);
        assert_eq!(&out[..2], ["-p", "hi"]);
        assert_eq!(out[2], "--session-id");
        let id = &out[3];
        assert!(id.len() == 36 && id.as_bytes()[14] == b'4', "{id}");
        assert_eq!(&out[4..], ["--mcp-config", cfg.to_str().unwrap()]);
        let (_, out) = run(&shim, Some(&real), "claude", &["--resume", "abc"], &envs);
        assert_eq!(out, ["--resume", "abc", "--mcp-config", cfg.to_str().unwrap()]);
        let (_, out) = run(&shim, Some(&real), "claude", &["--session-id=x"], &envs);
        assert!(!out.contains(&"--session-id".to_owned()));
        for plain in [vec!["mcp", "list"], vec!["--version"], vec!["-p", "--", "x"]] {
            let (_, out) = run(&shim, Some(&real), "claude", &plain, &envs);
            assert_eq!(out, plain);
        }
        let (_, out) = run(&shim, Some(&real), "claude", &["x"], &[]);
        assert_eq!(out.len(), 3);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn codex_gets_the_mcp_url_only_with_api_and_token() {
        let (base, shim) = rig("codex");
        let real = base.join("real");
        let (_, out) = run(&shim, Some(&real), "codex", &["exec"], &[("ORRERY_API", "http://127.0.0.1:9"), ("ORRERY_TOKEN", "t")]);
        assert_eq!(out, ["-c", "mcp_servers.orrery.url=http://127.0.0.1:9/mcp", "-c", "mcp_servers.orrery.bearer_token_env_var=ORRERY_TOKEN", "exec"]);
        let (_, out) = run(&shim, Some(&real), "codex", &["exec"], &[]);
        assert_eq!(out, ["exec"]);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_agent_exits_127_and_the_shim_never_runs_itself() {
        let (base, shim) = rig("none");
        let (code, out) = run(&shim, None, "claude", &["x"], &[]);
        assert_eq!(code, 127);
        assert!(out.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }
}
