use crate::args::parse_procargs2;
use crate::deadline::run_with_deadline;
use crate::kinfo::{parse_kinfo, Kinfo, KINFO_SIZE};
use crate::start::unix_start;
use crate::tcp::{parse_lsof, TcpRow};
use crate::{Proc, Usage};
use libc::{c_int, c_void, proc_taskinfo, CTL_KERN, KERN_ARGMAX, KERN_PROC, KERN_PROCARGS2, KERN_PROC_ALL, PROC_PIDPATHINFO_MAXSIZE, PROC_PIDTASKINFO};
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::OnceLock;
use std::time::Duration;

const LSOF_DEADLINE: Duration = Duration::from_secs(15);
const ARGS_MAX: usize = 4 * 1024 * 1024;
const TABLE_MAX: usize = 64 * 1024 * 1024;
const ATTEMPTS: usize = 4;

#[repr(C)]
struct Timebase {
    numer: u32,
    denom: u32,
}

extern "C" {
    fn mach_timebase_info(info: *mut Timebase) -> c_int;
}

static TIMEBASE: OnceLock<(u64, u64)> = OnceLock::new();

fn timebase() -> (u64, u64) {
    *TIMEBASE.get_or_init(|| {
        let mut t = Timebase { numer: 0, denom: 0 };
        let ok = unsafe { mach_timebase_info(&mut t) } == 0;
        if ok && t.numer > 0 && t.denom > 0 {
            (u64::from(t.numer), u64::from(t.denom))
        } else {
            (1, 1)
        }
    })
}

fn sysctl(mib: &mut [c_int], buf: *mut c_void, len: &mut usize) -> bool {
    unsafe { libc::sysctl(mib.as_mut_ptr(), mib.len() as u32, buf, len, null_mut(), 0) == 0 }
}

fn kinfo_all() -> Option<Vec<Kinfo>> {
    let mut mib = [CTL_KERN, KERN_PROC, KERN_PROC_ALL];
    for _ in 0..ATTEMPTS {
        let mut need = 0usize;
        if !sysctl(&mut mib, null_mut(), &mut need) || need == 0 {
            return None;
        }
        let bytes = need + need / 8 + KINFO_SIZE * 16;
        if bytes > TABLE_MAX {
            return None;
        }
        let mut buf = vec![0u64; bytes.div_ceil(size_of::<u64>())];
        let mut len = buf.len() * size_of::<u64>();
        if sysctl(&mut mib, buf.as_mut_ptr().cast::<c_void>(), &mut len) {
            let raw: Vec<u8> = buf.iter().flat_map(|w| w.to_ne_bytes()).take(len).collect();
            return Some(parse_kinfo(&raw));
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::ENOMEM) {
            return None;
        }
    }
    None
}

fn exe_name(pid: u32, comm: String) -> String {
    let Ok(pid) = c_int::try_from(pid) else { return comm };
    let mut buf = vec![0u8; PROC_PIDPATHINFO_MAXSIZE as usize];
    let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast::<c_void>(), buf.len() as u32) };
    if n > 0 {
        let path = String::from_utf8_lossy(&buf[..(n as usize).min(buf.len())]).into_owned();
        if let Some(base) = path.rsplit('/').next().filter(|b| !b.is_empty()) {
            return base.to_owned();
        }
    }
    comm
}

pub fn table() -> Option<Vec<Proc>> {
    let rows = kinfo_all()?
        .into_iter()
        .map(|k| {
            let (start_ms, start_raw) = unix_start(k.start_sec);
            Proc { pid: k.pid, ppid: k.ppid, name: exe_name(k.pid, k.comm), start_ms, start_raw }
        })
        .collect();
    Some(rows)
}

pub fn parents() -> Option<Vec<(u32, u32)>> {
    Some(kinfo_all()?.into_iter().map(|k| (k.pid, k.ppid)).collect())
}

fn pid_info<T>(pid: c_int, flavor: c_int) -> Option<T> {
    let mut info: T = unsafe { zeroed() };
    let size = c_int::try_from(size_of::<T>()).ok()?;
    let got = unsafe { libc::proc_pidinfo(pid, flavor, 0, (&mut info as *mut T).cast::<c_void>(), size) };
    (got == size).then_some(info)
}

pub fn usage(pid: u32) -> Option<Usage> {
    let info: proc_taskinfo = pid_info(c_int::try_from(pid).ok()?, PROC_PIDTASKINFO)?;
    let (numer, denom) = timebase();
    let ns = u128::from(info.pti_total_user + info.pti_total_system) * u128::from(numer) / u128::from(denom);
    Some(Usage { pid, working_set: Some(info.pti_resident_size), private_bytes: None, cpu_ms: Some(ns as f64 / 1_000_000.0) })
}

fn arg_max() -> usize {
    let mut v: c_int = 0;
    let mut len = size_of::<c_int>();
    let ok = sysctl(&mut [CTL_KERN, KERN_ARGMAX], (&mut v as *mut c_int).cast::<c_void>(), &mut len);
    if ok && v > 0 {
        (v as usize).min(ARGS_MAX)
    } else {
        256 * 1024
    }
}

pub fn cmdline(pid: u32) -> Option<String> {
    let pid = c_int::try_from(pid).ok()?;
    let mut buf = vec![0u8; arg_max()];
    let mut len = buf.len();
    if !sysctl(&mut [CTL_KERN, KERN_PROCARGS2, pid], buf.as_mut_ptr().cast::<c_void>(), &mut len) {
        return None;
    }
    parse_procargs2(&buf[..len.min(buf.len())])
}

pub fn ports() -> Option<Vec<TcpRow>> {
    let out = run_with_deadline("lsof", &["-nP", "-iTCP", "-sTCP:LISTEN,ESTABLISHED", "-F", "pnT"], LSOF_DEADLINE)?;
    (out.success || !out.stdout.is_empty()).then(|| parse_lsof(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_root_processes_like_ps() {
        let rows = table().expect("table");
        let launchd = rows.iter().find(|r| r.pid == 1).expect("launchd");
        assert_eq!(launchd.ppid, 0);
        assert!(launchd.start_raw.is_some());
        let ps = run_with_deadline("/bin/ps", &["-A", "-o", "pid="], Duration::from_secs(15)).expect("ps");
        let ours: std::collections::HashSet<u32> = rows.iter().map(|r| r.pid).collect();
        let theirs: Vec<u32> = ps.stdout.split_whitespace().filter_map(|p| p.parse().ok()).collect();
        let seen = theirs.iter().filter(|p| ours.contains(p)).count();
        assert!(seen * 100 >= theirs.len() * 95, "{seen} of {} ps pids", theirs.len());
        assert!(parents().expect("parents").iter().any(|&(p, pp)| p == 1 && pp == 0));
    }
}
