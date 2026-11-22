use std::cell::Cell;
use std::time::{Duration, Instant};

use super::*;

fn t(idle: u64, total: u64) -> CpuTimes {
    CpuTimes { idle, total }
}

#[test]
fn cpu_is_busy_share_of_the_delta() {
    assert_eq!(cpu_percent(t(100, 400), t(175, 500)), Some(25.0));
    assert_eq!(cpu_percent(t(0, 0), t(0, 100)), Some(100.0));
    assert_eq!(cpu_percent(t(0, 0), t(100, 100)), Some(0.0));
}

#[test]
fn cpu_needs_a_positive_delta() {
    assert_eq!(cpu_percent(t(5, 100), t(5, 100)), None);
    assert_eq!(cpu_percent(t(5, 100), t(5, 90)), None);
}

#[test]
fn cpu_clamps_idle_above_total() {
    assert_eq!(cpu_percent(t(0, 0), t(500, 100)), Some(0.0));
}

#[test]
fn ram_is_used_share() {
    assert_eq!(ram_percent(64, 16), Some(75.0));
    assert_eq!(ram_percent(0, 0), None);
    assert_eq!(ram_percent(10, 20), Some(0.0));
}

#[test]
fn sampler_uses_delta_between_two_calls() {
    let s = Sampler::new();
    let n = Cell::new(0u32);
    let read = || {
        n.set(n.get() + 1);
        Some(match n.get() {
            1 => t(0, 10),
            2 => t(0, 20),
            _ => t(30, 100),
        })
    };
    let t0 = Instant::now();
    assert_eq!(s.cpu(&read, t0), Some(100.0));
    assert_eq!(s.cpu(&read, t0 + Duration::from_secs(3)), Some(62.5));
}

#[test]
fn sampler_primes_again_after_a_long_gap() {
    let s = Sampler::new();
    let reads = Cell::new(0u32);
    let read = || {
        reads.set(reads.get() + 1);
        Some(t(u64::from(reads.get()), u64::from(reads.get()) * 10))
    };
    let t0 = Instant::now();
    s.cpu(&read, t0);
    let after_first = reads.get();
    s.cpu(&read, t0 + Duration::from_secs(60));
    assert_eq!(reads.get() - after_first, 2);
}
