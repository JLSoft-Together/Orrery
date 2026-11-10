use super::adb::{Adb, TIMING};
use super::emulators::{AvdDeps, AvdManager};
use super::history::{num, History};
use super::parse::*;
use super::qr::{new_qr_secret, qr_payload, run_qr_pairing, status_event, QrOptions, QrTiming};
use super::toggles::{launch_command, parse_density, parse_launch};
use super::ui_tester::{UiTester, UI_ORIGINALS_FILE};
use crate::apk::analyze::locale_cmp;
use crate::run::android::adb_watch::{AdbWatch, AdbWatchOpts, AdbWatchTiming};
use crate::run::android::deps::{EnvDeps, FsEnvDeps};
use crate::run::android::env::find_adb_tools;
use crate::run::proc::kill_pid_only;
use crate::run::settings::SettingsStore;
use crate::run::types::Env;
use crate::util::now_ms;
use crate::{Emitter, HostLink};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

const WIFI_PORT: u64 = 5555;
const BATCH_LIMIT: usize = 5;
const WIFI_SWITCH_MS: u64 = 1_000;
const STATE_ORDER: [&str; 9] = ["ready", "unauthorized", "authorizing", "no-permissions", "recovery", "sideload", "bootloader", "unknown", "offline"];

fn missing() -> Value {
    err("adb-missing")
}

pub fn map_limit<I: Sync, O: Send>(items: &[I], limit: usize, f: impl Fn(&I) -> O + Sync) -> Vec<O> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<O>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..limit.max(1).min(items.len()) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let v = f(&items[i]);
                out.lock().unwrap()[i] = Some(v);
            });
        }
    });
    out.into_inner().unwrap().into_iter().flatten().collect()
}

pub fn merge_devices(live: &[Row], history: &History) -> Vec<Value> {
    let mut out: Vec<(bool, usize, String, Value)> = Vec::new();
    for r in live {
        let h = history.get(&r.id);
        let alias = h.and_then(|h| h.alias.clone());
        let name = alias.clone().or_else(|| r.model.clone()).unwrap_or_else(|| r.id.clone());
        let v = json!({"id": r.id, "state": r.state, "transport": transport_of(r), "model": r.model, "product": r.product, "device": r.device, "transportId": r.transport_id, "alias": alias, "name": name, "online": true, "lastSeen": h.map(|h| num(h.last_seen))});
        out.push((true, STATE_ORDER.iter().position(|s| *s == r.state).unwrap_or(0), name, v));
    }
    for (id, h) in history.all() {
        if live.iter().any(|r| r.id == *id) {
            continue;
        }
        let name = h.alias.clone().or_else(|| h.model.clone()).unwrap_or_else(|| id.clone());
        let v = json!({"id": id, "state": "offline", "transport": if h.wifi { "wifi" } else { "usb" }, "model": h.model, "product": null, "device": null, "transportId": null, "alias": h.alias, "name": name, "online": false, "lastSeen": num(h.last_seen)});
        out.push((false, STATE_ORDER.len() - 1, name, v));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then_with(|| locale_cmp(&a.2, &b.2)));
    out.into_iter().map(|x| x.3).collect()
}

#[derive(Default)]
struct State {
    holds: HashSet<String>,
    watch: Option<(Arc<AdbWatch<Row>>, String)>,
    rows: Vec<Row>,
    auto_done: bool,
    qr: HashMap<String, (String, Arc<AtomicBool>)>,
}

pub struct DeviceManager {
    me: Weak<DeviceManager>,
    pub adb: Adb,
    emulator: Mutex<Option<String>>,
    settings: Arc<SettingsStore>,
    history: Mutex<History>,
    emitter: Emitter,
    host: HostLink,
    state: Mutex<State>,
    avds: AvdManager,
    ui: UiTester,
    win: bool,
}

