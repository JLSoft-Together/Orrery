use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::json;

use crate::procs::ProcCache;

pub struct Stats {
    on: bool,
    ticks: u64,
    lock_ms: Vec<f64>,
    phase_ms: [Vec<f64>; 3],
    prefetch_ms: Vec<f64>,
    outside_ms: Vec<f64>,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn summary(mut v: Vec<f64>) -> serde_json::Value {
    v.sort_by(f64::total_cmp);
    let round = |x: f64| (x * 100.0).round() / 100.0;
    json!({ "n": v.len(), "p50": round(pct(&v, 0.5)), "p99": round(pct(&v, 0.99)), "max": round(v.last().copied().unwrap_or(0.0)) })
}

impl Stats {
    pub fn new(on: bool) -> Stats {
        Stats { on, ticks: 0, lock_ms: Vec::new(), phase_ms: Default::default(), prefetch_ms: Vec::new(), outside_ms: Vec::new() }
    }

    pub fn tick(&mut self, held: Duration, phases: [Duration; 3]) {
        self.ticks += 1;
        if self.on {
            self.lock_ms.push(ms(held));
            for (list, d) in self.phase_ms.iter_mut().zip(phases) {
                list.push(ms(d));
            }
        }
    }

    pub fn prefetch(&mut self, ms: f64) {
        if self.on {
            self.prefetch_ms.push(ms);
        }
    }

    pub fn outside(&mut self, d: Duration) {
        if self.on {
            self.outside_ms.push(ms(d));
        }
    }

    pub fn report(&mut self, cache: &ProcCache) {
        if !self.on {
            return;
        }
        let out = json!({
            "type": "serve.stats",
            "ticks": self.ticks,
            "lockMs": summary(std::mem::take(&mut self.lock_ms)),
            "commandsMs": summary(std::mem::take(&mut self.phase_ms[0])),
            "tickMs": summary(std::mem::take(&mut self.phase_ms[1])),
            "afterTickMs": summary(std::mem::take(&mut self.phase_ms[2])),
            "prefetchMs": summary(std::mem::take(&mut self.prefetch_ms)),
            "outsideLockMs": summary(std::mem::take(&mut self.outside_ms)),
            "tableReadsUnderLock": cache.missed.load(Ordering::Relaxed),
        });
        eprintln!("{out}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarises_percentiles() {
        let s = summary(vec![5.0, 1.0, 3.0, 2.0, 4.0]);
        assert_eq!(s["n"], 5);
        assert_eq!(s["p50"], 3.0);
        assert_eq!(s["max"], 5.0);
        assert_eq!(summary(Vec::new())["max"], 0.0);
    }
}
