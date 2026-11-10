use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::desk::DeskState;
use crate::state::{lock, Shell};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prepared {
    Nsis(PathBuf),
    Mac(PathBuf),
}

static PREPARED: Mutex<Option<Prepared>> = Mutex::new(None);

pub fn install_kind(debug: bool, windows: bool) -> &'static str {
    match (debug, windows) {
        (true, _) => "dev",
        (false, true) => "nsis",
        (false, false) => "mac",
    }
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    }
}

pub fn env_value(debug: bool, windows: bool, test_feed: bool) -> Value {
    let kind = install_kind(debug, windows);
    let asset = if kind == "dev" { install_kind(false, windows) } else { kind };
    json!({ "kind": kind, "assetKind": asset, "canDownload": if kind == "dev" { test_feed } else { true }, "arch": arch() })
}

pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn mac_bundle_of(exe: &str) -> Option<String> {
    exe.find(".app/Contents/MacOS/").filter(|i| *i > 0).map(|i| exe[..i + 4].to_owned())
}

pub fn mac_script(pid: u32, mount: &str, src_app: &str, dest_app: &str) -> String {
    let next = format!("{dest_app}.ccm-new");
    [
        "#!/bin/sh".to_owned(),
        format!("while kill -0 {pid} 2>/dev/null; do sleep 0.5; done"),
        format!("rm -rf {}", sh_quote(&next)),
        format!("if ditto {} {}; then", sh_quote(src_app), sh_quote(&next)),
        format!("  rm -rf {} && mv {} {}", sh_quote(dest_app), sh_quote(&next), sh_quote(dest_app)),
        "fi".to_owned(),
        format!("rm -rf {}", sh_quote(&next)),
        format!("xattr -dr com.apple.quarantine {} 2>/dev/null", sh_quote(dest_app)),
        format!("hdiutil detach {} -quiet || hdiutil detach {} -force -quiet", sh_quote(mount), sh_quote(mount)),
        format!("open {}", sh_quote(dest_app)),
        "rm -f \"$0\"".to_owned(),
        String::new(),
    ]
    .join("\n")
}

pub fn nsis_args() -> [&'static str; 3] {
    ["/S", "--updated", "--force-run"]
}

pub fn update_name_ok(name: &str, windows: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    let (prefix, suffix) = if windows { ("orrery-setup-", ".exe") } else { ("orrery-", ".dmg") };
    lower.len() > prefix.len() + suffix.len()
        && lower.starts_with(prefix)
        && lower.ends_with(suffix)
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
}

pub fn accept_update_file(raw: &str, data_dir: &Path, windows: bool) -> Option<PathBuf> {
    if raw.is_empty() {
        return None;
    }
    let root = std::fs::canonicalize(data_dir.join("updates")).ok()?;
    let file = std::fs::canonicalize(raw).ok()?;
    let name = file.file_name()?.to_str()?;
    (file.parent()? == root && file.is_file() && update_name_ok(name, windows)).then_some(file)
}

pub fn inside_data_dir(raw: &str, data_dir: &Path) -> Option<PathBuf> {
    let root = std::fs::canonicalize(data_dir).ok()?;
    let path = Path::new(raw);
    let name = path.file_name()?;
    let parent = std::fs::canonicalize(path.parent()?).ok()?;
    let resolved = parent.join(name);
    if std::fs::symlink_metadata(&resolved).is_ok_and(|m| m.file_type().is_symlink()) {
        return None;
    }
    resolved.starts_with(&root).then_some(resolved)
}

fn writable(p: &Path) -> bool {
    std::fs::metadata(p).is_ok_and(|m| !m.permissions().readonly()) && {
        let probe = p.join(format!(".orrery-write-{}", std::process::id()));
        let ok = std::fs::write(&probe, b"").is_ok();
        let _ = std::fs::remove_file(&probe);
        ok
    }
}

fn run(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

#[cfg(unix)]
fn private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(path)
}

#[cfg(not(unix))]
fn private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

fn make_private_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir(path)
    }
}

pub fn write_private_new(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)?.write_all(text.as_bytes())
}

fn prepare_mac(file: &Path, data_dir: &Path) -> Result<Prepared, &'static str> {
    let exe = std::env::current_exe().map_err(|_| "install")?;
    let bundle = mac_bundle_of(&exe.to_string_lossy()).ok_or("install")?;
    let bundle = PathBuf::from(bundle);
    let parent = bundle.parent().ok_or("install")?;
    if !writable(&bundle) || !writable(parent) {
        return Err("mac-readonly");
    }
    let pid = std::process::id();
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let work = data_dir.join("updates");
    private_dir(&work).map_err(|_| "install")?;
    if std::fs::symlink_metadata(&work).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("install");
    }
    let mount = work.join(format!("mnt-{pid}-{stamp}"));
    make_private_dir(&mount).map_err(|_| "install")?;
    let m = mount.to_string_lossy().into_owned();
    if !run("hdiutil", &["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint", &m, &file.to_string_lossy()]) {
        return Err("install");
    }
    let app = std::fs::read_dir(&mount).ok().and_then(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).find(|n| n.ends_with(".app")));
    let script = app.map(|a| mac_script(pid, &m, &mount.join(a).to_string_lossy(), &bundle.to_string_lossy()));
    let sh = work.join(format!("install-{pid}-{stamp}.sh"));
    let written = script.is_some_and(|s| write_private_new(&sh, &s).is_ok());
    if !written {
        run("hdiutil", &["detach", &m, "-force", "-quiet"]);
        return Err("install");
    }
    Ok(Prepared::Mac(sh))
}

