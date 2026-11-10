use super::cache::RenderCache;
use super::daemon::{DaemonCfg, LayoutDaemon};
use super::download::{Bundle, BundleCfg, KitSpec, UreqFetcher};
use super::jdk::{find_jdk, Jdk};
use super::model::{LogFn, ModelCfg, ModelDaemon, ModelSvc, Timing};
use super::proc::default_spawn;
use super::{LayoutError, LAYOUTLIB_API_LEVEL};
use crate::run::settings::SettingsStore;
use crate::{Desk, DeskEnv};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

const JDK_TTL: Duration = Duration::from_secs(30);
pub const RENDER_CACHE: usize = 32;
pub const WARM_PEEK: Duration = Duration::from_millis(1500);

#[derive(Clone)]
pub struct Cached {
    pub result: Value,
    pub png: Option<String>,
}

pub struct Svc {
    pub bundle: Arc<Bundle>,
    pub daemon: LayoutDaemon,
    pub model: Arc<ModelSvc>,
    pub jar: String,
    pub find_java: Arc<dyn Fn() -> Jdk + Send + Sync>,
    pub render_dir: String,
    pub snap_dir: String,
    pub win: bool,
    pub cache: Mutex<RenderCache<Cached>>,
    pub warm: Mutex<Option<Arc<Warm>>>,
}

#[derive(Default)]
pub struct Warm {
    pub done: Mutex<Option<Result<(), LayoutError>>>,
    pub cv: Condvar,
}

pub type Tune = Box<dyn FnOnce(&mut DaemonCfg) + Send>;

pub struct SvcCfg {
    pub data_dir: String,
    pub jar: String,
    pub override_dir: Option<String>,
    pub kit: Option<KitSpec>,
    pub log_file: Option<String>,
    pub loose_kit_dir: Option<String>,
    pub init_script: String,
    pub model_timing: Option<Timing>,
    pub settings: Arc<SettingsStore>,
    pub emit: Arc<dyn Fn(Value) + Send + Sync>,
    pub spawn: Option<super::proc::Spawn>,
    pub fetcher: Option<Arc<dyn super::download::Fetcher>>,
    pub platform: String,
    pub arch: String,
    pub tune: Option<Tune>,
}

pub fn host_arch() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "arm64".into(),
        "x86_64" => "x64".into(),
        other => other.into(),
    }
}

pub fn host_platform() -> String {
    crate::backup::service::platform_name().to_string()
}

fn join(a: &str, b: &str) -> String {
    Path::new(a).join(b).to_string_lossy().to_string()
}

fn append_log(file: &str, home: &str, platform: &str, level: &str, msg: &str) {
    use std::io::Write;
    let line = format!("{} [{level}] {}\n", crate::util::iso_string(crate::util::now_ms()), core_sources::mask::mask_secrets(msg, home, platform));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = f.write_all(line.as_bytes());
    }
}