impl DeviceManager {
    pub fn new(data_dir: &std::path::Path, env: Env, win: bool, home: String, settings: Arc<SettingsStore>, emitter: Emitter, host: HostLink) -> Arc<Self> {
        Arc::new_cyclic(|me: &Weak<DeviceManager>| {
            let adb = Adb { path: Arc::new(Mutex::new(None)), home: home.clone(), env: env.clone(), host: host.clone(), t: TIMING };
            let (a, b, c, d) = (me.clone(), me.clone(), me.clone(), me.clone());
            let avds = AvdManager::new(AvdDeps {
                env: env.clone(),
                win,
                home: home.clone(),
                tools: Arc::new(move || a.upgrade().map(|m| (m.resolve_adb(), m.emulator.lock().unwrap().clone())).unwrap_or((None, None))),
                live_rows: Arc::new(move || {
                    let m = b.upgrade()?;
                    let path = m.adb.path.lock().unwrap().clone()?;
                    m.live_rows(&path)
                }),
                read_text: Arc::new(move |f| c.upgrade().and_then(|_| FsEnvDeps.read_text(f))),
                confirm_wipe: Arc::new(move |name| d.upgrade().is_some_and(|m| matches!(m.host.call("devices.confirm", json!({"kind": "wipe", "name": name})), Ok(Value::Bool(true))))),
            });
            DeviceManager {
                me: me.clone(),
                ui: UiTester::new(adb.clone(), data_dir.join(UI_ORIGINALS_FILE)),
                adb,
                emulator: Mutex::new(None),
                settings,
                history: Mutex::new(History::load(data_dir.join(super::history::DEVICES_FILE))),
                emitter,
                host,
                state: Mutex::new(State::default()),
                avds,
                win,
            }
        })
    }

    pub fn resolve_adb(&self) -> Option<String> {
        let sdk = self.settings.get().android_sdk;
        let (adb, emu) = find_adb_tools(sdk.as_deref(), &self.adb.env, self.win, &self.adb.home);
        *self.emulator.lock().unwrap() = emu;
        *self.adb.path.lock().unwrap() = adb.clone();
        adb
    }

    pub fn avd_list(&self) -> Value {
        self.avds.list()
    }

    pub fn avd_start(&self, name: &str, mode: &str) -> Value {
        self.avds.start(name, mode)
    }

    pub fn avd_stop(&self, name: &str) -> Value {
        self.avds.stop(name)
    }

    pub fn notify(&self, event: &str, payload: Value) {
        self.emitter.emit(event, payload);
    }

    fn emit(&self) {
        let rows = self.state.lock().unwrap().rows.clone();
        let devices = merge_devices(&rows, &self.history.lock().unwrap());
        self.emitter.emit("ccm:dev-event", json!({"type": "devices", "devices": devices}));
    }

    fn drop_watch(&self) {
        let w = self.state.lock().unwrap().watch.take();
        if let Some((w, _)) = w {
            w.stop(false);
        }
    }

    fn make_watch(&self, adb: &str) -> Arc<AdbWatch<Row>> {
        let path = adb.to_string();
        Arc::new(AdbWatch::new(AdbWatchOpts { cwd: self.adb.home.clone(), env: self.adb.env.clone(), adb: Arc::new(move || Some(path.clone())), parse: Arc::new(|b: &str| parse_adb_device_rows(b)), timing: AdbWatchTiming::default(), kill: Some(kill_pid_only) }))
    }

    pub fn live_rows(&self, adb: &str) -> Option<Vec<Row>> {
        let watch = self.state.lock().unwrap().watch.as_ref().filter(|(_, p)| p == adb).map(|(w, _)| w.clone());
        let rows = match watch {
            Some(w) => Some(w.list()),
            None => self.adb.list(),
        };
        if let Some(r) = &rows {
            self.state.lock().unwrap().rows = r.clone();
        }
        rows
    }

    pub fn snapshot(&self) -> Value {
        let Some(adb) = self.resolve_adb() else {
            self.state.lock().unwrap().rows = Vec::new();
            let h = self.history.lock().unwrap();
            return json!({"adb": null, "devices": merge_devices(&[], &h), "autoConnect": h.auto_connect()});
        };
        let rows = self.live_rows(&adb).unwrap_or_default();
        let mut h = self.history.lock().unwrap();
        h.seen(&rows, now_ms());
        json!({"adb": adb, "devices": merge_devices(&rows, &h), "autoConnect": h.auto_connect()})
    }

