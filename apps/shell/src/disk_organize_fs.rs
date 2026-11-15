use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
pub struct Stat {
    pub size: u64,
    pub mtime: f64,
    pub atime: f64,
    pub touched: f64,
    pub file: bool,
    pub dir: bool,
    pub link: bool,
}

fn ms(t: std::io::Result<SystemTime>) -> f64 {
    t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs() as f64 * 1000.0 + d.subsec_nanos() as f64 / 1e6)
}

#[cfg(windows)]
fn change_ms(p: &Path) -> f64 {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{FileBasicInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT};
    let Ok(f) = std::fs::OpenOptions::new().access_mode(0x80).share_mode(7).custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).open(p) else { return 0.0 };
    let mut info: FILE_BASIC_INFO = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandleEx(f.as_raw_handle() as _, FileBasicInfo, &mut info as *mut _ as *mut _, std::mem::size_of::<FILE_BASIC_INFO>() as u32) } != 0;
    if !ok || info.ChangeTime <= 0 {
        return 0.0;
    }
    info.ChangeTime as f64 / 10_000.0 - 11_644_473_600_000.0
}

#[cfg(unix)]
fn change_ms(p: &Path) -> f64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(p).map_or(0.0, |m| m.ctime() as f64 * 1000.0 + m.ctime_nsec() as f64 / 1e6)
}

fn from_meta(p: &Path, m: &Metadata) -> Stat {
    let mtime = ms(m.modified());
    let birth = ms(m.created());
    let ctime = if m.is_file() { change_ms(p) } else { 0.0 };
    Stat { size: m.len(), mtime, atime: ms(m.accessed()), touched: mtime.max(ctime).max(birth), file: m.is_file(), dir: m.is_dir(), link: m.file_type().is_symlink() }
}

pub fn lstat(p: &str) -> std::io::Result<Stat> {
    let path = Path::new(p);
    std::fs::symlink_metadata(path).map(|m| from_meta(path, &m))
}

pub fn code_of(e: &std::io::Error) -> String {
    if let Some(n) = e.raw_os_error() {
        let named = if cfg!(windows) {
            match n {
                2 | 3 => Some("ENOENT"),
                5 => Some("EPERM"),
                32 | 33 => Some("EBUSY"),
                80 | 183 => Some("EEXIST"),
                17 => Some("EXDEV"),
                145 => Some("ENOTEMPTY"),
                _ => None,
            }
        } else {
            match n {
                1 => Some("EPERM"),
                2 => Some("ENOENT"),
                13 => Some("EACCES"),
                16 => Some("EBUSY"),
                17 => Some("EEXIST"),
                18 => Some("EXDEV"),
                20 => Some("ENOTDIR"),
                39 | 66 => Some("ENOTEMPTY"),
                _ => None,
            }
        };
        if let Some(c) = named {
            return c.into();
        }
    }
    match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT".into(),
        std::io::ErrorKind::PermissionDenied => "EPERM".into(),
        std::io::ErrorKind::AlreadyExists => "EEXIST".into(),
        _ => "failed".into(),
    }
}

pub fn exists(p: &str) -> Result<bool, String> {
    match std::fs::symlink_metadata(p) {
        Ok(_) => Ok(true),
        Err(e) if code_of(&e) == "ENOENT" => Ok(false),
        Err(e) => Err(code_of(&e)),
    }
}

#[cfg(windows)]
fn rename_no_replace(from: &str, to: &str) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (a, b) = (wide(from), wide(to));
    if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 0) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn rename_no_replace(from: &str, to: &str) -> std::io::Result<()> {
    match std::fs::hard_link(from, to) {
        Ok(()) => std::fs::remove_file(from).inspect_err(|_| {
            let _ = std::fs::remove_file(to);
        }),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists || code_of(&e) == "EXDEV" || code_of(&e) == "ENOENT" => Err(e),
        Err(_) => {
            if std::fs::symlink_metadata(to).is_ok() {
                return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
            }
            std::fs::rename(from, to)
        }
    }
}

fn copy_excl(from: &str, to: &str, src: &Stat) -> Result<(), String> {
    let mut input = std::fs::File::open(from).map_err(|e| code_of(&e))?;
    let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(to).map_err(|e| code_of(&e))?;
    let r = (|| -> Result<(), String> {
        std::io::copy(&mut input, &mut out).map_err(|e| code_of(&e))?;
        out.sync_all().map_err(|e| code_of(&e))?;
        let t = UNIX_EPOCH + std::time::Duration::from_micros((src.mtime * 1000.0).max(0.0) as u64);
        let _ = out.set_modified(t);
        drop(out);
        let dst = lstat(to).map_err(|e| code_of(&e))?;
        if dst.size != src.size {
            return Err("verify".into());
        }
        let again = lstat(from).map_err(|e| code_of(&e))?;
        if again.size != src.size || again.mtime != src.mtime {
            return Err("changed".into());
        }
        Ok(())
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(to);
    }
    r
}

pub fn move_file(from: &str, to: &str) -> Result<&'static str, String> {
    if exists(to)? {
        return Err("EEXIST".into());
    }
    let src = lstat(from).map_err(|e| code_of(&e))?;
    if !src.file || src.link {
        return Err("notfile".into());
    }
    match rename_no_replace(from, to) {
        Ok(()) => return Ok("rename"),
        Err(e) if code_of(&e) != "EXDEV" => return Err(code_of(&e)),
        Err(_) => {}
    }
    copy_excl(from, to, &src)?;
    if let Err(e) = std::fs::remove_file(from) {
        let _ = std::fs::remove_file(to);
        return Err(code_of(&e));
    }
    Ok("copy")
}
