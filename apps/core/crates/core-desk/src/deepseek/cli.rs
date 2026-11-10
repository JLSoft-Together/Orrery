use crate::preview::format::{resolve_command, with_args, Resolved};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Codewhale {
    pub cmd: Resolved,
    pub needs_download: bool,
    pub origin: &'static str,
    pub wrapper: Option<Resolved>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub cmd: Resolved,
    pub display: String,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub route: &'static str,
    pub steps: Vec<Step>,
}

pub type Env = BTreeMap<String, String>;

pub const WINGET_UPGRADE_ARGS: [&str; 6] = ["upgrade", "--id", "HunterBown.CodeWhale", "-e", "--accept-source-agreements", "--accept-package-agreements"];
pub const NPM_UPGRADE_ARGS: [&str; 3] = ["i", "-g", "codewhale@latest"];
pub const WINGET_ARGS: [&str; 6] = ["install", "--id", "HunterBown.CodeWhale", "-e", "--accept-source-agreements", "--accept-package-agreements"];
pub const NPM_ARGS: [&str; 3] = ["i", "-g", "codewhale"];
pub const INSTALL_SCRIPT: &str = "curl -fsSL https://codewhale.net/install.sh | sh";

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn var(env: &Env, key: &str) -> Option<String> {
    env.get(key).filter(|v| !v.is_empty()).cloned()
}

pub fn extra_dirs(env: &Env, home: &str, win: bool) -> Vec<String> {
    let s = |p: PathBuf| p.to_string_lossy().to_string();
    if win {
        let mut out = Vec::new();
        if let Some(a) = var(env, "APPDATA") {
            out.push(s(Path::new(&a).join("npm")));
        }
        if let Some(a) = var(env, "LOCALAPPDATA") {
            out.push(s(Path::new(&a).join("Microsoft").join("WinGet").join("Links")));
        }
        return out;
    }
    let h = Path::new(home);
    vec![s(h.join(".local").join("bin")), s(h.join(".codewhale").join("bin")), "/opt/homebrew/bin".into(), "/usr/local/bin".into()]
}

fn with_extra(env: &Env, dirs: &[String], win: bool) -> Env {
    let cur = env.get("PATH").or_else(|| env.get("Path")).cloned().unwrap_or_default();
    let sep = if win { ";" } else { ":" };
    let joined: Vec<&str> = std::iter::once(cur.as_str()).chain(dirs.iter().map(String::as_str)).filter(|s| !s.is_empty()).collect();
    let mut out = env.clone();
    out.remove("Path");
    out.insert("PATH".into(), joined.join(sep));
    out
}

pub fn is_file(p: &str) -> bool {
    let path = Path::new(p);
    match std::fs::metadata(path) {
        Ok(m) => m.is_file(),
        Err(_) => std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()),
    }
}

fn realpath(p: &str) -> String {
    std::fs::canonicalize(p).map(|r| crate::claude::strip_verbatim(&r)).unwrap_or_else(|_| p.to_string())
}

pub fn wrapper_downloads(found: &str, win: bool) -> Option<String> {
    let lower = found.to_lowercase();
    if win && (lower.ends_with(".cmd") || lower.ends_with(".ps1")) {
        return Some(crate::util::path_str(&Path::new(&crate::claude::dirname(found)).join("node_modules").join("codewhale").join("bin").join("downloads")));
    }
    if win && lower.ends_with(".exe") {
        return None;
    }
    let real = realpath(found);
    real.to_lowercase().ends_with(".js").then(|| crate::util::path_str(&Path::new(&crate::claude::dirname(&real)).join("downloads")))
}

fn cmd_file(cmd: &Resolved) -> Option<String> {
    let line = cmd.args.get(3)?;
    let mut chars = line.chars();
    chars.next();
    chars.next_back();
    let inner = chars.as_str();
    let mut out = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c == '^' {
            if let Some(n) = it.next() {
                out.push(n);
                continue;
            }
        }
        out.push(c);
    }
    let t = crate::claude::js_trim(&out);
    (!t.is_empty()).then(|| t.to_string())
}

pub fn find_codewhale(env: &Env, home: &str, win: bool) -> Option<Codewhale> {
    let lookup = with_extra(env, &extra_dirs(env, home, win), win);
    let cmd = resolve_command("codewhale", home, &lookup, win)?;
    let found = if cmd.verbatim { cmd_file(&cmd) } else { Some(cmd.file.clone()) };
    let downloads = found.as_deref().and_then(|f| wrapper_downloads(f, win));
    let Some(downloads) = downloads else {
        let winget = found.as_deref().is_some_and(|f| {
            let n = f.replace('\\', "/").to_lowercase();
            n.contains("/microsoft/winget/")
        });
        return Some(Codewhale { cmd, needs_download: false, origin: if winget { "winget" } else { "standalone" }, wrapper: None });
    };
    let exe = crate::util::path_str(&Path::new(&downloads).join(if win { "codewhale.exe" } else { "codewhale" }));
    if is_file(&exe) {
        Some(Codewhale { cmd: Resolved { file: exe, args: Vec::new(), verbatim: false }, needs_download: false, origin: "npm", wrapper: Some(cmd) })
    } else {
        Some(Codewhale { cmd: cmd.clone(), needs_download: true, origin: "npm", wrapper: Some(cmd) })
    }
}

