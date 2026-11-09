use crate::jobs::unix::{group_alive, hangup, hangup_then_kill, kill_if_alive};
use crate::session::{dedupe_env, PtyEvent, Reader, Sink, SpawnSpec, EXIT_DRAIN, READ_CHUNK};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

const KILL_GRACE: Duration = Duration::from_secs(2);
const POLL_MS: i32 = 100;

pub struct Sys {
    pid: u32,
    master: Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>,
}

pub struct Force(u32);

impl Force {
    pub fn fire(&self) {
        kill_if_alive(self.0);
    }
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { rows: rows.clamp(1, 1000), cols: cols.clamp(1, 1000), pixel_width: 0, pixel_height: 0 }
}

fn dup_master(master: &dyn MasterPty) -> io::Result<File> {
    let fd = master.as_raw_fd().ok_or_else(|| io::Error::other("pty master has no fd"))?;
    let dup = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if dup < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(dup) }))
}

fn start_reader(mut file: File, sink: Sink, stop: Arc<AtomicBool>) -> io::Result<Reader> {
    use std::os::fd::AsRawFd;
    let (done_tx, done) = mpsc::channel::<()>();
    let epoch = Instant::now();
    let last_data = Arc::new(AtomicU64::new(0));
    let stamp = last_data.clone();
    std::thread::Builder::new().name("pty-read".into()).spawn(move || {
        let mut buf = vec![0u8; READ_CHUNK];
        while !stop.load(Ordering::Acquire) {
            let mut p = libc::pollfd { fd: file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let n = unsafe { libc::poll(&mut p, 1, POLL_MS) };
            if n < 0 {
                if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if n == 0 {
                continue;
            }
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    stamp.store(epoch.elapsed().as_micros() as u64, Ordering::Release);
                    sink(PtyEvent::Data(buf[..n].to_vec()));
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted || e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => break,
            }
        }
        drop(file);
        drop(done_tx);
    })?;
    Ok(Reader { done, last_data, epoch })
}

pub fn spawn(spec: &SpawnSpec, sink: Sink) -> io::Result<(Sys, u32, Box<dyn Write + Send>)> {
    let pair = native_pty_system().openpty(size(spec.cols, spec.rows)).map_err(io::Error::other)?;
    let mut cmd = CommandBuilder::new(&spec.file);
    cmd.args(&spec.args);
    cmd.env_clear();
    for (k, v) in dedupe_env(&spec.env, false) {
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(&spec.cwd);
    let mut child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
    drop(pair.slave);
    let pid = child.process_id().unwrap_or(0);
    let stop = Arc::new(AtomicBool::new(false));
    let setup = (|| -> io::Result<(Reader, Box<dyn Write + Send>)> {
        let reader = start_reader(dup_master(pair.master.as_ref())?, sink.clone(), stop.clone())?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;
        Ok((reader, writer))
    })();
    let (reader, writer) = match setup {
        Ok(v) => v,
        Err(e) => {
            stop.store(true, Ordering::Release);
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let master = Arc::new(Mutex::new(Some(pair.master)));
    let waiter_master = master.clone();
    let waiter = move |mut child: Box<dyn Child + Send + Sync>| {
        let (code, signal) = match child.wait() {
            Ok(status) => match status.signal() {
                Some(sig) => (0, Some(sig.to_owned())),
                None => (status.exit_code() as i64, None),
            },
            Err(_) => (-1, None),
        };
        sink(PtyEvent::Draining);
        if !reader.wait_done(EXIT_DRAIN) {
            if group_alive(pid) {
                hangup(pid);
            }
            stop.store(true, Ordering::Release);
            reader.wait_done(Duration::from_millis(POLL_MS as u64 * 2));
        }
        drop(waiter_master.lock().ok().and_then(|mut g| g.take()));
        sink(PtyEvent::Exit { code, signal });
    };
    if let Err(e) = std::thread::Builder::new().name("pty-wait".into()).spawn(move || waiter(child)) {
        hangup_then_kill(pid, Duration::ZERO);
        return Err(e);
    }
    Ok((Sys { pid, master }, pid, writer))
}

impl Sys {
    pub fn resize(&self, cols: u16, rows: u16) {
        if let Ok(g) = self.master.lock() {
            if let Some(m) = g.as_ref() {
                let _ = m.resize(size(cols, rows));
            }
        }
    }

    pub fn kill(&self) {
        hangup_then_kill(self.pid, KILL_GRACE);
    }

    pub fn hangup(&self) -> Force {
        hangup(self.pid);
        Force(self.pid)
    }
}
