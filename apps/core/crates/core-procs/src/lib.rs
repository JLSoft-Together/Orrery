pub mod args;
pub mod deadline;
pub mod kinfo;
pub mod lines;
pub mod procfs;
pub mod serve;
pub mod start;
pub mod tcp;

#[cfg(windows)]
#[path = "win/mod.rs"]
mod sys;
#[cfg(target_os = "macos")]
#[path = "mac.rs"]
mod sys;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod sys;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
#[path = "other.rs"]
mod sys;

use serde::Serialize;
pub use tcp::TcpRow;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub start_ms: Option<i64>,
    pub start_raw: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub pid: u32,
    pub working_set: Option<u64>,
    pub private_bytes: Option<u64>,
    pub cpu_ms: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Cmd {
    pub pid: u32,
    pub cmd: String,
}

pub fn table() -> Option<Vec<Proc>> {
    sys::table()
}

pub fn pid_parents() -> Option<Vec<(u32, u32)>> {
    sys::parents()
}

pub fn usage(pids: &[u32]) -> Vec<Usage> {
    pids.iter().filter_map(|&p| sys::usage(p)).collect()
}

pub fn cmdline(pids: &[u32]) -> Vec<Cmd> {
    pids.iter().filter_map(|&pid| sys::cmdline(pid).map(|cmd| Cmd { pid, cmd })).collect()
}

pub fn ports() -> Option<Vec<TcpRow>> {
    sys::ports().map(|rows| rows.into_iter().filter(|r| r.port != 0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_table_has_this_process_with_its_parent_and_start() {
        let rows = table().expect("table");
        let me = std::process::id();
        let row = rows.iter().find(|r| r.pid == me).expect("own row");
        assert!(row.ppid > 0);
        assert!(rows.iter().any(|r| r.pid == row.ppid), "parent listed");
        assert!(row.start_ms.is_some() && row.start_raw.is_some());
        assert!(!row.name.is_empty());
        let parents = pid_parents().expect("parents");
        assert!(parents.contains(&(me, row.ppid)));
    }

    #[test]
    fn own_usage_and_cmdline_are_readable() {
        let me = std::process::id();
        let u = usage(&[me, u32::MAX - 7]);
        assert_eq!(u.len(), 1);
        assert!(u[0].working_set.unwrap_or(0) > 0);
        assert!(u[0].cpu_ms.is_some());
        let c = cmdline(&[me]);
        assert_eq!(c.len(), 1);
        assert!(!c[0].cmd.is_empty());
    }

    #[test]
    fn ports_see_a_listening_socket_of_this_process() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let rows = ports().expect("ports");
        let me = std::process::id();
        assert!(rows.iter().any(|r| r.pid == me && r.port == port && r.state == tcp::TCP_LISTEN && r.address == "127.0.0.1"), "{port} not in {} rows", rows.len());
    }
}
