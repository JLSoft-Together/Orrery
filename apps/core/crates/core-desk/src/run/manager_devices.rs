use super::manager::Inner;
use super::types::{DeviceService, RunDevice, RunEmulator, RunResult};
use super::{android, flutter};
use core_transcript::collate::locale_cmp;
use serde_json::{json, Value};
use std::sync::Arc;

impl Inner {
    fn ensure_devices(self: &Arc<Self>) -> (Arc<dyn DeviceService>, Arc<dyn DeviceService>) {
        let settings = self.get_settings();
        let mut created: Vec<(&'static str, Arc<dyn DeviceService>)> = Vec::new();
        let (f, a) = {
            let mut g = self.state.lock().unwrap();
            if g.flutter_devices.is_none() {
                let svc = flutter::devices::create(flutter::devices::DevicesOpts { settings: settings.clone(), env: self.env.clone(), win: self.win, home: self.home.clone() });
                g.flutter_devices = Some(svc.clone());
                created.push(("flutter", svc));
            }
            if g.adb_devices.is_none() {
                let svc = android::devices::create(android::devices::DevicesOpts { settings, env: self.env.clone(), win: self.win, home: self.home.clone() });
                g.adb_devices = Some(svc.clone());
                created.push(("adb", svc));
            }
            (g.flutter_devices.clone().unwrap(), g.adb_devices.clone().unwrap())
        };
        for (source, svc) in created {
            let weak = Arc::downgrade(self);
            svc.on_change(Arc::new(move |d| {
                if let Some(m) = weak.upgrade() {
                    m.devices_changed(source, d);
                }
            }));
        }
        (f, a)
    }

    pub fn restart_devices(self: &Arc<Self>, sync: bool) {
        let (f, a) = {
            let mut g = self.state.lock().unwrap();
            (g.flutter_devices.take(), g.adb_devices.take())
        };
        if let Some(f) = f {
            f.stop(sync);
        }
        if let Some(a) = a {
            a.stop(sync);
        }
    }

    fn merged(&self) -> Vec<RunDevice> {
        let g = self.state.lock().unwrap();
        let mut by_id: Vec<RunDevice> = Vec::new();
        for d in &g.last_adb {
            match by_id.iter().position(|x| x.id == d.id) {
                Some(i) => by_id[i] = d.clone(),
                None => by_id.push(d.clone()),
            }
        }
        for d in &g.last_flutter {
            let next = match by_id.iter().find(|x| x.id == d.id) {
                Some(adb) => RunDevice { state: adb.state.clone(), ..d.clone() },
                None => d.clone(),
            };
            match by_id.iter().position(|x| x.id == d.id) {
                Some(i) => by_id[i] = next,
                None => by_id.push(next),
            }
        }
        by_id.sort_by(|a, b| locale_cmp(&a.name, &b.name));
        by_id
    }

    pub fn merged_devices(&self) -> Vec<RunDevice> {
        self.merged()
    }

    fn devices_changed(self: &Arc<Self>, source: &str, devices: Vec<RunDevice>) {
        {
            let mut g = self.state.lock().unwrap();
            if source == "flutter" {
                g.last_flutter = devices;
            } else {
                g.last_adb = devices;
            }
        }
        let list: Vec<Value> = self.merged().iter().map(RunDevice::to_value).collect();
        self.broadcast(json!({"type": "devices", "devices": list}));
    }

    pub fn devices(self: &Arc<Self>) -> Vec<RunDevice> {
        let (f, a) = self.ensure_devices();
        let t = std::thread::spawn(move || a.list());
        let flutter_list = f.list();
        let adb_list = t.join().unwrap_or_default();
        {
            let mut g = self.state.lock().unwrap();
            g.last_flutter = flutter_list;
            g.last_adb = adb_list;
        }
        self.merged()
    }

    pub fn emulators(self: &Arc<Self>) -> Vec<RunEmulator> {
        let (f, a) = self.ensure_devices();
        let t = std::thread::spawn(move || a.emulators());
        let flutter_list = f.emulators();
        let adb_list = t.join().unwrap_or_default();
        let seen: Vec<String> = flutter_list.iter().map(|e| e.id.clone()).collect();
        let mut out = flutter_list;
        out.extend(adb_list.into_iter().filter(|e| !seen.contains(&e.id)));
        out
    }

    pub fn launch_emulator(self: &Arc<Self>, id: &str, source: &str) -> RunResult {
        if id.starts_with('-') || !self.emulators().iter().any(|e| e.id == id) {
            return RunResult::err("emulator-missing");
        }
        let (f, a) = self.ensure_devices();
        if source == "flutter" {
            f.launch_emulator(id)
        } else {
            a.launch_emulator(id)
        }
    }

    pub fn release_devices_if_idle(self: &Arc<Self>) {
        let idle = {
            let g = self.state.lock().unwrap();
            g.holds.is_empty() && !g.sessions.iter().any(|(_, l)| super::sessions::is_live(l.session["state"].as_str().unwrap_or(""))) && (g.flutter_devices.is_some() || g.adb_devices.is_some())
        };
        if idle {
            self.restart_devices(false);
        }
    }
}
