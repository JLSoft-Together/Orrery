use super::download::{download_asset, Progress};
use super::github::{fetch_latest, Feed};
use super::pick::{pick_asset, FeedAsset};
use super::semver::{compare_versions, is_newer};
use crate::{Emitter, HostLink};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub const FIRST_CHECK_MS: f64 = 30_000.0;
pub const CHECK_EVERY_MS: f64 = 12.0 * 60.0 * 60.0 * 1000.0;
pub const SETTINGS_FILE: &str = "update.json";
pub const PENDING_FILE: &str = "pending.json";
pub const EVENT: &str = "ccm:update-event";

#[derive(Debug, Clone)]
pub struct Cfg {
    pub current: String,
    pub kind: String,
    pub asset_kind: String,
    pub can_download: bool,
    pub arch: String,
    pub feed: String,
    pub allow_local: bool,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct Release {
    version: String,
    name: String,
    date: Option<String>,
    notes: String,
    url: String,
    asset: Option<(String, f64)>,
}

#[derive(Debug, Clone)]
struct St {
    phase: &'static str,
    release: Option<Release>,
    checked_at: Option<f64>,
    progress: Option<(f64, f64, f64)>,
    error: Option<String>,
    auto_check: bool,
    skipped: Option<String>,
}

#[derive(Default)]
struct Patch {
    phase: Option<&'static str>,
    release: Option<Option<Release>>,
    progress: Option<Option<(f64, f64, f64)>>,
    error: Option<Option<String>>,
    auto_check: Option<bool>,
    skipped: Option<Option<String>>,
}

pub struct Core {
    cfg: Cfg,
    dir: PathBuf,
    settings: PathBuf,
    st: Mutex<St>,
    asset: Mutex<Option<FeedAsset>>,
    file: Mutex<Option<PathBuf>>,
    abort: Mutex<Option<Arc<AtomicBool>>>,
    timer: AtomicU64,
    checking: (Mutex<bool>, Condvar),
    emitter: Emitter,
    host: HostLink,
}

fn read_saved(file: &Path) -> (bool, Option<String>) {
    let Some(v) = std::fs::read(file).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return (true, None) };
    (v.get("autoCheck") != Some(&Value::Bool(false)), v.get("skipped").and_then(Value::as_str).map(str::to_string))
}

impl Core {
    pub fn new(cfg: Cfg, emitter: Emitter, host: HostLink) -> Arc<Core> {
        let dir = cfg.data_dir.join("updates");
        let settings = cfg.data_dir.join(SETTINGS_FILE);
        let (auto_check, skipped) = read_saved(&settings);
        let st = St { phase: "idle", release: None, checked_at: None, progress: None, error: None, auto_check, skipped };
        Arc::new(Core { cfg, dir, settings, st: Mutex::new(st), asset: Mutex::new(None), file: Mutex::new(None), abort: Mutex::new(None), timer: AtomicU64::new(0), checking: (Mutex::new(false), Condvar::new()), emitter, host })
    }

    pub fn state(&self) -> Value {
        let s = self.st.lock().unwrap().clone();
        let release = s.release.as_ref().map(|r| {
            json!({
                "version": r.version, "name": r.name, "date": r.date, "notes": r.notes, "url": r.url,
                "asset": r.asset.as_ref().map(|(n, z)| json!({"name": n, "size": z})),
            })
        });
        json!({
            "current": self.cfg.current, "kind": self.cfg.kind, "canDownload": self.cfg.can_download, "phase": s.phase,
            "release": release, "checkedAt": s.checked_at,
            "progress": s.progress.map(|(r, t, b)| json!({"received": r, "total": t, "bps": b})),
            "error": s.error, "autoCheck": s.auto_check, "skipped": s.skipped,
        })
    }

    fn set(&self, p: Patch) {
        {
            let mut s = self.st.lock().unwrap();
            if let Some(v) = p.phase {
                s.phase = v;
            }
            if let Some(v) = p.release {
                s.release = v;
            }
            if let Some(v) = p.progress {
                s.progress = v;
            }
            if let Some(v) = p.error {
                s.error = v;
            }
            if let Some(v) = p.auto_check {
                s.auto_check = v;
            }
            if let Some(v) = p.skipped {
                s.skipped = v;
            }
        }
        self.emitter.emit(EVENT, self.state());
    }

