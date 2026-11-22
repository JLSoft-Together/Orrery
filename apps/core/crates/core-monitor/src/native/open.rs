use std::path::{Path, PathBuf};

use core_store::event::{OpenApp, OpenResult};

use super::launch::find_on_path;

pub fn vscode_candidates(path: Option<&str>, local_app_data: Option<&str>, program_files: &[Option<String>]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in path.unwrap_or("").split(';').filter(|d| !d.is_empty()) {
        let trimmed = dir.trim_end_matches(['\\', '/']);
        let leaf = Path::new(trimmed).file_name().and_then(|n| n.to_str()).unwrap_or("");
        if leaf.eq_ignore_ascii_case("bin") {
            if let Some(parent) = Path::new(trimmed).parent() {
                out.push(parent.join("Code.exe"));
            }
        }
    }
    if let Some(l) = local_app_data.filter(|s| !s.is_empty()) {
        out.push(Path::new(l).join("Programs").join("Microsoft VS Code").join("Code.exe"));
    }
    for root in program_files.iter().flatten().filter(|s| !s.is_empty()) {
        out.push(Path::new(root).join("Microsoft VS Code").join("Code.exe"));
    }
    out
}

pub fn mac_vscode_bundles(home: Option<&str>) -> Vec<PathBuf> {
    let mut out = vec![PathBuf::from("/Applications/Visual Studio Code.app")];
    if let Some(h) = home.filter(|s| !s.is_empty()) {
        out.push(Path::new(h).join("Applications").join("Visual Studio Code.app"));
    }
    out
}

pub type Command = (PathBuf, Vec<String>);

pub fn vscode_command(dir: &str, windows: bool, exists: &dyn Fn(&Path) -> bool, path_lookup: &dyn Fn(&str) -> Option<PathBuf>, candidates: Vec<PathBuf>) -> Option<Command> {
    if windows {
        return candidates.into_iter().find(|c| exists(c)).map(|c| (c, vec![dir.to_owned()]));
    }
    if let Some(cli) = path_lookup("code") {
        return Some((cli, vec![dir.to_owned()]));
    }
    candidates.into_iter().find(|c| exists(c)).map(|b| (PathBuf::from("/usr/bin/open"), vec!["-a".into(), b.to_string_lossy().into_owned(), dir.to_owned()]))
}

pub fn folder_command(dir: &str, windows: bool, system_root: Option<&str>) -> Command {
    if windows {
        (Path::new(system_root.filter(|s| !s.is_empty()).unwrap_or("C:\\Windows")).join("explorer.exe"), vec![dir.to_owned()])
    } else {
        (PathBuf::from("/usr/bin/open"), vec![dir.to_owned()])
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn spawn(cmd: &Command) -> std::io::Result<()> {
    use std::process::Stdio;
    let mut c = std::process::Command::new(&cmd.0);
    c.args(&cmd.1).env_remove("ELECTRON_RUN_AS_NODE").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    let mut child = c.spawn()?;
    std::thread::Builder::new().name("open-reap".into()).spawn(move || drop(child.wait())).map(|_| ())
}

#[cfg(any(windows, target_os = "macos"))]
pub fn open_folder(dir: &str, app: OpenApp) -> Option<OpenResult> {
    let windows = cfg!(windows);
    let path = Path::new(dir);
    if !path.is_dir() {
        return Some(OpenResult::NotFound);
    }
    let target = path.components().collect::<PathBuf>().to_string_lossy().into_owned();
    let var = |k: &str| std::env::var(k).ok();
    let command = match app {
        OpenApp::Explorer => Some(folder_command(&target, windows, var("SystemRoot").as_deref())),
        OpenApp::Vscode => {
            let candidates = if windows { vscode_candidates(var("PATH").as_deref(), var("LOCALAPPDATA").as_deref(), &[var("ProgramFiles"), var("ProgramFiles(x86)")]) } else { mac_vscode_bundles(var("HOME").as_deref()) };
            let lookup = |name: &str| find_on_path(name, var("PATH").as_deref(), ':');
            vscode_command(&target, windows, &|p| p.exists(), &lookup, candidates)
        }
    };
    let Some(command) = command else { return Some(OpenResult::NoApp) };
    Some(match spawn(&command) {
        Ok(()) => OpenResult::Ok,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => OpenResult::NoApp,
        Err(_) => OpenResult::Failed,
    })
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn open_folder(_dir: &str, _app: OpenApp) -> Option<OpenResult> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_finds_code_exe_next_to_its_bin_folder() {
        let c = vscode_candidates(Some("C:\\Windows;D:\\VSCode\\bin\\;E:\\x\\BIN"), Some("C:\\Users\\me\\AppData\\Local"), &[Some("C:\\Program Files".into()), None]);
        assert_eq!(c[0], Path::new("D:\\VSCode").join("Code.exe"));
        assert_eq!(c[1], Path::new("E:\\x").join("Code.exe"));
        assert_eq!(c[2], Path::new("C:\\Users\\me\\AppData\\Local").join("Programs").join("Microsoft VS Code").join("Code.exe"));
        assert_eq!(c[3], Path::new("C:\\Program Files").join("Microsoft VS Code").join("Code.exe"));
        let pick = vscode_command("C:\\w", true, &|p| p == c[2], &|_| None, c.clone());
        assert_eq!(pick, Some((c[2].clone(), vec!["C:\\w".to_owned()])));
        assert_eq!(vscode_command("C:\\w", true, &|_| false, &|_| None, c), None);
    }

    #[test]
    fn mac_prefers_the_code_cli_then_the_app_bundle() {
        let bundles = mac_vscode_bundles(Some("/Users/me"));
        assert_eq!(bundles[1], Path::new("/Users/me/Applications/Visual Studio Code.app"));
        let cli = vscode_command("/w", false, &|_| true, &|_| Some(PathBuf::from("/usr/local/bin/code")), bundles.clone());
        assert_eq!(cli, Some((PathBuf::from("/usr/local/bin/code"), vec!["/w".to_owned()])));
        let app = vscode_command("/w", false, &|p| p == bundles[0], &|_| None, bundles.clone());
        assert_eq!(app, Some((PathBuf::from("/usr/bin/open"), vec!["-a".to_owned(), bundles[0].to_string_lossy().into_owned(), "/w".to_owned()])));
        assert_eq!(vscode_command("/w", false, &|_| false, &|_| None, bundles), None);
        assert_eq!(folder_command("/w", false, None).0, PathBuf::from("/usr/bin/open"));
        assert_eq!(folder_command("C:\\w", true, Some("D:\\Win")).0, Path::new("D:\\Win").join("explorer.exe"));
    }
}
