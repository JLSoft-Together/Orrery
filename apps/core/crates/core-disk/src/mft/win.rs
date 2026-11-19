use super::record::{apply_fixup, data_runs, parse_fixed, Parsed, Run};
use super::wire::{check_hello, hello, hex, parse_helper_args, write_frame, HelperArgs, ACK, HELLO, KIND_ERROR, PIPE_PREFIX, TOKEN_BYTES};
use super::{consume, produce, Job, MemorySource, MftError, Source, MAX_FIXTURE, TIMEOUT};
use crate::index::Index;
use crate::walk::Control;
use std::ffi::c_void;
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE, ERROR_CANCELLED, ERROR_IO_PENDING, ERROR_MORE_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
use windows_sys::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDriveTypeW, GetVolumeInformationW, ReadFile, WriteFile, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows_sys::Win32::System::Ioctl::{FSCTL_GET_NTFS_VOLUME_DATA, FSCTL_GET_RETRIEVAL_POINTERS, NTFS_VOLUME_DATA_BUFFER, STARTING_VCN_INPUT_BUFFER};
use windows_sys::Win32::System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT};
use windows_sys::Win32::System::Threading::{CreateEventW, GetCurrentProcess, GetExitCodeProcess, GetProcessId, OpenProcessToken, WaitForMultipleObjects, WaitForSingleObject, CREATE_NO_WINDOW};
use windows_sys::Win32::System::IO::{CancelIoEx, DeviceIoControl, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

const SLICE_MS: u32 = 200;
const HELLO_WAIT: Duration = Duration::from_secs(30);
const EXIT_WAIT_MS: u32 = 3_000;
const READ_CHUNK: usize = 4 << 20;
const SW_HIDE: i32 = 0;

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn wide_path(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn random(buf: &mut [u8]) -> bool {
    unsafe { BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG) == 0 }
}

fn token_info<T: Copy>(class: i32, extra: usize) -> Option<Vec<u8>> {
    let mut tok: HANDLE = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok) } == 0 {
        return None;
    }
    let tok = Owned(tok);
    let mut buf = vec![0u8; std::mem::size_of::<T>() + extra];
    let mut got = 0u32;
    (unsafe { GetTokenInformation(tok.0, class, buf.as_mut_ptr().cast(), buf.len() as u32, &mut got) } != 0).then_some(buf)
}

pub fn elevated() -> bool {
    token_info::<TOKEN_ELEVATION>(TokenElevation, 0).is_some_and(|b| unsafe { std::ptr::read_unaligned(b.as_ptr().cast::<TOKEN_ELEVATION>()) }.TokenIsElevated != 0)
}

fn user_sid() -> Option<String> {
    let buf = token_info::<TOKEN_USER>(TokenUser, 256)?;
    let user = unsafe { std::ptr::read_unaligned(buf.as_ptr().cast::<TOKEN_USER>()) };
    let mut s: *mut u16 = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut s) } == 0 || s.is_null() {
        return None;
    }
    let mut n = 0;
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    let out = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(s, n) });
    unsafe { LocalFree(s.cast()) };
    Some(out)
}

pub fn supported(drive: char) -> bool {
    let root = wide(&format!("{drive}:\\"));
    if !matches!(unsafe { GetDriveTypeW(root.as_ptr()) }, 2 | 3) {
        return false;
    }
    let mut fs = [0u16; 64];
    let (mut serial, mut maxlen, mut flags) = (0u32, 0u32, 0u32);
    if unsafe { GetVolumeInformationW(root.as_ptr(), std::ptr::null_mut(), 0, &mut serial, &mut maxlen, &mut flags, fs.as_mut_ptr(), fs.len() as u32) } == 0 {
        return false;
    }
    let n = fs.iter().position(|c| *c == 0).unwrap_or(fs.len());
    String::from_utf16_lossy(&fs[..n]) == "NTFS"
}

struct Event(Owned);

impl Event {
    fn new() -> Option<Event> {
        let h = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        (!h.is_null()).then(|| Event(Owned(h)))
    }

    fn overlapped(&self) -> OVERLAPPED {
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.hEvent = self.0 .0;
        ov
    }
}

struct Server {
    pipe: Owned,
    event: Event,
    connected: bool,
}

