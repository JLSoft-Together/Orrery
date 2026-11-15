use super::{DirCtx, Kind, Level, ListError, Raw, NO_ALT};
use crate::index::flag;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const SF_DATALESS: u32 = 0x4000_0000;

fn err_kind(e: &std::io::Error) -> ListError {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => ListError::Denied,
        std::io::ErrorKind::NotFound => ListError::Missing,
        _ => ListError::Other,
    }
}

#[cfg(target_os = "macos")]
fn st_flags(m: &std::fs::Metadata) -> u32 {
    std::os::macos::fs::MetadataExt::st_flags(m)
}

#[cfg(not(target_os = "macos"))]
fn st_flags(_m: &std::fs::Metadata) -> u32 {
    0
}

fn check_not_link(dir: &Path) -> Result<(), ListError> {
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).map_err(|_| ListError::Other)?;
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error().unwrap_or(0) {
            libc::EACCES | libc::EPERM => ListError::Denied,
            libc::ENOENT => ListError::Missing,
            _ => ListError::Other,
        });
    }
    unsafe { libc::close(fd) };
    Ok(())
}

pub fn list(dir: &Path, ctx: &DirCtx, names: &mut String, out: &mut Vec<Raw>, alts: &mut Vec<OsString>) -> Result<(), ListError> {
    #[cfg(target_os = "macos")]
    {
        if let Some(r) = super::mac::list_bulk(dir, ctx, names, out, alts) {
            return r;
        }
    }
    if !ctx.root {
        check_not_link(dir)?;
    }
    let rd = std::fs::read_dir(dir).map_err(|e| err_kind(&e))?;
    let mut failed = false;
    for e in rd {
        let Ok(e) = e else {
            failed = true;
            continue;
        };
        let name = e.file_name();
        let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
            failed = true;
            continue;
        };
        let t = meta.file_type();
        let kind = if t.is_symlink() {
            Kind::Link
        } else if t.is_dir() {
            Kind::Dir
        } else {
            Kind::File
        };
        let cloud = st_flags(&meta) & SF_DATALESS != 0;
        let mut flags = if cloud { flag::CLOUD } else { 0 };
        let mut alt = NO_ALT;
        let text = match name.to_str() {
            Some(t) => std::borrow::Cow::Borrowed(t),
            None => {
                alt = alts.len() as u32;
                alts.push(name.clone());
                name.to_string_lossy()
            }
        };
        if text.starts_with('.') {
            flags |= flag::HIDDEN;
        }
        let file = kind == Kind::File;
        let name_start = names.len() as u32;
        names.push_str(&text);
        out.push(Raw {
            name_start,
            name_len: names.len() as u32 - name_start,
            kind,
            flags,
            len: if file { meta.size() } else { 0 },
            alloc: if file && !cloud { meta.blocks() * 512 } else { 0 },
            mtime: meta.mtime().clamp(0, u32::MAX as i64) as u32,
            dev: meta.dev(),
            ino: meta.ino(),
            nlink: meta.nlink() as u32,
            alt,
        });
    }
    if failed {
        Err(ListError::Other)
    } else {
        Ok(())
    }
}

pub fn normalize_root(input: &str) -> Option<(String, PathBuf)> {
    let s = crate::index::clean_path(input);
    if s.is_empty() {
        return None;
    }
    let abs = std::path::absolute(&s).ok()?;
    let display = crate::index::clean_path(&abs.to_string_lossy());
    Some((display.clone(), PathBuf::from(display)))
}

pub fn display(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

pub fn root_meta(path: &Path) -> Option<(u64, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return None;
    }
    Some((meta.dev(), meta.mtime().clamp(0, u32::MAX as i64) as u32))
}

pub fn cluster_size(_path: &Path) -> u64 {
    0
}

#[cfg(target_os = "macos")]
pub fn lower_priority(level: Level) -> bool {
    extern "C" {
        fn setiopolicy_np(iotype: libc::c_int, scope: libc::c_int, policy: libc::c_int) -> libc::c_int;
    }
    const IOPOL_TYPE_DISK: libc::c_int = 0;
    const IOPOL_SCOPE_THREAD: libc::c_int = 1;
    const IOPOL_THROTTLE: libc::c_int = 3;
    const IOPOL_UTILITY: libc::c_int = 4;
    let policy = if level == Level::Max { IOPOL_UTILITY } else { IOPOL_THROTTLE };
    let io = unsafe { setiopolicy_np(IOPOL_TYPE_DISK, IOPOL_SCOPE_THREAD, policy) } == 0;
    let cpu = level == Level::Max || unsafe { libc::setpriority(libc::PRIO_DARWIN_THREAD, 0, libc::PRIO_DARWIN_BG) } == 0;
    io && cpu
}

#[cfg(not(target_os = "macos"))]
pub fn lower_priority(_level: Level) -> bool {
    false
}