    fn phase(&self) -> &'static str {
        self.st.lock().unwrap().phase
    }

    fn write_saved(&self) {
        let s = self.st.lock().unwrap();
        let _ = std::fs::write(&self.settings, crate::js::stringify(&json!({"autoCheck": s.auto_check, "skipped": s.skipped})));
    }

    fn read_pending(&self) -> Option<String> {
        let v: Value = serde_json::from_slice(&std::fs::read(self.dir.join(PENDING_FILE)).ok()?).ok()?;
        v.get("version").and_then(Value::as_str).map(str::to_string)
    }

    fn find_downloaded(&self) -> Option<PathBuf> {
        let rd = std::fs::read_dir(&self.dir).ok()?;
        rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().to_string()).find(|n| !n.ends_with(".part") && n != PENDING_FILE).map(|n| self.dir.join(n))
    }

    pub fn start(self: &Arc<Self>) {
        match self.read_pending() {
            Some(p) if is_newer(&p, &self.cfg.current) => {
                self.set(Patch { phase: Some("error"), error: Some(Some("incomplete".into())), ..Default::default() });
                *self.file.lock().unwrap() = self.find_downloaded();
            }
            _ => {
                let _ = std::fs::remove_dir_all(&self.dir);
            }
        }
        self.schedule(FIRST_CHECK_MS);
    }

    pub fn dispose(&self) {
        self.timer.fetch_add(1, Ordering::SeqCst);
        if let Some(a) = self.abort.lock().unwrap().as_ref() {
            a.store(true, Ordering::SeqCst);
        }
    }

    fn schedule(self: &Arc<Self>, ms: f64) {
        let gen = self.timer.fetch_add(1, Ordering::SeqCst) + 1;
        if !self.st.lock().unwrap().auto_check {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            let until = std::time::Instant::now() + Duration::from_millis(ms.max(0.0) as u64);
            while std::time::Instant::now() < until {
                if me.timer.load(Ordering::SeqCst) != gen {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if me.timer.load(Ordering::SeqCst) == gen {
                me.check();
            }
        });
    }

    pub fn set_auto_check(self: &Arc<Self>, on: bool) -> Value {
        self.set(Patch { auto_check: Some(on), ..Default::default() });
        self.write_saved();
        if on {
            let checked = self.st.lock().unwrap().checked_at;
            let ms = checked.map_or(FIRST_CHECK_MS, |c| FIRST_CHECK_MS.max(c + CHECK_EVERY_MS - crate::util::now_ms()));
            self.schedule(ms);
        } else {
            self.timer.fetch_add(1, Ordering::SeqCst);
        }
        self.state()
    }

    pub fn skip(&self, version: &Value) -> Value {
        let current = self.st.lock().unwrap().release.as_ref().map(|r| r.version.clone());
        if let (Some(v), Some(c)) = (version.as_str(), current) {
            if v == c {
                self.set(Patch { skipped: Some(Some(v.to_string())), ..Default::default() });
                self.write_saved();
            }
        }
        self.state()
    }

    fn busy(&self) -> bool {
        matches!(self.phase(), "downloading" | "ready" | "installing")
    }

    pub fn check(self: &Arc<Self>) -> Value {
        if self.busy() {
            return self.state();
        }
        let (lock, cv) = &self.checking;
        let mut running = lock.lock().unwrap();
        if *running {
            while *running {
                running = cv.wait(running).unwrap();
            }
            return self.state();
        }
        *running = true;
        drop(running);
        self.run_check();
        *lock.lock().unwrap() = false;
        cv.notify_all();
        self.state()
    }

    fn run_check(self: &Arc<Self>) {
        let incomplete = self.st.lock().unwrap().error.as_deref() == Some("incomplete");
        self.set(Patch { phase: Some("checking"), error: Some(None), ..Default::default() });
        let res = fetch_latest(&self.cfg.feed, self.cfg.allow_local, Duration::from_secs(15));
        self.st.lock().unwrap().checked_at = Some(crate::util::now_ms());
        if self.st.lock().unwrap().auto_check {
            self.schedule(CHECK_EVERY_MS);
        }
        let rel = match res {
            Feed::Error(e) => {
                self.set(Patch { phase: Some("error"), error: Some(Some(if incomplete { "incomplete" } else { e }.into())), ..Default::default() });
                return;
            }
            Feed::Release(r) => r,
        };
        let Some(rel) = rel.filter(|r| is_newer(&r.version, &self.cfg.current)) else {
            *self.asset.lock().unwrap() = None;
            self.set(Patch { phase: Some("none"), release: Some(None), ..Default::default() });
            return;
        };
        let asset = pick_asset(&rel.assets, &rel.version, &self.cfg.asset_kind, &self.cfg.arch);
        *self.asset.lock().unwrap() = asset.clone();
        let release = Release { version: rel.version.clone(), name: rel.name.clone(), date: rel.date.clone(), notes: rel.notes.clone(), url: rel.url.clone(), asset: asset.as_ref().map(|a| (a.name.clone(), a.size)) };
        let error = if !self.cfg.can_download {
            None
        } else if asset.is_none() {
            Some("no-asset")
        } else if asset.as_ref().is_some_and(|a| a.sha256.is_none()) {
            Some("no-digest")
        } else {
            None
        };
        let skipped = self.st.lock().unwrap().skipped.clone();
        if skipped.is_some_and(|s| compare_versions(&rel.version, &s) > 0) {
            self.set(Patch { skipped: Some(None), ..Default::default() });
        }
        let error = if incomplete { Some("incomplete") } else { error };
        self.set(Patch { phase: Some("available"), release: Some(Some(release)), error: Some(error.map(str::to_string)), ..Default::default() });
    }

    pub fn download(self: &Arc<Self>) -> Value {
        let asset = self.asset.lock().unwrap().clone();
        let phase = self.phase();
        let Some(asset) = asset.filter(|_| self.cfg.can_download && !matches!(phase, "downloading" | "ready" | "installing")) else { return self.state() };
        let cancel = Arc::new(AtomicBool::new(false));
        *self.abort.lock().unwrap() = Some(cancel.clone());
        self.set(Patch { phase: Some("downloading"), error: Some(None), progress: Some(Some((0.0, asset.size, 0.0))), ..Default::default() });
        let me = self.clone();
        let mine = cancel.clone();
        let res = download_asset(&asset, &self.dir, self.cfg.allow_local, &cancel, &mut |p: Progress| {
            let current = me.abort.lock().unwrap().as_ref().is_some_and(|a| Arc::ptr_eq(a, &mine));
            if current {
                me.set(Patch { progress: Some(Some((p.received, p.total, p.bps))), ..Default::default() });
            }
        });
        let same = self.abort.lock().unwrap().as_ref().is_some_and(|a| Arc::ptr_eq(a, &cancel));
        if !same {
            return self.state();
        }
        *self.abort.lock().unwrap() = None;
        match res {
            Ok(file) => {
                *self.file.lock().unwrap() = Some(file);
                self.set(Patch { phase: Some("ready"), progress: Some(None), ..Default::default() });
            }
            Err("cancelled") => self.set(Patch { phase: Some("available"), progress: Some(None), ..Default::default() }),
            Err(e) => self.set(Patch { phase: Some("error"), progress: Some(None), error: Some(Some(e.into())), ..Default::default() }),
        }
        self.state()
    }

    pub fn cancel(&self) -> Value {
        if self.phase() != "downloading" {
            return self.state();
        }
        let Some(a) = self.abort.lock().unwrap().take() else { return self.state() };
        a.store(true, Ordering::SeqCst);
        self.set(Patch { phase: Some("available"), progress: Some(None), ..Default::default() });
        self.state()
    }

    pub fn install(&self) -> Value {
        let file = self.file.lock().unwrap().clone();
        let version = self.st.lock().unwrap().release.as_ref().map(|r| r.version.clone());
        let (Some(file), Some(version)) = (file.filter(|f| f.exists()), version) else { return self.state() };
        if self.phase() != "ready" {
            return self.state();
        }
        let path = file.to_string_lossy().to_string();
        if self.cfg.kind == "dev" || self.cfg.kind == "unsupported" {
            let _ = self.host.call("update.reveal", json!({"file": path}));
            return self.state();
        }
        self.set(Patch { phase: Some("installing"), error: Some(None), ..Default::default() });
        let prepared = self.host.call("update.prepare", json!({"file": path})).unwrap_or_else(|_| json!({"ok": false, "error": "install"}));
        if prepared["ok"] != Value::Bool(true) {
            let error = prepared["error"].as_str().unwrap_or("install").to_string();
            let readonly = error == "mac-readonly";
            if readonly {
                let _ = self.host.call("update.open", json!({"target": path}));
            }
            self.set(Patch { phase: Some(if readonly { "ready" } else { "error" }), error: Some(Some(error)), ..Default::default() });
            return self.state();
        }
        let pending = json!({"file": self.dir.join(PENDING_FILE).to_string_lossy(), "text": crate::js::stringify(&json!({"version": version}))});
        let ok = matches!(self.host.call("update.restart", json!({"version": version, "pending": pending})), Ok(Value::Bool(true)));
        if !ok {
            self.set(Patch { phase: Some("ready"), ..Default::default() });
        }
        self.state()
    }

    pub fn open_page(&self) {
        let url = self.st.lock().unwrap().release.as_ref().map(|r| r.url.clone());
        if let Some(url) = url {
            let _ = self.host.call("update.open", json!({"target": url}));
        }
    }

    pub fn reveal_file(&self) {
        let file = self.file.lock().unwrap().clone().filter(|f| f.exists());
        if let Some(f) = file {
            let _ = self.host.call("update.reveal", json!({"file": f.to_string_lossy()}));
        }
    }
}
