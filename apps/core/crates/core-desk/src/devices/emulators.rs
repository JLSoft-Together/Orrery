use super::avd::*;
use super::parse::*;
use crate::run::collect::{collect, CollectOpts};
use crate::run::proc::{kill_pid_only, spawn, SpawnSpec};
use crate::run::types::Env;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type ReadText = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub struct AvdDeps {
    pub env: Env,
    pub win: bool,
    pub home: String,
    pub tools: Arc<dyn Fn() -> (Option<String>, Option<String>) + Send + Sync>,
    pub live_rows: Arc<dyn Fn() -> Option<Vec<Row>> + Send + Sync>,
    pub read_text: ReadText,
    pub confirm_wipe: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

const LIST_MS: u64 = 15_000;
const CMD_MS: u64 = 5_000;
const START_GRACE_MS: u64 = 3_000;

pub struct AvdManager {
    d: AvdDeps,
    starting: Mutex<HashSet<String>>,
}

impl AvdManager {
    pub fn new(d: AvdDeps) -> Self {
        AvdManager { d, starting: Mutex::new(HashSet::new()) }
    }

    fn names(&self, emulator: &str) -> Option<Vec<String>> {
        let r = collect(emulator, &["-list-avds".to_string()], &self.d.home, &self.d.env, &CollectOpts { timeout_ms: Some(LIST_MS), ..Default::default() });
        (r.code == Some(0)).then(|| parse_list_avds(&r.out))
    }

    pub fn running(&self, adb: Option<&str>) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let Some(adb) = adb else { return out };
        let rows: Vec<Row> = (self.d.live_rows)().unwrap_or_default().into_iter().filter(|r| is_emulator_serial(&r.id)).collect();
        let found: Vec<Option<(String, String)>> = std::thread::scope(|s| {
            let handles: Vec<_> = rows
                .iter()
                .map(|r| {
                    s.spawn(move || {
                        let args: Vec<String> = ["-s", &r.id, "emu", "avd", "name"].iter().map(|a| a.to_string()).collect();
                        let res = collect(adb, &args, &self.d.home, &self.d.env, &CollectOpts { timeout_ms: Some(CMD_MS), kill: Some(kill_pid_only), ..Default::default() });
                        if res.code == Some(0) { parse_emu_avd_name(&res.out).map(|n| (n, r.id.clone())) } else { None }
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()
        });
        for (n, id) in found.into_iter().flatten() {
            out.insert(n, id);
        }
        out
    }

    fn read_ini(&self, file: &str) -> Option<Ini> {
        (self.d.read_text)(file).map(|t| parse_ini(&t))
    }

    pub fn list(&self) -> Value {
        let (adb, emulator) = (self.d.tools)();
        let Some(emulator) = emulator else { return json!({"ok": false, "error": "emulator-missing"}) };
        let Some(names) = self.names(&emulator) else { return json!({"ok": false, "error": "emulator-failed"}) };
        let running = self.running(adb.as_deref());
        let homes = avd_home(&self.d.env, self.d.win, &self.d.home);
        let mut avds: Vec<Value> = names
            .iter()
            .map(|name| {
                let ini = self.read_ini(&crate::run::android::pathx::join(self.d.win, &[&homes.avd, &format!("{name}.ini")]));
                let dir = avd_dir(name, ini.as_ref(), &homes, self.d.win);
                let config = self.read_ini(&crate::run::android::pathx::join(self.d.win, &[&dir, "config.ini"]));
                avd_info(name, ini.as_ref(), config.as_ref(), running.get(name).map(String::as_str))
            })
            .collect();
        avds.sort_by(|a, b| crate::apk::analyze::locale_cmp(a["displayName"].as_str().unwrap_or(""), b["displayName"].as_str().unwrap_or("")));
        json!({"ok": true, "avds": avds})
    }

    pub fn start(&self, name: &str, mode: &str) -> Value {
        let (adb, emulator) = (self.d.tools)();
        let Some(emulator) = emulator else { return json!({"ok": false, "error": "emulator-missing"}) };
        if !self.names(&emulator).is_some_and(|n| n.iter().any(|x| x == name)) {
            return err("invalid");
        }
        if self.starting.lock().unwrap().contains(name) || self.running(adb.as_deref()).contains_key(name) {
            return err("avd-running");
        }
        if mode == "wipe" && !(self.d.confirm_wipe)(name) {
            return err("cancelled");
        }
        self.starting.lock().unwrap().insert(name.to_string());
        let result = (|| {
            let spec = SpawnSpec { file: emulator.clone(), args: emulator_args(name, mode), cwd: self.d.home.clone(), env: Env::new(), detached: true, ..Default::default() };
            let Ok(mut child) = spawn(&spec, &self.d.env) else { return err("emulator-failed") };
            let (tx, rx) = mpsc::channel::<Option<i32>>();
            std::thread::spawn(move || {
                let _ = tx.send(child.wait().ok().and_then(|s| s.code()));
            });
            match rx.recv_timeout(Duration::from_millis(START_GRACE_MS)) {
                Err(_) => ok(),
                Ok(code) => match code {
                    Some(c) => json!({"ok": false, "error": "emulator-failed", "detail": format!("exit {c}")}),
                    None => err("emulator-failed"),
                },
            }
        })();
        self.starting.lock().unwrap().remove(name);
        result
    }

    pub fn stop(&self, name: &str) -> Value {
        let (adb, _) = (self.d.tools)();
        let Some(adb) = adb else { return err("adb-missing") };
        let Some(serial) = self.running(Some(&adb)).get(name).cloned() else { return err("avd-not-running") };
        let args: Vec<String> = ["-s", &serial, "emu", "kill"].iter().map(|a| a.to_string()).collect();
        let r = collect(&adb, &args, &self.d.home, &self.d.env, &CollectOpts { timeout_ms: Some(CMD_MS), kill: Some(kill_pid_only), ..Default::default() });
        if r.timed_out {
            return err("timeout");
        }
        if r.code == Some(0) {
            return ok();
        }
        let detail = crate::js::slice(r.out.trim(), 300);
        err_detail("emulator-failed", &detail)
    }
}
