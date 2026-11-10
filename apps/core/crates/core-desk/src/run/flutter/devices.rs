use super::env::{find_flutter_sdk, flutter_spawn, FlutterEnvOpts};
use super::machine::{MachineClient, MachineHandlers, MachineReply};
use crate::run::parse::flutter_device;
use crate::run::proc::{kill_pid, pump_lines, spawn, SpawnSpec};
use crate::run::types::{DeviceService, DevicesCb, Env, EnvIssue, RunDevice, RunEmulator, RunResult, RunSettings};
use serde_json::{json, Value};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const FIRST_BATCH_MS: u64 = 8_000;
const BATCH_SETTLE_MS: u64 = 500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const FALLBACK_TIMEOUT_MS: u64 = 30_000;
const FALLBACK_TTL: Duration = Duration::from_secs(10);
const LAUNCH_TIMEOUT_MS: u64 = 60_000;
const MAX_RESTARTS: usize = 3;
const RESTART_WINDOW: Duration = Duration::from_secs(5 * 60);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct DevicesOpts {
    pub settings: RunSettings,
    pub env: Env,
    pub win: bool,
    pub home: String,
}

pub type SdkForFn = Arc<dyn Fn() -> Option<String> + Send + Sync>;

#[derive(Clone, Default)]
pub struct DevicesDeps {
    pub sdk_for: Option<SdkForFn>,
    pub first_batch_ms: Option<u64>,
}

pub fn machine_json_array(stdout: &str) -> Option<Vec<Value>> {
    let start = stdout.find('[')?;
    let end = stdout.rfind(']')?;
    if end <= start {
        return None;
    }
    match serde_json::from_str::<Value>(&stdout[start..=end]).ok()? {
        Value::Array(a) => Some(a),
        _ => None,
    }
}

pub fn flutter_emulator(raw: &Value) -> Option<RunEmulator> {
    let id = raw.get("id")?.as_str().filter(|s| !s.is_empty())?;
    let name = raw.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or(id);
    Some(RunEmulator { id: id.into(), name: name.into(), source: "flutter".into() })
}

pub fn parse_emulators_text(out: &str) -> Vec<RunEmulator> {
    let mut list = Vec::new();
    for line in out.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let cols: Vec<&str> = line.split('•').map(str::trim).collect();
        if cols.len() < 2 || cols[0].is_empty() || cols[0].contains(char::is_whitespace) || cols[0].to_lowercase() == "id" {
            continue;
        }
        let name = if cols[1].is_empty() { cols[0] } else { cols[1] };
        list.push(RunEmulator { id: cols[0].into(), name: name.into(), source: "flutter".into() });
    }
    list
}

struct Ready {
    done: Mutex<bool>,
    cv: Condvar,
}

impl Ready {
    fn mark(&self) {
        *self.done.lock().unwrap() = true;
        self.cv.notify_all();
    }

    fn wait(&self) {
        let g = self.done.lock().unwrap();
        let _g = self.cv.wait_while(g, |d| !*d).unwrap();
    }
}

struct Daemon {
    pid: u32,
    client: Arc<MachineClient>,
    alive: AtomicBool,
    ready: Ready,
    settling: AtomicBool,
}

#[derive(Default)]
struct St {
    devices: Vec<RunDevice>,
    starts: Vec<Instant>,
    daemon: Option<Arc<Daemon>>,
    one_shots: Vec<u32>,
}

struct Inner {
    opts: DevicesOpts,
    deps: DevicesDeps,
    st: Mutex<St>,
    listeners: Mutex<Vec<DevicesCb>>,
    fallback: Mutex<Option<(Instant, Vec<RunDevice>)>>,
}

pub struct FlutterDevices {
    inner: Arc<Inner>,
}

pub fn create(opts: DevicesOpts) -> Arc<dyn DeviceService> {
    create_with(opts, DevicesDeps::default())
}

pub fn create_with(opts: DevicesOpts, deps: DevicesDeps) -> Arc<dyn DeviceService> {
    Arc::new(FlutterDevices { inner: Arc::new(Inner { opts, deps, st: Mutex::new(St::default()), listeners: Mutex::new(Vec::new()), fallback: Mutex::new(None) }) })
}

impl Inner {
    fn sdk(&self) -> Option<String> {
        if let Some(f) = &self.deps.sdk_for {
            return f();
        }
        let o = &self.opts;
        find_flutter_sdk(&o.home, &FlutterEnvOpts { setting: o.settings.flutter_sdk.clone(), env: o.env.clone(), win: o.win, home: o.home.clone(), ..Default::default() }).sdk
    }

    fn snapshot(&self) -> Vec<RunDevice> {
        self.st.lock().unwrap().devices.clone()
    }