fn prepare(data_dir: &Path, p: &Value) -> Value {
    let raw = p.get("file").and_then(Value::as_str).unwrap_or_default();
    let Some(file) = accept_update_file(raw, data_dir, cfg!(windows)) else {
        return json!({ "ok": false, "error": "install" });
    };
    let got = if cfg!(windows) { Ok(Prepared::Nsis(file)) } else { prepare_mac(&file, data_dir) };
    match got {
        Ok(prep) => {
            *lock(&PREPARED) = Some(prep);
            json!({ "ok": true })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

fn launch(prep: &Prepared) -> bool {
    let mut cmd = match prep {
        Prepared::Nsis(file) => {
            let mut c = Command::new(file);
            c.args(nsis_args());
            c
        }
        Prepared::Mac(sh) => {
            let mut c = Command::new("/bin/sh");
            c.arg(sh);
            c
        }
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0000_0200 | 0x0800_0000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn().is_ok()
}

fn restart(app: &AppHandle, p: &Value) -> bool {
    let Some(prep) = lock(&PREPARED).clone() else { return false };
    let version = p.get("version").and_then(Value::as_str).unwrap_or_default().to_owned();
    let pending = p.get("pending").cloned().unwrap_or(Value::Null);
    let local = crate::hosts::read_local(app, None);
    if let Some(desk) = app.try_state::<DeskState>() {
        let _ = desk.desk.dispatch("desk:backup-write-auto", json!(["pre-update", version, local]));
    }
    if let (Some(file), Some(text)) = (pending.get("file").and_then(Value::as_str), pending.get("text").and_then(Value::as_str)) {
        let data_dir = app.state::<Shell>().data_dir.clone();
        if let Some(target) = inside_data_dir(file, &data_dir) {
            let _ = std::fs::write(target, text);
        }
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let shell = handle.state::<Shell>();
        shell.quitting.store(true, Ordering::SeqCst);
        shell.shutdown();
        if launch(&prep) {
            handle.exit(0);
        } else {
            handle.restart();
        }
    });
    true
}

fn open(app: &AppHandle, target: &str) {
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        crate::window::open_external(app, target);
    } else if Path::new(target).is_absolute() {
        let _ = app.opener().open_path(target, None::<&str>);
    }
}

pub fn call(app: &AppHandle, method: &str, p: &Value) -> Option<Value> {
    let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    Some(match method {
        "update.env" => env_value(cfg!(debug_assertions), cfg!(windows), std::env::var("CCM_UPDATE_FEED").is_ok_and(|v| !v.is_empty())),
        "update.prepare" => prepare(&app.state::<Shell>().data_dir, p),
        "update.restart" => json!(restart(app, p)),
        "update.open" => {
            open(app, &text("target"));
            json!(true)
        }
        "update.reveal" => {
            let file = text("file");
            if Path::new(&file).is_absolute() {
                let _ = app.opener().reveal_item_in_dir(&file);
            }
            json!(true)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_electron_rules() {
        assert_eq!(env_value(false, true, false)["kind"], "nsis");
        assert_eq!(env_value(false, false, false)["kind"], "mac");
        let dev = env_value(true, true, false);
        assert_eq!(dev["kind"], "dev");
        assert_eq!(dev["assetKind"], "nsis");
        assert_eq!(dev["canDownload"], false);
        assert_eq!(env_value(true, false, true)["canDownload"], true);
        assert_eq!(env_value(true, false, true)["assetKind"], "mac");
        assert_eq!(env_value(false, true, false)["canDownload"], true);
    }

    #[test]
    fn the_mac_bundle_and_script_match_the_electron_installer() {
        assert_eq!(mac_bundle_of("/Applications/Orrery.app/Contents/MacOS/Orrery").as_deref(), Some("/Applications/Orrery.app"));
        assert!(mac_bundle_of("/usr/bin/x").is_none());
        let s = mac_script(42, "/tmp/m", "/tmp/m/Orrery.app", "/Applications/O'rrery.app");
        assert!(s.starts_with("#!/bin/sh\nwhile kill -0 42"));
        assert!(s.contains("ditto '/tmp/m/Orrery.app' '/Applications/O'\\''rrery.app.ccm-new'"));
        assert!(s.contains("open '/Applications/O'\\''rrery.app'"));
        assert!(s.ends_with("rm -f \"$0\"\n"));
    }

    #[test]
    fn the_silent_installer_flags_are_the_ones_the_hooks_read() {
        assert_eq!(nsis_args(), ["/S", "--updated", "--force-run"]);
        let hooks = include_str!("../nsis/hooks.nsh");
        assert!(hooks.contains("--force-run"));
        assert!(hooks.contains("ExecWait '\"$R5\" /S"));
        assert!(hooks.contains("${If} $R2 == \"0\""));
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("orrery-uh-test-{name}-{}-{}", std::process::id(), crate::config::random_hex(4)));
        std::fs::create_dir_all(d.join("updates")).unwrap();
        std::fs::canonicalize(&d).unwrap()
    }

    #[test]
    fn a_missing_file_is_not_prepared() {
        let d = scratch("missing");
        assert_eq!(prepare(&d, &json!({ "file": "/definitely/missing.dmg" }))["ok"], false);
        assert_eq!(prepare(&d, &json!({}))["error"], "install");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn update_names_follow_the_release_assets() {
        assert!(update_name_ok("Orrery-Setup-2.5.0.exe", true));
        assert!(update_name_ok("orrery-setup-2.5.0.EXE", true));
        assert!(!update_name_ok("Orrery-Setup-.exe", true));
        assert!(!update_name_ok("Orrery-2.5.0-arm64.dmg", true));
        assert!(update_name_ok("Orrery-2.5.0-arm64.dmg", false));
        assert!(!update_name_ok("Orrery-Setup-2.5.0.exe", false));
        assert!(!update_name_ok("Orrery-evil name.dmg", false));
        assert!(!update_name_ok("evil.dmg", false));
    }

    #[test]
    fn only_a_named_file_inside_the_updates_folder_is_accepted() {
        let d = scratch("accept");
        let good = d.join("updates").join("Orrery-Setup-2.5.0.exe");
        std::fs::write(&good, b"x").unwrap();
        assert_eq!(accept_update_file(&good.to_string_lossy(), &d, true), Some(good.clone()));
        let wrong_name = d.join("updates").join("evil.exe");
        std::fs::write(&wrong_name, b"x").unwrap();
        assert!(accept_update_file(&wrong_name.to_string_lossy(), &d, true).is_none());
        let outside = d.join("Orrery-Setup-2.5.0.exe");
        std::fs::write(&outside, b"x").unwrap();
        assert!(accept_update_file(&outside.to_string_lossy(), &d, true).is_none());
        let nested = d.join("updates").join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("Orrery-Setup-2.5.0.exe"), b"x").unwrap();
        assert!(accept_update_file(&nested.join("Orrery-Setup-2.5.0.exe").to_string_lossy(), &d, true).is_none());
        let dotted = format!("{}/updates/../Orrery-Setup-2.5.0.exe", d.display());
        assert!(accept_update_file(&dotted, &d, true).is_none());
        assert!(accept_update_file("", &d, true).is_none());
        #[cfg(unix)]
        {
            let link = d.join("updates").join("Orrery-Setup-9.9.9.exe");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            assert!(accept_update_file(&link.to_string_lossy(), &d, true).is_none());
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn the_pending_file_must_stay_inside_the_data_dir() {
        let d = scratch("pending");
        let ok = d.join("updates").join("pending.json");
        assert_eq!(inside_data_dir(&ok.to_string_lossy(), &d), Some(ok));
        assert!(inside_data_dir("/etc/passwd", &d).is_none());
        let dotted = format!("{}/../escape.json", d.display());
        assert!(inside_data_dir(&dotted, &d).is_none());
        #[cfg(unix)]
        {
            let target = std::env::temp_dir().join(format!("orrery-uh-victim-{}", std::process::id()));
            std::fs::write(&target, b"x").unwrap();
            let link = d.join("updates").join("link.json");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(inside_data_dir(&link.to_string_lossy(), &d).is_none());
            let dir_link = d.join("dl");
            std::os::unix::fs::symlink(std::env::temp_dir(), &dir_link).unwrap();
            assert!(inside_data_dir(&dir_link.join("x.json").to_string_lossy(), &d).is_none());
            let _ = std::fs::remove_file(target);
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[cfg(unix)]
    #[test]
    fn the_install_script_is_private_and_never_overwritten() {
        use std::os::unix::fs::PermissionsExt;
        let d = scratch("script");
        let sh = d.join("updates").join("install-1.sh");
        write_private_new(&sh, "#!/bin/sh\n").unwrap();
        assert_eq!(std::fs::metadata(&sh).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(write_private_new(&sh, "x").is_err());
        let m = d.join("updates").join("mnt-1");
        make_private_dir(&m).unwrap();
        assert_eq!(std::fs::metadata(&m).unwrap().permissions().mode() & 0o777, 0o700);
        assert!(make_private_dir(&m).is_err());
        let _ = std::fs::remove_dir_all(d);
    }
}
