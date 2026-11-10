use std::path::PathBuf;

use serde_json::{json, Value};

use super::json_file::JsonFile;
use super::jsstr::js_round;
use super::localtime::Zone;
use super::{system_clock, Clock};
use crate::extra::ResponseStats;
use crate::num::JsNum;

const LONG_WAIT_MS: f64 = 5.0 * 60_000.0;
const MAX_SAMPLES: usize = 1000;
const MAX_WAIT_MS: f64 = 8.0 * 3_600_000.0;

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() >> 1;
    Some(if sorted.len() % 2 == 1 { sorted[mid] } else { js_round((sorted[mid - 1] + sorted[mid]) / 2.0) })
}

pub struct ResponseTracker {
    day: String,
    samples: Vec<f64>,
    long_waits: f64,
    file: Option<JsonFile>,
    zone: Zone,
    clock: Clock,
}

impl ResponseTracker {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self::with_clock(file, Zone::Local, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, zone: Zone, clock: Clock) -> Self {
        let day = zone.local_date(clock());
        Self { day, samples: Vec::new(), long_waits: 0.0, file: file.map(|f| JsonFile::new(f, 10_000)), zone, clock }
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    pub fn load(&mut self) {
        let Some(Value::Object(raw)) = self.file.as_ref().and_then(JsonFile::read) else { return };
        let (Some(Value::String(day)), Some(Value::Array(samples))) = (raw.get("day"), raw.get("samples")) else { return };
        if *day != self.zone.local_date(self.now()) {
            return;
        }
        self.day = day.clone();
        let kept: Vec<f64> = samples.iter().filter_map(Value::as_f64).filter(|n| *n >= 0.0).collect();
        self.samples = kept[kept.len().saturating_sub(MAX_SAMPLES)..].to_vec();
        self.long_waits = raw.get("longWaits").and_then(Value::as_f64).unwrap_or(0.0);
    }

    fn schedule_save(&mut self) {
        let now = self.now();
        if let Some(f) = &mut self.file {
            f.schedule(now);
        }
    }

    pub fn roll_day(&mut self) -> bool {
        let today = self.zone.local_date(self.now());
        if today == self.day {
            return false;
        }
        self.day = today;
        self.samples.clear();
        self.long_waits = 0.0;
        self.schedule_save();
        true
    }

    pub fn record(&mut self, wait_ms: f64) -> bool {
        if !wait_ms.is_finite() || !(0.0..=MAX_WAIT_MS).contains(&wait_ms) {
            return false;
        }
        self.roll_day();
        self.samples.push(js_round(wait_ms));
        if self.samples.len() > MAX_SAMPLES {
            self.samples.remove(0);
        }
        if wait_ms > LONG_WAIT_MS {
            self.long_waits += 1.0;
        }
        self.schedule_save();
        true
    }

    pub fn stats(&self) -> ResponseStats {
        ResponseStats {
            day: self.day.clone(),
            count: self.samples.len() as u64,
            total_ms: JsNum(self.samples.iter().sum()),
            median_ms: median(&self.samples).map(JsNum),
            long_waits: self.long_waits as u64,
        }
    }

    fn state(&self) -> Value {
        json!({ "day": self.day, "samples": self.samples.iter().map(|n| JsNum(*n)).collect::<Vec<_>>(), "longWaits": JsNum(self.long_waits) })
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let state = self.state();
        self.file.as_mut().is_some_and(|f| f.poll(now, || state))
    }

    pub fn save(&mut self) -> bool {
        let state = self.state();
        self.file.as_mut().is_some_and(|f| f.flush(|| state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use std::fs;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    fn rig(file: Option<PathBuf>, start: i64) -> (ResponseTracker, Arc<AtomicI64>) {
        let t = Arc::new(AtomicI64::new(start));
        let c = t.clone();
        (ResponseTracker::with_clock(file, Zone::Fixed(0), Box::new(move || c.load(Ordering::SeqCst))), t)
    }

    #[test]
    fn records_waits_with_median_and_long_waits() {
        let (mut r, _) = rig(None, 1_800_000_000_000);
        assert!(!r.record(-1.0) && !r.record(f64::NAN) && !r.record(MAX_WAIT_MS + 1.0));
        assert_eq!(r.stats().median_ms, None);
        for w in [1000.0, 400_000.5, 3000.0, 2000.0] {
            assert!(r.record(w));
        }
        let s = r.stats();
        assert_eq!((s.count, s.long_waits), (4, 1));
        assert_eq!(s.total_ms, JsNum(1000.0 + 400_001.0 + 3000.0 + 2000.0));
        assert_eq!(s.median_ms, Some(JsNum(2500.0)));
        assert_eq!(s.day, "2027-01-15");
    }

    #[test]
    fn rolls_at_a_new_day_and_survives_a_restart_on_the_same_day() {
        let dir = temp_dir("response");
        let file = dir.join("response-stats.json");
        let (mut r, t) = rig(Some(file.clone()), 1_800_000_000_000);
        r.record(1000.0);
        assert_eq!(r.next_flush_at(), Some(1_800_000_010_000));
        assert!(r.poll(1_800_000_010_000));
        let (mut same, _) = rig(Some(file.clone()), 1_800_000_000_000 + 3_600_000);
        same.load();
        assert_eq!(same.stats().count, 1);
        t.fetch_add(86_400_000, Ordering::SeqCst);
        assert!(r.roll_day());
        assert!(!r.roll_day());
        assert_eq!(r.stats().count, 0);
        let (mut next, _) = rig(Some(file), 1_800_000_000_000 + 86_400_000);
        next.load();
        assert_eq!(next.stats().count, 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
