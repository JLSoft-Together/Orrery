use super::awake::Power;
use std::io::Write;
use std::sync::Mutex;

pub struct LogPower {
    file: String,
    battery: Option<bool>,
}

impl Power for LogPower {
    fn start(&self) {
        log_line(&self.file, "start");
    }
    fn stop(&self) {
        log_line(&self.file, "stop");
    }
    fn on_battery(&self) -> bool {
        self.battery.unwrap_or(false)
    }
}

fn log_line(file: &str, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(f, "{line}");
    }
}

fn forced_battery() -> Option<bool> {
    match std::env::var("CCM_E2E_POWER").ok().as_deref() {
        Some("battery") => Some(true),
        Some("ac") => Some(false),
        _ => None,
    }
}

pub fn power() -> Box<dyn Power> {
    let battery = forced_battery();
    if let Some(file) = std::env::var("CCM_E2E_AWAKE_LOG").ok().filter(|v| !v.is_empty()) {
        return Box::new(LogPower { file, battery });
    }
    Box::new(SystemPower { battery, hold: Mutex::new(None) })
}

pub fn grace_ms() -> f64 {
    std::env::var("CCM_E2E_AWAKE_GRACE_MS").ok().and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(super::awake::AWAKE_GRACE_MS)
}

pub struct SystemPower {
    battery: Option<bool>,
    hold: Mutex<Option<Hold>>,
}

impl Power for SystemPower {
    fn start(&self) {
        let mut h = self.hold.lock().unwrap();
        if h.is_none() {
            *h = Hold::start();
        }
    }
    fn stop(&self) {
        if let Some(h) = self.hold.lock().unwrap().take() {
            h.stop();
        }
    }
    fn on_battery(&self) -> bool {
        self.battery.unwrap_or_else(system_on_battery)
    }
}

#[cfg(target_os = "macos")]
pub struct Hold(std::process::Child);

#[cfg(target_os = "macos")]
impl Hold {
    fn start() -> Option<Hold> {
        let pid = std::process::id().to_string();
        std::process::Command::new("/usr/bin/caffeinate")
            .args(["-i", "-w", &pid])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()
            .map(Hold)
    }

    fn stop(mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(target_os = "macos")]
fn system_on_battery() -> bool {
    std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "ps"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("'Battery Power'"))
        .unwrap_or(false)
}

#[cfg(windows)]
pub struct Hold(std::sync::mpsc::Sender<()>);

#[cfg(windows)]
impl Hold {
    fn start() -> Option<Hold> {
        use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
            let _ = rx.recv();
            unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        });
        Some(Hold(tx))
    }

    fn stop(self) {
        let _ = self.0.send(());
    }
}

#[cfg(windows)]
fn system_on_battery() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut st: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    unsafe { GetSystemPowerStatus(&mut st) != 0 && st.ACLineStatus == 0 }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub struct Hold;

#[cfg(not(any(windows, target_os = "macos")))]
impl Hold {
    fn start() -> Option<Hold> {
        Some(Hold)
    }
    fn stop(self) {}
}

#[cfg(not(any(windows, target_os = "macos")))]
fn system_on_battery() -> bool {
    false
}
