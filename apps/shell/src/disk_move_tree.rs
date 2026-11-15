use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::disk_guard::{self as guard, Plat};

pub const CANCELLED: &str = "cancelled";
pub const SAMPLE_COUNT: usize = 6;
const SAMPLE_MAX: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeStat {
    pub files: u64,
    pub bytes: u64,
    pub dirs: u64,
    pub links: u64,
}

pub fn code_of(e: &io::Error) -> String {
    #[cfg(windows)]
    if let Some(c) = e.raw_os_error() {
        match c {
            32 | 33 => return "EBUSY".into(),
            5 => return "EPERM".into(),
            39 | 112 => return "ENOSPC".into(),
            145 => return "ENOTEMPTY".into(),
            183 | 80 => return "EEXIST".into(),
            _ => {}
        }
    }
    use io::ErrorKind as K;
    match e.kind() {
        K::NotFound => "ENOENT",
        K::PermissionDenied => "EACCES",
        K::AlreadyExists => "EEXIST",
        K::ResourceBusy => "EBUSY",
        K::DirectoryNotEmpty => "ENOTEMPTY",
        K::StorageFull => "ENOSPC",
        K::NotADirectory => "ENOTDIR",
        K::CrossesDevices => "EXDEV",
        _ => "failed",
    }
    .into()
}

pub fn lock_code(c: &str) -> bool {
    matches!(c, "EBUSY" | "EPERM" | "EACCES")
}

fn aborted(abort: Option<&AtomicBool>) -> bool {
    abort.is_some_and(|a| a.load(Ordering::SeqCst))
}

pub fn lstat(p: &Path) -> Option<fs::Metadata> {
    fs::symlink_metadata(p).ok()
}

pub fn is_link(p: &Path) -> bool {
    lstat(p).is_some_and(|m| m.file_type().is_symlink())
}

pub fn walk_tree(root: &Path, abort: Option<&AtomicBool>, on_file: &mut dyn FnMut(&Path, u64)) -> Result<TreeStat, String> {
    let mut stat = TreeStat::default();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if aborted(abort) {
            return Err(CANCELLED.into());
        }
        let rd = fs::read_dir(&dir).map_err(|e| code_of(&e))?;
        stat.dirs += 1;
        for entry in rd {
            let entry = entry.map_err(|e| code_of(&e))?;
            let p = entry.path();
            let ft = entry.file_type().map_err(|e| code_of(&e))?;
            if ft.is_symlink() {
                stat.links += 1;
            } else if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                let st = fs::symlink_metadata(&p).map_err(|e| code_of(&e))?;
                if st.file_type().is_symlink() {
                    stat.links += 1;
                    continue;
                }
                stat.files += 1;
                stat.bytes += st.len();
                on_file(&p, st.len());
            }
        }
    }
    Ok(stat)
}

pub fn plain_target(t: &Path) -> PathBuf {
    let s = t.to_string_lossy();
    if cfg!(windows) {
        if let Some(plain) = guard::plain_canonical(&s, Plat::Win) {
            return PathBuf::from(plain);
        }
    }
    t.to_path_buf()
}

#[cfg(windows)]
pub fn junction(target: &Path, link: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::CreateFileW;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const OPEN_EXISTING: u32 = 3;
    const FLAGS: u32 = 0x0020_0000 | 0x0200_0000;
    const FSCTL_SET_REPARSE_POINT: u32 = 0x0009_00A4;
    const TAG_MOUNT_POINT: u32 = 0xA000_0003;
    let plain = plain_target(target);
    let t = plain.to_string_lossy().into_owned();
    if !guard::is_absolute(&t, Plat::Win) || t.starts_with("\\\\") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "junction target"));
    }
    let sub: Vec<u16> = format!("\\??\\{t}").encode_utf16().collect();
    let print: Vec<u16> = t.encode_utf16().collect();
    let (sub_bytes, print_bytes) = (sub.len() * 2, print.len() * 2);
    let data_len = 8 + sub_bytes + 2 + print_bytes + 2;
    if data_len > 16 * 1024 - 8 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "junction target"));
    }
    let mut buf: Vec<u8> = Vec::with_capacity(8 + data_len);
    buf.extend(TAG_MOUNT_POINT.to_le_bytes());
    buf.extend((data_len as u16).to_le_bytes());
    buf.extend(0u16.to_le_bytes());
    buf.extend(0u16.to_le_bytes());
    buf.extend((sub_bytes as u16).to_le_bytes());
    buf.extend(((sub_bytes + 2) as u16).to_le_bytes());
    buf.extend((print_bytes as u16).to_le_bytes());
    for u in sub.iter().chain(std::iter::once(&0u16)).chain(print.iter()).chain(std::iter::once(&0u16)) {
        buf.extend(u.to_le_bytes());
    }
    fs::create_dir(link)?;
    let wide: Vec<u16> = link.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let h = unsafe { CreateFileW(wide.as_ptr(), GENERIC_WRITE, 0, std::ptr::null(), OPEN_EXISTING, FLAGS, std::ptr::null_mut()) };
    if h == INVALID_HANDLE_VALUE {
        let e = io::Error::last_os_error();
        let _ = fs::remove_dir(link);
        return Err(e);
    }
    let mut ret = 0u32;
    let ok = unsafe { DeviceIoControl(h, FSCTL_SET_REPARSE_POINT, buf.as_ptr().cast(), buf.len() as u32, std::ptr::null_mut(), 0, &mut ret, std::ptr::null_mut()) };
    let e = io::Error::last_os_error();
    unsafe { CloseHandle(h) };
    if ok == 0 {
        let _ = fs::remove_dir(link);
        return Err(e);
    }
    Ok(())
}

