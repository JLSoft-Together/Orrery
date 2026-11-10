use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use core_http::StoreHandle;
use core_sources::resources::fixture::FixtureReader;
use core_sources::resources::ports::{ports_by_pid, ConflictCall, PortMap};
use core_sources::resources::procs::{CoreReader, ProcReader};
use core_sources::resources::runner::{Runner, RES_SAMPLE_MS};
use core_sources::resources::sampler::{ResourceSampler, SampleInput};
use core_store::time::now_ms;
use serde_json::{json, Value};

use crate::alerts::{lock, Emit, SharedAlerts};

static LATEST: OnceLock<Mutex<Option<Value>>> = OnceLock::new();

pub fn latest_resources() -> Option<Value> {
    LATEST.get().and_then(|m| m.lock().ok()).and_then(|r| r.clone())
}

fn keep_latest(report: &Value) {
    if let Ok(mut r) = LATEST.get_or_init(|| Mutex::new(None)).lock() {
        *r = Some(report.clone());
    }
}

#[derive(Default, Clone)]
pub struct SampleExtras {
    pub idle_minutes: f64,
    pub last_device_call_at: Option<f64>,
    pub conflicts: Vec<ConflictCall>,
}

pub struct Sampling {
    sampler: Arc<Mutex<ResourceSampler>>,
    runner: Mutex<Option<Runner>>,
    windows: AtomicUsize,
    pub extras: Arc<Mutex<SampleExtras>>,
}

fn ports_of(fixture: Option<&PathBuf>) -> Option<PortMap> {
    match fixture {
        Some(f) => FixtureReader::new(f).ports(),
        None => core_procs::ports().map(|rows| ports_by_pid(&rows)),
    }
}

impl Sampling {
    pub fn start(fixture: Option<PathBuf>, store: StoreHandle, alerts: SharedAlerts, emit: Emit) -> Arc<Sampling> {
        let reader: Box<dyn ProcReader> = match &fixture {
            Some(f) => Box::new(FixtureReader::new(f)),
            None => Box::new(CoreReader::host()),
        };
        let sampler = Arc::new(Mutex::new(ResourceSampler::new(reader)));
        let extras = Arc::new(Mutex::new(SampleExtras::default()));
        let (s, x) = (sampler.clone(), extras.clone());
        let runner = Runner::spawn(Duration::from_millis(RES_SAMPLE_MS), move || {
            let (terminals, agents, jobs) = store.with(|st| (st.terminal_list().cloned().collect::<Vec<_>>(), st.agent_list().cloned().collect::<Vec<_>>(), st.job_list().to_vec()));
            let extra = x.lock().map(|e| e.clone()).unwrap_or_default();
            let input = SampleInput {
                terminals: &terminals,
                agents: &agents,
                jobs: &jobs,
                idle_minutes: extra.idle_minutes,
                last_device_call_at: extra.last_device_call_at,
                conflicts: &extra.conflicts,
                ports: ports_of(fixture.as_ref()),
                now: now_ms() as f64,
            };
            let Some(report) = s.lock().ok().and_then(|mut smp| smp.sample(&input)) else { return };
            store.with(|st| {
                st.set_resources(report.clone());
                let mut a = lock(&alerts);
                a.toaster.observe_idle(&report);
                a.pump();
            });
            keep_latest(&report);
            emit(&json!({ "type": "resources", "report": report }));
        });
        Arc::new(Sampling { sampler, runner: Mutex::new(Some(runner)), windows: AtomicUsize::new(0), extras })
    }

    pub fn set_windows(&self, n: usize) {
        self.windows.store(n, Ordering::SeqCst);
    }

    pub fn sync(&self, idle_watch: bool) {
        let on = self.windows.load(Ordering::SeqCst) > 0 || idle_watch;
        if let Some(r) = self.runner.lock().ok().as_ref().and_then(|r| r.as_ref()) {
            r.set_active(on);
        }
    }

    pub fn device_used(&self, v: &Value) {
        let Some((serial, avd, at)) = device_used(v) else { return };
        if let Ok(mut s) = self.sampler.lock() {
            s.device_used(&serial, avd.as_deref(), at);
        }
    }

    pub fn stop(&self) {
        let runner = self.runner.lock().ok().and_then(|mut r| r.take());
        drop(runner);
    }
}

fn charset_ok(s: &str, max: usize, colon: bool) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-' || (colon && b == b':'))
}

pub fn device_used(v: &Value) -> Option<(String, Option<String>, f64)> {
    if v.get("type").and_then(Value::as_str) != Some("device.used") {
        return None;
    }
    let serial = v.get("serial")?.as_str().filter(|s| charset_ok(s, 64, true))?.to_owned();
    let at = v.get("at")?.as_f64().filter(|a| a.is_finite() && *a > 0.0)?;
    let avd = match v.get("avd") {
        None => None,
        Some(a) => Some(a.as_str().filter(|s| charset_ok(s, 100, false))?.to_owned()),
    };
    Some((serial, avd, at))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn device_used_is_checked_like_parse_device_used() {
        assert_eq!(device_used(&json!({"type": "device.used", "serial": "emulator-5554", "at": 5})), Some(("emulator-5554".into(), None, 5.0)));
        assert_eq!(device_used(&json!({"type": "device.used", "serial": "1.2.3.4:5555", "avd": "Pixel_8", "at": 1})).unwrap().1.as_deref(), Some("Pixel_8"));
        for bad in [json!({"type": "device.used", "serial": "a b", "at": 1}), json!({"type": "device.used", "serial": "x", "at": 0}), json!({"type": "device.used", "serial": "x", "at": 1, "avd": "a:b"}), json!({"type": "x", "serial": "x", "at": 1})] {
            assert!(device_used(&bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn the_latest_report_is_kept_for_in_process_readers() {
        keep_latest(&json!({"servers": [], "marker": 41}));
        keep_latest(&json!({"servers": [], "marker": 42}));
        assert_eq!(latest_resources().unwrap()["marker"], 42);
    }
}