    fn notify(&self) {
        let list = self.snapshot();
        let cbs = self.listeners.lock().unwrap().clone();
        for cb in cbs {
            cb(list.clone());
        }
    }

    fn spawn_flutter(&self, root: &str, args: &[String], stdin: bool) -> Option<std::process::Child> {
        let spec = flutter_spawn(root, args, self.opts.win, &self.opts.env)?;
        spawn(&SpawnSpec { file: spec.file, args: spec.args, cwd: self.opts.home.clone(), verbatim: spec.verbatim, stdin_pipe: stdin, ..Default::default() }, &self.opts.env).ok()
    }

    fn collect(&self, args: &[&str], timeout_ms: u64) -> (Option<i32>, String) {
        let Some(root) = self.sdk() else { return (None, String::new()) };
        let argv: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let Some(mut child) = self.spawn_flutter(&root, &argv, false) else { return (None, String::new()) };
        let pid = child.id();
        self.st.lock().unwrap().one_shots.push(pid);
        let out = child.stdout.take().map(|mut s| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = s.read_to_end(&mut buf);
                String::from_utf8_lossy(&buf).to_string()
            })
        });
        if let Some(mut e) = child.stderr.take() {
            std::thread::spawn(move || std::io::copy(&mut e, &mut std::io::sink()));
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait().ok().and_then(|s| s.code()));
        });
        let code = match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(c) => c,
            Err(_) => {
                kill_pid(pid, false);
                None
            }
        };
        self.st.lock().unwrap().one_shots.retain(|p| *p != pid);
        let stdout = if code.is_some() { out.and_then(|h| h.join().ok()).unwrap_or_default() } else { String::new() };
        (code, stdout)
    }

    fn request(&self, d: &Daemon, method: &str, params: Value) -> MachineReply {
        if !d.alive.load(Ordering::SeqCst) {
            return MachineReply::error("daemon-down");
        }
        let r = d.client.request(method, params, Some(REQUEST_TIMEOUT));
        if r.error_is("not-running") || r.error_is("exited") {
            return MachineReply::error("daemon-down");
        }
        r
    }

    fn settle(d: &Arc<Daemon>) {
        if d.settling.swap(true, Ordering::SeqCst) {
            return;
        }
        let d = d.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(BATCH_SETTLE_MS));
            d.ready.mark();
        });
    }

    fn on_event(self: &Arc<Self>, d: &Arc<Daemon>, event: &str, params: &serde_json::Map<String, Value>) {
        match event {
            "device.added" => {
                let Some(dev) = flutter_device(&Value::Object(params.clone())) else { return };
                {
                    let mut s = self.st.lock().unwrap();
                    match s.devices.iter_mut().find(|x| x.id == dev.id) {
                        Some(slot) => *slot = dev,
                        None => s.devices.push(dev),
                    }
                }
                self.notify();
                Self::settle(d);
            }
            "device.removed" => {
                let Some(id) = params.get("id").and_then(Value::as_str) else { return };
                let removed = {
                    let mut s = self.st.lock().unwrap();
                    let before = s.devices.len();
                    s.devices.retain(|x| x.id != id);
                    s.devices.len() != before
                };
                if removed {
                    self.notify();
                }
            }
            _ => {}
        }
    }

    fn start_daemon(self: &Arc<Self>) -> Option<Arc<Daemon>> {
        let mut s = self.st.lock().unwrap();
        if let Some(d) = s.daemon.as_ref().filter(|d| d.alive.load(Ordering::SeqCst)) {
            return Some(d.clone());
        }
        let now = Instant::now();
        s.starts.retain(|t| now.duration_since(*t) <= RESTART_WINDOW);
        if s.starts.len() > MAX_RESTARTS {
            return None;
        }
        let root = self.sdk()?;
        let mut child = self.spawn_flutter(&root, &["daemon".into()], true)?;
        s.starts.push(now);
        s.devices.clear();
        let stdin = child.stdin.take().map(|w| Box::new(w) as Box<dyn std::io::Write + Send>);
        let d = Arc::new(Daemon { pid: child.id(), client: MachineClient::new(stdin), alive: AtomicBool::new(true), ready: Ready { done: Mutex::new(false), cv: Condvar::new() }, settling: AtomicBool::new(false) });
        s.daemon = Some(d.clone());
        drop(s);
        let first = self.deps.first_batch_ms.unwrap_or(FIRST_BATCH_MS);
        let dt = d.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(first));
            dt.ready.mark();
        });
        let (me, de) = (self.clone(), d.clone());
        let handlers = MachineHandlers { on_event: Arc::new(move |ev, p| me.on_event(&de, ev, p)), on_text: Arc::new(|_| {}) };
        let pump = child.stdout.take().map(|out| d.client.attach(out, handlers));
        if let Some(err) = child.stderr.take() {
            pump_lines(err, |_| {});
        }
        let (me, dw) = (self.clone(), d.clone());
        std::thread::spawn(move || {
            let _ = child.wait();
            if let Some(p) = pump {
                let _ = p.join();
            }
            if dw.alive.swap(false, Ordering::SeqCst) {
                let mut s = me.st.lock().unwrap();
                if s.daemon.as_ref().is_some_and(|x| Arc::ptr_eq(x, &dw)) {
                    s.daemon = None;
                }
                drop(s);
                dw.client.fail_pending("daemon-down");
                dw.ready.mark();
            }
        });
        let _ = d.client.send("device.enable", json!({}));
        let (me, dg) = (self.clone(), d.clone());
        std::thread::spawn(move || {
            let r = me.request(&dg, "device.getDevices", json!({}));
            let Some(Value::Array(list)) = r.result else { return };
            let (changed, any) = {
                let mut s = me.st.lock().unwrap();
                let mut changed = false;
                for raw in &list {
                    if let Some(dev) = flutter_device(raw) {
                        if !s.devices.iter().any(|x| x.id == dev.id) {
                            s.devices.push(dev);
                            changed = true;
                        }
                    }
                }
                (changed, !s.devices.is_empty())
            };
            if changed {
                me.notify();
            }
            if any {
                Self::settle(&dg);
            }
        });
        Some(d)
    }

    fn fallback_list(&self) -> Vec<RunDevice> {
        let mut g = self.fallback.lock().unwrap();
        if let Some((at, list)) = g.as_ref() {
            if at.elapsed() < FALLBACK_TTL {
                return list.clone();
            }
        }
        let at = Instant::now();
        let (_, stdout) = self.collect(&["devices", "--machine"], FALLBACK_TIMEOUT_MS);
        let list: Vec<RunDevice> = machine_json_array(&stdout).unwrap_or_default().iter().filter_map(flutter_device).collect();
        *g = Some((at, list.clone()));
        list
    }
}

