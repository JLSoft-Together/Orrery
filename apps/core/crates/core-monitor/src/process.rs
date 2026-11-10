use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStart {
    Filetime(u64),
    Lstart(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub start: ProcessStart,
    pub parent_pid: Option<u32>,
    pub parent_name: Option<String>,
}

const FILETIME_TOLERANCE: i128 = 10_000;

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn word(t: &str, n: usize) -> bool {
    t.len() == n && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn digits(t: &str, lo: usize, hi: usize) -> bool {
    (lo..=hi).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_digit())
}

fn clock(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 8 && b[2] == b':' && b[5] == b':' && [0, 1, 3, 4, 6, 7].iter().all(|i| b[*i].is_ascii_digit())
}

pub fn is_lstart(s: &str) -> bool {
    let t: Vec<&str> = s.split_whitespace().collect();
    t.len() == 5 && word(t[0], 3) && word(t[1], 3) && digits(t[2], 1, 2) && clock(t[3]) && digits(t[4], 4, 4)
}

pub fn proc_start_matches(proc_start: Option<&str>, start: &ProcessStart) -> bool {
    let Some(ps) = proc_start else { return true };
    match start {
        ProcessStart::Lstart(v) => !is_lstart(ps.trim()) || squash(ps) == squash(v),
        ProcessStart::Filetime(v) => {
            if ps.is_empty() || !ps.bytes().all(|b| b.is_ascii_digit()) {
                return true;
            }
            let Ok(n) = ps.parse::<i128>() else { return false };
            let diff = n - i128::from(*v);
            (-FILETIME_TOLERANCE..=FILETIME_TOLERANCE).contains(&diff)
        }
    }
}

#[cfg(windows)]
pub fn is_pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    if pid <= 1 {
        return false;
    }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return GetLastError() == ERROR_ACCESS_DENIED;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        !ok || code == 259
    }
}

#[cfg(unix)]
pub fn is_pid_alive(pid: u32) -> bool {
    if pid <= 1 || pid > i32::MAX as u32 {
        return false;
    }
    let ok = unsafe { libc::kill(pid as i32, 0) } == 0;
    ok || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn info_from_rows(pids: &[u32], rows: &[core_procs::Proc]) -> Option<HashMap<u32, ProcessInfo>> {
    let by_pid: HashMap<u32, &core_procs::Proc> = rows.iter().map(|r| (r.pid, r)).collect();
    let mut out = HashMap::new();
    for pid in pids {
        let Some(r) = by_pid.get(pid) else { continue };
        let start = start_of(r)?;
        out.insert(*pid, ProcessInfo { pid: *pid, start, parent_pid: Some(r.ppid), parent_name: by_pid.get(&r.ppid).map(|p| p.name.clone()) });
    }
    Some(out)
}

#[cfg(windows)]
fn start_of(r: &core_procs::Proc) -> Option<ProcessStart> {
    let raw = r.start_raw.as_deref()?;
    raw.parse::<u64>().ok().map(ProcessStart::Filetime)
}

#[cfg(not(windows))]
fn start_of(r: &core_procs::Proc) -> Option<ProcessStart> {
    let raw = squash(r.start_raw.as_deref().unwrap_or(""));
    is_lstart(&raw).then_some(ProcessStart::Lstart(raw))
}

fn valid_pids(pids: &[u32]) -> Option<Vec<u32>> {
    let valid: Vec<u32> = pids.iter().copied().filter(|p| *p > 0).collect();
    (!valid.is_empty() && (cfg!(windows) || cfg!(target_os = "macos"))).then_some(valid)
}

pub fn query_processes(pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
    let valid = valid_pids(pids)?;
    let rows = core_procs::table()?;
    info_from_rows(&valid, &rows)
}

pub fn query_processes_in(pids: &[u32], rows: &[core_procs::Proc]) -> Option<HashMap<u32, ProcessInfo>> {
    info_from_rows(&valid_pids(pids)?, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_filetime_within_a_millisecond_and_lstart_by_text() {
        let ft = ProcessStart::Filetime(134_358_400_013_081_891);
        assert!(proc_start_matches(None, &ft));
        assert!(proc_start_matches(Some("134358400013081891"), &ft));
        assert!(proc_start_matches(Some("134358400013091891"), &ft));
        assert!(!proc_start_matches(Some("134358400013091892"), &ft));
        assert!(proc_start_matches(Some("Tue Oct  7 10:00:00 2026"), &ft));
        let ls = ProcessStart::Lstart("Tue Oct 7 10:00:00 2026".into());
        assert!(proc_start_matches(Some("Tue Oct  7 10:00:00 2026"), &ls));
        assert!(!proc_start_matches(Some("Tue Oct  7 10:00:01 2026"), &ls));
        assert!(proc_start_matches(Some("134358400013081891"), &ls));
    }

    #[test]
    fn this_process_is_alive_and_found_with_its_parent() {
        assert!(is_pid_alive(std::process::id()));
        assert!(!is_pid_alive(0));
        assert!(!is_pid_alive(1));
        if cfg!(any(windows, target_os = "macos")) {
            let me = std::process::id();
            let info = query_processes(&[me]).expect("table");
            assert!(info[&me].parent_pid.is_some());
        }
    }
}
