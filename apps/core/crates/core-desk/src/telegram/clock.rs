#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Local {
    pub y: i64,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
}

impl Local {
    pub fn minute_of_day(&self) -> i64 {
        self.h as i64 * 60 + self.mi as i64
    }

    pub fn hhmm(&self) -> String {
        format!("{:02}:{:02}", self.h, self.mi)
    }
}

#[cfg(target_os = "macos")]
pub fn local(ms: i64) -> Local {
    let t: libc::time_t = ms.div_euclid(1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    Local { y: tm.tm_year as i64 + 1900, mo: tm.tm_mon as u32 + 1, d: tm.tm_mday as u32, h: tm.tm_hour as u32, mi: tm.tm_min as u32, s: tm.tm_sec as u32 }
}

#[cfg(windows)]
pub fn local(ms: i64) -> Local {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    let ticks = (ms + 11_644_473_600_000) as u64 * 10_000;
    let ft = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut loc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let ok = unsafe { FileTimeToSystemTime(&ft, &mut utc) != 0 && SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut loc) != 0 };
    let s = if ok { loc } else { utc };
    Local { y: s.wYear as i64, mo: s.wMonth as u32, d: s.wDay as u32, h: s.wHour as u32, mi: s.wMinute as u32, s: s.wSecond as u32 }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn local(ms: i64) -> Local {
    let iso = crate::util::iso_string(ms as f64);
    let n = |a: usize, b: usize| iso[a..b].parse::<u32>().unwrap_or(0);
    Local { y: iso[0..4].parse().unwrap_or(1970), mo: n(5, 7), d: n(8, 10), h: n(11, 13), mi: n(14, 16), s: n(17, 19) }
}
