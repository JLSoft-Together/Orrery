use crate::tcp::{parse_tcp_table4, parse_tcp_table6, TcpRow};
use std::ptr::null_mut;
use windows_sys::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, TCP_TABLE_OWNER_PID_ALL};

const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const SLACK: usize = 4096;
const ATTEMPTS: usize = 4;
const TABLE_MAX: usize = 64 * 1024 * 1024;

fn read(af: u32) -> Option<Vec<u8>> {
    let mut size = 0u32;
    let mut status = unsafe { GetExtendedTcpTable(null_mut(), &mut size, 0, af, TCP_TABLE_OWNER_PID_ALL, 0) };
    for _ in 0..ATTEMPTS {
        if status != ERROR_INSUFFICIENT_BUFFER {
            break;
        }
        let bytes = size as usize + SLACK;
        if bytes > TABLE_MAX {
            return None;
        }
        let mut buf = vec![0u32; bytes.div_ceil(4)];
        let mut len = u32::try_from(buf.len() * 4).ok()?;
        status = unsafe { GetExtendedTcpTable(buf.as_mut_ptr().cast(), &mut len, 0, af, TCP_TABLE_OWNER_PID_ALL, 0) };
        if status == 0 {
            return Some(buf.iter().flat_map(|w| w.to_ne_bytes()).collect());
        }
        size = len;
    }
    (status == 0).then(|| vec![0u8; 4])
}

pub fn ports() -> Option<Vec<TcpRow>> {
    let v4 = read(AF_INET);
    let v6 = read(AF_INET6);
    if v4.is_none() && v6.is_none() {
        return None;
    }
    let mut rows = v4.map(|b| parse_tcp_table4(&b)).unwrap_or_default();
    rows.extend(v6.map(|b| parse_tcp_table6(&b)).unwrap_or_default());
    Some(rows)
}
