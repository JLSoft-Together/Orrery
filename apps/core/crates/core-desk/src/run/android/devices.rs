use super::adb_watch::{AdbWatch, AdbWatchOpts, AdbWatchTiming};
use super::deps::{EnvDeps, FsEnvDeps};
use super::env::find_adb_tools_with;
use super::jsre::Js;
use crate::run::collect::{collect, CollectOpts};
use crate::run::parse::parse_adb_devices;
use crate::run::proc::{kill_pid_only, spawn, SpawnSpec};
use crate::run::types::{DeviceService, DevicesCb, Env, EnvIssue, RunDevice, RunEmulator, RunResult, RunSettings};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub struct DevicesOpts {
    pub settings: RunSettings,
    pub env: Env,
    pub win: bool,
    pub home: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevicesTiming {
    pub boot_poll_ms: u64,
    pub boot_timeout_ms: u64,
    pub watch: AdbWatchTiming,
}

impl Default for DevicesTiming {
    fn default() -> Self {
        DevicesTiming { boot_poll_ms: 2_000, boot_timeout_ms: 180_000, watch: AdbWatchTiming::default() }
    }
}

type Tools = (Option<String>, Option<String>);

struct Shared {
    opts: DevicesOpts,
    deps: Arc<dyn EnvDeps>,
    tools: OnceLock<Tools>,
    devices: Mutex<Vec<RunDevice>>,
    stopped: AtomicBool,
}

impl Shared {
    fn tools(&self) -> Tools {
        self.tools.get_or_init(|| find_adb_tools_with(self.opts.settings.android_sdk.as_deref(), &self.opts.env, self.opts.win, &self.opts.home, self.deps.as_ref())).clone()
    }

    fn run(&self, file: &str, args: &[&str], timeout_ms: u64, adb: bool) -> crate::run::collect::Collected {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let opts = CollectOpts { timeout_ms: Some(timeout_ms), kill: adb.then_some(kill_pid_only as fn(u32)), ..Default::default() };
        collect(file, &args, &self.opts.home, &self.opts.env, &opts)
    }
}

struct AdbDevices {
    shared: Arc<Shared>,
    watch: AdbWatch<RunDevice>,
    timing: DevicesTiming,
}

pub fn create(opts: DevicesOpts) -> Arc<dyn DeviceService> {
    create_with(opts, DevicesTiming::default(), Arc::new(FsEnvDeps))
}

pub fn create_with(opts: DevicesOpts, timing: DevicesTiming, deps: Arc<dyn EnvDeps>) -> Arc<dyn DeviceService> {
    let shared = Arc::new(Shared { deps, tools: OnceLock::new(), devices: Mutex::new(Vec::new()), stopped: AtomicBool::new(false), opts });
    let (a, p) = (shared.clone(), shared.clone());
    let watch = AdbWatch::new(AdbWatchOpts {
        cwd: shared.opts.home.clone(),
        env: shared.opts.env.clone(),
        adb: Arc::new(move || a.tools().0),
        parse: Arc::new(move |body: &str| {
            let list = parse_adb_devices(body);
            *p.devices.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
            list
        }),
        timing: timing.watch,
        kill: Some(kill_pid_only),
    });
    Arc::new(AdbDevices { shared, watch, timing })
}

fn emulator_serials(list: &[RunDevice]) -> HashSet<String> {
    list.iter().filter(|d| d.emulator).map(|d| d.id.clone()).collect()
}

fn failed(code: &str, id: &str) -> RunResult {
    RunResult::Err(EnvIssue::with(code, id))
}

impl DeviceService for AdbDevices {
    fn list(&self) -> Vec<RunDevice> {
        self.watch.list()
    }

    fn on_change(&self, cb: DevicesCb) {
        self.watch.on_change(cb)
    }

    fn emulators(&self) -> Vec<RunEmulator> {
        static AVD: Js = Js::new(r"^[\w.-]+$", "");
        let Some(emulator) = self.shared.tools().1 else { return Vec::new() };
        let r = self.shared.run(&emulator, &["-list-avds"], 15_000, false);
        if r.code != Some(0) {
            return Vec::new();
        }
        super::detect::split_lines(&r.out)
            .into_iter()
            .map(str::trim)
            .filter(|l| !l.is_empty() && AVD.test(l))
            .map(|id| RunEmulator { id: id.to_string(), name: id.replace('_', " "), source: "adb".into() })
            .collect()
    }

    fn launch_emulator(&self, id: &str) -> RunResult {
        let s = &self.shared;
        let (Some(adb), Some(emulator)) = s.tools() else { return RunResult::err("emulator-missing") };
        let r0 = s.run(&adb, &["devices", "-l"], 10_000, true);
        let before = emulator_serials(&if r0.code == Some(0) { parse_adb_devices(&r0.out) } else { s.devices.lock().unwrap_or_else(|e| e.into_inner()).clone() });
        let spec = SpawnSpec { file: emulator, args: vec!["-avd".into(), id.into()], cwd: s.opts.home.clone(), detached: true, ..Default::default() };
        let Ok(mut child) = spawn(&spec, &s.opts.env) else { return failed("emulator-failed", id) };
        let exited = Arc::new(AtomicBool::new(false));
        let flag = exited.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            flag.store(true, Ordering::SeqCst);
        });
        let deadline = Instant::now() + Duration::from_millis(self.timing.boot_timeout_ms);
        let mut serial: Option<String> = None;
        while Instant::now() < deadline && !s.stopped.load(Ordering::SeqCst) {
            if exited.load(Ordering::SeqCst) && serial.is_none() {
                return failed("emulator-failed", id);
            }
            if serial.is_none() {
                let r = s.run(&adb, &["devices", "-l"], 10_000, true);
                let fresh: Vec<RunDevice> = if r.code == Some(0) { parse_adb_devices(&r.out) } else { Vec::new() }.into_iter().filter(|d| d.emulator && !before.contains(&d.id)).collect();
                for d in fresh {
                    let name = s.run(&adb, &["-s", &d.id, "emu", "avd", "name"], 10_000, true);
                    let avd = name.out.trim().split('\n').next().map(|l| l.trim().to_string()).unwrap_or_default();
                    if avd.is_empty() || avd == id {
                        serial = Some(d.id);
                        break;
                    }
                }
            }
            if let Some(sr) = &serial {
                let boot = s.run(&adb, &["-s", sr, "shell", "getprop", "sys.boot_completed"], 10_000, true);
                if boot.out.trim() == "1" {
                    return RunResult::ok();
                }
            }
            std::thread::sleep(Duration::from_millis(self.timing.boot_poll_ms));
        }
        failed("emulator-timeout", id)
    }

    fn stop(&self, sync: bool) {
        self.shared.stopped.store(true, Ordering::SeqCst);
        self.watch.stop(sync);
    }
}

#[cfg(all(test, unix))]
#[path = "devices_tests.rs"]
mod tests;
