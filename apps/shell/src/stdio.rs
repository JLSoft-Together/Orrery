use std::fs::File;
use std::io::Write;

pub struct ControlPipe(File);

impl ControlPipe {
    pub fn send(&mut self, line: &str) {
        let _ = self.0.write_all(line.as_bytes());
        let _ = self.0.write_all(b"\n");
        let _ = self.0.flush();
    }
}

#[cfg(windows)]
pub fn take_stdin() -> std::io::Result<ControlPipe> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Console::{SetStdHandle, STD_INPUT_HANDLE};
    use windows_sys::Win32::System::Pipes::CreatePipe;
    let mut read: HANDLE = std::ptr::null_mut();
    let mut write: HANDLE = std::ptr::null_mut();
    if unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 0) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { SetStdHandle(STD_INPUT_HANDLE, read) } == 0 {
        let err = std::io::Error::last_os_error();
        unsafe {
            CloseHandle(read);
            CloseHandle(write);
        }
        return Err(err);
    }
    Ok(ControlPipe(unsafe { File::from_raw_handle(write) }))
}

#[cfg(target_os = "macos")]
pub fn take_stdin() -> std::io::Result<ControlPipe> {
    use std::os::fd::FromRawFd;
    let mut fds = [0 as libc::c_int; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let [read, write] = fds;
    unsafe { libc::fcntl(write, libc::F_SETFD, libc::FD_CLOEXEC) };
    if unsafe { libc::dup2(read, 0) } < 0 {
        let err = std::io::Error::last_os_error();
        unsafe {
            libc::close(read);
            libc::close(write);
        }
        return Err(err);
    }
    unsafe { libc::close(read) };
    Ok(ControlPipe(unsafe { File::from_raw_fd(write) }))
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn lines_written_to_the_pipe_arrive_on_stdin() {
        let mut pipe = take_stdin().unwrap();
        pipe.send("{\"type\":\"shutdown\"}");
        let mut buf = [0u8; 64];
        let n = std::io::stdin().read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"{\"type\":\"shutdown\"}\n");
    }
}
