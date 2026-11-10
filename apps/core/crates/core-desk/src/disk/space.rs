use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    pub free: f64,
    pub total: f64,
}

pub fn fake_space(file: &str) -> Option<Space> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    Some(Space { free: v.get("free")?.as_f64()?, total: v.get("total")?.as_f64()? })
}

pub fn drive_label(mount: &str) -> String {
    let b = mount.as_bytes();
    if cfg!(windows) && (b.len() == 2 || (b.len() == 3 && matches!(b[2], b'\\' | b'/'))) && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return format!("{}:", (b[0] as char).to_ascii_uppercase());
    }
    mount.to_string()
}

#[cfg(target_os = "macos")]
pub fn statfs(dir: &str) -> Option<Space> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(Path::new(dir).as_os_str().as_bytes()).ok()?;
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 {
        return None;
    }
    let b = s.f_bsize as f64;
    Some(Space { free: s.f_bavail as f64 * b, total: s.f_blocks as f64 * b })
}

#[cfg(target_os = "macos")]
pub fn mount_of(dir: &str) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let real = std::fs::canonicalize(dir).ok()?;
    let dev = std::fs::metadata(&real).ok()?.dev();
    let mut at: PathBuf = real;
    loop {
        let Some(up) = at.parent().map(Path::to_path_buf) else { return Some(at.to_string_lossy().to_string()) };
        match std::fs::metadata(&up) {
            Ok(m) if m.dev() == dev => at = up,
            _ => return Some(at.to_string_lossy().to_string()),
        }
    }
}

#[cfg(windows)]
fn wide(p: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(p).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn statfs(dir: &str) -> Option<Space> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceW;
    let (mut spc, mut bps, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    let w = wide(dir);
    if unsafe { GetDiskFreeSpaceW(w.as_ptr(), &mut spc, &mut bps, &mut free, &mut total) } == 0 {
        return None;
    }
    let b = spc as f64 * bps as f64;
    Some(Space { free: free as f64 * b, total: total as f64 * b })
}

#[cfg(windows)]
pub fn mount_of(dir: &str) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW;
    let real = crate::claude::strip_verbatim(&std::fs::canonicalize(dir).ok()?);
    let w = wide(&real);
    let mut buf = vec![0u16; 1024];
    if unsafe { GetVolumePathNameW(w.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) } == 0 {
        return None;
    }
    let n = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..n]))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn statfs(_dir: &str) -> Option<Space> {
    None
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn mount_of(dir: &str) -> Option<String> {
    std::fs::canonicalize(dir).ok().map(|_| "/".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_real_drive() {
        if cfg!(windows) {
            assert_eq!(drive_label("c:\\"), "C:");
        } else {
            assert_eq!(drive_label("/Volumes/X"), "/Volumes/X");
        }
        let tmp = std::env::temp_dir();
        let m = mount_of(&tmp.to_string_lossy()).unwrap();
        let s = statfs(&m).unwrap();
        assert!(s.total >= s.free && s.total > 0.0);
        assert!(mount_of("/definitely/not/here/x").is_none());
    }
}
