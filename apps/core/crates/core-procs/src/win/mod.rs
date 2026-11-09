mod ports;

pub use ports::ports;

use crate::start::filetime_ms;
use crate::{Proc, Usage};
use std::mem::{size_of, zeroed};
use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessCommandLineInformation};
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
const CMD_FIRST: usize = 1024;
const CMD_MAX: usize = 1024 * 1024;
const UNICODE_STRING_SIZE: usize = size_of::<[usize; 2]>();

struct Handle(HANDLE);

impl Handle {
    fn new(h: HANDLE) -> Option<Handle> {
        (!h.is_null() && h != INVALID_HANDLE_VALUE).then_some(Handle(h))
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

fn open(pid: u32) -> Option<Handle> {
    Handle::new(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })
}

fn ticks(f: &FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

struct Times {
    created: u64,
    kernel: u64,
    user: u64,
}

fn times(h: &Handle) -> Option<Times> {
    let mut c: FILETIME = unsafe { zeroed() };
    let mut e: FILETIME = unsafe { zeroed() };
    let mut k: FILETIME = unsafe { zeroed() };
    let mut u: FILETIME = unsafe { zeroed() };
    let ok = unsafe { GetProcessTimes(h.0, &mut c, &mut e, &mut k, &mut u) } != 0;
    ok.then(|| Times { created: ticks(&c), kernel: ticks(&k), user: ticks(&u) })
}

fn exe_name(raw: &[u16]) -> String {
    let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

fn snapshot() -> Option<Vec<Proc>> {
    let snap = Handle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })?;
    let mut e: PROCESSENTRY32W = unsafe { zeroed() };
    e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut rows = Vec::new();
    let mut more = unsafe { Process32FirstW(snap.0, &mut e) } != 0;
    while more {
        rows.push(Proc { pid: e.th32ProcessID, ppid: e.th32ParentProcessID, name: exe_name(&e.szExeFile), start_ms: None, start_raw: None });
        more = unsafe { Process32NextW(snap.0, &mut e) } != 0;
    }
    Some(rows)
}

pub fn parents() -> Option<Vec<(u32, u32)>> {
    Some(snapshot()?.into_iter().map(|p| (p.pid, p.ppid)).collect())
}

pub fn table() -> Option<Vec<Proc>> {
    let mut rows = snapshot()?;
    for r in rows.iter_mut().filter(|r| r.pid > 4) {
        if let Some(t) = open(r.pid).as_ref().and_then(times) {
            r.start_ms = Some(filetime_ms(t.created));
            r.start_raw = Some(t.created.to_string());
        }
    }
    Some(rows)
}

pub fn usage(pid: u32) -> Option<Usage> {
    let h = open(pid)?;
    let cpu_ms = times(&h).map(|t| (t.kernel + t.user) as f64 / 10_000.0);
    let mut m: PROCESS_MEMORY_COUNTERS_EX = unsafe { zeroed() };
    m.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let ok = unsafe { K32GetProcessMemoryInfo(h.0, (&mut m as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(), m.cb) } != 0;
    Some(Usage { pid, working_set: ok.then_some(m.WorkingSetSize as u64), private_bytes: ok.then_some(m.PrivateUsage as u64), cpu_ms })
}

fn query_cmdline(h: &Handle, buf: &mut [u64], ret: &mut u32) -> i32 {
    let len = u32::try_from(std::mem::size_of_val(buf)).unwrap_or(u32::MAX);
    unsafe { NtQueryInformationProcess(h.0, ProcessCommandLineInformation, buf.as_mut_ptr().cast(), len, ret) }
}

pub fn unicode_string_text(b: &[u8]) -> Option<String> {
    let len = usize::from(u16::from_le_bytes([*b.first()?, *b.get(1)?]));
    if len % 2 != 0 || UNICODE_STRING_SIZE + len > b.len() {
        return None;
    }
    let units: Vec<u16> = b[UNICODE_STRING_SIZE..UNICODE_STRING_SIZE + len].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Some(String::from_utf16_lossy(&units))
}

pub fn cmdline(pid: u32) -> Option<String> {
    let h = open(pid)?;
    let mut buf = vec![0u64; CMD_FIRST / size_of::<u64>()];
    let mut ret = 0u32;
    let mut status = query_cmdline(&h, &mut buf, &mut ret);
    let need = ret as usize;
    if status == STATUS_INFO_LENGTH_MISMATCH && need > buf.len() * size_of::<u64>() {
        if need > CMD_MAX {
            return None;
        }
        buf = vec![0u64; need.div_ceil(size_of::<u64>())];
        status = query_cmdline(&h, &mut buf, &mut ret);
    }
    if status != 0 {
        return None;
    }
    let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_ne_bytes()).collect();
    unicode_string_text(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_string_header_is_checked() {
        let mut b = vec![0u8; UNICODE_STRING_SIZE];
        b[0] = 4;
        b.extend([b'h', 0, b'i', 0, 0, 0]);
        assert_eq!(unicode_string_text(&b).as_deref(), Some("hi"));
        b[0] = 3;
        assert!(unicode_string_text(&b).is_none());
        b[0] = 200;
        assert!(unicode_string_text(&b).is_none());
        assert!(unicode_string_text(&[1]).is_none());
    }

    #[test]
    fn system_pids_have_no_start_and_long_cmdlines_are_read() {
        let rows = table().unwrap();
        assert!(rows.iter().filter(|r| r.pid <= 4).all(|r| r.start_raw.is_none()));
        let arg = "x".repeat(3000);
        let mut child = std::process::Command::new("cmd.exe").args(["/d", "/c", "ping", "-n", "3", "127.0.0.1", ">nul", "&", "rem", &arg]).spawn().unwrap();
        let mut got = None;
        for _ in 0..50 {
            got = cmdline(child.id());
            if got.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = child.kill();
        let _ = child.wait();
        assert!(got.expect("cmdline").ends_with(&arg));
    }
}
