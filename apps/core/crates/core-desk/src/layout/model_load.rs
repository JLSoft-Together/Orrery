use super::model::{exists, msg, now_ms, show, ModelSvc, SourceHit, KIT_KEY, SOURCE_PREFIX, LoadedSource};
use super::project_model::{parse_loose_kit, Payload};
use super::source_model::{build_source_model, SourceOpts};
use super::gradle_graph::Project;
use super::LayoutError;
use crate::preview::paths::root_key;
use crate::run::android::detect::analyze_gradle_root;
use crate::run::android::pathx;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::sync::Arc;
use std::time::Instant;

impl ModelSvc {
    fn loose_kit(&self) -> Option<Payload> {
        if let Some(k) = self.lock().kit.clone() {
            return Some(k);
        }
        let dir = self.cfg.loose_kit_dir.as_deref()?;
        let text = std::fs::read_to_string(pathx::join(self.cfg.win, &[dir, "kit.json"])).ok()?;
        let raw: Value = serde_json::from_str(&text).map_err(|e| self.log("warn", &format!("loose kit unusable: {e}"))).ok()?;
        match parse_loose_kit(&raw, dir, &exists, self.cfg.win) {
            Ok(k) => {
                self.lock().kit = Some(k.clone());
                Some(k)
            }
            Err(e) => {
                self.log("warn", &format!("loose kit unusable: {e}"));
                None
            }
        }
    }

