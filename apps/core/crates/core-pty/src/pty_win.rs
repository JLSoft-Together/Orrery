use crate::jobs::windows::{descendants, terminate, terminate_all, Job, Owned};
use crate::session::{dedupe_env, start_reader, PtyEvent, Sink, SpawnSpec, EXIT_DRAIN};
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Write};
use std::mem::{size_of, zeroed};
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Console::{ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess, InitializeProcThreadAttributeList, ResumeThread, UpdateProcThreadAttribute, WaitForSingleObject,
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

const PSEUDOCONSOLE_RESIZE_QUIRK: u32 = 0x2;
const PSEUDOCONSOLE_WIN32_INPUT_MODE: u32 = 0x4;
const KILLED_CODE: u32 = 1;
pub const SETTLE_QUIET: Duration = Duration::from_millis(30);
pub const SETTLE_MAX: Duration = Duration::from_millis(150);

pub struct Force;

impl Force {
    pub fn fire(&self) {}
}

struct Hpc(HPCON);

unsafe impl Send for Hpc {}

impl Drop for Hpc {
    fn drop(&mut self) {
        unsafe { ClosePseudoConsole(self.0) };
    }
}

struct AttrList(LPPROC_THREAD_ATTRIBUTE_LIST);

impl Drop for AttrList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.0) };
    }
}

pub struct Sys {
    pid: u32,
    hpc: Arc<Mutex<Option<Hpc>>>,
    process: Arc<Owned>,
    job: Option<Job>,
}

fn pipe() -> io::Result<(Owned, Owned)> {
    let mut r = null_mut();
    let mut w = null_mut();
    if unsafe { CreatePipe(&mut r, &mut w, null(), 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((Owned(r), Owned(w)))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn coord(cols: u16, rows: u16) -> COORD {
    COORD { X: cols.clamp(1, 1000) as i16, Y: rows.clamp(1, 1000) as i16 }
}

pub fn command_line(file: &str, args: &[String]) -> String {
    let mut out = String::new();
    for (i, arg) in std::iter::once(file).chain(args.iter().map(String::as_str)).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let chars: Vec<char> = arg.chars().collect();
        let first_q = chars.first() == Some(&'"');
        let last_q = chars.last() == Some(&'"');
        let lopsided = first_q != last_q;
        let none = !first_q && !last_q;
        let quote = arg.is_empty() || ((arg.contains(' ') || arg.contains('\t')) && chars.len() > 1 && (lopsided || none));
        if quote {
            out.push('"');
        }
        let mut bs = 0usize;
        for &c in &chars {
            match c {
                '\\' => bs += 1,
                '"' => {
                    out.extend(std::iter::repeat_n('\\', bs * 2 + 1));
                    out.push('"');
                    bs = 0;
                }
                _ => {
                    out.extend(std::iter::repeat_n('\\', bs));
                    bs = 0;
                    out.push(c);
                }
            }
        }
        if quote {
            out.extend(std::iter::repeat_n('\\', bs * 2));
            out.push('"');
        } else {
            out.extend(std::iter::repeat_n('\\', bs));
        }
    }
    out
}

fn env_get<'a>(env: &'a [(String, String)], key: &str) -> Option<&'a str> {
    env.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
}

pub fn resolve_exe(file: &str, env: &[(String, String)], cwd: &str) -> Option<PathBuf> {
    let p = Path::new(file);
    let exts: Vec<String> = env_get(env, "PATHEXT").unwrap_or(".COM;.EXE;.BAT;.CMD").split(';').filter(|e| !e.is_empty()).map(str::to_owned).collect();
    let try_in = |base: &Path| -> Option<PathBuf> {
        if base.is_file() {
            return Some(base.to_path_buf());
        }
        exts.iter().map(|e| PathBuf::from(format!("{}{}", base.display(), e))).find(|c| c.is_file())
    };
    if p.is_absolute() {
        return try_in(p);
    }
    if p.components().count() > 1 {
        return try_in(&Path::new(cwd).join(p));
    }
    let path = env_get(env, "PATH").unwrap_or("");
    std::env::split_paths(path).filter(|d| !d.as_os_str().is_empty()).find_map(|d| try_in(&d.join(p)))
}

