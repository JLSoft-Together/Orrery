use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    pub mount: String,
    pub label: String,
    pub fs: String,
    pub kind: &'static str,
    pub network: bool,
    pub total: u64,
    pub free: u64,
}

impl Volume {
    pub fn to_json(&self) -> Value {
        json!({"mount": self.mount, "label": self.label, "fs": self.fs, "kind": self.kind, "network": self.network, "total": self.total, "free": self.free})
    }
}

#[cfg(windows)]
pub fn list() -> Vec<Volume> {
    use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW};
    let mut buf = vec![0u16; 512];
    let n = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) } as usize;
    if n == 0 || n > buf.len() {
        return Vec::new();
    }
    let text = |b: &[u16]| String::from_utf16_lossy(&b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())]);
    let mut out = Vec::new();
    for root in buf[..n].split(|c| *c == 0).filter(|s| !s.is_empty()) {
        let mut w: Vec<u16> = root.to_vec();
        w.push(0);
        let kind = match unsafe { GetDriveTypeW(w.as_ptr()) } {
            2 => "removable",
            3 => "fixed",
            4 => "network",
            5 => "cdrom",
            6 => "ram",
            _ => "unknown",
        };
        let network = kind == "network";
        let mut v = Volume { mount: String::from_utf16_lossy(root), label: String::new(), fs: String::new(), kind, network, total: 0, free: 0 };
        if !network {
            let (mut label, mut fs) = (vec![0u16; 261], vec![0u16; 261]);
            let (mut serial, mut maxlen, mut fsflags) = (0u32, 0u32, 0u32);
            if unsafe { GetVolumeInformationW(w.as_ptr(), label.as_mut_ptr(), label.len() as u32, &mut serial, &mut maxlen, &mut fsflags, fs.as_mut_ptr(), fs.len() as u32) } != 0 {
                v.label = text(&label);
                v.fs = text(&fs);
            }
            let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
            if unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free) } != 0 {
                v.total = total;
                v.free = avail;
            }
        }
        out.push(v);
    }
    out
}

#[cfg(target_os = "macos")]
pub fn list() -> Vec<Volume> {
    let mut p: *mut libc::statfs = std::ptr::null_mut();
    let n = unsafe { libc::getmntinfo(&mut p, libc::MNT_NOWAIT) };
    if n <= 0 || p.is_null() {
        return Vec::new();
    }
    let all = unsafe { std::slice::from_raw_parts(p, n as usize) };
    let text = |b: &[libc::c_char]| {
        let bytes: Vec<u8> = b.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
        String::from_utf8_lossy(&bytes).to_string()
    };
    all.iter()
        .filter_map(|s| {
            let fs = text(&s.f_fstypename);
            if matches!(fs.as_str(), "devfs" | "autofs" | "nullfs") {
                return None;
            }
            let mount = text(&s.f_mntonname);
            let network = s.f_flags & libc::MNT_LOCAL as u32 == 0;
            let bs = s.f_bsize as u64;
            let label = mount.rsplit('/').find(|p| !p.is_empty()).unwrap_or("/").to_string();
            Some(Volume { mount, label, fs, kind: if network { "network" } else { "fixed" }, network, total: s.f_blocks * bs, free: s.f_bavail * bs })
        })
        .collect()
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn list() -> Vec<Volume> {
    let c = std::ffi::CString::new("/").unwrap();
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return Vec::new();
    }
    let bs = s.f_frsize as u64;
    vec![Volume { mount: "/".into(), label: "/".into(), fs: String::new(), kind: "fixed", network: false, total: s.f_blocks as u64 * bs, free: s.f_bavail as u64 * bs }]
}
