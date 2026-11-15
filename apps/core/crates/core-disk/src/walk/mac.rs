use super::unix::SF_DATALESS;
use super::{DirCtx, Kind, ListError, Raw, NO_ALT};
use crate::index::flag;
use std::cell::RefCell;
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

const ATTR_CMN_ERROR: u32 = 0x2000_0000;
const VREG: u32 = 1;
const VDIR: u32 = 2;
const VLNK: u32 = 5;
const BUF: usize = 256 * 1024;

thread_local! {
    static BULK: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

struct Fd(libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

fn errno() -> libc::c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn rd<T: Copy>(buf: &[u8], at: usize) -> Option<T> {
    if at.checked_add(std::mem::size_of::<T>())? > buf.len() {
        return None;
    }
    Some(unsafe { std::ptr::read_unaligned(buf.as_ptr().add(at) as *const T) })
}

pub fn list_bulk(dir: &Path, ctx: &DirCtx, names: &mut String, out: &mut Vec<Raw>, alts: &mut Vec<OsString>) -> Option<Result<(), ListError>> {
    BULK.with(|b| {
        let mut buf = b.borrow_mut();
        if buf.len() != BUF {
            buf.resize(BUF, 0);
        }
        list_with(dir, ctx, names, out, alts, &mut buf)
    })
}

fn list_with(dir: &Path, ctx: &DirCtx, names: &mut String, out: &mut Vec<Raw>, alts: &mut Vec<OsString>, buf: &mut [u8]) -> Option<Result<(), ListError>> {
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let nofollow = if ctx.root { 0 } else { libc::O_NOFOLLOW };
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | nofollow) };
    if fd < 0 {
        return Some(Err(match errno() {
            libc::EACCES | libc::EPERM => ListError::Denied,
            libc::ENOENT => ListError::Missing,
            _ => ListError::Other,
        }));
    }
    let fd = Fd(fd);
    let mut al: libc::attrlist = unsafe { std::mem::zeroed() };
    al.bitmapcount = libc::ATTR_BIT_MAP_COUNT;
    al.commonattr = libc::ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_ERROR | libc::ATTR_CMN_NAME | libc::ATTR_CMN_DEVID | libc::ATTR_CMN_OBJTYPE | libc::ATTR_CMN_MODTIME | libc::ATTR_CMN_FLAGS | libc::ATTR_CMN_FILEID;
    al.fileattr = libc::ATTR_FILE_LINKCOUNT | libc::ATTR_FILE_ALLOCSIZE | libc::ATTR_FILE_DATALENGTH;
    let start = out.len();
    let mut failed = false;
    loop {
        let n = unsafe { libc::getattrlistbulk(fd.0, &mut al as *mut libc::attrlist as *mut libc::c_void, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0) };
        if n < 0 {
            let e = errno();
            if out.len() == start && matches!(e, libc::ENOTSUP | libc::EINVAL) {
                return None;
            }
            return Some(Err(if e == libc::EACCES || e == libc::EPERM { ListError::Denied } else { ListError::Other }));
        }
        if n == 0 {
            break;
        }
        let mut at = 0usize;
        for _ in 0..n {
            let Some(len) = rd::<u32>(buf, at) else {
                failed = true;
                break;
            };
            let len = len as usize;
            let Some(end) = at.checked_add(len).filter(|e| len != 0 && *e <= buf.len()) else {
                failed = true;
                break;
            };
            let entry = &buf[at..end];
            if parse(entry, names, out, alts).is_none() {
                failed = true;
            }
            at = end;
        }
    }
    Some(if failed { Err(ListError::Other) } else { Ok(()) })
}

fn parse(e: &[u8], names: &mut String, out: &mut Vec<Raw>, alts: &mut Vec<OsString>) -> Option<()> {
    let ret: libc::attribute_set_t = rd(e, 4)?;
    let mut f = 4 + std::mem::size_of::<libc::attribute_set_t>();
    let c = ret.commonattr;
    if c & ATTR_CMN_ERROR != 0 {
        let err: u32 = rd(e, f)?;
        f += 4;
        if err != 0 {
            return None;
        }
    }
    let mut name: &[u8] = &[];
    if c & libc::ATTR_CMN_NAME != 0 {
        let r: libc::attrreference_t = rd(e, f)?;
        let s = usize::try_from(f as i64 + r.attr_dataoffset as i64).ok()?;
        let l = (r.attr_length as usize).saturating_sub(1);
        name = e.get(s..s.checked_add(l)?)?;
        f += 8;
    }
    let mut dev = 0u64;
    if c & libc::ATTR_CMN_DEVID != 0 {
        dev = rd::<i32>(e, f)? as u32 as u64;
        f += 4;
    }
    let mut obj = 0u32;
    if c & libc::ATTR_CMN_OBJTYPE != 0 {
        obj = rd(e, f)?;
        f += 4;
    }
    let mut mtime = 0u32;
    if c & libc::ATTR_CMN_MODTIME != 0 {
        mtime = rd::<i64>(e, f)?.clamp(0, u32::MAX as i64) as u32;
        f += 16;
    }
    let mut st_flags = 0u32;
    if c & libc::ATTR_CMN_FLAGS != 0 {
        st_flags = rd(e, f)?;
        f += 4;
    }
    let mut ino = 0u64;
    if c & libc::ATTR_CMN_FILEID != 0 {
        ino = rd(e, f)?;
        f += 8;
    }
    let (mut nlink, mut alloc, mut len) = (1u32, 0u64, 0u64);
    let fa = ret.fileattr;
    if fa & libc::ATTR_FILE_LINKCOUNT != 0 {
        nlink = rd(e, f)?;
        f += 4;
    }
    if fa & libc::ATTR_FILE_ALLOCSIZE != 0 {
        alloc = rd::<i64>(e, f)?.max(0) as u64;
        f += 8;
    }
    if fa & libc::ATTR_FILE_DATALENGTH != 0 {
        len = rd::<i64>(e, f)?.max(0) as u64;
    }
    if name.is_empty() {
        return None;
    }
    let kind = match obj {
        VDIR => Kind::Dir,
        VLNK => Kind::Link,
        VREG => Kind::File,
        _ => Kind::File,
    };
    let cloud = st_flags & SF_DATALESS != 0;
    let mut flags = if cloud { flag::CLOUD } else { 0 };
    if name.first() == Some(&b'.') {
        flags |= flag::HIDDEN;
    }
    let file = kind == Kind::File;
    let name_start = names.len() as u32;
    let mut alt = NO_ALT;
    match std::str::from_utf8(name) {
        Ok(t) => names.push_str(t),
        Err(_) => {
            names.push_str(&String::from_utf8_lossy(name));
            alt = alts.len() as u32;
            alts.push(OsString::from_vec(name.to_vec()));
        }
    }
    out.push(Raw {
        name_start,
        name_len: names.len() as u32 - name_start,
        kind,
        flags,
        len: if file { len } else { 0 },
        alloc: if file && !cloud { alloc } else { 0 },
        mtime,
        dev,
        ino,
        nlink,
        alt,
    });
    Some(())
}