    pub fn hold(&self, owner: &str, on: bool) {
        if !on {
            let empty = {
                let mut st = self.state.lock().unwrap();
                st.holds.remove(owner);
                st.holds.is_empty()
            };
            if empty {
                self.drop_watch();
            }
            return;
        }
        self.state.lock().unwrap().holds.insert(owner.to_string());
        let adb = self.resolve_adb();
        let stale = {
            let st = self.state.lock().unwrap();
            if st.holds.is_empty() {
                return;
            }
            st.watch.as_ref().is_some_and(|(_, p)| Some(p) != adb.as_ref())
        };
        if stale {
            self.drop_watch();
        }
        let Some(adb) = adb else { return };
        if self.state.lock().unwrap().watch.is_some() {
            return;
        }
        let watch = self.make_watch(&adb);
        self.state.lock().unwrap().watch = Some((watch.clone(), adb));
        let me = self.me.clone();
        watch.on_change(Arc::new(move |rows| {
            let Some(m) = me.upgrade() else { return };
            m.state.lock().unwrap().rows = rows.clone();
            m.history.lock().unwrap().seen(&rows, now_ms());
            m.emit();
        }));
    }

    pub fn release_owner(&self, prefix: &str) {
        let (empty, aborts) = {
            let mut st = self.state.lock().unwrap();
            st.holds.retain(|o| !o.starts_with(prefix));
            let aborts: Vec<Arc<AtomicBool>> = st.qr.values().filter(|(o, _)| o.starts_with(prefix)).map(|(_, a)| a.clone()).collect();
            (st.holds.is_empty(), aborts)
        };
        for a in aborts {
            a.store(true, Ordering::SeqCst);
        }
        if empty {
            self.drop_watch();
        }
    }

    fn last_port_for(&self, host: &str) -> Option<u64> {
        let key = address_key(host, 0);
        let h = self.history.lock().unwrap();
        let mut best: Option<(u64, f64)> = None;
        for (id, e) in h.all() {
            if !e.wifi {
                continue;
            }
            let Some(a) = parse_address(id) else { continue };
            let (Some(ah), Some(ap)) = (a.host.as_deref(), a.port) else { continue };
            if ap == WIFI_PORT || address_key(ah, 0) != key {
                continue;
            }
            if best.is_none_or(|b| e.last_seen > b.1) {
                best = Some((ap, e.last_seen));
            }
        }
        best.map(|b| b.0)
    }

    pub fn qr_start(&self, owner: &str) -> Value {
        if self.resolve_adb().is_none() {
            return missing();
        }
        if !self.adb.mdns_check() {
            return err("mdns-unavailable");
        }
        let (name, password) = new_qr_secret();
        let abort = Arc::new(AtomicBool::new(false));
        self.state.lock().unwrap().qr.insert(name.clone(), (owner.to_string(), abort.clone()));
        let payload = qr_payload(&name, &password);
        let (me, me2, emitter, id) = (self.me.clone(), self.me.clone(), self.emitter.clone(), name.clone());
        let opts = QrOptions {
            adb: self.adb.clone(),
            name: name.clone(),
            password,
            abort,
            on_status: Arc::new(move |status, detail| emitter.emit("ccm:dev-event", status_event(&id, status, detail))),
            last_port_for: Arc::new(move |h| me.upgrade().and_then(|m| m.last_port_for(h))),
            timing: QrTiming { pair_wait_ms: 60_000, connect_wait_ms: 30_000, poll_ms: 1_000 },
        };
        let done = name.clone();
        std::thread::spawn(move || {
            run_qr_pairing(opts);
            if let Some(m) = me2.upgrade() {
                m.state.lock().unwrap().qr.remove(&done);
            }
        });
        json!({"id": name, "payload": payload})
    }

