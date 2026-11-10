use super::host::{host_context, host_source, read_package, write_host};
use super::live::Live;
use super::project::{scan_project, Entry};
use super::snap::{res_value, FwpError, Res, Snap};
use super::watcher::Unwatch;
use crate::run::flutter::deps::FsEnvDeps;
use crate::run::flutter::env::{env_opts, find_flutter_sdk, flutter_spawn, sdk_inside_project};
use crate::run::flutter::machine::{MachineClient, MachineHandlers};
use crate::run::proc::{kill_pid, pump_lines, spawn, SpawnSpec};
use crate::run::types::{Env, RunSettings};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type OnSave = Arc<dyn Fn(Vec<String>) + Send + Sync>;
pub type OnPubspec = Arc<dyn Fn() + Send + Sync>;
pub type WatchFn = Arc<dyn Fn(&Path, OnSave) -> Unwatch + Send + Sync>;
pub type WatchPubspecFn = Arc<dyn Fn(&Path, OnPubspec) -> Unwatch + Send + Sync>;

pub struct Deps {
    pub env: Env,
    pub win: bool,
    pub platform: &'static str,
    pub home: String,
    pub settings: Arc<dyn Fn() -> RunSettings + Send + Sync>,
    pub emit: Arc<dyn Fn(Value) + Send + Sync>,
    pub watch: WatchFn,
    pub watch_pubspec: WatchPubspecFn,
    pub stop_timeout: Duration,
    pub log_flush: Duration,
    pub pub_get_timeout: Duration,
}

pub const LOG_CAP: usize = 300;

pub struct Inner {
    pub deps: Deps,
    pub snap: Mutex<Snap>,
    pub live: Mutex<Option<Arc<Live>>>,
    pub gen: AtomicU64,
    pub log_pending: AtomicBool,
}

#[derive(Clone)]
pub struct FlutterPreview(pub Arc<Inner>);

