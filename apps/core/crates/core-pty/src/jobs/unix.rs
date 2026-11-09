use std::time::Duration;

pub fn signal_group(pgid: u32, sig: i32) -> bool {
    if pgid == 0 || pgid > i32::MAX as u32 {
        return false;
    }
    unsafe { libc::killpg(pgid as libc::pid_t, sig) == 0 }
}

pub fn group_alive(pgid: u32) -> bool {
    signal_group(pgid, 0)
}

pub fn hangup(pgid: u32) -> bool {
    signal_group(pgid, libc::SIGHUP)
}

pub fn kill_if_alive(pgid: u32) {
    if group_alive(pgid) {
        signal_group(pgid, libc::SIGKILL);
    }
}

pub fn hangup_then_kill(pgid: u32, grace: Duration) {
    if !hangup(pgid) {
        return;
    }
    let _ = std::thread::Builder::new().name("pty-kill".into()).spawn(move || {
        std::thread::sleep(grace);
        kill_if_alive(pgid);
    });
}