impl Server {
    fn create(name: &str) -> Result<Server, MftError> {
        let sid = user_sid().ok_or_else(|| MftError::Helper("no user sid".into()))?;
        let sddl = wide(&format!("D:P(A;;GA;;;{sid})"));
        let mut sd: *mut c_void = std::ptr::null_mut();
        if unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut sd, std::ptr::null_mut()) } == 0 {
            return Err(MftError::Helper("pipe acl".into()));
        }
        let sa = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
        let w = wide(name);
        let h = unsafe { CreateNamedPipeW(w.as_ptr(), PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 1, 1 << 16, 1 << 16, 0, &sa) };
        unsafe { LocalFree(sd) };
        if h == INVALID_HANDLE_VALUE {
            return Err(MftError::Helper(format!("pipe create failed ({})", unsafe { GetLastError() })));
        }
        let event = Event::new().ok_or_else(|| MftError::Helper("event".into()))?;
        Ok(Server { pipe: Owned(h), event, connected: false })
    }

    fn settle(&self, ov: &mut OVERLAPPED) {
        let mut n = 0u32;
        unsafe {
            CancelIoEx(self.pipe.0, ov);
            GetOverlappedResult(self.pipe.0, ov, &mut n, 1);
        }
    }

    fn wait(&self, ov: &mut OVERLAPPED, wait: &Waiter) -> Result<u32, MftError> {
        loop {
            let mut n = 0u32;
            if unsafe { GetOverlappedResultEx(self.pipe.0, ov, &mut n, SLICE_MS, 0) } != 0 {
                return Ok(n);
            }
            let e = unsafe { GetLastError() };
            if e == ERROR_BROKEN_PIPE {
                return Ok(0);
            }
            if e != WAIT_TIMEOUT && e != ERROR_IO_PENDING {
                return Err(MftError::Helper(format!("pipe error ({e})")));
            }
            if let Err(why) = wait.check() {
                self.settle(ov);
                return Err(why);
            }
        }
    }

    fn connect(&mut self, wait: &Waiter, proc: HANDLE) -> Result<(), MftError> {
        let mut ov = self.event.overlapped();
        if unsafe { ConnectNamedPipe(self.pipe.0, &mut ov) } == 0 {
            match unsafe { GetLastError() } {
                ERROR_PIPE_CONNECTED => {}
                ERROR_IO_PENDING => loop {
                    let handles = [self.event.0 .0, proc];
                    let r = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, SLICE_MS) };
                    if r == WAIT_OBJECT_0 {
                        let mut n = 0u32;
                        if unsafe { GetOverlappedResult(self.pipe.0, &ov, &mut n, 0) } == 0 {
                            return Err(MftError::Helper("pipe connect failed".into()));
                        }
                        break;
                    }
                    if r == WAIT_OBJECT_0 + 1 {
                        self.settle(&mut ov);
                        return Err(MftError::Helper(format!("helper exited ({})", exit_code(proc))));
                    }
                    if r != WAIT_TIMEOUT {
                        self.settle(&mut ov);
                        return Err(MftError::Helper("wait failed".into()));
                    }
                    if let Err(why) = wait.check() {
                        self.settle(&mut ov);
                        return Err(why);
                    }
                },
                e => return Err(MftError::Helper(format!("pipe connect failed ({e})"))),
            }
        }
        self.connected = true;
        Ok(())
    }

    fn client_pid(&self) -> Option<u32> {
        let mut pid = 0u32;
        (unsafe { GetNamedPipeClientProcessId(self.pipe.0, &mut pid) } != 0).then_some(pid)
    }

    fn write_all(&self, data: &[u8], wait: &Waiter) -> Result<(), MftError> {
        let mut ov = self.event.overlapped();
        if unsafe { WriteFile(self.pipe.0, data.as_ptr(), data.len() as u32, std::ptr::null_mut(), &mut ov) } == 0 && unsafe { GetLastError() } != ERROR_IO_PENDING {
            return Err(MftError::Helper("pipe write failed".into()));
        }
        let n = self.wait(&mut ov, wait)?;
        if n as usize != data.len() {
            return Err(MftError::Helper("pipe write short".into()));
        }
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if self.connected {
            unsafe { DisconnectNamedPipe(self.pipe.0) };
        }
    }
}

