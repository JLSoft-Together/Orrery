use super::command::{preview_command, PreviewCtx};
use super::config_store::{RunConfigStore, TrustStore};
use super::configs::{clean_run_config, clean_templates, default_configs};
use super::settings::{read_json, write_json, SettingsStore, TEMPLATES_FILE};
use super::types::{DeviceService, Env, RunDevice, RunResult, RunSettings};
use super::watch::{watch_dirs, ProjectWatchers};
use super::{android, flutter, toolchain};
use crate::preview::paths::is_within;
use crate::{Emitter, HostLink};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const DETECT_TTL_MS: f64 = 5_000.0;
pub const IMPORTED_FILE: &str = "run-imported.json";
pub const STATE_EVENT: &str = "desk:run-state";
pub const SETTINGS_EVENT: &str = "desk:run-settings";

pub type PubSlot = Arc<(Mutex<Option<RunResult>>, std::sync::Condvar)>;

pub struct Live {
    pub config: Value,
    pub device: Value,
    pub session: Value,
    pub runner: Arc<dyn super::types::Runner>,
    pub key: String,
}

#[derive(Default)]
pub struct State {
    pub detected: HashMap<String, Value>,
    pub holds: HashMap<String, HashSet<String>>,
    pub sessions: Vec<(String, Live)>,
    pub pending: Vec<(String, Vec<Value>)>,
    pub dirty: HashSet<String>,
    pub starting: HashSet<String>,
    pub pub_gets: HashMap<String, PubSlot>,
    pub pub_pids: HashSet<u32>,
    pub flush_scheduled: bool,
    pub flutter_devices: Option<Arc<dyn DeviceService>>,
    pub adb_devices: Option<Arc<dyn DeviceService>>,
    pub last_flutter: Vec<RunDevice>,
    pub last_adb: Vec<RunDevice>,
}

pub struct Inner {
    pub data_dir: PathBuf,
    pub env: Env,
    pub win: bool,
    pub home: String,
    pub settings: Arc<SettingsStore>,
    pub store: RunConfigStore,
    pub trust: TrustStore,
    pub templates: Mutex<Value>,
    pub emitter: Emitter,
    pub host: HostLink,
    pub state: Mutex<State>,
    pub watchers: ProjectWatchers,
    pub mask: Arc<dyn Fn(&str) -> String + Send + Sync>,
    pub restart_stop_ms: u64,
    pub pub_get_timeout_ms: u64,
}

pub type RunManager = Arc<Inner>;

pub fn new_manager(data_dir: PathBuf, env: Env, win: bool, home: String, settings: Arc<SettingsStore>, emitter: Emitter, host: HostLink) -> RunManager {
    let platform = if win { "win32" } else { "darwin" };
    let mask_home = home.clone();
    let templates = clean_templates(read_json(&data_dir.join(TEMPLATES_FILE)).as_ref());
    Arc::new_cyclic(|weak: &std::sync::Weak<Inner>| {
        let w = weak.clone();
        Inner {
            store: RunConfigStore::new(&data_dir),
            trust: TrustStore::new(&data_dir),
            data_dir,
            env,
            win,
            home,
            settings,
            templates: Mutex::new(templates),
            emitter,
            host,
            state: Mutex::new(State::default()),
            watchers: ProjectWatchers::new(Arc::new(move |root| {
                if let Some(m) = w.upgrade() {
                    m.project_changed(&root);
                }
            })),
            mask: Arc::new(move |t| core_sources::mask::mask_secrets(t, &mask_home, platform)),
            restart_stop_ms: 8_000,
            pub_get_timeout_ms: 5 * 60_000,
        }
    })
}

impl Inner {
    pub fn broadcast(&self, event: Value) {
        self.emitter.emit("ccm:run-event", event);
    }

    pub fn settings_changed(&self, s: &RunSettings) {
        self.emitter.emit(SETTINGS_EVENT, s.to_value());
    }

    pub fn get_settings(&self) -> RunSettings {
        self.settings.get()
    }

    pub fn set_settings(self: &Arc<Self>, next: &Value) -> RunSettings {
        let s = self.settings.set(next);
        self.settings_changed(&s);
        android::jdk_pick::clear_jdk_cache();
        self.restart_devices(false);
        s
    }

    pub fn get_templates(&self) -> Value {
        self.templates.lock().unwrap().clone()
    }