pub fn make_dir_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(windows)]
    return junction(target, link);
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, link);
}

fn copy_link(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    #[cfg(unix)]
    return std::os::unix::fs::symlink(&target, dst);
    #[cfg(windows)]
    {
        let is_dir = fs::metadata(src).map(|m| m.is_dir()).unwrap_or(false);
        let plain = plain_target(&target);
        let abs = guard::is_absolute(&plain.to_string_lossy(), Plat::Win);
        if is_dir && abs {
            junction(&plain, dst)
        } else if is_dir {
            std::os::windows::fs::symlink_dir(&target, dst)
        } else {
            std::os::windows::fs::symlink_file(&target, dst)
        }
    }
}

fn times_of(m: &fs::Metadata) -> fs::FileTimes {
    let mut t = fs::FileTimes::new();
    if let Ok(a) = m.accessed() {
        t = t.set_accessed(a);
    }
    if let Ok(w) = m.modified() {
        t = t.set_modified(w);
    }
    t
}

fn set_times(p: &Path, t: fs::FileTimes, dir: bool) {
    let mut o = fs::OpenOptions::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.access_mode(0x100);
        if dir {
            o.custom_flags(0x0200_0000);
        }
    }
    #[cfg(unix)]
    {
        let _ = dir;
        o.read(true);
    }
    if let Ok(f) = o.open(p) {
        let _ = f.set_times(t);
    }
}

pub fn copy_tree(src: &Path, dst: &Path, abort: Option<&AtomicBool>, on_bytes: &mut dyn FnMut(u64, u64)) -> Result<TreeStat, String> {
    let mut stat = TreeStat::default();
    let mut dir_times: Vec<(PathBuf, fs::FileTimes)> = Vec::new();
    let io = |e: io::Error| code_of(&e);
    fs::create_dir(dst).map_err(io)?;
    let mut stack: Vec<(PathBuf, PathBuf)> = vec![(src.to_path_buf(), dst.to_path_buf())];
    while let Some((s, d)) = stack.pop() {
        if aborted(abort) {
            return Err(CANCELLED.into());
        }
        let sst = fs::symlink_metadata(&s).map_err(io)?;
        dir_times.push((d.clone(), times_of(&sst)));
        stat.dirs += 1;
        for entry in fs::read_dir(&s).map_err(io)? {
            if aborted(abort) {
                return Err(CANCELLED.into());
            }
            let entry = entry.map_err(io)?;
            let sp = entry.path();
            let dp = d.join(entry.file_name());
            let st = fs::symlink_metadata(&sp).map_err(io)?;
            let ft = st.file_type();
            if ft.is_symlink() {
                copy_link(&sp, &dp).map_err(io)?;
                stat.links += 1;
            } else if ft.is_dir() {
                fs::create_dir(&dp).map_err(io)?;
                stack.push((sp, dp));
            } else if ft.is_file() {
                if fs::symlink_metadata(&dp).is_ok() {
                    return Err("EEXIST".into());
                }
                fs::copy(&sp, &dp).map_err(io)?;
                set_times(&dp, times_of(&st), false);
                stat.files += 1;
                stat.bytes += st.len();
                on_bytes(stat.bytes, stat.files);
            }
        }
    }
    for (p, t) in dir_times.into_iter().rev() {
        set_times(&p, t, true);
    }
    Ok(stat)
}

pub fn pick_samples(files: &[(String, u64)], n: usize) -> Vec<String> {
    let ok: Vec<&(String, u64)> = files.iter().filter(|f| f.1 <= SAMPLE_MAX).collect();
    if ok.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    let mut sorted = ok.clone();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for f in sorted.iter().take(n.div_ceil(2)) {
        if !out.contains(&f.0) {
            out.push(f.0.clone());
        }
    }
    let step = (ok.len() / n.saturating_sub(out.len()).max(1)).max(1);
    let mut i = 0;
    while i < ok.len() && out.len() < n {
        if !out.contains(&ok[i].0) {
            out.push(ok[i].0.clone());
        }
        i += step;
    }
    out
}

