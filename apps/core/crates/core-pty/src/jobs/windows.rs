use std::collections::{HashMap, HashSet};
use std::io;
use std::mem::{size_of, zeroed};
use std::ptr::null;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};

pub struct Owned(pub HANDLE);

unsafe impl Send for Owned {}
unsafe impl Sync for Owned {}

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

pub struct Job(Owned);

impl Job {
    pub fn new() -> io::Result<Job> {
        let h = unsafe { CreateJobObjectW(null(), null()) };
        if h.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = Job(Owned(h));
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_BREAKAWAY_OK;
        let ok = unsafe {
            SetInformationJobObject(h, JobObjectExtendedLimitInformation, &info as *const _ as *const _, size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32)
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    pub fn assign(&self, process: &Owned) -> io::Result<()> {
        unsafe { self.assign_raw(process.0) }
    }

    unsafe fn assign_raw(&self, process: HANDLE) -> io::Result<()> {
        if unsafe { AssignProcessToJobObject(self.0 .0, process) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn terminate(&self, code: u32) -> bool {
        unsafe { TerminateJobObject(self.0 .0, code) != 0 }
    }
}

pub fn adopt_self() -> io::Result<()> {
    let job = Job::new()?;
    unsafe { job.assign_raw(GetCurrentProcess()) }?;
    std::mem::forget(job);
    Ok(())
}

fn creation_time(h: HANDLE) -> Option<u64> {
    let mut c: FILETIME = unsafe { zeroed() };
    let mut e: FILETIME = unsafe { zeroed() };
    let mut k: FILETIME = unsafe { zeroed() };
    let mut u: FILETIME = unsafe { zeroed() };
    if unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) } == 0 {
        return None;
    }
    Some(((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64)
}

fn parent_table() -> HashMap<u32, Vec<u32>> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return kids;
    }
    let snap = Owned(snap);
    let mut e: PROCESSENTRY32W = unsafe { zeroed() };
    e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut more = unsafe { Process32FirstW(snap.0, &mut e) } != 0;
    while more {
        if e.th32ProcessID != 0 && e.th32ProcessID != e.th32ParentProcessID {
            kids.entry(e.th32ParentProcessID).or_default().push(e.th32ProcessID);
        }
        more = unsafe { Process32NextW(snap.0, &mut e) } != 0;
    }
    kids
}

pub struct Descendant {
    pid: u32,
    handle: Owned,
}

pub fn descendants(root_pid: u32, root: &Owned) -> Vec<Descendant> {
    let Some(root_born) = creation_time(root.0) else { return Vec::new() };
    let kids = parent_table();
    let mut out = Vec::new();
    let mut seen = HashSet::from([root_pid]);
    let mut stack = vec![(root_pid, root_born)];
    while let Some((pid, born)) = stack.pop() {
        for &child in kids.get(&pid).map(Vec::as_slice).unwrap_or(&[]) {
            if !seen.insert(child) || out.len() >= 4096 {
                continue;
            }
            let h = unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, 0, child) };
            if h.is_null() {
                continue;
            }
            let handle = Owned(h);
            match creation_time(h) {
                Some(t) if t >= born => {
                    stack.push((child, t));
                    out.push(Descendant { pid: child, handle });
                }
                _ => {}
            }
        }
    }
    out
}

pub fn terminate_all(list: &[Descendant], code: u32) -> usize {
    list.iter().filter(|d| d.pid != 0 && unsafe { TerminateProcess(d.handle.0, code) } != 0).count()
}

pub fn terminate(process: &Owned, code: u32) -> bool {
    unsafe { TerminateProcess(process.0, code) != 0 }
}
