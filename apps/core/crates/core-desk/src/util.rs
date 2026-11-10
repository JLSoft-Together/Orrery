use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

pub fn mtime_ms(meta: &std::fs::Metadata) -> f64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as f64 * 1000.0 + d.subsec_nanos() as f64 / 1e6)
        .unwrap_or(0.0)
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn iso_string(ms: f64) -> String {
    let ms = ms.floor() as i64;
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y,
        m,
        d,
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}

#[cfg(windows)]
pub fn local_date() -> (i64, u32, u32) {
    let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st) };
    (st.wYear as i64, st.wMonth as u32, st.wDay as u32)
}

#[cfg(target_os = "macos")]
pub fn local_date() -> (i64, u32, u32) {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut tm) };
    (tm.tm_year as i64 + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn local_date() -> (i64, u32, u32) {
    civil((now_ms() / 86_400_000.0).floor() as i64)
}

pub fn uuid() -> String {
    let mut b = [0u8; 16];
    if getrandom::fill(&mut b).is_err() {
        let n = now_ms().to_bits().to_le_bytes();
        b[..8].copy_from_slice(&n);
        b[8..].copy_from_slice(&std::process::id().to_le_bytes().repeat(2));
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

pub fn write_atomic(file: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, file)
}

pub fn resolve(p: &str) -> PathBuf {
    let raw = Path::new(p);
    let base = if raw.is_absolute() { raw.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(raw) };
    let mut out = PathBuf::new();
    for c in base.components() {
        match c {
            Component::ParentDir => {
                if out.parent().is_some() {
                    out.pop();
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn norm(p: &str) -> Vec<String> {
    resolve(p)
        .components()
        .map(|c| {
            let s = c.as_os_str().to_string_lossy().to_string();
            if cfg!(windows) {
                s.replace('/', "\\").to_lowercase()
            } else {
                s
            }
        })
        .collect()
}

pub fn is_inside_or_same(parent: &str, child: &str) -> bool {
    let a = norm(parent);
    let b = norm(child);
    b.len() >= a.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

pub fn basename(p: &str) -> String {
    let trimmed = p.trim_end_matches(|c| c == '/' || (cfg!(windows) && c == '\\'));
    let cut = trimmed.rfind(|c| c == '/' || (cfg!(windows) && c == '\\')).map(|i| i + 1).unwrap_or(0);
    trimmed[cut..].to_string()
}

pub fn fmt_bytes(n: f64) -> String {
    const G: f64 = 1024.0 * 1024.0 * 1024.0;
    const M: f64 = 1024.0 * 1024.0;
    if n >= G {
        format!("{} GB", to_fixed1(n / G))
    } else if n >= M {
        format!("{} MB", to_fixed1(n / M))
    } else if n >= 1024.0 {
        format!("{} KB", (n / 1024.0).round())
    } else {
        format!("{} B", crate::js::number(n))
    }
}

pub fn to_fixed1(v: f64) -> String {
    let exact = format!("{:.1100}", v);
    if let Some((_, frac)) = exact.split_once('.') {
        let tail = &frac[1..];
        if tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0') {
            return format!("{:.1}", v + 0.05 * v.signum() / 2.0);
        }
    }
    format!("{:.1}", v)
}

pub fn default_data_dir(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        return home.join("Library").join("Application Support").join("ccm");
    }
    std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| home.join("AppData").join("Local")).join("ccm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_and_uuid() {
        assert_eq!(iso_string(0.0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_string(1_762_732_800_123.0), "2025-11-10T00:00:00.123Z");
        let u = uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
    }

    #[test]
    fn inside_checks() {
        let root = if cfg!(windows) { "C:\\p\\a" } else { "/p/a" };
        let kid = if cfg!(windows) { "c:/P/a/b" } else { "/p/a/b" };
        assert!(is_inside_or_same(root, kid));
        assert!(is_inside_or_same(root, root));
        assert!(!is_inside_or_same(kid, root));
        assert!(!is_inside_or_same(root, &format!("{root}b")));
        assert_eq!(fmt_bytes(1536.0), "2 KB");
        assert_eq!(fmt_bytes(5.0 * 1024.0 * 1024.0), "5.0 MB");
    }
}