    pub fn set_templates(self: &Arc<Self>, raw: &Value) -> Value {
        let t = clean_templates(Some(raw));
        *self.templates.lock().unwrap() = t.clone();
        let _ = write_json(&self.data_dir.join(TEMPLATES_FILE), &t);
        let roots: Vec<String> = self.state.lock().unwrap().holds.keys().cloned().collect();
        for root in roots {
            let configs = self.configs(&root);
            self.broadcast(json!({"type": "configs", "root": root, "configs": configs}));
        }
        t
    }

    pub fn detect(self: &Arc<Self>, root: &str, fresh: bool) -> Value {
        let cached = self.state.lock().unwrap().detected.get(root).cloned();
        if let Some(c) = &cached {
            if !fresh && crate::util::now_ms() - c["at"].as_f64().unwrap_or(0.0) < DETECT_TTL_MS {
                return c.clone();
            }
        }
        let (r1, r2) = (root.to_string(), root.to_string());
        let fl = std::thread::spawn(move || flutter::detect::detect_flutter(&r1));
        let and = std::thread::spawn(move || android::detect::detect_android(&r2));
        let mut projects = fl.join().unwrap_or_default();
        projects.extend(and.join().unwrap_or_default());
        let result = json!({"root": root, "projects": projects, "at": crate::util::now_ms() as i64});
        let changed = cached.as_ref().map(|c| c["projects"] != result["projects"]).unwrap_or(true);
        self.state.lock().unwrap().detected.insert(root.to_string(), result.clone());
        if cached.is_some() && changed {
            self.broadcast(json!({"type": "projects", "result": result}));
        }
        result
    }

    pub fn repo_root_of(&self, dir: &str) -> Option<String> {
        let g = self.state.lock().unwrap();
        g.detected.keys().filter(|r| is_within(r, dir)).max_by_key(|r| r.len()).cloned()
    }

    pub fn env_of(self: &Arc<Self>, dir: &str, kind: &str) -> Value {
        let repo_root = self.repo_root_of(dir);
        let settings = self.get_settings();
        if kind == "flutter" {
            let mut opts = flutter::env::env_opts(&settings, dir, &self.env, self.win, &self.home);
            opts.repo_root = repo_root;
            return flutter::env::flutter_env(dir, &opts);
        }
        let opts = android::env::AndroidEnvOpts { sdk_setting: settings.android_sdk, jdk_setting: settings.jdk, env: self.env.clone(), win: self.win, home: self.home.clone(), trusted: false, repo_root };
        android::env::android_env(dir, &opts)
    }

    pub fn configs(self: &Arc<Self>, root: &str) -> Vec<Value> {
        let stored = self.store.list(root);
        let ids: HashSet<String> = stored.iter().filter_map(|c| c["id"].as_str().map(String::from)).collect();
        let projects: Vec<Value> = self.detect(root, false)["projects"].as_array().cloned().unwrap_or_default();
        let templates = self.get_templates();
        let mut out = stored;
        out.extend(default_configs(&projects, root, &templates).into_iter().filter(|c| !ids.contains(c["id"].as_str().unwrap_or(""))));
        out
    }

    pub fn save_config(self: &Arc<Self>, root: &str, raw: &Value) -> Option<Value> {
        let saved = self.store.save(root, raw);
        if saved.is_some() {
            self.broadcast(json!({"type": "configs", "root": root, "configs": self.configs(root)}));
        }
        saved
    }

    pub fn delete_config(self: &Arc<Self>, root: &str, id: &str) -> bool {
        let ok = self.store.delete(root, id);
        if ok {
            self.broadcast(json!({"type": "configs", "root": root, "configs": self.configs(root)}));
        }
        ok
    }

    pub fn import_configs(self: &Arc<Self>, root: &str) -> Vec<Value> {
        let have: HashSet<String> = self.store.list(root).iter().filter_map(|c| c["id"].as_str().map(String::from)).collect();
        let mut found = super::importers::import_intellij(root);
        found.extend(super::importers::import_vscode(root));
        let saved: Vec<Value> = found.into_iter().filter(|c| !have.contains(c["id"].as_str().unwrap_or(""))).filter_map(|c| self.store.save(root, &c)).collect();
        if !saved.is_empty() {
            self.broadcast(json!({"type": "configs", "root": root, "configs": self.configs(root)}));
        }
        saved
    }