impl Svc {
    pub fn new(cfg: SvcCfg) -> Arc<Svc> {
        let win = cfg.platform == "win32";
        let base = join(&cfg.data_dir, "layoutlib");
        let cache_dir = join(&base, "cache");
        let render_dir = join(&cache_dir, "render");
        let snap_dir = join(&cache_dir, "snap");
        let emit = cfg.emit.clone();
        let bundle_emit = emit.clone();
        let bundle = Arc::new(Bundle::new(BundleCfg {
            base: base.clone(),
            platform: cfg.platform.clone(),
            arch: cfg.arch.clone(),
            override_dir: cfg.override_dir.clone(),
            kit: cfg.kit.clone(),
            fetcher: cfg.fetcher.clone().unwrap_or_else(|| Arc::new(UreqFetcher::default())),
            on_status: Arc::new(move |status| bundle_emit(json!({"type": "bundle", "status": status}))),
        }));
        let jdk_cache: Mutex<Option<(Option<String>, Instant, Jdk)>> = Mutex::new(None);
        let settings = cfg.settings.clone();
        let find_java: Arc<dyn Fn() -> Jdk + Send + Sync> = Arc::new(move || {
            let key = settings.get().jdk;
            if let Some((k, at, v)) = jdk_cache.lock().unwrap().as_ref() {
                if *k == key && at.elapsed() < JDK_TTL {
                    return v.clone();
                }
            }
            let env = crate::run::proc::process_env();
            let v = find_jdk(key.as_deref(), &env, cfg!(windows), &crate::run::android::deps::FsEnvDeps);
            *jdk_cache.lock().unwrap() = Some((key, Instant::now(), v.clone()));
            v
        });
        let b = bundle.clone();
        let mut dcfg = DaemonCfg::new(cfg.spawn.clone().unwrap_or_else(default_spawn), find_java.clone(), cfg.jar.clone(), Arc::new(move || b.dir()), cache_dir, win);
        let (state_emit, stage_emit) = (emit.clone(), emit.clone());
        let slot: Arc<OnceLock<Weak<ModelSvc>>> = Arc::new(OnceLock::new());
        let state_slot = slot.clone();
        dcfg.on_state = Arc::new(move |state, detail| {
            let mut ev = json!({"type": "daemon", "state": state});
            if let Some(d) = detail {
                ev["detail"] = json!(d);
            }
            state_emit(ev);
            if let Some(m) = state_slot.get().and_then(Weak::upgrade) {
                m.on_daemon_state(state);
            }
        });
        dcfg.on_stage = Arc::new(move |stage| stage_emit(json!({"type": "daemon-stage", "stage": stage})));
        let (home, platform) = (crate::run::ipc::home_dir(), cfg.platform.clone());
        let log: LogFn = match cfg.log_file.clone() {
            Some(file) => Arc::new(move |level, msg| append_log(&file, &home, &platform, level, msg)),
            None => Arc::new(|_, _| {}),
        };
        if cfg.log_file.is_some() {
            dcfg.log = log.clone();
        }
        if let Some(tune) = cfg.tune {
            tune(&mut dcfg);
        }
        let _ = std::fs::remove_dir_all(&snap_dir);
        let _ = std::fs::create_dir_all(&snap_dir);
        let daemon = LayoutDaemon::new(dcfg);
        let env = crate::run::proc::process_env();
        let home = crate::run::ipc::home_dir();
        let gradle_home = env.get("GRADLE_USER_HOME").map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).unwrap_or_else(|| join(&home, ".gradle"));
        let (can_bundle, can_jar) = (bundle.clone(), cfg.jar.clone());
        let model = ModelSvc::new(ModelCfg {
            models_dir: join(&base, "models"),
            init_script: cfg.init_script.clone(),
            win,
            env,
            home,
            settings: cfg.settings.clone(),
            loose_kit_dir: cfg.loose_kit_dir.clone(),
            gradle_home,
            aar_cache_dir: join(&base, "aars"),
            daemon: Arc::new(daemon.clone()) as Arc<dyn ModelDaemon>,
            emit: emit.clone(),
            log,
            can_run: Arc::new(move || ((!can_jar.is_empty() && Path::new(&can_jar).exists()) || can_bundle.kit_pending()) && can_bundle.is_ready()),
            timing: cfg.model_timing.clone().unwrap_or_default(),
        });
        let _ = slot.set(Arc::downgrade(&model));
        Arc::new(Svc {
            bundle,
            daemon,
            model,
            jar: cfg.jar,
            find_java,
            render_dir,
            snap_dir,
            win,
            cache: Mutex::new(RenderCache::new(RENDER_CACHE, Box::new(|v: &Cached| drop_snap(v)))),
            warm: Mutex::new(None),
        })
    }

    pub fn jar_exists(&self) -> bool {
        (!self.jar.is_empty() && Path::new(&self.jar).exists()) || self.bundle.kit_pending()
    }

    pub fn status(&self) -> Value {
        let state = self.daemon.state();
        json!({
            "bundle": self.bundle.status(),
            "jdk": (self.find_java)().public(),
            "daemon": state,
            "jar": self.jar_exists(),
            "apiLevel": self.daemon.api_level().unwrap_or(LAYOUTLIB_API_LEVEL),
            "stage": if state == "starting" { self.daemon.stage().map(Value::from).unwrap_or(Value::Null) } else { Value::Null },
        })
    }
}

fn drop_snap(v: &Cached) {
    if let Some(p) = &v.png {
        let _ = std::fs::remove_file(p);
    }
}

pub struct LazySvc {
    cell: std::sync::OnceLock<Arc<Svc>>,
    make: Box<dyn Fn() -> Arc<Svc> + Send + Sync>,
}

impl LazySvc {
    pub fn new(make: Box<dyn Fn() -> Arc<Svc> + Send + Sync>) -> Arc<LazySvc> {
        Arc::new(LazySvc { cell: std::sync::OnceLock::new(), make })
    }

    pub fn ready(svc: Arc<Svc>) -> Arc<LazySvc> {
        let cell = std::sync::OnceLock::new();
        let _ = cell.set(svc);
        Arc::new(LazySvc { cell, make: Box::new(|| unreachable!()) })
    }

    pub fn get(&self) -> &Arc<Svc> {
        self.cell.get_or_init(|| (self.make)())
    }

    pub fn kill_now(&self) {
        if let Some(s) = self.cell.get() {
            s.model.kill_now();
            s.daemon.kill_now();
        }
    }
}

pub fn svc_cfg_from_env(env: &DeskEnv, emitter: crate::Emitter, settings: Arc<SettingsStore>) -> SvcCfg {
    let var = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    SvcCfg {
        data_dir: env.data_dir.to_string_lossy().to_string(),
        jar: var("CCM_LAYOUT_JAR").unwrap_or_default(),
        override_dir: var("CCM_LAYOUT_OVERRIDE"),
        kit: var("CCM_LAYOUT_KIT").and_then(|v| serde_json::from_str::<Value>(&v).ok()).and_then(|v| KitSpec::from_value(&v)),
        log_file: var("CCM_LAYOUT_LOG"),
        loose_kit_dir: var("CCM_LAYOUT_LOOSEKIT"),
        init_script: var("CCM_LAYOUT_INIT").unwrap_or_default(),
        model_timing: None,
        settings,
        emit: Arc::new(move |v| emitter.emit(super::EVENT, v)),
        spawn: None,
        fetcher: None,
        platform: host_platform(),
        arch: host_arch(),
        tune: None,
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv, settings: Arc<SettingsStore>) {
    let (env, emitter) = (env.clone(), desk.emitter());
    let make: Box<dyn Fn() -> Arc<Svc> + Send + Sync> = Box::new(move || {
        Svc::new(svc_cfg_from_env(&env, emitter.clone(), settings.clone()))
    });
    let lazy = LazySvc::new(make);
    let closing = lazy.clone();
    desk.on_close(Arc::new(move || closing.kill_now()));
    super::model_ipc::register(desk, lazy.clone());
    super::ipc_render::register(desk, lazy);
}