struct Waiter<'a> {
    control: &'a Control,
    deadline: Instant,
}

impl Waiter<'_> {
    fn check(&self) -> Result<(), MftError> {
        if self.control.is_cancelled() {
            return Err(MftError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(MftError::Timeout);
        }
        Ok(())
    }
}

struct PipeReader<'a> {
    server: &'a Server,
    wait: Waiter<'a>,
    why: Option<MftError>,
}

impl Read for PipeReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = buf.len().min(1 << 20) as u32;
        let mut ov = self.server.event.overlapped();
        if unsafe { ReadFile(self.server.pipe.0, buf.as_mut_ptr(), len, std::ptr::null_mut(), &mut ov) } == 0 {
            let e = unsafe { GetLastError() };
            if e == ERROR_BROKEN_PIPE {
                return Ok(0);
            }
            if e != ERROR_IO_PENDING && e != ERROR_MORE_DATA {
                return Err(std::io::Error::other(format!("pipe read failed ({e})")));
            }
        }
        match self.server.wait(&mut ov, &self.wait) {
            Ok(n) => Ok(n as usize),
            Err(why) => {
                let msg = why.to_string();
                self.why = Some(why);
                Err(std::io::Error::other(msg))
            }
        }
    }
}

enum Proc {
    Elevated(Owned),
    Plain(Child),
}

impl Proc {
    fn handle(&self) -> HANDLE {
        match self {
            Proc::Elevated(h) => h.0,
            Proc::Plain(c) => c.as_raw_handle() as HANDLE,
        }
    }

    fn pid(&self) -> u32 {
        match self {
            Proc::Elevated(h) => unsafe { GetProcessId(h.0) },
            Proc::Plain(c) => c.id(),
        }
    }

    fn finish(mut self) {
        let h = self.handle();
        if unsafe { WaitForSingleObject(h, EXIT_WAIT_MS) } != WAIT_OBJECT_0 {
            if let Proc::Plain(c) = &mut self {
                let _ = c.kill();
            }
        }
        if let Proc::Plain(c) = &mut self {
            let _ = c.wait();
        }
    }
}

fn exit_code(h: HANDLE) -> u32 {
    let mut code = 0u32;
    unsafe { GetExitCodeProcess(h, &mut code) };
    code
}

fn launch_elevated(exe: &Path, params: &str) -> Result<Proc, MftError> {
    let verb = wide("runas");
    let file = wide_path(exe);
    let args = wide(params);
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = args.as_ptr();
    info.nShow = SW_HIDE;
    let com = unsafe { CoInitializeEx(std::ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32) };
    let ok = unsafe { ShellExecuteExW(&mut info) };
    let err = unsafe { GetLastError() };
    if com >= 0 {
        unsafe { CoUninitialize() };
    }
    if ok == 0 {
        return Err(if err == ERROR_CANCELLED { MftError::Declined } else { MftError::Helper(format!("launch failed ({err})")) });
    }
    if info.hProcess.is_null() {
        return Err(MftError::Helper("launch failed (no process)".into()));
    }
    Ok(Proc::Elevated(Owned(info.hProcess)))
}

fn launch_fixture(exe: &Path, drive: char, pipe: &str, token: &str, file: &Path) -> Result<Proc, MftError> {
    Command::new(exe)
        .args(["disk", "mft-fixture", &drive.to_string(), pipe, token])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(Proc::Plain)
        .map_err(|e| MftError::Helper(format!("launch failed: {e}")))
}

pub fn run(job: &Job, control: &Control, progress: &dyn Fn(u64, u64)) -> Result<Index, MftError> {
    let mut id = [0u8; 16];
    let mut token = [0u8; TOKEN_BYTES];
    if !random(&mut id) || !random(&mut token) {
        return Err(MftError::Helper("no random".into()));
    }
    let name = format!("{PIPE_PREFIX}{}", hex(&id));
    let token_hex = hex(&token);
    let mut server = Server::create(&name)?;
    let exe = std::env::current_exe().map_err(|e| MftError::Helper(format!("exe: {e}")))?;
    let proc = match &job.fixture {
        Some(file) => launch_fixture(&exe, job.drive, &name, &token_hex, file)?,
        None => launch_elevated(&exe, &format!("disk mft-helper {} {} {}", job.drive, name, token_hex))?,
    };
    let started = Instant::now();
    let r = serve(&mut server, proc.handle(), proc.pid(), &token, job.drive, control, started, progress);
    drop(server);
    proc.finish();
    r
}

