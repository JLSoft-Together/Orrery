use crate::args::join_cmdline;
use crate::procfs::{parse_btime, parse_net_tcp, parse_stat, socket_inode, Stat};
use crate::start::unix_start;
use crate::tcp::TcpRow;
use crate::{Proc, Usage};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::OnceLock;

struct Clock {
    btime: Option<i64>,
    tck: u64,
    page: u64,
}

static CLOCK: OnceLock<Clock> = OnceLock::new();

fn clock() -> &'static Clock {
    CLOCK.get_or_init(|| {
        let tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        Clock {
            btime: fs::read_to_string("/proc/stat").ok().as_deref().and_then(parse_btime),
            tck: if tck > 0 { tck as u64 } else { 100 },
            page: if page > 0 { page as u64 } else { 4096 },
        }
    })
}

fn pids() -> Option<Vec<u32>> {
    let dir = fs::read_dir("/proc").ok()?;
    Some(dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok()).collect())
}

fn stat(pid: u32) -> Option<Stat> {
    parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

pub fn table() -> Option<Vec<Proc>> {
    let c = clock();
    let rows = pids()?
        .into_iter()
        .filter_map(|pid| {
            let s = stat(pid)?;
            let secs = c.btime.and_then(|b| i64::try_from(s.start_ticks / c.tck).ok().map(|t| b + t));
            let (start_ms, start_raw) = secs.map(unix_start).unwrap_or((None, None));
            Some(Proc { pid, ppid: s.ppid, name: s.comm, start_ms, start_raw })
        })
        .collect();
    Some(rows)
}

pub fn parents() -> Option<Vec<(u32, u32)>> {
    Some(pids()?.into_iter().filter_map(|pid| Some((pid, stat(pid)?.ppid))).collect())
}

pub fn usage(pid: u32) -> Option<Usage> {
    let c = clock();
    let s = stat(pid)?;
    let cpu_ms = (s.utime + s.stime) as f64 * 1000.0 / c.tck as f64;
    Some(Usage { pid, working_set: Some(s.rss_pages * c.page), private_bytes: None, cpu_ms: Some(cpu_ms) })
}

pub fn cmdline(pid: u32) -> Option<String> {
    let raw = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    join_cmdline(&raw).or_else(|| stat(pid).map(|s| format!("[{}]", s.comm)))
}

fn socket_owners(wanted: &HashSet<u64>) -> HashMap<u64, u32> {
    let mut owners = HashMap::new();
    for pid in pids().unwrap_or_default() {
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else { continue };
        for fd in fds.flatten() {
            let Ok(link) = fs::read_link(fd.path()) else { continue };
            if let Some(inode) = link.to_str().and_then(socket_inode).filter(|i| wanted.contains(i)) {
                owners.entry(inode).or_insert(pid);
            }
        }
    }
    owners
}

pub fn ports() -> Option<Vec<TcpRow>> {
    let v4 = fs::read_to_string("/proc/net/tcp").ok();
    let v6 = fs::read_to_string("/proc/net/tcp6").ok();
    if v4.is_none() && v6.is_none() {
        return None;
    }
    let mut rows = v4.map(|t| parse_net_tcp(&t, false)).unwrap_or_default();
    rows.extend(v6.map(|t| parse_net_tcp(&t, true)).unwrap_or_default());
    let wanted: HashSet<u64> = rows.iter().map(|(inode, _)| *inode).collect();
    let owners = socket_owners(&wanted);
    Some(
        rows.into_iter()
            .filter_map(|(inode, mut r)| {
                r.pid = *owners.get(&inode)?;
                Some(r)
            })
            .collect(),
    )
}