    pub fn ensure_loaded(&self, key: Option<&str>, file: Option<&str>) -> Result<(), LayoutError> {
        if let Some(k) = key {
            let usable = self.get(k).is_some_and(|e| e.model.is_some() && !e.broken);
            if usable {
                if self.lock().loaded_key.as_deref() != Some(k) {
                    self.load(k)?;
                }
                return Ok(());
            }
        }
        let kit = if self.lock().kit_failed { None } else { self.loose_kit() };
        for prebuilt in [true, false] {
            let src = match (key, file) {
                (Some(k), Some(f)) => self.source_for(k, f, kit.as_ref(), prebuilt),
                _ => None,
            };
            let Some(src) = src else { continue };
            if self.lock().source_failed.contains(&src.key) {
                continue;
            }
            if self.lock().loaded_key.as_deref() == Some(src.key.as_str()) {
                return Ok(());
            }
            match self.load_source(key.unwrap_or(""), &src) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    self.lock().source_failed.insert(src.key.clone());
                    self.log("error", &format!("project/load source: {}", msg(&err)));
                }
            }
        }
        if let Some(kit) = kit {
            if self.lock().loaded_key.as_deref() != Some(KIT_KEY) {
                self.load_kit(&kit)?;
            }
            return Ok(());
        }
        {
            let mut st = self.lock();
            if st.loaded_key.is_none() {
                return Ok(());
            }
            st.loaded_key = None;
            st.version += 1;
        }
        self.stop_watch();
        self.cfg.daemon.request("project/load", json!({}), self.cfg.timing.load).map(|_| ())
    }

    pub fn source_for(&self, key: &str, file: &str, kit: Option<&Payload>, prebuilt: bool) -> Option<SourceHit> {
        let e = self.get(key)?;
        let scope = format!("{}\u{0}{}\u{0}{}\u{0}{}", e.key, root_key(&crate::preview::paths::dirname(file)), if kit.is_some() { "kit" } else { "" }, if prebuilt { "prebuilt" } else { "" });
        if let Some((at, v)) = self.lock().sources.get(&scope) {
            if at.elapsed() < self.cfg.timing.resolve_ttl {
                return v.clone();
            }
        }
        let remember = |v: Option<SourceHit>| {
            self.lock().sources.insert(scope.clone(), (Instant::now(), v.clone()));
            v
        };
        let project = analyze_gradle_root(&e.root).map(|v| Project::from_value(&v));
        let model = project.and_then(|p| build_source_model(&p, &e.module, file, &SourceOpts { win: self.cfg.win, gradle_home: &self.cfg.gradle_home, aar_cache_dir: &self.cfg.aar_cache_dir, kit, prebuilt }));
        let Some(model) = model else { return remember(None) };
        if prebuilt && model.prebuilt_at.is_none() {
            return remember(None);
        }
        let digest: String = Sha1::digest(model.payload.to_value().to_string().as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        let hit = SourceHit { key: format!("{SOURCE_PREFIX}{}:{}", e.hash, &digest[..12]), model: Arc::new(model) };
        if let Some(en) = self.lock().entries.get_mut(key) {
            en.source_themes = hit.model.themes.clone();
        }
        remember(Some(hit))
    }

    pub fn loading(&self, kind: &str, active: bool) {
        self.emit(json!({"type": "loading", "kind": kind, "active": active}));
    }

    pub fn warn_list(&self, own: &[String], res: &Value) {
        let extra: Vec<String> = res["warnings"].as_array().map(|a| a.iter().map(show).collect()).unwrap_or_default();
        for w in own.iter().cloned().chain(extra).take(20) {
            self.log("warn", &w.chars().take(500).collect::<String>());
        }
    }

    pub fn warn_all(&self, res: &Value) {
        self.warn_list(&[], res)
    }

    pub fn project_load(&self, kind: &str, params: Value) -> Result<Value, LayoutError> {
        self.loading(kind, true);
        let r = self.cfg.daemon.request("project/load", params, self.cfg.timing.load);
        self.loading(kind, false);
        r
    }

    pub fn with_id(payload: &Payload, id: &str) -> Value {
        let mut v = payload.to_value();
        v["projectId"] = json!(id);
        v
    }

    pub fn load_source(&self, key: &str, src: &SourceHit) -> Result<(), LayoutError> {
        self.stop_watch();
        self.lock().loaded_key = None;
        let res = match self.project_load("project", Self::with_id(&src.model.payload, &src.key)) {
            Ok(v) => v,
            Err(err) => {
                self.lock().version += 1;
                return Err(err);
            }
        };
        let p = &src.model.payload;
        let (root, module) = self.get(key).map(|e| (e.root, e.module)).unwrap_or_default();
        {
            let mut st = self.lock();
            st.loaded_key = Some(src.key.clone());
            st.loaded_source = Some(LoadedSource { kit: self.cfg.loose_kit_dir.is_some() && !p.classpath.is_empty(), prebuilt_at: src.model.prebuilt_at });
            st.source_entries.insert(src.key.clone(), (root.clone(), module.clone()));
            st.version += 1;
        }
        let how = match src.model.prebuilt_at {
            Some(at) => format!("prebuilt {}, {} classpath", crate::util::iso_string(at), p.classpath.len()),
            None => "res only".into(),
        };
        self.log("info", &format!("project/load source {module} ({how}){}: {} res dirs, {} aars, {} ms", if res["reused"] == json!(true) { " (warm)" } else { "" }, p.res_dirs.len(), p.aar_res_dirs.len(), show(&res["loadMs"])));
        self.warn_list(&src.model.warnings, &res);
        self.watch_res(&src.key, &root, p, false);
        if res["restartSuggested"] == json!(true) {
            self.cfg.daemon.request_restart();
        }
        Ok(())
    }

    pub fn load_kit(&self, kit: &Payload) -> Result<(), LayoutError> {
        self.stop_watch();
        self.lock().loaded_key = None;
        let res = match self.project_load("kit", Self::with_id(kit, KIT_KEY)) {
            Ok(v) => v,
            Err(err) => {
                let mut st = self.lock();
                st.kit_failed = true;
                st.version += 1;
                return Err(err);
            }
        };
        {
            let mut st = self.lock();
            st.loaded_key = Some(KIT_KEY.into());
            st.version += 1;
        }
        self.log("info", &format!("project/load loose kit{}: {} res, {} ms", if res["reused"] == json!(true) { " (warm)" } else { "" }, show(&res["resCount"]), show(&res["loadMs"])));
        self.warn_all(&res);
        if res["restartSuggested"] == json!(true) {
            self.cfg.daemon.request_restart();
        }
        Ok(())
    }

    pub fn load(&self, key: &str) -> Result<(), LayoutError> {
        let Some(e) = self.get(key) else { return Ok(()) };
        let model = e.model.clone().expect("model");
        let mut params = model.payload.to_value();
        params["projectId"] = json!(e.hash);
        params["classesChanged"] = json!(e.classes_changed);
        let res = match self.project_load("project", params) {
            Ok(v) => v,
            Err(err) => {
                self.lock().loaded_key = None;
                if let Some(en) = self.lock().entries.get_mut(key) {
                    en.broken = true;
                }
                self.set_state(key, json!({"state": "error", "code": "load-failed", "detail": msg(&err).chars().take(600).collect::<String>(), "at": now_ms()}));
                return Err(err);
            }
        };
        {
            let mut st = self.lock();
            st.loaded_key = Some(key.to_string());
            st.version += 1;
            if let Some(en) = st.entries.get_mut(key) {
                en.classes_changed = false;
            }
        }
        self.log("info", &format!("project/load {}{}: {} res, {} classpath, {} ms", e.module, if res["reused"] == json!(true) { " (warm)" } else { "" }, show(&res["resCount"]), show(&res["classpathCount"]), show(&res["loadMs"])));
        self.warn_all(&res);
        self.stop_watch();
        self.watch_res(key, &e.root, &model.payload, true);
        if res["restartSuggested"] == json!(true) {
            self.cfg.daemon.request_restart();
        }
        Ok(())
    }
}
