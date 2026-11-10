pub mod space;

use crate::claude::Sys;
use crate::{arg, js, util, Desk, DeskEnv, Emitter};
use serde_json::{json, Value};
use space::{drive_label, fake_space, mount_of, statfs, Space};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

pub const MODULE: &str = "disk";
pub const DISK_FILE: &str = "disk.json";
pub const EVENT: &str = "ccm:disk-event";
pub const ALERT_EVENT: &str = "desk:disk-alert";
const WARN_CHOICES: [i64; 5] = [0, 5, 10, 20, 50];
const DEFAULT_WARN: i64 = 10;
const CHECK_MS: u64 = 10 * 60_000;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;
const PROJECTS_MAX: usize = 50;

pub fn level(free: f64, total: f64, warn: i64) -> &'static str {
    if warn <= 0 {
        return "ok";
    }
    if free < 2.0 * GB {
        return "danger";
    }
    if free < warn as f64 * GB || (total > 0.0 && free / total * 100.0 < 5.0) {
        return "warn";
    }
    "ok"
}

pub fn rearmed(free: f64, total: f64, warn: i64) -> bool {
    free >= (warn as f64 * GB).max(total * 5.0 / 100.0) + GB
}

fn warn_choice(v: &Value) -> Option<i64> {
    let f = v.as_f64()?;
    WARN_CHOICES.iter().copied().find(|c| *c as f64 == f)
}

pub fn read_warn(file: &Path) -> i64 {
    std::fs::read_to_string(file).ok().and_then(|t| js::parse(&t)).and_then(|v| v.get("warnGb").and_then(warn_choice)).unwrap_or(DEFAULT_WARN)
}

struct Inner {
    warn: i64,
    last: Value,
    projects: Vec<String>,
    alerted: HashMap<String, &'static str>,
    timer: u64,
    poking: bool,
}

pub struct DiskWatch {
    file: PathBuf,
    data_dir: PathBuf,
    sys: Sys,
    fake: Option<String>,
    inner: Mutex<Inner>,
    pass: Mutex<()>,
    emitter: Emitter,
    me: Weak<DiskWatch>,
}