#[allow(clippy::too_many_arguments)]
fn serve(server: &mut Server, proc: HANDLE, pid: u32, token: &[u8; TOKEN_BYTES], drive: char, control: &Control, started: Instant, progress: &dyn Fn(u64, u64)) -> Result<Index, MftError> {
    let wait = Waiter { control, deadline: started + TIMEOUT };
    server.connect(&wait, proc)?;
    if server.client_pid() != Some(pid) {
        return Err(MftError::Helper("pipe client is not the helper".into()));
    }
    let mut first = [0u8; HELLO.len() + TOKEN_BYTES];
    {
        let mut r = PipeReader { server, wait: Waiter { control, deadline: (Instant::now() + HELLO_WAIT).min(started + TIMEOUT) }, why: None };
        if r.read_exact(&mut first).is_err() {
            return Err(r.why.take().unwrap_or_else(|| MftError::Helper("helper sent no token".into())));
        }
    }
    if !check_hello(&first, token) {
        return Err(MftError::Helper("bad token".into()));
    }
    server.write_all(&[ACK], &wait)?;
    let mut r = PipeReader { server, wait, why: None };
    match consume(&mut r, drive, progress) {
        Ok(ix) => Ok(ix),
        Err(e) => Err(r.why.take().unwrap_or(e)),
    }
}

struct Client(Owned);

impl Client {
    fn open(name: &str) -> Option<Client> {
        let w = wide(name);
        for _ in 0..5 {
            let h = unsafe { CreateFileW(w.as_ptr(), GENERIC_READ | GENERIC_WRITE, 0, std::ptr::null(), OPEN_EXISTING, SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, std::ptr::null_mut()) };
            if h != INVALID_HANDLE_VALUE {
                return Some(Client(Owned(h)));
            }
            if unsafe { GetLastError() } != ERROR_PIPE_BUSY || unsafe { WaitNamedPipeW(w.as_ptr(), 2_000) } == 0 {
                return None;
            }
        }
        None
    }
}

