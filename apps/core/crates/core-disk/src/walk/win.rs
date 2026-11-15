use super::{DirCtx, Kind, Level, ListError, Raw};
use crate::index::flag;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND, FILETIME, HANDLE, INVALID_HANDLE_VALUE, NO_ERROR};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW, GetCompressedFileSizeW, GetDiskFreeSpaceW, GetFileInformationByHandle, GetVolumePathNameW, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FIND_FIRST_EX_LARGE_FETCH, INVALID_FILE_SIZE, OPEN_EXISTING, WIN32_FIND_DATAW,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN, THREAD_PRIORITY_BELOW_NORMAL};

const ATTR_HIDDEN: u32 = 0x2;
const ATTR_SYSTEM: u32 = 0x4;
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_SPARSE: u32 = 0x200;
const ATTR_REPARSE: u32 = 0x400;
const ATTR_COMPRESSED: u32 = 0x800;
const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x40000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x400000;
const CLOUD_ATTRS: u32 = ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS;
const LINK_TAGS: [u32; 5] = [0xA000_0003, 0xA000_000C, 0x8000_001B, 0xA000_001D, 0x8000_0023];
const EPOCH_DIFF: u64 = 116_444_736_000_000_000;

pub fn is_cloud_tag(tag: u32) -> bool {
    tag & 0xFFFF_0FFF == 0x9000_001A
}

pub fn wide(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

fn unix_secs(ft: FILETIME) -> u32 {
    let t = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    (t.saturating_sub(EPOCH_DIFF) / 10_000_000).min(u32::MAX as u64) as u32
}

struct Find(HANDLE);

impl Drop for Find {
    fn drop(&mut self) {
        unsafe { FindClose(self.0) };
    }
}

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

fn file_name(d: &WIN32_FIND_DATAW) -> &[u16] {
    let n = d.cFileName.iter().position(|c| *c == 0).unwrap_or(d.cFileName.len());
    &d.cFileName[..n]
}

fn child_wide(dir: &[u16], name: &[u16]) -> Vec<u16> {
    let mut w = Vec::with_capacity(dir.len() + name.len() + 2);
    w.extend_from_slice(dir);
    if w.last() != Some(&(b'\\' as u16)) {
        w.push(b'\\' as u16);
    }
    w.extend_from_slice(name);
    w.push(0);
    w
}

fn compressed_size(path: &[u16]) -> Option<u64> {
    let mut high = 0u32;
    let low = unsafe { GetCompressedFileSizeW(path.as_ptr(), &mut high) };
    if low == INVALID_FILE_SIZE && unsafe { GetLastError() } != NO_ERROR {
        return None;
    }
    Some(((high as u64) << 32) | low as u64)
}

fn link_info(path: &[u16]) -> Option<(u64, u64, u32)> {
    let h = unsafe { CreateFileW(path.as_ptr(), FILE_READ_ATTRIBUTES, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, std::ptr::null(), OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, std::ptr::null_mut()) };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    let h = Handle(h);
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(h.0, &mut info) } == 0 {
        return None;
    }
    Some((info.dwVolumeSerialNumber as u64, ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64, info.nNumberOfLinks))
}

fn round_up(len: u64, cluster: u64) -> u64 {
    if cluster == 0 {
        len
    } else {
        len.div_ceil(cluster) * cluster
    }
}

pub fn list(dir: &Path, ctx: &DirCtx, names: &mut String, out: &mut Vec<Raw>) -> Result<(), ListError> {
    let base: Vec<u16> = dir.as_os_str().encode_wide().collect();
    let mut pattern = base.clone();
    if pattern.last() != Some(&(b'\\' as u16)) {
        pattern.push(b'\\' as u16);
    }
    pattern.extend_from_slice(&[b'*' as u16, 0]);
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let h = unsafe { FindFirstFileExW(pattern.as_ptr(), FindExInfoBasic, &mut data as *mut WIN32_FIND_DATAW as *mut c_void, FindExSearchNameMatch, std::ptr::null(), FIND_FIRST_EX_LARGE_FETCH) };
    if h == INVALID_HANDLE_VALUE {
        return match unsafe { GetLastError() } {
            ERROR_FILE_NOT_FOUND => Ok(()),
            ERROR_ACCESS_DENIED => Err(ListError::Denied),
            ERROR_PATH_NOT_FOUND => Err(ListError::Missing),
            _ => Err(ListError::Other),
        };
    }
    let find = Find(h);
    loop {
        push(&base, &data, ctx, names, out);
        if unsafe { FindNextFileW(find.0, &mut data) } == 0 {
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES { Ok(()) } else { Err(ListError::Other) };
        }
    }
}