fn flavor_ok(f: &str) -> bool {
    (1..=64).contains(&f.len()) && f.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl FlutterPreview {
    pub fn new(deps: Deps) -> Self {
        FlutterPreview(Arc::new(Inner { deps, snap: Mutex::new(Snap::new()), live: Mutex::new(None), gen: AtomicU64::new(0), log_pending: AtomicBool::new(false) }))
    }

    pub fn get(&self) -> Value {
        self.0.snap_value()
    }

    pub fn scan(&self, root: &str) -> Vec<Entry> {
        let entries = scan_project(Path::new(root));
        if self.0.snap.lock().unwrap().root.as_deref() == Some(root) {
            let e = entries.clone();
            self.0.patch(move |s| s.entries = e);
        }
        entries
    }

    pub fn start(&self, root: &str, device_id: &str, flavor: Option<&str>) -> Value {
        res_value(&self.0.start(root, device_id, flavor))
    }

    pub fn show(&self, id: &str) -> Value {
        res_value(&self.0.show(id))
    }

    pub fn stop(&self) {
        self.0.stop()
    }

    pub fn kill_now(&self) {
        self.0.kill_now()
    }
}

impl Inner {
    pub fn snap_value(&self) -> Value {
        self.snap.lock().unwrap().to_value()
    }

    pub fn patch(&self, f: impl FnOnce(&mut Snap)) {
        let v = {
            let mut s = self.snap.lock().unwrap();
            f(&mut s);
            s.to_value()
        };
        (self.deps.emit)(v);
    }

    pub fn log(self: &Arc<Self>, line: &str) {
        {
            let mut s = self.snap.lock().unwrap();
            s.log.push(line.to_string());
            if s.log.len() > LOG_CAP {
                let drop = s.log.len() - LOG_CAP;
                s.log.drain(..drop);
            }
        }
        if self.log_pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(me.deps.log_flush);
            me.log_pending.store(false, Ordering::SeqCst);
            (me.deps.emit)(me.snap_value());
        });
    }

    pub fn fail(&self, error: FwpError) -> Res {
        let e = error.clone();
        self.patch(move |s| {
            s.state = "failed";
            s.progress = None;
            s.app_id = None;
            s.error = Some(e);
        });
        Err(error)
    }

    pub fn is_current(&self, live: &Arc<Live>) -> bool {
        self.live.lock().unwrap().as_ref().is_some_and(|l| Arc::ptr_eq(l, live))
    }

    pub fn kill_live(&self, live: &Live, sync: bool) {
        if !live.is_exited() {
            kill_pid(live.pid, sync);
        }
    }

    pub fn unwatch(&self, live: &Live) {
        let handles = live.st(|s| std::mem::take(&mut s.unwatch));
        for h in handles {
            h();
        }
    }

    pub fn kill_pub_get(&self, live: &Live, sync: bool) {
        if let Some(pid) = live.st(|s| s.pub_pid.take()) {
            kill_pid(pid, sync);
        }
    }

    pub fn show(self: &Arc<Self>, id: &str) -> Res {
        let id_owned = id.to_string();
        self.patch(move |s| {
            if s.error.as_ref().is_some_and(|e| e.code == "widget") {
                s.error = None;
            }
            s.current = Some(id_owned);
        });
        let live = self.live.lock().unwrap().clone();
        match live {
            Some(l) if l.st(|s| s.started && s.app_id.is_some() && !s.stop_requested) => self.send_show(&l, id),
            _ => Ok(()),
        }
    }

    pub fn stop(self: &Arc<Self>) {
        self.gen.fetch_add(1, Ordering::SeqCst);
        let live = self.live.lock().unwrap().clone();
        let Some(live) = live else {
            if self.snap.lock().unwrap().state == "starting" {
                self.patch(|s| {
                    s.state = "stopped";
                    s.progress = None;
                    s.error = None;
                });
            }
            return;
        };
        let first = live.st(|s| !std::mem::replace(&mut s.stop_requested, true));
        if first {
            self.unwatch(&live);
            self.patch(|s| {
                s.state = "stopped";
                s.progress = None;
                s.error = None;
            });
            self.kill_pub_get(&live, false);
            match live.st(|s| s.app_id.clone()) {
                Some(id) => {
                    let _ = live.machine.send("app.stop", json!({"appId": id}));
                }
                None => self.kill_live(&live, false),
            }
        }
        if !live.wait_exit(self.deps.stop_timeout) {
            self.kill_live(&live, false);
            live.wait_exit(Duration::from_secs(5));
        }
    }

    pub fn kill_now(self: &Arc<Self>) {
        self.gen.fetch_add(1, Ordering::SeqCst);
        let live = self.live.lock().unwrap().clone();
        let Some(live) = live else { return };
        live.st(|s| s.stop_requested = true);
        self.unwatch(&live);
        self.kill_pub_get(&live, true);
        self.kill_live(&live, true);
    }

    pub fn start(self: &Arc<Self>, root: &str, device_id: &str, flavor: Option<&str>) -> Res {
        if flavor.is_some_and(|f| !flavor_ok(f)) {
            return Err(FwpError::new("bad-arg"));
        }
        self.stop();
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        let cancelled = || gen != self.gen.load(Ordering::SeqCst);
        let (root_s, device_s) = (root.to_string(), device_id.to_string());
        self.patch(move |s| {
            let same = s.root.as_deref() == Some(root_s.as_str());
            s.log.clear();
            s.state = "starting";
            if !(same || s.root.is_none()) {
                s.current = None;
            }
            if !same {
                s.entries = Vec::new();
            }
            s.root = Some(root_s);
            s.device_id = Some(device_s);
            s.app_id = None;
            s.progress = None;
            s.error = None;
        });
        let d = &self.deps;
        let pubspec = std::fs::read_to_string(Path::new(root).join("pubspec.yaml")).ok();
        if cancelled() {
            return Err(FwpError::new("cancelled"));
        }
        let Some(pkg) = pubspec.as_deref().and_then(read_package).filter(|p| p.flutter) else { return self.fail(FwpError::new("not-flutter")) };
        let opts = env_opts(&(d.settings)(), root, &d.env, d.win, &d.home);
        let found = find_flutter_sdk(root, &opts);
        let Some(sdk) = found.sdk.clone() else {
            return self.fail(FwpError::new(found.issues.first().map_or("flutter-missing", |i| i.code.as_str())));
        };
        if !matches!(found.source, Some("setting" | "project")) && sdk_inside_project(&sdk, root, &opts, &FsEnvDeps) {
            return self.fail(FwpError::with("fvm-local-unverified", sdk));
        }
        let host = host_context(Path::new(root), pubspec.as_deref().unwrap_or(""));
        let entries = scan_project(Path::new(root));
        let written = write_host(Path::new(root), &host_source(&pkg.name, &entries, &host));
        if cancelled() {
            return Err(FwpError::new("cancelled"));
        }
        let host_file = match written {
            Ok((file, _)) => file.to_string_lossy().to_string(),
            Err(e) => return self.fail(FwpError::with("host-write", e.to_string())),
        };
        let e = entries.clone();
        self.patch(move |s| s.entries = e);
        let mut args: Vec<String> = ["run", "--machine", "-t", &host_file, "-d", device_id, "--debug"].iter().map(|s| s.to_string()).collect();
        if let Some(f) = flavor.filter(|f| !f.is_empty()) {
            args.extend(["--flavor".to_string(), f.to_string()]);
        }
        let wrappers: Vec<String> = [host.riverpod.as_ref().map(|_| "ProviderScope".to_string()), host.l10n.as_ref().map(|l| l.class_name.clone())].into_iter().flatten().collect();
        if !wrappers.is_empty() {
            self.log(&format!("> preview wrappers: {}", wrappers.join(", ")));
        }
        self.log(&format!("> flutter {}", args.join(" ")));
        let mut guard = self.live.lock().unwrap();
        if cancelled() {
            return Err(FwpError::new("cancelled"));
        }
        let Some(resolved) = flutter_spawn(&sdk, &args, d.win, &d.env) else { return self.fail(FwpError::new("bad-arg")) };
        let spec = SpawnSpec { file: resolved.file, args: resolved.args, cwd: root.to_string(), verbatim: resolved.verbatim, stdin_pipe: true, ..Default::default() };
        let mut child = match spawn(&spec, &d.env) {
            Ok(c) => c,
            Err(e) => return self.fail(FwpError::with("spawn", e.to_string())),
        };
        let (stdin, stdout, stderr) = (child.stdin.take(), child.stdout.take(), child.stderr.take());
        let machine = MachineClient::new(stdin.map(|s| Box::new(s) as Box<dyn std::io::Write + Send>));
        let live = Live::new(child.id(), machine.clone(), root.to_string(), pkg.name.clone(), host, sdk, device_id.to_string(), flavor.filter(|f| !f.is_empty()).map(String::from));
        *guard = Some(live.clone());
        drop(guard);
        let (me, lv) = (self.clone(), live.clone());
        let on_text = move |line: &str| me.text(&lv, line);
        let (me2, lv2, me3, lv3) = (self.clone(), live.clone(), self.clone(), live.clone());
        let handlers = MachineHandlers {
            on_event: Arc::new(move |event, params| me2.on_event(&lv2, event, params)),
            on_text: Arc::new(move |line| me3.text(&lv3, line)),
        };
        let out = stdout.map(|o| machine.attach(o, handlers));
        let err = stderr.map(|e| pump_lines(e, on_text));
        let (me, lv) = (self.clone(), live);
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            for h in [out, err].into_iter().flatten() {
                let _ = h.join();
            }
            me.close(&lv, code);
        });
        Ok(())
    }

    fn close(self: &Arc<Self>, live: &Arc<Live>, code: Option<i32>) {
        live.machine.dispose();
        self.unwatch(live);
        let current = {
            let mut g = self.live.lock().unwrap();
            if g.as_ref().is_some_and(|l| Arc::ptr_eq(l, live)) {
                *g = None;
                true
            } else {
                false
            }
        };
        if current {
            let (stop_requested, started, lost, build) = live.st(|s| (s.stop_requested, s.started, s.device_lost, s.build_error.clone()));
            let code_s = code.map_or("signal".to_string(), |c| c.to_string());
            if stop_requested {
                self.patch(|s| {
                    s.state = "stopped";
                    s.progress = None;
                    s.app_id = None;
                });
            } else if started {
                let error = if lost { FwpError::new("device-lost") } else { FwpError::with("exited", code_s) };
                self.patch(move |s| {
                    s.state = "stopped";
                    s.progress = None;
                    s.app_id = None;
                    s.error = Some(error);
                });
            } else {
                let _ = self.fail(match build {
                    Some(b) => FwpError { build: Some(Box::new(b)), ..FwpError::new("build") },
                    None if lost => FwpError::new("device-lost"),
                    None => FwpError::with("exit", code_s),
                });
            }
        }
        live.mark_exited();
    }
}