impl DeviceService for FlutterDevices {
    fn list(&self) -> Vec<RunDevice> {
        let me = &self.inner;
        if let Some(d) = me.start_daemon() {
            d.ready.wait();
            let list = me.snapshot();
            if d.alive.load(Ordering::SeqCst) || !list.is_empty() {
                return list;
            }
        }
        me.fallback_list()
    }

    fn on_change(&self, cb: DevicesCb) {
        self.inner.listeners.lock().unwrap().push(cb);
    }

    fn emulators(&self) -> Vec<RunEmulator> {
        let me = &self.inner;
        if let Some(d) = me.start_daemon() {
            if let Some(Value::Array(list)) = me.request(&d, "emulator.getEmulators", json!({})).result {
                return list.iter().filter_map(flutter_emulator).collect();
            }
        }
        let (_, stdout) = me.collect(&["emulators", "--machine"], FALLBACK_TIMEOUT_MS);
        match machine_json_array(&stdout) {
            Some(json) => json.iter().filter_map(flutter_emulator).collect(),
            None => parse_emulators_text(&stdout),
        }
    }

    fn launch_emulator(&self, id: &str) -> RunResult {
        let me = &self.inner;
        if let Some(d) = me.start_daemon() {
            let r = me.request(&d, "emulator.launch", json!({"emulatorId": id}));
            match r.error.filter(|e| !e.is_null()) {
                None => return RunResult::ok(),
                Some(Value::String(s)) if s == "daemon-down" => {}
                Some(e) => return RunResult::Err(EnvIssue::with("emulator-launch", e.as_str().map(String::from).unwrap_or_else(|| crate::js::stringify(&e)))),
            }
        }
        let (code, _) = me.collect(&["emulators", "--launch", id], LAUNCH_TIMEOUT_MS);
        if code == Some(0) {
            RunResult::ok()
        } else {
            RunResult::Err(EnvIssue::with("emulator-launch", id))
        }
    }

    fn stop(&self, sync: bool) {
        let (shots, d) = {
            let mut s = self.inner.st.lock().unwrap();
            let shots = std::mem::take(&mut s.one_shots);
            let d = s.daemon.take();
            if d.is_some() {
                s.devices.clear();
            }
            (shots, d)
        };
        for p in shots {
            kill_pid(p, sync);
        }
        let Some(d) = d else { return };
        if sync {
            kill_pid(d.pid, true);
            return;
        }
        let _ = d.client.send("daemon.shutdown", json!({}));
        d.client.close_stdin();
        std::thread::spawn(move || {
            let t = Instant::now();
            while d.alive.load(Ordering::SeqCst) && t.elapsed() < SHUTDOWN_GRACE {
                std::thread::sleep(Duration::from_millis(25));
            }
            if d.alive.load(Ordering::SeqCst) {
                kill_pid(d.pid, false);
            }
        });
    }
}

#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;