pub fn find_tool(name: &str, env: &Env, home: &str, win: bool) -> Option<Resolved> {
    let dirs: Vec<String> = if win { Vec::new() } else { vec!["/opt/homebrew/bin".into(), "/usr/local/bin".into()] };
    resolve_command(name, home, &with_extra(env, &dirs, win), win)
}

pub fn find_winget(env: &Env, home: &str) -> Option<Resolved> {
    if let Some(local) = var(env, "LOCALAPPDATA") {
        let alias = crate::util::path_str(&Path::new(&local).join("Microsoft").join("WindowsApps").join("winget.exe"));
        if is_file(&alias) {
            return Some(Resolved { file: alias, args: Vec::new(), verbatim: false });
        }
    }
    find_tool("winget", env, home, true)
}

fn step(base: &Resolved, a: &[&str], display: String) -> Option<Step> {
    Some(Step { cmd: with_args(base, &args(a))?, display })
}

pub fn update_plan(cw: &Codewhale, env: &Env, home: &str, win: bool) -> Option<Vec<Step>> {
    match cw.origin {
        "npm" => {
            let npm = find_tool("npm", env, home, win)?;
            Some(vec![step(&npm, &NPM_UPGRADE_ARGS, format!("npm {}", NPM_UPGRADE_ARGS.join(" ")))?])
        }
        "winget" => {
            let winget = if win { find_winget(env, home) } else { None }?;
            Some(vec![step(&winget, &WINGET_UPGRADE_ARGS, format!("winget {}", WINGET_UPGRADE_ARGS.join(" ")))?])
        }
        _ => Some(vec![step(&cw.cmd, &["update"], "codewhale update".into())?]),
    }
}

pub fn install_plan(env: &Env, home: &str, win: bool) -> Option<Plan> {
    if win {
        if let Some(winget) = find_winget(env, home) {
            return Some(Plan { route: "winget", steps: vec![step(&winget, &WINGET_ARGS, format!("winget {}", WINGET_ARGS.join(" ")))?] });
        }
    }
    if let Some(npm) = find_tool("npm", env, home, win) {
        return Some(Plan { route: "npm", steps: vec![step(&npm, &NPM_ARGS, format!("npm {}", NPM_ARGS.join(" ")))?] });
    }
    if win {
        return None;
    }
    let cmd = Resolved { file: "/bin/sh".into(), args: vec!["-c".into(), INSTALL_SCRIPT.into()], verbatim: false };
    Some(Plan { route: "script", steps: vec![Step { cmd, display: INSTALL_SCRIPT.into() }] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("core-desk-ds-{name}-{}", crate::util::uuid()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[cfg(unix)]
    #[test]
    fn standalone_and_npm_wrapper() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = tmp("find");
        let home = root.join("home");
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let exe = bin.join("codewhale");
        std::fs::write(&exe, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut env = Env::new();
        env.insert("PATH".into(), bin.to_string_lossy().to_string());
        let cw = find_codewhale(&env, &home.to_string_lossy(), false).unwrap();
        assert_eq!((cw.origin, cw.needs_download), ("standalone", false));
        std::fs::remove_file(&exe).unwrap();
        let pkg = root.join("lib").join("codewhale").join("bin");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("run.js"), "").unwrap();
        symlink(pkg.join("run.js"), &exe).unwrap();
        let cw = find_codewhale(&env, &home.to_string_lossy(), false).unwrap();
        assert_eq!((cw.origin, cw.needs_download), ("npm", true));
        std::fs::create_dir_all(pkg.join("downloads")).unwrap();
        std::fs::write(pkg.join("downloads").join("codewhale"), "").unwrap();
        let cw = find_codewhale(&env, &home.to_string_lossy(), false).unwrap();
        assert_eq!((cw.origin, cw.needs_download), ("npm", false));
        assert!(cw.cmd.file.ends_with("downloads/codewhale") && cw.wrapper.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plans() {
        let cw = Codewhale { cmd: Resolved { file: "/x/codewhale".into(), args: vec![], verbatim: false }, needs_download: false, origin: "standalone", wrapper: None };
        let steps = update_plan(&cw, &Env::new(), "/h", false).unwrap();
        assert_eq!(steps[0].display, "codewhale update");
        assert_eq!(steps[0].cmd.args, vec!["update".to_string()]);
        let p = install_plan(&Env::new(), "/h", false);
        if let Some(p) = p {
            assert!(p.route == "npm" || p.route == "script");
        }
    }
}
