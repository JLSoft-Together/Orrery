#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub cpu_ms: u64,
    pub rss: u64,
    pub peak_rss: u64,
}

#[cfg(windows)]
pub fn usage() -> Usage {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    let mut u = Usage::default();
    let h = unsafe { GetCurrentProcess() };
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut c, mut e, mut k, mut user) = (zero, zero, zero, zero);
    if unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut user) } != 0 {
        u.cpu_ms = (ft(k) + ft(user)) / 10_000;
    }
    let mut m: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    m.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    if unsafe { K32GetProcessMemoryInfo(h, &mut m, m.cb) } != 0 {
        u.rss = m.WorkingSetSize as u64;
        u.peak_rss = m.PeakWorkingSetSize as u64;
    }
    u
}

#[cfg(unix)]
pub fn usage() -> Usage {
    let mut r: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut r) } != 0 {
        return Usage::default();
    }
    let ms = |t: libc::timeval| t.tv_sec as u64 * 1000 + t.tv_usec as u64 / 1000;
    let unit = if cfg!(target_os = "macos") { 1 } else { 1024 };
    let peak = r.ru_maxrss as u64 * unit;
    Usage { cpu_ms: ms(r.ru_utime) + ms(r.ru_stime), rss: peak, peak_rss: peak }
}