    pub fn qr_cancel(&self, id: &str) -> Value {
        match self.state.lock().unwrap().qr.get(id) {
            Some((_, a)) => {
                a.store(true, Ordering::SeqCst);
                ok()
            }
            None => err("invalid"),
        }
    }

    pub fn connect(&self, address: &str) -> Value {
        let Some(a) = parse_address(address) else { return err("invalid") };
        if self.resolve_adb().is_none() {
            return missing();
        }
        self.adb.connect(&a.target)
    }

    pub fn pair(&self, address: &str, code: &str) -> Value {
        let a = parse_address(address);
        let (Some(host), Some(port)) = (a.as_ref().and_then(|a| a.host.clone()), a.as_ref().and_then(|a| a.port)) else { return err("invalid") };
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return err("invalid");
        }
        if self.resolve_adb().is_none() {
            return missing();
        }
        let r = self.adb.pair(&format!("{host}:{port}"), code);
        if !is_ok(&r) {
            return r;
        }
        let key = address_key(&host, 0);
        match self.adb.mdns_services().into_iter().find(|s| s.kind == "connect" && address_key(&s.host, 0) == key) {
            Some(s) => self.connect(&format!("{}:{}", s.host, s.port)),
            None => json!({"ok": true, "detail": "paired"}),
        }
    }

    pub fn disconnect(&self, ids: &[String]) -> Vec<Value> {
        let Some(adb) = self.resolve_adb() else { return ids.iter().map(|id| json!({"id": id, "result": missing()})).collect() };
        let live = self.live_rows(&adb);
        let online: HashSet<String> = live.unwrap_or_default().into_iter().map(|r| r.id).collect();
        map_limit(ids, BATCH_LIMIT, |id| json!({"id": id, "result": if online.contains(id) && is_wifi_id(id) { self.adb.disconnect(id) } else { err("not-connected") }}))
    }

    pub fn connect_many(&self, ids: &[String]) -> Vec<Value> {
        map_limit(ids, BATCH_LIMIT, |id| json!({"id": id, "result": self.connect(id)}))
    }

    pub fn to_wifi(&self, id: &str) -> Value {
        if self.resolve_adb().is_none() {
            return missing();
        }
        let Some(ip) = self.adb.wlan_ip(id) else { return err("no-ip") };
        let r = self.adb.tcpip(id, WIFI_PORT as u16);
        if !is_ok(&r) {
            return r;
        }
        std::thread::sleep(Duration::from_millis(WIFI_SWITCH_MS));
        self.connect(&format!("{ip}:{WIFI_PORT}"))
    }

    pub fn remove(&self, ids: &[String]) -> Value {
        let Some(adb) = self.resolve_adb() else { return missing() };
        let Some(live) = self.live_rows(&adb) else { return missing() };
        let targets: Vec<String> = {
            let h = self.history.lock().unwrap();
            ids.iter().filter(|id| !live.iter().any(|r| &r.id == *id) && h.get(id).is_some()).cloned().collect()
        };
        if targets.is_empty() {
            return ok();
        }
        if !matches!(self.host.call("devices.confirm", json!({"kind": "remove", "ids": targets})), Ok(Value::Bool(true))) {
            return err("cancelled");
        }
        self.history.lock().unwrap().remove(&targets);
        self.emit();
        ok()
    }

    pub fn ready_check(&self, id: &str) -> Value {
        let Some(adb) = self.resolve_adb() else { return missing() };
        match self.live_rows(&adb) {
            Some(rows) if rows.iter().any(|r| r.id == id && r.state == "ready") => ok(),
            _ => err("not-connected"),
        }
    }

    pub fn screencap(&self, id: &str) -> Value {
        let r = self.ready_check(id);
        if is_ok(&r) {
            self.adb.screencap(id)
        } else {
            r
        }
    }

    pub fn ui_dump(&self, id: &str) -> Value {
        let r = self.ready_check(id);
        if !is_ok(&r) {
            return r;
        }
        let mut dump = self.adb.ui_dump(id);
        if !is_ok(&dump) {
            return dump;
        }
        let wm = self.adb.shell(id, &["wm".to_string(), "density".to_string()], None);
        let (physical, over) = match wm {
            Some(w) if w.code == Some(0) => parse_density(&w.out),
            _ => (None, None),
        };
        dump["density"] = over.or(physical).map(|d| json!(d)).unwrap_or(Value::Null);
        dump
    }

    pub fn ui_read(&self, id: &str, pkg: Option<&str>) -> Value {
        let r = self.ready_check(id);
        if is_ok(&r) {
            self.ui.read(id, pkg)
        } else {
            r
        }
    }

    pub fn ui_apply(&self, id: &str, change: &Value, pkg: Option<&str>) -> Value {
        let r = self.ready_check(id);
        if is_ok(&r) {
            self.ui.apply(id, change, pkg)
        } else {
            r
        }
    }

    pub fn ui_restore(&self, id: &str, pkg: Option<&str>) -> Value {
        let r = self.ready_check(id);
        if is_ok(&r) {
            self.ui.restore(id, pkg)
        } else {
            r
        }
    }

    pub fn launch(&self, id: &str, target: &Value) -> Value {
        let r = self.ready_check(id);
        if !is_ok(&r) {
            return r;
        }
        let Some(out) = self.adb.shell(id, &launch_command(target), Some(TIMING.connect_ms)) else { return missing() };
        if out.timed_out {
            return err("timeout");
        }
        let (success, detail) = parse_launch(&out.out);
        if success {
            return detail.map(|d| json!({"ok": true, "detail": d})).unwrap_or_else(ok);
        }
        json!({"ok": false, "error": "launch-failed", "detail": detail.unwrap_or_default()})
    }

    pub fn set_alias(&self, id: &str, alias: Option<&str>) -> Value {
        let changed = {
            let mut h = self.history.lock().unwrap();
            let Some(before) = h.get(id).map(|e| e.alias.clone()) else { return err("invalid") };
            h.set_alias(id, alias);
            h.get(id).map(|e| e.alias.clone()) != Some(before)
        };
        if changed {
            self.emit();
        }
        ok()
    }

    pub fn set_auto_connect(&self, on: bool) -> Value {
        let changed = {
            let mut h = self.history.lock().unwrap();
            let changed = h.auto_connect() != on;
            if changed {
                h.set_auto_connect(on);
            }
            changed
        };
        if changed {
            self.emit();
        }
        ok()
    }

    pub fn discover(&self) -> Value {
        let Some(adb) = self.resolve_adb() else { return missing() };
        if !self.adb.mdns_check() {
            return err("mdns-unavailable");
        }
        let online: HashSet<String> = self.live_rows(&adb).unwrap_or_default().into_iter().map(|r| r.id).collect();
        let mut targets: Vec<String> = Vec::new();
        for s in self.adb.mdns_services() {
            if s.kind == "pairing" || online.contains(&address_key(&s.host, s.port as u64)) || online.contains(&format!("{}._adb-tls-connect._tcp", s.name)) {
                continue;
            }
            let t = format!("{}:{}", s.host, s.port);
            if !targets.contains(&t) {
                targets.push(t);
            }
        }
        if targets.is_empty() {
            err("no-devices")
        } else {
            Value::Array(self.connect_many(&targets))
        }
    }

    pub fn auto_connect_once(&self) {
        {
            let mut st = self.state.lock().unwrap();
            if st.auto_done {
                return;
            }
            st.auto_done = true;
        }
        let (on, ids) = {
            let h = self.history.lock().unwrap();
            (h.auto_connect(), h.wifi_ids())
        };
        if !on || ids.is_empty() {
            return;
        }
        map_limit(&ids, BATCH_LIMIT, |id| self.connect(id));
    }

    pub fn stop(&self) {
        let aborts: Vec<Arc<AtomicBool>> = {
            let mut st = self.state.lock().unwrap();
            st.holds.clear();
            st.qr.values().map(|(_, a)| a.clone()).collect()
        };
        for a in aborts {
            a.store(true, Ordering::SeqCst);
        }
        self.drop_watch();
    }
}