fn env_block(env: &[(String, String)]) -> Vec<u16> {
    let mut block = Vec::new();
    for (k, v) in env {
        block.extend(format!("{k}={v}").encode_utf16());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    block
}

pub fn spawn(spec: &SpawnSpec, sink: Sink) -> io::Result<(Sys, u32, Box<dyn Write + Send>)> {
    let env = dedupe_env(&spec.env, true);
    let (in_r, in_w) = pipe()?;
    let (out_r, out_w) = pipe()?;
    let mut raw: HPCON = 0;
    let hr = unsafe { CreatePseudoConsole(coord(spec.cols, spec.rows), in_r.0, out_w.0, PSEUDOCONSOLE_RESIZE_QUIRK | PSEUDOCONSOLE_WIN32_INPUT_MODE, &mut raw) };
    if hr != 0 {
        return Err(io::Error::other(format!("CreatePseudoConsole failed: 0x{:08X}", hr as u32)));
    }
    let hpc = Hpc(raw);

    let mut size = 0usize;
    unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size) };
    let mut attr_buf = vec![0usize; size.div_ceil(size_of::<usize>()).max(1)];
    let list = attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    if unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let attrs = AttrList(list);
    if unsafe { UpdateProcThreadAttribute(attrs.0, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize, hpc.0 as *const c_void, size_of::<HPCON>(), null_mut(), null()) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut si: STARTUPINFOEXW = unsafe { zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
    si.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
    si.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
    si.lpAttributeList = attrs.0;

    let app = resolve_exe(&spec.file, &env, &spec.cwd).filter(|p| {
        let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        ext != "bat" && ext != "cmd"
    });
    let app_w = app.as_ref().map(|p| wide(&p.to_string_lossy()));
    let mut cmdline = wide(&command_line(&spec.file, &spec.args));
    let block = env_block(&env);
    let cwd_w = wide(&spec.cwd);
    let mut pi: PROCESS_INFORMATION = unsafe { zeroed() };
    let ok = unsafe {
        CreateProcessW(
            app_w.as_ref().map_or(null(), |w| w.as_ptr()),
            cmdline.as_mut_ptr(),
            null(),
            null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED,
            block.as_ptr() as *const c_void,
            cwd_w.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    drop(attrs);
    let process = Arc::new(Owned(pi.hProcess));
    let thread = Owned(pi.hThread);
    let job = match Job::new().and_then(|j| j.assign(&process).map(|_| j)) {
        Ok(j) => Some(j),
        Err(e) => {
            eprintln!("orrery-core: job assign failed pid={} error={}", pi.dwProcessId, e.raw_os_error().unwrap_or(0));
            None
        }
    };
    if unsafe { ResumeThread(thread.0) } == u32::MAX {
        let err = io::Error::last_os_error();
        terminate(&process, KILLED_CODE);
        return Err(err);
    }
    drop(thread);
    drop(in_r);
    drop(out_w);

    let reader = start_reader(Box::new(unsafe { File::from_raw_handle(take(out_r)) }), sink.clone());
    let writer: Box<dyn Write + Send> = Box::new(unsafe { File::from_raw_handle(take(in_w)) });
    let hpc = Arc::new(Mutex::new(Some(hpc)));

    let waiter_hpc = hpc.clone();
    let waiter_proc = process.clone();
    std::thread::Builder::new().name("pty-wait".into()).spawn(move || {
        unsafe { WaitForSingleObject(waiter_proc.0, INFINITE) };
        sink(PtyEvent::Draining);
        let mut code = 0u32;
        if unsafe { GetExitCodeProcess(waiter_proc.0, &mut code) } == 0 {
            code = u32::MAX;
        }
        let exited = Instant::now();
        while reader.quiet_for() < SETTLE_QUIET && exited.elapsed() < SETTLE_MAX {
            std::thread::sleep(Duration::from_millis(5));
        }
        let closing = waiter_hpc.lock().ok().and_then(|mut g| g.take());
        drop(closing);
        reader.wait_done(EXIT_DRAIN);
        sink(PtyEvent::Exit { code: code as i64, signal: None });
    })?;

    Ok((Sys { pid: pi.dwProcessId, hpc, process, job }, pi.dwProcessId, writer))
}

fn take(h: Owned) -> *mut c_void {
    let raw = h.0;
    std::mem::forget(h);
    raw
}

impl Sys {
    pub fn resize(&self, cols: u16, rows: u16) {
        if let Ok(g) = self.hpc.lock() {
            if let Some(h) = g.as_ref() {
                unsafe { ResizePseudoConsole(h.0, coord(cols, rows)) };
            }
        }
    }

    pub fn kill(&self) {
        if self.job.as_ref().is_some_and(|j| j.terminate(KILLED_CODE)) {
            return;
        }
        let strays = descendants(self.pid, &self.process);
        terminate(&self.process, KILLED_CODE);
        terminate_all(&strays, KILLED_CODE);
    }

    pub fn hangup(&self) -> Force {
        self.kill();
        Force
    }
}