impl Read for Client {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut n = 0u32;
        if unsafe { ReadFile(self.0 .0, buf.as_mut_ptr(), buf.len().min(1 << 20) as u32, &mut n, std::ptr::null_mut()) } == 0 {
            let e = unsafe { GetLastError() };
            return if e == ERROR_BROKEN_PIPE { Ok(0) } else { Err(std::io::Error::from_raw_os_error(e as i32)) };
        }
        Ok(n as usize)
    }
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut n = 0u32;
        if unsafe { WriteFile(self.0 .0, buf.as_ptr(), buf.len().min(1 << 20) as u32, &mut n, std::ptr::null_mut()) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(n as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn watchdog() {
    std::thread::spawn(|| {
        std::thread::sleep(TIMEOUT);
        std::process::exit(4);
    });
}

fn handshake(a: &HelperArgs) -> Option<Client> {
    let mut pipe = Client::open(&a.pipe)?;
    pipe.write_all(&hello(&a.token)).ok()?;
    let mut ack = [0u8; 1];
    pipe.read_exact(&mut ack).ok()?;
    (ack[0] == ACK).then_some(pipe)
}

fn send(pipe: &mut Client, src: Result<Box<dyn Source>, String>, drive: char) -> i32 {
    let r = match src {
        Ok(mut s) => produce(s.as_mut(), drive, pipe),
        Err(e) => write_frame(pipe, KIND_ERROR, e.as_bytes()),
    };
    if r.is_ok() {
        0
    } else {
        1
    }
}

pub fn helper_main(args: &[String]) -> i32 {
    let Some(a) = parse_helper_args(args) else { return 2 };
    watchdog();
    let Some(mut pipe) = handshake(&a) else { return 3 };
    let src = Volume::open(a.drive).map(|v| Box::new(v) as Box<dyn Source>);
    send(&mut pipe, src, a.drive)
}

pub fn fixture_main(args: &[String]) -> i32 {
    let Some((file, rest)) = args.split_last() else { return 2 };
    let Some(a) = parse_helper_args(rest) else { return 2 };
    if elevated() {
        return 5;
    }
    watchdog();
    let Some(mut pipe) = handshake(&a) else { return 3 };
    let src = read_fixture(Path::new(file)).map(|m| Box::new(m) as Box<dyn Source>);
    send(&mut pipe, src, a.drive)
}

fn read_fixture(p: &Path) -> Result<MemorySource, String> {
    let f = std::fs::File::open(p).map_err(|e| format!("fixture: {e}"))?;
    let mut bytes = Vec::new();
    f.take(MAX_FIXTURE + 1).read_to_end(&mut bytes).map_err(|e| format!("fixture: {e}"))?;
    if bytes.len() as u64 > MAX_FIXTURE {
        return Err("fixture too large".into());
    }
    MemorySource::new(bytes).map_err(|e| e.to_string())
}

struct Volume {
    h: Owned,
    cluster: u64,
    sector: u64,
    size: usize,
    runs: Vec<Run>,
    total: u64,
    run: usize,
    in_run: u64,
    done: u64,
}

fn read_at(h: HANDLE, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
    let mut at = 0usize;
    while at < buf.len() {
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        let pos = offset + at as u64;
        ov.Anonymous.Anonymous.Offset = pos as u32;
        ov.Anonymous.Anonymous.OffsetHigh = (pos >> 32) as u32;
        let mut n = 0u32;
        let want = (buf.len() - at).min(READ_CHUNK) as u32;
        if unsafe { ReadFile(h, buf[at..].as_mut_ptr(), want, &mut n, &mut ov) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if n == 0 {
            return Err(std::io::Error::other("short read"));
        }
        at += n as usize;
    }
    Ok(())
}

fn round_up(n: u64, to: u64) -> u64 {
    n.div_ceil(to) * to
}

impl Volume {
    fn open(drive: char) -> Result<Volume, String> {
        let path = wide(&format!("\\\\.\\{drive}:"));
        let h = unsafe { CreateFileW(path.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut()) };
        if h == INVALID_HANDLE_VALUE {
            let e = unsafe { GetLastError() };
            return Err(if e == ERROR_ACCESS_DENIED { "access denied".into() } else { format!("volume open failed ({e})") });
        }
        let h = Owned(h);
        let mut d: NTFS_VOLUME_DATA_BUFFER = unsafe { std::mem::zeroed() };
        let mut got = 0u32;
        if unsafe { DeviceIoControl(h.0, FSCTL_GET_NTFS_VOLUME_DATA, std::ptr::null(), 0, (&mut d as *mut NTFS_VOLUME_DATA_BUFFER).cast(), std::mem::size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32, &mut got, std::ptr::null_mut()) } == 0 {
            return Err(format!("not a readable NTFS volume ({})", unsafe { GetLastError() }));
        }
        let cluster = d.BytesPerCluster as u64;
        let sector = d.BytesPerSector as u64;
        let size = d.BytesPerFileRecordSegment as usize;
        if !cluster.is_power_of_two() || !sector.is_power_of_two() || sector < 512 || cluster < sector || !size.is_power_of_two() || !(1024..=65536).contains(&size) || d.MftStartLcn <= 0 || d.MftValidDataLength <= 0 {
            return Err("unexpected volume geometry".into());
        }
        let first_len = round_up(size as u64, sector) as usize;
        let mut first = vec![0u8; first_len];
        read_at(h.0, d.MftStartLcn as u64 * cluster, &mut first).map_err(|e| format!("read $MFT: {e}"))?;
        let rec = &mut first[..size];
        apply_fixup(rec).map_err(|_| "bad $MFT record".to_string())?;
        let mut p = Parsed::default();
        parse_fixed(rec, &mut p).map_err(|_| "bad $MFT record".to_string())?;
        let mut runs = p.runs.and_then(|(a, b)| data_runs(&rec[a..b])).unwrap_or_default();
        let valid = d.MftValidDataLength as u64;
        let covered: u64 = runs.iter().map(|r| r.clusters * cluster).sum();
        if covered < valid {
            runs = retrieval_pointers(drive).ok_or_else(|| "fragmented $MFT".to_string())?;
        }
        let covered: u64 = runs.iter().map(|r| r.clusters * cluster).sum();
        let total = valid.min(covered) / size as u64 * size as u64;
        Ok(Volume { h, cluster, sector, size, runs, total, run: 0, in_run: 0, done: 0 })
    }
}

fn retrieval_pointers(drive: char) -> Option<Vec<Run>> {
    let path = wide(&format!("\\\\?\\{drive}:\\$MFT"));
    let h = unsafe { CreateFileW(path.as_ptr(), FILE_READ_ATTRIBUTES, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, std::ptr::null(), OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, std::ptr::null_mut()) };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    let h = Owned(h);
    let mut runs = Vec::new();
    let mut vcn: i64 = 0;
    let mut out = vec![0u8; 64 * 1024];
    loop {
        let input = STARTING_VCN_INPUT_BUFFER { StartingVcn: vcn };
        let mut got = 0u32;
        let ok = unsafe { DeviceIoControl(h.0, FSCTL_GET_RETRIEVAL_POINTERS, (&input as *const STARTING_VCN_INPUT_BUFFER).cast(), std::mem::size_of::<STARTING_VCN_INPUT_BUFFER>() as u32, out.as_mut_ptr().cast(), out.len() as u32, &mut got, std::ptr::null_mut()) };
        let more = ok == 0 && unsafe { GetLastError() } == ERROR_MORE_DATA;
        if ok == 0 && !more {
            return None;
        }
        let count = u32::from_le_bytes(out.get(0..4)?.try_into().ok()?) as usize;
        let mut prev = i64::from_le_bytes(out.get(8..16)?.try_into().ok()?);
        for k in 0..count {
            let at = 16 + k * 16;
            let next = i64::from_le_bytes(out.get(at..at + 8)?.try_into().ok()?);
            let lcn = i64::from_le_bytes(out.get(at + 8..at + 16)?.try_into().ok()?);
            if lcn < 0 || next <= prev {
                return None;
            }
            runs.push(Run { lcn, clusters: (next - prev) as u64 });
            prev = next;
        }
        if !more || count == 0 {
            return Some(runs);
        }
        vcn = prev;
    }
}

impl Source for Volume {
    fn record_size(&self) -> usize {
        self.size
    }

    fn total_records(&self) -> u64 {
        self.total / self.size as u64
    }

    fn next(&mut self, buf: &mut Vec<u8>) -> std::io::Result<Option<u64>> {
        while self.run < self.runs.len() && self.in_run >= self.runs[self.run].clusters * self.cluster {
            self.run += 1;
            self.in_run = 0;
        }
        if self.done >= self.total || self.run >= self.runs.len() {
            return Ok(None);
        }
        let r = self.runs[self.run];
        let left_run = r.clusters * self.cluster - self.in_run;
        let unit = (self.size as u64).max(self.cluster);
        let want = (READ_CHUNK as u64 / unit * unit).max(unit).min(left_run).min(self.total - self.done);
        let want = want / self.size as u64 * self.size as u64;
        if want == 0 {
            return Ok(None);
        }
        let read = round_up(want, self.sector) as usize;
        buf.resize(read, 0);
        read_at(self.h.0, r.lcn as u64 * self.cluster + self.in_run, buf)?;
        buf.truncate(want as usize);
        let first = self.done / self.size as u64;
        self.in_run += want;
        self.done += want;
        Ok(Some(first))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mft::sample;

    fn pipe_name() -> String {
        let mut id = [0u8; 16];
        assert!(random(&mut id));
        format!("{PIPE_PREFIX}{}", hex(&id))
    }

    fn args(name: &str, token: &[u8; TOKEN_BYTES]) -> HelperArgs {
        HelperArgs { drive: 'Q', pipe: name.to_string(), token: *token }
    }

    fn me() -> HANDLE {
        static NEVER: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        *NEVER.get_or_init(|| {
            let e = Event::new().unwrap();
            let h = e.0 .0 as usize;
            std::mem::forget(e);
            h
        }) as HANDLE
    }

    #[test]
    fn helper_and_server_round_trip_in_process() {
        let name = pipe_name();
        let token = [7u8; TOKEN_BYTES];
        let mut server = Server::create(&name).unwrap();
        let a = args(&name, &token);
        let t = std::thread::spawn(move || {
            let mut pipe = handshake(&a).expect("handshake");
            let src = MemorySource::new(sample::image()).map(|m| Box::new(m) as Box<dyn Source>).map_err(|e| e.to_string());
            send(&mut pipe, src, 'Q')
        });
        let control = Control::default();
        let ix = serve(&mut server, me(), std::process::id(), &token, 'Q', &control, Instant::now(), &|_, _| {}).unwrap();
        assert_eq!(t.join().unwrap(), 0);
        assert_eq!(ix.root_paths(), vec!["Q:\\".to_string()]);
        assert_eq!(ix.stats.bytes, sample::EXPECTED_BYTES);
    }

    #[test]
    fn server_drops_a_client_with_the_wrong_token() {
        let name = pipe_name();
        let mut server = Server::create(&name).unwrap();
        let a = args(&name, &[1u8; TOKEN_BYTES]);
        let t = std::thread::spawn(move || handshake(&a).is_some());
        let control = Control::default();
        let r = serve(&mut server, me(), std::process::id(), &[2u8; TOKEN_BYTES], 'Q', &control, Instant::now(), &|_, _| {});
        assert_eq!(r.unwrap_err(), MftError::Helper("bad token".into()));
        drop(server);
        assert!(!t.join().unwrap(), "helper must give up without an ack");
    }

    #[test]
    fn server_refuses_a_client_that_is_not_the_helper() {
        let name = pipe_name();
        let token = [3u8; TOKEN_BYTES];
        let mut server = Server::create(&name).unwrap();
        let a = args(&name, &token);
        let t = std::thread::spawn(move || handshake(&a).is_some());
        let control = Control::default();
        let r = serve(&mut server, me(), std::process::id() ^ 0x5555, &token, 'Q', &control, Instant::now(), &|_, _| {});
        assert_eq!(r.unwrap_err(), MftError::Helper("pipe client is not the helper".into()));
        drop(server);
        assert!(!t.join().unwrap());
    }

    #[test]
    fn helper_gives_up_on_a_wrong_ack() {
        let name = pipe_name();
        let token = [4u8; TOKEN_BYTES];
        let mut server = Server::create(&name).unwrap();
        let a = args(&name, &token);
        let t = std::thread::spawn(move || handshake(&a).is_some());
        let control = Control::default();
        let wait = Waiter { control: &control, deadline: Instant::now() + Duration::from_secs(10) };
        server.connect(&wait, me()).unwrap();
        let mut first = [0u8; HELLO.len() + TOKEN_BYTES];
        PipeReader { server: &server, wait: Waiter { control: &control, deadline: Instant::now() + Duration::from_secs(10) }, why: None }.read_exact(&mut first).unwrap();
        assert!(check_hello(&first, &token));
        server.write_all(&[0x15], &wait).unwrap();
        assert!(!t.join().unwrap());
    }

    #[test]
    fn second_server_on_the_same_name_fails() {
        let name = pipe_name();
        let _first = Server::create(&name).unwrap();
        assert!(Server::create(&name).is_err());
    }

    #[test]
    fn cancel_stops_waiting_for_the_helper() {
        let name = pipe_name();
        let mut server = Server::create(&name).unwrap();
        let control = Control::default();
        control.cancel();
        let r = serve(&mut server, me(), 1, &[0u8; TOKEN_BYTES], 'Q', &control, Instant::now(), &|_, _| {});
        assert_eq!(r.unwrap_err(), MftError::Cancelled);
    }

    #[test]
    fn helper_entry_points_reject_bad_arguments() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let pipe = format!("{PIPE_PREFIX}{}", "a".repeat(32));
        let tok = "b".repeat(64);
        assert_eq!(helper_main(&s(&["c", &pipe, &tok])), 2);
        assert_eq!(helper_main(&s(&["C", &pipe, &tok, "extra"])), 2);
        assert_eq!(helper_main(&s(&["C", r"\\.\pipe\other", &tok])), 2);
        assert_eq!(fixture_main(&s(&["C", &pipe, &tok])), 2);
        assert_eq!(fixture_main(&s(&[])), 2);
    }
}