fn n(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

impl DiskWatch {
    pub fn new(env: &DeskEnv, emitter: Emitter) -> Arc<Self> {
        let file = env.data_dir.join(DISK_FILE);
        let warn = read_warn(&file);
        Arc::new_cyclic(|me| DiskWatch {
            file,
            data_dir: env.data_dir.clone(),
            sys: Sys::from_env(),
            fake: std::env::var("CCM_E2E_DISK").ok().filter(|v| !v.is_empty()),
            inner: Mutex::new(Inner { warn, last: json!({"warnGb": warn, "at": null, "drives": []}), projects: Vec::new(), alerted: HashMap::new(), timer: 0, poking: false }),
            pass: Mutex::new(()),
            emitter,
            me: me.clone(),
        })
    }

    pub fn state(&self) -> Value {
        self.inner.lock().unwrap().last.clone()
    }

    fn android_dirs(&self) -> Vec<String> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let home = Path::new(&self.sys.home);
        let setting = std::fs::read_to_string(self.data_dir.join("run-settings.json"))
            .ok()
            .and_then(|t| js::parse(&t))
            .and_then(|v| v.get("androidSdk").and_then(Value::as_str).filter(|s| crate::claude::is_absolute(s)).map(str::to_string));
        let default = if cfg!(target_os = "macos") {
            home.join("Library").join("Android").join("sdk")
        } else {
            var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData").join("Local")).join("Android").join("Sdk")
        };
        let sdk = setting.or_else(|| var("ANDROID_HOME")).or_else(|| var("ANDROID_SDK_ROOT")).unwrap_or_else(|| util::path_str(&default));
        let avd = var("ANDROID_AVD_HOME").unwrap_or_else(|| util::path_str(&home.join(".android").join("avd")));
        [sdk, avd].into_iter().filter(|d| Path::new(d).exists()).collect()
    }

    fn capture_dir(&self) -> Option<String> {
        let folder = crate::capture::folder::Folder::new(self.data_dir.join(crate::capture::files::CAPTURE_FILE));
        let videos = || std::env::var("CCM_VIDEOS_DIR").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| util::path_str(&Path::new(&self.sys.home).join(if cfg!(target_os = "macos") { "Movies" } else { "Videos" })));
        Some(match folder.status().active {
            None => videos(),
            Some(dir) => {
                let rec = Path::new(&dir).join(crate::capture::files::REC_SUB);
                if std::fs::create_dir_all(&rec).is_ok() {
                    util::path_str(&rec)
                } else {
                    videos()
                }
            }
        })
    }

    fn places(&self) -> Vec<(&'static str, String)> {
        let codex = std::env::var("CODEX_HOME").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| util::path_str(&Path::new(&self.sys.home).join(".codex")));
        let mut out = vec![("claude", util::path_str(&self.sys.claude_root())), ("codex", codex), ("data", util::path_str(&self.data_dir))];
        if let Some(c) = self.capture_dir() {
            out.push(("capture", c));
        }
        let projects = self.inner.lock().unwrap().projects.clone();
        out.extend(projects.into_iter().map(|d| ("project", d)));
        out.extend(self.android_dirs().into_iter().map(|d| ("android", d)));
        out
    }

    fn space(&self, mount: &str) -> Option<Space> {
        match &self.fake {
            Some(f) => fake_space(f),
            None => statfs(mount),
        }
    }

    pub fn measure(&self) -> Value {
        let Ok(_guard) = self.pass.try_lock() else {
            drop(self.pass.lock().unwrap());
            return self.state();
        };
        let mut by_mount: Vec<(String, String, Vec<&'static str>)> = Vec::new();
        for (place, dir) in self.places() {
            let Some(mount) = mount_of(&dir) else { continue };
            let key = if cfg!(windows) { mount.to_lowercase() } else { mount.clone() };
            match by_mount.iter_mut().find(|(k, _, _)| *k == key) {
                Some((_, _, places)) => {
                    if !places.contains(&place) {
                        places.push(place)
                    }
                }
                None => by_mount.push((key, mount, vec![place])),
            }
        }
        let warn = self.inner.lock().unwrap().warn;
        let mut drives: Vec<(f64, Value)> = Vec::new();
        for (_, mount, places) in by_mount {
            let Some(s) = self.space(&mount) else { continue };
            let d = json!({"mount": mount, "label": drive_label(&mount), "freeBytes": n(s.free), "totalBytes": n(s.total), "level": level(s.free, s.total, warn), "places": places});
            drives.push((s.free, d));
        }
        drives.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut g = self.inner.lock().unwrap();
        let warn = g.warn;
        let alerts = Self::step(&mut g.alerted, &drives, warn);
        let state = json!({"warnGb": g.warn, "at": util::now_ms() as i64, "drives": drives.into_iter().map(|(_, d)| d).collect::<Vec<_>>()});
        g.last = state.clone();
        drop(g);
        for a in alerts {
            self.emitter.emit(ALERT_EVENT, a);
        }
        self.emitter.emit(EVENT, state.clone());
        state
    }

    fn step(alerted: &mut HashMap<String, &'static str>, drives: &[(f64, Value)], warn: i64) -> Vec<Value> {
        let mut out = Vec::new();
        for (_, d) in drives {
            let mount = d["mount"].as_str().unwrap_or("").to_string();
            let (free, total) = (d["freeBytes"].as_f64().unwrap_or(0.0), d["totalBytes"].as_f64().unwrap_or(0.0));
            let lvl: &'static str = match d["level"].as_str() {
                Some("danger") => "danger",
                Some("warn") => "warn",
                _ => "ok",
            };
            let was = alerted.get(&mount).copied();
            if lvl == "ok" {
                if was.is_some() && rearmed(free, total, warn) {
                    alerted.remove(&mount);
                }
                continue;
            }
            if was == Some("danger") || (was == Some("warn") && lvl == "warn") {
                continue;
            }
            alerted.insert(mount, lvl);
            out.push(json!({"type": "disk.alert", "label": d["label"], "level": lvl, "freeBytes": d["freeBytes"], "totalBytes": d["totalBytes"]}));
        }
        out
    }

    fn spawn_measure(&self, delay: Duration, gate: impl Fn(&DiskWatch) -> bool + Send + 'static) {
        let me = self.me.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            if let Some(w) = me.upgrade() {
                if gate(&w) {
                    w.measure();
                }
            }
        });
    }

    fn arm(&self, first: bool) {
        let gen = {
            let mut g = self.inner.lock().unwrap();
            g.timer += 1;
            g.poking = false;
            if g.warn <= 0 {
                return;
            }
            g.timer
        };
        if first {
            self.spawn_measure(Duration::ZERO, move |w| w.inner.lock().unwrap().timer == gen);
        }
        let me = self.me.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(CHECK_MS));
            let Some(w) = me.upgrade() else { return };
            if w.inner.lock().unwrap().timer != gen {
                return;
            }
            w.measure();
        });
    }

    pub fn start(&self) {
        self.arm(true)
    }

    pub fn stop(&self) {
        let mut g = self.inner.lock().unwrap();
        g.timer += 1;
        g.poking = false;
    }

    pub fn poke(&self, delay_ms: u64) {
        let gen = {
            let mut g = self.inner.lock().unwrap();
            if g.warn <= 0 || g.poking {
                return;
            }
            g.poking = true;
            g.timer
        };
        self.spawn_measure(Duration::from_millis(delay_ms), move |w| {
            let mut g = w.inner.lock().unwrap();
            let go = g.poking && g.timer == gen;
            g.poking = false;
            go
        });
    }

    pub fn set_warn(&self, gb: i64) -> Value {
        {
            let mut g = self.inner.lock().unwrap();
            g.warn = gb;
            g.alerted.clear();
        }
        let _ = util::write_atomic(&self.file, &js::stringify(&json!({"warnGb": gb})));
        if gb <= 0 {
            self.stop();
            let mut g = self.inner.lock().unwrap();
            let drives: Vec<Value> = g.last["drives"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|mut d| {
                    d["level"] = json!("ok");
                    d
                })
                .collect();
            g.last = json!({"warnGb": 0, "at": g.last["at"].clone(), "drives": drives});
            let state = g.last.clone();
            drop(g);
            self.emitter.emit(EVENT, state.clone());
            return state;
        }
        self.arm(false);
        self.measure()
    }

    pub fn set_projects(&self, dirs: Vec<String>) {
        let changed = {
            let mut g = self.inner.lock().unwrap();
            let changed = g.projects != dirs;
            g.projects = dirs;
            changed
        };
        if changed {
            self.poke(2_000);
        }
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let watch = DiskWatch::new(env, desk.emitter());
    let w = watch.clone();
    desk.on(MODULE, "ccm:disk-state", move |_, _| Ok(w.state()));
    let w = watch.clone();
    desk.on(MODULE, "ccm:disk-refresh", move |_, _| Ok(w.measure()));
    let w = watch.clone();
    desk.on(MODULE, "ccm:disk-set", move |_, args| {
        let gb = arg(args, 0).as_object().and_then(|o| o.get("warnGb")).and_then(warn_choice);
        Ok(gb.map(|g| w.set_warn(g)).unwrap_or(Value::Null))
    });
    let w = watch.clone();
    desk.on(MODULE, "ccm:disk-projects", move |_, args| {
        let Some(list) = arg(args, 0).as_array() else { return Ok(json!(false)) };
        let dirs: Vec<String> = list.iter().filter_map(Value::as_str).filter(|d| !d.is_empty() && js::len(d) <= 1024).take(PROJECTS_MAX).map(str::to_string).collect();
        w.set_projects(dirs);
        Ok(json!(true))
    });
    let w = watch.clone();
    desk.on(MODULE, "ccm:disk-open", move |ctx, args| {
        let Some(raw) = arg(args, 0).as_str() else { return Ok(json!(false)) };
        let known = w.state()["drives"].as_array().is_some_and(|d| d.iter().any(|x| x["mount"].as_str() == Some(raw)));
        if !known {
            return Ok(json!(false));
        }
        Ok(json!(matches!(ctx.host("disk.open", json!({"dir": raw})), Ok(Value::Bool(true)))))
    });
    let w = watch.clone();
    desk.on(MODULE, "desk:disk-start", move |_, _| {
        w.start();
        Ok(json!(true))
    });
    let w = watch.clone();
    desk.on(MODULE, "desk:disk-stop", move |_, _| {
        w.stop();
        Ok(json!(true))
    });
    desk.on(MODULE, "desk:disk-poke", move |_, args| {
        let ms = arg(args, 0).get("ms").and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0).unwrap_or(5_000.0);
        watch.poke(ms.min(600_000.0) as u64);
        Ok(json!(true))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_and_alerts() {
        assert_eq!(level(1.0 * GB, 100.0 * GB, 10), "danger");
        assert_eq!(level(5.0 * GB, 100.0 * GB, 10), "warn");
        assert_eq!(level(20.0 * GB, 1000.0 * GB, 10), "warn");
        assert_eq!(level(20.0 * GB, 100.0 * GB, 10), "ok");
        assert_eq!(level(0.0, 100.0, 0), "ok");
        assert!(rearmed(11.0 * GB, 100.0 * GB, 10));
        assert!(!rearmed(10.5 * GB, 100.0 * GB, 10));
        let mut alerted = HashMap::new();
        let d = |free: f64, lvl: &str| (free, json!({"mount": "/", "label": "/", "freeBytes": free, "totalBytes": 100.0 * GB, "level": lvl}));
        assert_eq!(DiskWatch::step(&mut alerted, &[d(5.0 * GB, "warn")], 10).len(), 1);
        assert_eq!(DiskWatch::step(&mut alerted, &[d(5.0 * GB, "warn")], 10).len(), 0);
        assert_eq!(DiskWatch::step(&mut alerted, &[d(1.0 * GB, "danger")], 10).len(), 1);
        assert_eq!(DiskWatch::step(&mut alerted, &[d(10.5 * GB, "ok")], 10).len(), 0);
        assert_eq!(DiskWatch::step(&mut alerted, &[d(5.0 * GB, "warn")], 10).len(), 0);
    }
}
