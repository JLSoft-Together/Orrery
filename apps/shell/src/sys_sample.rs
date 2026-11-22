use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const CHANNEL: &str = "ccm:sys-sample";
const STALE: Duration = Duration::from_secs(8);
const PRIME: Duration = Duration::from_millis(150);

pub fn handles(channel: &str) -> bool {
    channel == CHANNEL
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuTimes {
    pub idle: u64,
    pub total: u64,
}

pub fn cpu_percent(prev: CpuTimes, cur: CpuTimes) -> Option<f64> {
    let total = cur.total.checked_sub(prev.total).filter(|t| *t > 0)?;
    let idle = cur.idle.saturating_sub(prev.idle).min(total);
    Some(((total - idle) as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

pub fn ram_percent(total: u64, avail: u64) -> Option<f64> {
    if total == 0 {
        return None;
    }
    Some((total.saturating_sub(avail.min(total)) as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(windows)]
fn cpu_times() -> Option<CpuTimes> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(idle: *mut u64, kernel: *mut u64, user: *mut u64) -> i32;
    }
    let (mut idle, mut kernel, mut user) = (0u64, 0u64, 0u64);
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    Some(CpuTimes { idle, total: kernel.saturating_add(user) })
}

#[cfg(target_os = "macos")]
fn cpu_times() -> Option<CpuTimes> {
    extern "C" {
        fn host_statistics(host: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    }
    let mut ticks = [0i32; 4];
    let mut count = 4u32;
    let r = unsafe { host_statistics(libc::mach_host_self(), libc::HOST_CPU_LOAD_INFO, ticks.as_mut_ptr(), &mut count) };
    if r != 0 {
        return None;
    }
    let t: Vec<u64> = ticks.iter().map(|v| u64::from(*v as u32)).collect();
    Some(CpuTimes { idle: t[2], total: t.iter().sum() })
}

#[cfg(not(any(windows, target_os = "macos")))]
fn cpu_times() -> Option<CpuTimes> {
    None
}

pub struct Sampler {
    prev: Mutex<Option<(CpuTimes, Instant)>>,
}

impl Sampler {
    pub const fn new() -> Self {
        Self { prev: Mutex::new(None) }
    }

    pub fn cpu(&self, read: impl Fn() -> Option<CpuTimes>, now: Instant) -> Option<f64> {
        let mut slot = self.prev.lock().unwrap_or_else(|e| e.into_inner());
        let cur = read()?;
        let base = match *slot {
            Some((t, at)) if now.saturating_duration_since(at) <= STALE => t,
            _ => {
                std::thread::sleep(PRIME);
                let next = read()?;
                let pct = cpu_percent(cur, next);
                *slot = Some((next, Instant::now()));
                return pct;
            }
        };
        *slot = Some((cur, now));
        cpu_percent(base, cur)
    }
}

static SAMPLER: Sampler = Sampler::new();

pub fn sample() -> Value {
    let cpu = SAMPLER.cpu(cpu_times, Instant::now());
    let (total, avail) = crate::disk_ram::memory();
    let ram = ram_percent(total, avail);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    json!({
        "cpu": cpu.map(round1),
        "ram": ram.map(round1),
        "totalBytes": total,
        "usedBytes": total.saturating_sub(avail.min(total)),
        "threads": threads,
    })
}

pub fn dispatch(channel: &str) -> Result<Value, String> {
    if !handles(channel) {
        return Err("unknown channel".into());
    }
    Ok(sample())
}

#[cfg(test)]
#[path = "sys_sample_tests.rs"]
mod tests;