fn same_bytes(a: &Path, b: &Path, abort: Option<&AtomicBool>) -> Result<bool, String> {
    use std::io::Read;
    let mut fa = fs::File::open(a).map_err(|e| code_of(&e))?;
    let Ok(mut fb) = fs::File::open(b) else { return Ok(false) };
    let mut ba = vec![0u8; 256 * 1024];
    let mut bb = vec![0u8; 256 * 1024];
    loop {
        if aborted(abort) {
            return Err(CANCELLED.into());
        }
        let na = read_full(&mut fa, &mut ba).map_err(|e| code_of(&e))?;
        let Ok(nb) = read_full(&mut fb, &mut bb) else { return Ok(false) };
        if na != nb || ba[..na] != bb[..nb] {
            return Ok(false);
        }
        if na == 0 {
            return Ok(true);
        }
    }

    fn read_full(f: &mut fs::File, buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;
        while n < buf.len() {
            match f.read(&mut buf[n..])? {
                0 => break,
                k => n += k,
            }
        }
        Ok(n)
    }
}

pub fn verify_copy(src: &Path, dst: &Path, abort: Option<&AtomicBool>) -> Result<Option<&'static str>, String> {
    let mut files: Vec<(String, u64)> = Vec::new();
    let source = walk_tree(src, abort, &mut |p, size| {
        if let Ok(rel) = p.strip_prefix(src) {
            files.push((rel.to_string_lossy().into_owned(), size));
        }
    })?;
    let copy = walk_tree(dst, abort, &mut |_, _| {})?;
    if source.files != copy.files {
        return Ok(Some("count"));
    }
    if source.bytes != copy.bytes {
        return Ok(Some("bytes"));
    }
    if source.links != copy.links {
        return Ok(Some("links"));
    }
    for rel in pick_samples(&files, SAMPLE_COUNT) {
        if !same_bytes(&src.join(&rel), &dst.join(&rel), abort)? {
            return Ok(Some("hash"));
        }
    }
    Ok(None)
}

fn remove_link(p: &Path) -> io::Result<()> {
    fs::remove_file(p).or_else(|_| fs::remove_dir(p))
}

pub fn remove_tree(p: &Path) -> Result<(), String> {
    let Some(st) = lstat(p) else { return Ok(()) };
    if st.file_type().is_symlink() {
        return remove_link(p).map_err(|e| code_of(&e));
    }
    let mut last = String::new();
    for attempt in 0..4 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(200));
        }
        let r = if st.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
        match r {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => last = code_of(&e),
        }
    }
    Err(last)
}

pub fn unlink(p: &Path) -> Result<(), String> {
    remove_link(p).map_err(|e| code_of(&e))
}

pub fn retry<T>(mut f: impl FnMut() -> Result<T, String>) -> Result<T, String> {
    let mut last = String::from("failed");
    for _ in 0..20 {
        match f() {
            Ok(v) => return Ok(v),
            Err(c) => {
                let again = matches!(c.as_str(), "EBUSY" | "EPERM" | "EACCES" | "ENOTEMPTY");
                last = c;
                if !again {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(last)
}

pub fn rename(a: &Path, b: &Path) -> Result<(), String> {
    fs::rename(a, b).map_err(|e| code_of(&e))
}

pub fn realpath(p: &Path) -> Option<String> {
    fs::canonicalize(p).ok().map(|r| plain_target(&r).to_string_lossy().into_owned())
}

#[cfg(windows)]
fn wide(p: &str) -> Vec<u16> {
    p.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub fn space_of(p: &str) -> Option<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let w = wide(p);
    let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free) };
    (ok != 0).then_some((avail, total))
}

#[cfg(target_os = "macos")]
pub fn space_of(p: &str) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(Path::new(p).as_os_str().as_bytes()).ok()?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::statvfs(c.as_ptr(), &mut s) };
    (r == 0).then(|| (s.f_bavail as u64 * s.f_frsize as u64, s.f_blocks as u64 * s.f_frsize as u64))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn space_of(_p: &str) -> Option<(u64, u64)> {
    None
}

#[cfg(windows)]
pub fn dev_of(p: &str) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW;
    fs::metadata(p).ok()?;
    let w = wide(p);
    let mut out = vec![0u16; 1024];
    let ok = unsafe { GetVolumePathNameW(w.as_ptr(), out.as_mut_ptr(), out.len() as u32) };
    if ok == 0 {
        return None;
    }
    let n = out.iter().position(|c| *c == 0).unwrap_or(out.len());
    Some(String::from_utf16_lossy(&out[..n]).to_lowercase())
}

#[cfg(unix)]
pub fn dev_of(p: &str) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(p).ok().map(|m| m.dev().to_string())
}
