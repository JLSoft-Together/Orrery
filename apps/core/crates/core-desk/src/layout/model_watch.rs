use super::model::{exists, msg, now_ms, ModelSvc};
use super::project_model::Payload;
use super::LayoutError;
use crate::preview::paths::{is_within, relative};
use crate::run::android::pathx;
use notify::{RecursiveMode, Watcher};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_CODE_DIRS: usize = 50;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(50);

fn rel_parts(root: &str, dir: &str) -> Vec<String> {
    relative(root, dir).split(['/', '\\']).map(String::from).collect()
}

fn build_index(root: &str, dir: &str) -> Option<usize> {
    rel_parts(root, dir).iter().position(|p| p == "build")
}

fn is_generated(root: &str, dir: &str) -> bool {
    is_within(root, dir) && build_index(root, dir).is_some()
}

fn module_dir_of(win: bool, root: &str, dir: &str) -> String {
    let parts = rel_parts(root, dir);
    let upto = build_index(root, dir).unwrap_or(0);
    let mut all: Vec<&str> = vec![root];
    all.extend(parts[..upto].iter().map(String::as_str));
    pathx::join(win, &all)
}

pub fn code_dirs(win: bool, root: &str, payload: &Payload, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |d: String| {
        if !out.contains(&d) {
            out.push(d);
        }
    };
    for p in &payload.classpath {
        if is_generated(root, p) {
            add(pathx::join(win, &[&module_dir_of(win, root, p), "src"]));
        }
    }
    for d in &payload.res_dirs {
        if !is_within(root, d) || is_generated(root, d) {
            continue;
        }
        let parts = rel_parts(root, d);
        if let Some(i) = parts.iter().position(|p| p == "src") {
            let mut all: Vec<&str> = vec![root];
            all.extend(parts[..=i].iter().map(String::as_str));
            add(pathx::join(win, &all));
        }
    }
    out.into_iter().filter(|d| exists(d)).take(MAX_CODE_DIRS).collect()
}

type Changed = Arc<dyn Fn(Vec<String>) + Send + Sync>;

pub fn watch_dirs(dirs: &[String], cb: Changed) -> Vec<notify::RecommendedWatcher> {
    let pending: Arc<Mutex<(Vec<String>, bool)>> = Arc::new(Mutex::new((Vec::new(), false)));
    let mut out = Vec::new();
    for dir in dirs {
        let (pending, cb) = (pending.clone(), cb.clone());
        let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            let first = {
                let mut g = pending.lock().unwrap();
                for p in ev.paths {
                    let p = p.to_string_lossy().to_string();
                    if !g.0.contains(&p) {
                        g.0.push(p);
                    }
                }
                let first = !g.1;
                g.1 = true;
                first
            };
            if first {
                let (pending, cb) = (pending.clone(), cb.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(WATCH_DEBOUNCE);
                    let files = {
                        let mut g = pending.lock().unwrap();
                        g.1 = false;
                        std::mem::take(&mut g.0)
                    };
                    if !files.is_empty() {
                        cb(files);
                    }
                });
            }
        });
        if let Ok(mut w) = made {
            if w.watch(std::path::Path::new(dir), RecursiveMode::Recursive).is_ok() {
                out.push(w);
            }
        }
    }
    out
}

impl ModelSvc {
    pub fn watch_res(&self, key: &str, root: &str, payload: &Payload, with_code: bool) {
        if self.lock().watched_key.as_deref() == Some(key) {
            return;
        }
        self.stop_watch();
        let win = self.cfg.win;
        let (generated, sources): (Vec<String>, Vec<String>) = payload.res_dirs.iter().cloned().partition(|d| is_generated(root, d));
        let Some(me) = self.me.upgrade() else { return };
        let mut watchers = Vec::new();
        if !sources.is_empty() {
            let (m, k) = (me.clone(), key.to_string());
            watchers.extend(watch_dirs(&sources, Arc::new(move |files| m.on_res(&k, files, false))));
        }
        if !generated.is_empty() {
            let mut modules: Vec<String> = Vec::new();
            for g in &generated {
                let m = module_dir_of(win, root, g);
                if !modules.contains(&m) {
                    modules.push(m);
                }
            }
            let (m, k, gen) = (me.clone(), key.to_string(), generated.clone());
            watchers.extend(watch_dirs(
                &modules,
                Arc::new(move |files| {
                    let hits: Vec<String> = files.into_iter().filter(|f| gen.iter().any(|g| is_within(g, f))).collect();
                    if !hits.is_empty() {
                        m.on_res(&k, hits, true);
                    }
                }),
            ));
        }
        let code = if with_code { code_dirs(win, root, payload, &exists) } else { Vec::new() };
        if !code.is_empty() {
            let (m, k) = (me.clone(), key.to_string());
            watchers.extend(watch_dirs(&code, Arc::new(move |files| m.on_code(&k, files))));
        }
        if watchers.is_empty() {
            return;
        }
        let mut st = self.lock();
        st.watched_key = Some(key.to_string());
        st.watchers = watchers;
    }