    pub fn auto_import(self: &Arc<Self>, root: &str) -> Vec<Value> {
        let key = self.store.personal_file(root).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let file = self.data_dir.join(IMPORTED_FILE);
        let mut done: Vec<String> = read_json(&file).and_then(|v| v.as_array().cloned()).map(|a| a.iter().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default();
        if done.contains(&key) {
            return Vec::new();
        }
        done.push(key);
        let skip = done.len().saturating_sub(2000);
        let _ = write_json(&file, &json!(done.into_iter().skip(skip).collect::<Vec<_>>()));
        self.import_configs(root)
    }

    fn project_at(&self, root: &str, cwd: &str) -> Option<Value> {
        let g = self.state.lock().unwrap();
        g.detected.get(root)?["projects"].as_array()?.iter().find(|p| crate::preview::paths::relative(p["dir"].as_str().unwrap_or(""), cwd).is_empty()).cloned()
    }

    pub fn preview(self: &Arc<Self>, root: &str, raw: &Value, device_id: Option<&str>) -> Option<Value> {
        let cfg = clean_run_config(raw, root)?;
        self.detect(root, false);
        let settings = self.get_settings();
        let cwd = cfg["cwd"].as_str().unwrap_or("").to_string();
        let executable = match cfg["type"].as_str() {
            Some("flutter") => {
                let opts = flutter::env::env_opts(&settings, &cwd, &self.env, self.win, &self.home);
                match flutter::env::find_flutter_sdk(&cwd, &opts).sdk {
                    Some(sdk) => flutter::env::flutter_executable(&sdk, self.win),
                    None => "flutter".to_string(),
                }
            }
            Some("android") => android::env::gradlew_file(&cwd, self.win),
            _ => String::new(),
        };
        let project = self.project_at(root, &cwd);
        Some(preview_command(&cfg, &PreviewCtx { executable, device_id: device_id.map(String::from), workspace_dir: root.to_string(), env: &self.env, project: project.as_ref() }))
    }

    pub fn toolchain(self: &Arc<Self>, dir: &str, kind: &str) -> Value {
        let settings = self.get_settings();
        let mask = self.mask.clone();
        let m: &dyn Fn(&str) -> String = &*mask;
        if kind == "flutter" {
            let opts = flutter::env::env_opts(&settings, dir, &self.env, self.win, &self.home);
            let sdk = flutter::env::find_flutter_sdk(dir, &opts).sdk;
            return toolchain::flutter_toolchain(sdk.as_deref(), self.win, &self.env, &toolchain::default_exec, m);
        }
        let opts = android::env::AndroidEnvOpts { sdk_setting: settings.android_sdk, jdk_setting: settings.jdk, env: self.env.clone(), win: self.win, home: self.home.clone(), trusted: true, repo_root: None };
        let env = android::env::android_env(dir, &opts);
        toolchain::android_toolchain(&env, self.win, &toolchain::default_exec, m)
    }

    pub fn hold(self: &Arc<Self>, root: &str, owner: &str, on: bool) {
        if on {
            {
                let mut g = self.state.lock().unwrap();
                if let Some(set) = g.holds.get_mut(root) {
                    set.insert(owner.to_string());
                    return;
                }
                g.holds.insert(root.to_string(), HashSet::from([owner.to_string()]));
            }
            let result = self.detect(root, false);
            if self.state.lock().unwrap().holds.contains_key(root) {
                self.watchers.sync(root, &watch_dirs(&result));
            }
            return;
        }
        {
            let mut g = self.state.lock().unwrap();
            let Some(set) = g.holds.get_mut(root) else { return };
            if !set.remove(owner) || !set.is_empty() {
                return;
            }
            g.holds.remove(root);
        }
        self.watchers.close(root);
        self.release_devices_if_idle();
    }

    pub fn release_owner(self: &Arc<Self>, prefix: &str) {
        let mut dropped = Vec::new();
        {
            let mut g = self.state.lock().unwrap();
            let roots: Vec<String> = g.holds.keys().cloned().collect();
            for root in roots {
                let set = g.holds.get_mut(&root).unwrap();
                set.retain(|o| !(o == prefix || o.starts_with(&format!("{prefix}:"))));
                if set.is_empty() {
                    g.holds.remove(&root);
                    dropped.push(root);
                }
            }
        }
        for r in dropped {
            self.watchers.close(&r);
        }
        self.release_devices_if_idle();
    }

    pub fn project_changed(self: &Arc<Self>, root: &str) {
        if !self.state.lock().unwrap().holds.contains_key(root) {
            return;
        }
        let before = self.state.lock().unwrap().detected.get(root).map(|d| d["projects"].clone());
        let result = self.detect(root, true);
        if !self.state.lock().unwrap().holds.contains_key(root) {
            return;
        }
        self.watchers.sync(root, &watch_dirs(&result));
        if before.as_ref() != Some(&result["projects"]) {
            self.broadcast(json!({"type": "configs", "root": root, "configs": self.configs(root)}));
        }
    }
}
