use std::path::PathBuf;

use serde_json::{json, Value};

use super::json_file::JsonFile;
use super::jsstr::js_round;
use super::localtime::Zone;
use super::{system_clock, Clock};
use crate::extra::{LimitForecast, LimitWindow, PlanLimits};
use crate::num::JsNum;
use crate::time::iso_of_ms;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimitSample {
    pub at: f64,
    pub pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKey {
    FiveHour,
    SevenDay,
}

impl WindowKey {
    const ALL: [WindowKey; 2] = [WindowKey::FiveHour, WindowKey::SevenDay];

    fn name(self) -> &'static str {
        match self {
            WindowKey::FiveHour => "fiveHour",
            WindowKey::SevenDay => "sevenDay",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> WindowSpec {
        match self {
            WindowKey::FiveHour => WindowSpec { lookback_ms: 60.0 * 60_000.0, min_span_ms: 10.0 * 60_000.0, step_ms: 60_000.0 },
            WindowKey::SevenDay => WindowSpec { lookback_ms: 24.0 * 3_600_000.0, min_span_ms: 2.0 * 3_600_000.0, step_ms: 10.0 * 60_000.0 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSpec {
    pub lookback_ms: f64,
    pub min_span_ms: f64,
    pub step_ms: f64,
}

const MIN_PCT_PER_HOUR: f64 = 0.05;

fn round1(n: f64) -> f64 {
    js_round(n * 10.0) / 10.0
}

pub fn forecast_window(samples: &[LimitSample], resets_at: Option<&str>, spec: WindowSpec, zone: Zone) -> Option<LimitForecast> {
    let last = samples.last()?;
    let recent: Vec<&LimitSample> = samples.iter().filter(|s| last.at - s.at <= spec.lookback_ms).collect();
    let first = recent.first()?;
    if recent.len() < 2 || last.at - first.at < spec.min_span_ms {
        return None;
    }
    let n = recent.len() as f64;
    let mx = recent.iter().fold(0.0, |a, s| a + s.at) / n;
    let my = recent.iter().fold(0.0, |a, s| a + s.pct) / n;
    let (mut num, mut den) = (0.0, 0.0);
    for s in &recent {
        num += (s.at - mx) * (s.pct - my);
        den += (s.at - mx) * (s.at - mx);
    }
    let slope = if den > 0.0 { num / den } else { 0.0 };
    let pct_per_hour = slope * 3_600_000.0;
    if pct_per_hour < MIN_PCT_PER_HOUR || last.pct >= 100.0 {
        return Some(LimitForecast { pct_per_hour: JsNum(round1(pct_per_hour.max(0.0))), full_at: None, before_reset: None });
    }
    let full_ms = last.at + (100.0 - last.pct) / slope;
    let reset = resets_at.and_then(|r| zone.parse_iso(r)).map(|r| r as f64);
    Some(LimitForecast {
        pct_per_hour: JsNum(round1(pct_per_hour)),
        full_at: Some(iso_of_ms(full_ms as i64)),
        before_reset: Some(reset.is_none_or(|r| full_ms < r)),
    })
}

#[derive(Debug, Clone, Default)]
struct WindowState {
    resets_at: Option<String>,
    samples: Vec<LimitSample>,
}

pub struct LimitHistory {
    windows: [WindowState; 2],
    last_at: String,
    file: Option<JsonFile>,
    zone: Zone,
    clock: Clock,
}

impl LimitHistory {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self::with_clock(file, Zone::Local, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, zone: Zone, clock: Clock) -> Self {
        Self { windows: Default::default(), last_at: String::new(), file: file.map(|f| JsonFile::new(f, 60_000)), zone, clock }
    }

    pub fn load(&mut self) {
        let Some(Value::Object(raw)) = self.file.as_ref().and_then(JsonFile::read) else { return };
        for key in WindowKey::ALL {
            let Some(w) = raw.get(key.name()) else { continue };
            let Some(Value::Array(samples)) = w.get("samples") else { continue };
            let kept: Vec<LimitSample> = samples
                .iter()
                .filter_map(|s| Some(LimitSample { at: s.get("at")?.as_f64()?, pct: s.get("pct")?.as_f64()? }))
                .collect();
            self.windows[key.index()] = WindowState {
                resets_at: w.get("resetsAt").and_then(Value::as_str).map(String::from),
                samples: kept[kept.len().saturating_sub(500)..].to_vec(),
            };
        }
    }

    pub fn apply(&mut self, limits: Option<&PlanLimits>) -> Option<PlanLimits> {
        let limits = limits?;
        let at = self.zone.parse_iso(&limits.updated_at).map(|n| n as f64);
        let fresh = limits.updated_at != self.last_at && at.is_some();
        if fresh {
            self.last_at = limits.updated_at.clone();
        }
        let mut out = limits.clone();
        for key in WindowKey::ALL {
            let Some(w) = (match key {
                WindowKey::FiveHour => limits.five_hour.as_ref(),
                WindowKey::SevenDay => limits.seven_day.as_ref(),
            }) else {
                continue;
            };
            if let (true, Some(at)) = (fresh, at) {
                self.record(key, w, at);
            }
            let forecast = forecast_window(&self.windows[key.index()].samples, w.resets_at.as_deref(), key.spec(), self.zone);
            let mut next = w.clone();
            if forecast.is_some() {
                next.forecast = forecast;
            }
            match key {
                WindowKey::FiveHour => out.five_hour = Some(next),
                WindowKey::SevenDay => out.seven_day = Some(next),
            }
        }
        if fresh {
            let now = (self.clock)();
            if let Some(f) = &mut self.file {
                f.schedule(now);
            }
        }
        Some(out)
    }

    fn record(&mut self, key: WindowKey, w: &LimitWindow, at: f64) {
        let zone = self.zone;
        let spec = key.spec();
        let state = &mut self.windows[key.index()];
        let last = state.samples.last().copied();
        let reset_moved = match (&w.resets_at, &state.resets_at) {
            (Some(a), Some(b)) => match (zone.parse_iso(a), zone.parse_iso(b)) {
                (Some(x), Some(y)) => ((x - y) as f64).abs() > 5.0 * 60_000.0,
                _ => false,
            },
            _ => false,
        };
        let used = w.used_pct.0;
        if reset_moved || last.is_some_and(|l| used < l.pct - 0.5) {
            state.samples.clear();
        }
        if let Some(r) = &w.resets_at {
            state.resets_at = Some(r.clone());
        }
        match state.samples.last_mut() {
            Some(prev) if at - prev.at < spec.step_ms => prev.pct = used,
            _ => state.samples.push(LimitSample { at, pct: used }),
        }
        state.samples.retain(|s| at - s.at <= spec.lookback_ms * 2.0);
    }

    fn state(&self) -> Value {
        let mut out = serde_json::Map::new();
        for key in WindowKey::ALL {
            let w = &self.windows[key.index()];
            let samples: Vec<Value> = w.samples.iter().map(|s| json!({ "at": JsNum(s.at), "pct": JsNum(s.pct) })).collect();
            let mut o = serde_json::Map::new();
            if let Some(r) = &w.resets_at {
                o.insert("resetsAt".into(), json!(r));
            }
            o.insert("samples".into(), Value::Array(samples));
            out.insert(key.name().into(), Value::Object(o));
        }
        Value::Object(out)
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

    const Z: Zone = Zone::Fixed(0);

    fn limits(updated: &str, used: f64, resets: &str) -> PlanLimits {
        serde_json::from_value(json!({ "updatedAt": updated, "fiveHour": { "usedPct": used, "resetsAt": resets } })).unwrap()
    }

    fn rig(file: Option<PathBuf>) -> LimitHistory {
        let t = Arc::new(AtomicI64::new(1_800_000_000_000));
        LimitHistory::with_clock(file, Z, Box::new(move || t.load(Ordering::SeqCst)))
    }

    fn samples(points: &[(f64, f64)]) -> Vec<LimitSample> {
        points.iter().map(|&(at, pct)| LimitSample { at, pct }).collect()
    }

    #[test]
    fn forecast_needs_span_and_a_real_burn_rate() {
        let spec = WindowKey::FiveHour.spec();
        assert_eq!(forecast_window(&samples(&[(0.0, 10.0)]), None, spec, Z), None);
        assert_eq!(forecast_window(&samples(&[(0.0, 10.0), (60_000.0, 11.0)]), None, spec, Z), None);
        let flat = forecast_window(&samples(&[(0.0, 10.0), (900_000.0, 10.0)]), None, spec, Z).unwrap();
        assert_eq!(flat, LimitForecast { pct_per_hour: JsNum(0.0), full_at: None, before_reset: None });
        let steady = samples(&[(0.0, 10.0), (900_000.0, 20.0), (1_800_000.0, 30.0)]);
        let f = forecast_window(&steady, Some("1970-01-01T05:00:00.000Z"), spec, Z).unwrap();
        assert_eq!(f.pct_per_hour, JsNum(40.0));
        assert_eq!(f.full_at.as_deref(), Some("1970-01-01T02:15:00.000Z"));
        assert_eq!(f.before_reset, Some(true));
        let late = forecast_window(&steady, Some("1970-01-01T01:00:00.000Z"), spec, Z).unwrap();
        assert_eq!(late.before_reset, Some(false));
        assert_eq!(forecast_window(&steady, None, spec, Z).unwrap().before_reset, Some(true));
        let full = forecast_window(&samples(&[(0.0, 90.0), (900_000.0, 100.0)]), None, spec, Z).unwrap();
        assert_eq!((full.pct_per_hour, full.full_at), (JsNum(40.0), None));
    }

    #[test]
    fn apply_records_once_per_report_thins_samples_and_resets_on_drop() {
        let mut h = rig(None);
        assert_eq!(h.apply(None), None);
        let r = "2027-01-15T12:00:00.000Z";
        let first = h.apply(Some(&limits("2027-01-15T08:00:00.000Z", 10.0, r))).unwrap();
        assert_eq!(first.five_hour.as_ref().unwrap().forecast, None);
        h.apply(Some(&limits("2027-01-15T08:00:00.000Z", 50.0, r)));
        assert_eq!(h.windows[0].samples.len(), 1);
        h.apply(Some(&limits("2027-01-15T08:00:30.000Z", 11.0, r)));
        assert_eq!(h.windows[0].samples, samples(&[(1_800_000_000_000.0, 11.0)]));
        h.apply(Some(&limits("2027-01-15T08:15:00.000Z", 21.0, r)));
        let out = h.apply(Some(&limits("2027-01-15T08:30:00.000Z", 31.0, r))).unwrap();
        let f = out.five_hour.unwrap().forecast.unwrap();
        assert_eq!(f.pct_per_hour, JsNum(40.0));
        h.apply(Some(&limits("2027-01-15T08:45:00.000Z", 5.0, r)));
        assert_eq!(h.windows[0].samples.len(), 1);
        h.apply(Some(&limits("2027-01-15T09:00:00.000Z", 6.0, "2027-01-15T18:00:00.000Z")));
        assert_eq!(h.windows[0].samples.len(), 1);
        assert_eq!(h.windows[0].resets_at.as_deref(), Some("2027-01-15T18:00:00.000Z"));
        let bad = h.apply(Some(&limits("soon", 1.0, r))).unwrap();
        assert_eq!(bad.updated_at, "soon");
        assert_eq!(h.windows[0].samples.len(), 1);
    }

    #[test]
    fn saves_a_minute_after_a_fresh_report_and_reloads() {
        let dir = temp_dir("limits");
        let file = dir.join("limit-history.json");
        let mut h = rig(Some(file.clone()));
        h.apply(Some(&limits("2027-01-15T08:00:00.000Z", 10.0, "2027-01-15T12:00:00.000Z")));
        assert_eq!(h.next_flush_at(), Some(1_800_000_060_000));
        assert!(h.poll(1_800_000_060_000));
        let mut again = rig(Some(file.clone()));
        again.load();
        assert_eq!(again.windows[0].samples, h.windows[0].samples);
        assert_eq!(again.windows[0].resets_at, h.windows[0].resets_at);
        fs::write(&file, r#"{"fiveHour":{"samples":[{"at":1,"pct":2},{"at":"x","pct":1},null]},"sevenDay":{"samples":5}}"#).unwrap();
        let mut odd = rig(Some(file));
        odd.load();
        assert_eq!(odd.windows[0].samples, samples(&[(1.0, 2.0)]));
        assert!(odd.windows[1].samples.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