    pub fn stop_watch(&self) {
        let old = {
            let mut st = self.lock();
            st.watched_key = None;
            st.res_pending.clear();
            st.res_slow = false;
            st.res_due = None;
            std::mem::take(&mut st.watchers)
        };
        drop(old);
    }

    pub fn on_res(&self, key: &str, files: Vec<String>, generated: bool) {
        let spawn_timer = {
            let mut st = self.lock();
            if st.loaded_key.as_deref() != Some(key) {
                return;
            }
            for f in files {
                if !st.res_pending.contains(&f) {
                    st.res_pending.push(f);
                }
            }
            st.res_slow |= generated;
            let wait = if st.res_slow { self.cfg.timing.gen_debounce } else { self.cfg.timing.debounce };
            st.res_due = Some(Instant::now() + wait);
            let first = !st.res_timer;
            st.res_timer = true;
            first
        };
        if !spawn_timer {
            return;
        }
        let Some(me) = self.me.upgrade() else { return };
        std::thread::spawn(move || loop {
            let due = me.lock().res_due;
            match due {
                None => {
                    me.lock().res_timer = false;
                    return;
                }
                Some(at) if Instant::now() >= at => {
                    me.lock().res_timer = false;
                    me.flush_res(true);
                    return;
                }
                Some(at) => std::thread::sleep(at.saturating_duration_since(Instant::now()).min(Duration::from_millis(50))),
            }
        });
    }

    pub fn flush_res(&self, from_timer: bool) {
        let (files, key) = {
            let mut st = self.lock();
            st.res_due = None;
            st.res_slow = false;
            if st.res_pending.is_empty() {
                return;
            }
            let files: Vec<String> = st.res_pending.iter().take(50).cloned().collect();
            st.res_pending.clear();
            let Some(key) = st.loaded_key.clone() else { return };
            (files, key)
        };
        {
            let _g = self.serial.lock().unwrap();
            if self.lock().loaded_key.as_deref() == Some(key.as_str()) {
                if let Err(err) = self.changed("res", &files) {
                    self.log("error", &format!("project/changed res: {}", msg(&err)));
                }
            }
        }
        let target = {
            let st = self.lock();
            st.entries.get(&key).map(|e| (e.root.clone(), e.module.clone())).or_else(|| st.source_entries.get(&key).cloned())
        };
        if let (true, Some((root, module))) = (from_timer, target) {
            self.emit(json!({"type": "project-changed", "root": root, "module": module}));
        }
    }

    pub fn changed(&self, kind: &str, files: &[String]) -> Result<(), LayoutError> {
        match self.cfg.daemon.request("project/changed", json!({"kind": kind, "files": files}), self.cfg.timing.load) {
            Ok(res) => {
                self.lock().version += 1;
                if res["restartSuggested"] == json!(true) {
                    self.cfg.daemon.request_restart();
                }
                Ok(())
            }
            Err(err) => {
                let mut st = self.lock();
                st.loaded_key = None;
                st.version += 1;
                Err(err)
            }
        }
    }

    pub fn on_code(&self, key: &str, files: Vec<String>) {
        static CODE: crate::run::android::jsre::Js = crate::run::android::jsre::Js::new(r"\.(kt|java)$", "i");
        let target = {
            let mut st = self.lock();
            if st.loaded_key.as_deref() != Some(key) {
                return;
            }
            let Some(e) = st.entries.get_mut(key) else { return };
            if e.code_changed_at != 0 || !files.iter().any(|f| CODE.test(f)) {
                return;
            }
            e.code_changed_at = now_ms();
            (e.root.clone(), e.module.clone())
        };
        self.emit(json!({"type": "code-changed", "root": target.0, "module": target.1}));
    }
}
