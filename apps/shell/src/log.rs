use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FILE: &str = "shell.log";
const OLD: &str = "shell.old.log";
const ROTATE_AT: u64 = 2 * 1024 * 1024;

pub fn wanted(release: bool, env: Option<&str>) -> bool {
    match env.map(str::trim) {
        Some("0") => false,
        Some("1") => true,
        _ => release,
    }
}

fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn open(dir: &Path) -> std::io::Result<(File, PathBuf)> {
    let path = dir.join(FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > ROTATE_AT) {
        let _ = std::fs::rename(&path, dir.join(OLD));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((file, path))
}

#[cfg(windows)]
fn stderr_taken() -> bool {
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE};
    let h = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    !h.is_null() && h != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
}

#[cfg(windows)]
fn redirect(file: File) -> std::io::Result<()> {
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};
    let handle = file.into_raw_handle();
    if unsafe { SetStdHandle(STD_ERROR_HANDLE, handle) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn stderr_taken() -> bool {
    unsafe { libc::isatty(2) == 1 }
}

#[cfg(target_os = "macos")]
fn redirect(file: File) -> std::io::Result<()> {
    use std::os::fd::IntoRawFd;
    let fd = file.into_raw_fd();
    let done = unsafe { libc::dup2(fd, 2) };
    unsafe { libc::close(fd) };
    if done < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

pub fn install(dir: &Path, version: &str) {
    let env = std::env::var("CCM_SHELL_LOG").ok();
    if !wanted(!cfg!(debug_assertions), env.as_deref()) || (env.is_none() && stderr_taken()) {
        return;
    }
    let Ok((file, _)) = open(dir) else { return };
    if redirect(file).is_err() {
        return;
    }
    eprintln!("[shell] start Orrery {version} pid {} at {}", std::process::id(), unix_secs());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("[shell] panic at {}", unix_secs());
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_logs_unless_turned_off_and_debug_only_when_asked() {
        assert!(wanted(true, None));
        assert!(!wanted(true, Some("0")));
        assert!(!wanted(false, None));
        assert!(wanted(false, Some("1")));
        assert!(wanted(true, Some("yes")));
    }

    #[test]
    fn a_big_log_moves_aside_and_a_new_one_starts() {
        let dir = std::env::temp_dir().join(format!("orrery-shell-log-{}-{}", std::process::id(), unix_secs()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), vec![b'x'; (ROTATE_AT + 1) as usize]).unwrap();
        let (mut file, path) = open(&dir).unwrap();
        std::io::Write::write_all(&mut file, b"new\n").unwrap();
        drop(file);
        assert_eq!(std::fs::read(&path).unwrap(), b"new\n");
        assert_eq!(std::fs::metadata(dir.join(OLD)).unwrap().len(), ROTATE_AT + 1);
        let (mut again, _) = open(&dir).unwrap();
        std::io::Write::write_all(&mut again, b"more\n").unwrap();
        drop(again);
        assert_eq!(std::fs::read(&path).unwrap(), b"new\nmore\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