fn push(base: &[u16], d: &WIN32_FIND_DATAW, ctx: &DirCtx, names: &mut String, out: &mut Vec<Raw>) {
    let wname = file_name(d);
    if wname == [b'.' as u16] || wname == [b'.' as u16, b'.' as u16] {
        return;
    }
    let attrs = d.dwFileAttributes;
    let reparse = attrs & ATTR_REPARSE != 0;
    let tag = if reparse { d.dwReserved0 } else { 0 };
    let is_dir = attrs & ATTR_DIRECTORY != 0;
    let cloud = attrs & CLOUD_ATTRS != 0;
    let kind = if reparse && !is_cloud_tag(tag) && (is_dir || LINK_TAGS.contains(&tag)) {
        Kind::Link
    } else if is_dir {
        Kind::Dir
    } else {
        Kind::File
    };
    let mut flags = 0u16;
    if attrs & ATTR_HIDDEN != 0 {
        flags |= flag::HIDDEN;
    }
    if attrs & ATTR_SYSTEM != 0 {
        flags |= flag::SYSTEM;
    }
    if cloud {
        flags |= flag::CLOUD;
    }
    let len = if kind == Kind::File { ((d.nFileSizeHigh as u64) << 32) | d.nFileSizeLow as u64 } else { 0 };
    let (mut dev, mut ino, mut nlink) = (0u64, 0u64, 1u32);
    let alloc = if kind != Kind::File || cloud {
        0
    } else {
        let needs_path = attrs & (ATTR_COMPRESSED | ATTR_SPARSE) != 0 || ctx.hardlinks;
        let path = if needs_path { child_wide(base, wname) } else { Vec::new() };
        if ctx.hardlinks {
            if let Some((v, i, n)) = link_info(&path) {
                (dev, ino, nlink) = (v, i, n);
            }
        }
        if attrs & (ATTR_COMPRESSED | ATTR_SPARSE) != 0 {
            compressed_size(&path).map(|s| round_up(s, ctx.cluster)).unwrap_or_else(|| round_up(len, ctx.cluster))
        } else {
            round_up(len, ctx.cluster)
        }
    };
    let name_start = names.len() as u32;
    names.push_str(&String::from_utf16_lossy(wname));
    out.push(Raw { name_start, name_len: names.len() as u32 - name_start, kind, flags, len, alloc, mtime: unix_secs(d.ftLastWriteTime), dev, ino, nlink });
}

pub fn normalize_root(input: &str) -> Option<(String, PathBuf)> {
    let s = crate::index::clean_path(input);
    if s.is_empty() {
        return None;
    }
    let abs = std::path::absolute(&s).ok()?;
    let display = crate::index::clean_path(&abs.to_string_lossy());
    let sys = match display.strip_prefix("\\\\") {
        Some(rest) => format!("\\\\?\\UNC\\{rest}"),
        None => format!("\\\\?\\{display}"),
    };
    Some((display, PathBuf::from(sys)))
}

pub fn display(p: &Path) -> String {
    crate::index::clean_path(&p.to_string_lossy())
}

pub fn root_meta(path: &Path) -> Option<(u64, u32)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return None;
    }
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs().min(u32::MAX as u64) as u32).unwrap_or(0);
    Some((0, mtime))
}

pub fn cluster_size(path: &Path) -> u64 {
    let w = wide(path);
    let mut vol = vec![0u16; 1024];
    if unsafe { GetVolumePathNameW(w.as_ptr(), vol.as_mut_ptr(), vol.len() as u32) } == 0 {
        return 4096;
    }
    let (mut spc, mut bps, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    if unsafe { GetDiskFreeSpaceW(vol.as_ptr(), &mut spc, &mut bps, &mut free, &mut total) } == 0 || spc == 0 || bps == 0 {
        return 4096;
    }
    spc as u64 * bps as u64
}

pub fn lower_priority(level: Level) -> bool {
    let mode = match level {
        Level::Max => THREAD_PRIORITY_BELOW_NORMAL,
        _ => THREAD_MODE_BACKGROUND_BEGIN,
    };
    unsafe { SetThreadPriority(GetCurrentThread(), mode) != 0 }
}
