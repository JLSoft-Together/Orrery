use super::cds::{finish_dump, plan_cds, snapshot_class_list, CdsInput, CdsPlan, Record};
use super::jdk::Jdk;
use super::proc::{daemon_args, kill_pid, run_dump, Spawn};
use super::rpc::{FrameParser, RpcClient, RpcFail};
use super::{tail_chars, LayoutError};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

const PROTOCOL_VERSION: u64 = 1;
const CANCELLED: i64 = -32800;

pub type LogFn = Arc<dyn Fn(&str, &str) + Send + Sync>;
pub type StateFn = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;

pub struct DaemonCfg {
    pub spawn: Spawn,
    pub find_java: Arc<dyn Fn() -> Jdk + Send + Sync>,
    pub daemon_jar: String,
    pub layoutlib_dir: Arc<dyn Fn() -> String + Send + Sync>,
    pub cache_dir: String,
    pub win: bool,
    pub log: LogFn,
    pub on_state: StateFn,
    pub on_stage: Arc<dyn Fn(&str) + Send + Sync>,
    pub render_timeout: Duration,
    pub init_timeout: Duration,
    pub idle: Duration,
    pub max_restarts: usize,
    pub restart_window: Duration,
    pub cds: bool,
    pub cds_dump_delay: Duration,
}

impl DaemonCfg {
    pub fn new(spawn: Spawn, find_java: Arc<dyn Fn() -> Jdk + Send + Sync>, daemon_jar: String, layoutlib_dir: Arc<dyn Fn() -> String + Send + Sync>, cache_dir: String, win: bool) -> Self {
        DaemonCfg {
            spawn,
            find_java,
            daemon_jar,
            layoutlib_dir,
            cache_dir,
            win,
            log: Arc::new(|_, _| {}),
            on_state: Arc::new(|_, _| {}),
            on_stage: Arc::new(|_| {}),
            render_timeout: Duration::from_secs(15),
            init_timeout: Duration::from_secs(60),
            idle: Duration::from_secs(600),
            max_restarts: 3,
            restart_window: Duration::from_secs(300),
            cds: true,
            cds_dump_delay: Duration::from_secs(30),
        }
    }
}

struct Live {
    pid: u32,
    rpc: Arc<RpcClient>,
    stopping: AtomicBool,
    exited: Mutex<bool>,
    exited_cv: Condvar,
}

struct St {
    state: &'static str,
    restarts: Vec<Instant>,
    seq: u64,
    slots: HashMap<String, String>,
    in_flight: usize,
    restart_pending: bool,
    api: Option<u64>,
    stage: Option<String>,
    live: Option<Arc<Live>>,
    dump_active: bool,
    dumping: Option<u32>,
    idle_at: Option<Instant>,
}

pub struct Inner {
    cfg: DaemonCfg,
    st: Mutex<St>,
    start_lock: Mutex<()>,
    idle_cv: Condvar,
    cancel_dump: AtomicBool,
}

#[derive(Clone)]
pub struct LayoutDaemon(pub Arc<Inner>);

fn crashed(d: &str) -> LayoutError {
    if d.is_empty() {
        LayoutError::new("crashed")
    } else {
        LayoutError::with("crashed", d)
    }
}

impl LayoutDaemon {
    pub fn new(cfg: DaemonCfg) -> Self {
        let st = St { state: "stopped", restarts: vec![], seq: 0, slots: HashMap::new(), in_flight: 0, restart_pending: false, api: None, stage: None, live: None, dump_active: false, dumping: None, idle_at: None };
        let inner = Arc::new(Inner { cfg, st: Mutex::new(st), start_lock: Mutex::new(()), idle_cv: Condvar::new(), cancel_dump: AtomicBool::new(false) });
        let weak = Arc::downgrade(&inner);
        std::thread::spawn(move || idle_watch(weak));
        LayoutDaemon(inner)
    }

    pub fn state(&self) -> &'static str {
        self.0.st.lock().unwrap().state
    }

    pub fn stage(&self) -> Option<String> {
        self.0.st.lock().unwrap().stage.clone()
    }

    pub fn api_level(&self) -> Option<u64> {
        self.0.st.lock().unwrap().api
    }

    fn set(&self, state: &'static str, detail: Option<&str>) {
        {
            let mut st = self.0.st.lock().unwrap();
            if st.state == state && detail.is_none() {
                return;
            }
            st.state = state;
        }
        (self.0.cfg.on_state)(state, detail);
    }

    fn to_stage(&self, stage: &str) {
        self.0.st.lock().unwrap().stage = Some(stage.to_string());
        (self.0.cfg.on_stage)(stage);
    }

    fn budget_left(&self) -> bool {
        let mut st = self.0.st.lock().unwrap();
        let since = Instant::now().checked_sub(self.0.cfg.restart_window);
        st.restarts.retain(|t| since.is_none_or(|s| *t > s));
        st.restarts.len() < self.0.cfg.max_restarts
    }

    fn log(&self, level: &str, msg: &str) {
        (self.0.cfg.log)(level, msg)
    }

    pub fn ensure(&self, prefetch: Option<&Value>) -> Result<(), LayoutError> {
        self.ensure_live(prefetch).map(|_| ())
    }

    fn ensure_live(&self, prefetch: Option<&Value>) -> Result<Arc<Live>, LayoutError> {
        let _g = self.0.start_lock.lock().unwrap();
        if let Some(l) = self.0.st.lock().unwrap().live.clone().filter(|l| !l.stopping.load(Ordering::SeqCst)) {
            return Ok(l);
        }
        if self.state() == "failed" && !self.budget_left() {
            return Err(LayoutError::new("crash-loop"));
        }
        self.start(prefetch)
    }

    fn start(&self, prefetch: Option<&Value>) -> Result<Arc<Live>, LayoutError> {
        self.set("starting", None);
        let cfg = &self.0.cfg;
        let (java, home) = match (cfg.find_java)() {
            Jdk::Ok { java, home, .. } => (java, home),
            Jdk::Bad { code, detail } => {
                self.set("stopped", None);
                return Err(LayoutError { code: code.to_string(), detail });
            }
        };
        let dir = (cfg.layoutlib_dir)();
        let t0 = Instant::now();
        let plan = if cfg.cds { plan_cds(&CdsInput { jdk_home: &home, cache_dir: &cfg.cache_dir, daemon_jar: &cfg.daemon_jar, layoutlib_dir: &dir, win: cfg.win }) } else { None };
        let _ = std::fs::create_dir_all(&cfg.cache_dir);
        let args = daemon_args(&cfg.daemon_jar, &dir, cfg.win, plan.as_ref().map(CdsPlan::jvm_args).unwrap_or(&[]));
        let mut child = match (cfg.spawn)(&java, &args, &cfg.cache_dir) {
            Ok(c) => c,
            Err(e) => {
                self.set("stopped", None);
                return Err(LayoutError::with("spawn-failed", e.to_string()));
            }
        };
        let (Some(stdin), Some(stdout), Some(stderr)) = (child.stdin.take(), child.stdout.take(), child.stderr.take()) else {
            crate::layout::proc::kill_pid(child.id());
            self.set("stopped", None);
            return Err(LayoutError::with("spawn-failed", "no stdio"));
        };
        let me = self.clone();
        let notify = Box::new(move |method: &str, params: &Value| {
            if method == "progress" {
                if let Some(stage @ ("setup" | "prepare" | "warmup")) = params.get("stage").and_then(Value::as_str) {
                    me.to_stage(stage);
                }
            } else if method == "log" {
                let level = params.get("level").and_then(Value::as_str).unwrap_or("info");
                let msg = params.get("message").and_then(Value::as_str).unwrap_or("");
                me.log(level, &crate::js::slice(msg, 2000));
            }
        });
        let rpc = Arc::new(RpcClient::new(Box::new(stdin), notify));
        self.to_stage("jvm");
        let pid = child.id();
        let live = Arc::new(Live { pid, rpc: rpc.clone(), stopping: AtomicBool::new(false), exited: Mutex::new(false), exited_cv: Condvar::new() });
        self.0.st.lock().unwrap().live = Some(live.clone());
        self.pump(stdout, stderr, child, &live);
        let timeout = cfg.init_timeout;
        let mut params = json!({"protocolVersion": PROTOCOL_VERSION, "cacheDir": cfg.cache_dir});
        if let Some(a) = prefetch.and_then(Value::as_array).filter(|a| !a.is_empty()) {
            params["prefetchAars"] = Value::Array(a.clone());
        }
        let res = match rpc.request("initialize", params, timeout) {
            Ok(v) => v,
            Err(RpcFail::Timeout) => {
                kill_pid(pid);
                return Err(LayoutError::new("timeout"));
            }
            Err(RpcFail::Closed(d)) => return Err(LayoutError { code: "init-failed".into(), detail: Some(d) }),
            Err(RpcFail::Rpc { message, .. }) => {
                live.stopping.store(true, Ordering::SeqCst);
                kill_pid(pid);
                return Err(LayoutError::with("init-failed", message));
            }
        };
        if !self.0.st.lock().unwrap().live.as_ref().is_some_and(|l| Arc::ptr_eq(l, &live)) {
            return Err(LayoutError::new("crashed"));
        }
        if let Some(api) = res.get("apiLevel").and_then(Value::as_u64).filter(|a| *a > 0) {
            self.0.st.lock().unwrap().api = Some(api);
        }
        let show = |k: &str| res.get(k).map(|v| v.as_str().map(String::from).unwrap_or_else(|| crate::js::stringify(v))).unwrap_or_else(|| "?".into());
        let timings = res.get("timings").map(crate::js::stringify).unwrap_or_else(|| "{}".into());
        self.log("info", &format!("initialized layoutlib {} in {} ms (spawn -> ready {} ms) {timings}", show("layoutlibVersion"), show("initMs"), t0.elapsed().as_millis()));
        self.0.st.lock().unwrap().stage = None;
        self.set("ready", None);
        if let Some(CdsPlan::Record(rec)) = plan {
            self.schedule_dump(rec, java);
        }
        self.touch();
        Ok(live)
    }

    fn pump(&self, mut stdout: impl Read + Send + 'static, mut stderr: impl Read + Send + 'static, mut child: std::process::Child, live: &Arc<Live>) {
        let (rpc, pid) = (live.rpc.clone(), live.pid);
        let me = self.clone();
        std::thread::spawn(move || {
            let mut parser = FrameParser::default();
            let mut buf = vec![0u8; 64 * 1024];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                match parser.push(&buf[..n]) {
                    Ok(msgs) => msgs.iter().for_each(|m| rpc.handle(m)),
                    Err(e) => {
                        me.log("error", &format!("protocol: {e}"));
                        kill_pid(pid);
                        break;
                    }
                }
            }
        });
        let tail = Arc::new(Mutex::new(String::new()));
        let (t, me) = (tail.clone(), self.clone());
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 8192];
            while let Ok(n) = stderr.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                {
                    let mut g = t.lock().unwrap();
                    g.push_str(&text);
                    *g = tail_chars(&g, 4000);
                }
                text.lines().filter(|l| !l.trim().is_empty()).for_each(|l| me.log("stderr", &crate::js::slice(l, 2000)));
            }
        });
        let (me, live) = (self.clone(), live.clone());
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            std::thread::sleep(Duration::from_millis(20));
            let t = tail.lock().unwrap().clone();
            me.on_exit(&live, code, &t);
        });
    }

    fn on_exit(&self, live: &Arc<Live>, code: Option<i32>, err_tail: &str) {
        let detail = tail_chars(err_tail, 600);
        live.rpc.fail_all(&if detail.is_empty() { format!("exit {}", code.map(|c| c.to_string()).unwrap_or_else(|| "null".into())) } else { detail });
        *live.exited.lock().unwrap() = true;
        live.exited_cv.notify_all();
        {
            let mut st = self.0.st.lock().unwrap();
            if !st.live.as_ref().is_some_and(|l| Arc::ptr_eq(l, live)) {
                return;
            }
            st.live = None;
            st.idle_at = None;
        }
        if live.stopping.load(Ordering::SeqCst) {
            return self.set("stopped", None);
        }
        self.log("error", &format!("daemon exited with {}", code.map(|c| c.to_string()).unwrap_or_else(|| "null".into())));
        self.0.st.lock().unwrap().restarts.push(Instant::now());
        if !self.budget_left() {
            return self.set("failed", Some("crash-loop"));
        }
        self.set("stopped", Some("crashed"));
        let me = self.clone();
        std::thread::spawn(move || {
            let _ = me.ensure(None);
        });
    }

    fn map_rpc(&self, f: RpcFail, rpc_code: &str, live: &Live, what: &str) -> LayoutError {
        match f {
            RpcFail::Timeout => {
                self.log("error", &format!("{what} exceeded its deadline, restarting the daemon"));
                kill_pid(live.pid);
                LayoutError::new("timeout")
            }
            RpcFail::Rpc { code, .. } if code == CANCELLED && rpc_code == "render-failed" => LayoutError::new("cancelled"),
            RpcFail::Rpc { message, .. } => LayoutError::with(rpc_code, message),
            RpcFail::Closed(d) => crashed(&d),
        }
    }

    pub fn render(&self, params: &Value, slot: &str) -> Result<Value, LayoutError> {
        self.slotted("render", slot, |id| {
            let mut p = json!({"requestId": id, "layoutPath": params["layoutPath"], "xmlContent": params["xmlContent"], "device": params["device"], "night": params["night"]});
            for k in ["theme", "locale"] {
                if params[k].as_str().is_some_and(|s| !s.is_empty()) {
                    p[k] = params[k].clone();
                }
            }
            if params["scale"].as_f64().is_some_and(|s| s < 1.0) {
                p["scale"] = params["scale"].clone();
            }
            p
        })
    }

    pub fn render_drawable(&self, params: &Value, slot: &str) -> Result<Value, LayoutError> {
        self.slotted("drawable/render", slot, |id| {
            let mut p = json!({"requestId": id});
            for (k, v) in params.as_object().into_iter().flatten() {
                p[k] = v.clone();
            }
            p
        })
    }

    fn slotted(&self, method: &str, slot: &str, payload: impl Fn(&str) -> Value) -> Result<Value, LayoutError> {
        let id = {
            let mut st = self.0.st.lock().unwrap();
            st.seq += 1;
            let id = format!("r{}", st.seq);
            let prev = st.slots.insert(slot.to_string(), id.clone());
            st.in_flight += 1;
            st.idle_at = None;
            (id, prev)
        };
        let (id, prev) = id;
        let current = |me: &Self| me.0.st.lock().unwrap().slots.get(slot) == Some(&id);
        let result = (|| {
            let live = self.ensure_live(None)?;
            if let Some(p) = prev {
                let rpc = live.rpc.clone();
                std::thread::spawn(move || {
                    let _ = rpc.request("cancel", json!({"requestId": p}), Duration::from_secs(5));
                });
            }
            if !current(self) {
                return Err(LayoutError::new("cancelled"));
            }
            let res = live.rpc.request(method, payload(&id), self.0.cfg.render_timeout).map_err(|f| self.map_rpc(f, "render-failed", &live, &format!("render {id}")))?;
            if !current(self) {
                return Err(LayoutError::new("cancelled"));
            }
            Ok(res)
        })();
        {
            let mut st = self.0.st.lock().unwrap();
            st.in_flight -= 1;
            if st.slots.get(slot) == Some(&id) {
                st.slots.remove(slot);
            }
        }
        self.touch();
        self.maybe_restart();
        result
    }

    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, LayoutError> {
        {
            let mut st = self.0.st.lock().unwrap();
            st.in_flight += 1;
            st.idle_at = None;
        }
        let result = (|| {
            let aars = if method == "project/load" { params.get("aarResDirs") } else { None };
            let live = self.ensure_live(aars)?;
            live.rpc.request(method, params.clone(), timeout).map_err(|f| self.map_rpc(f, "request-failed", &live, method))
        })();
        self.0.st.lock().unwrap().in_flight -= 1;
        self.touch();
        self.maybe_restart();
        result
    }

    fn schedule_dump(&self, rec: Record, java: String) {
        {
            let mut st = self.0.st.lock().unwrap();
            if st.dump_active {
                return;
            }
            st.dump_active = true;
        }
        self.0.cancel_dump.store(false, Ordering::SeqCst);
        let me = self.clone();
        std::thread::spawn(move || {
            let until = Instant::now() + me.0.cfg.cds_dump_delay;
            while Instant::now() < until && !me.0.cancel_dump.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(20));
            }
            if !me.0.cancel_dump.load(Ordering::SeqCst) {
                me.dump(&rec, &java);
            }
            me.0.st.lock().unwrap().dump_active = false;
        });
    }

    fn dump(&self, rec: &Record, java: &str) {
        if !snapshot_class_list(rec) {
            return;
        }
        let t0 = Instant::now();
        let me = self.clone();
        let code = run_dump(&self.0.cfg.spawn, java, &self.0.cfg.cache_dir, rec, Duration::from_secs(180), &move |pid| me.0.st.lock().unwrap().dumping = Some(pid));
        self.0.st.lock().unwrap().dumping = None;
        let ok = finish_dump(rec, code == Some(0));
        let state = if ok { "written".to_string() } else { format!("failed (exit {})", code.map(|c| c.to_string()).unwrap_or_else(|| "null".into())) };
        self.log("info", &format!("AppCDS archive {state} in {} ms", t0.elapsed().as_millis()));
    }

    pub fn request_restart(&self) {
        self.0.st.lock().unwrap().restart_pending = true;
        self.maybe_restart();
    }

    fn maybe_restart(&self) {
        {
            let mut st = self.0.st.lock().unwrap();
            if !st.restart_pending || st.in_flight > 0 {
                return;
            }
            st.restart_pending = false;
            if !st.live.as_ref().is_some_and(|l| !l.stopping.load(Ordering::SeqCst)) {
                return;
            }
        }
        self.log("info", "restarting the daemon (restartSuggested)");
        let me = self.clone();
        std::thread::spawn(move || {
            me.stop();
            let _ = me.ensure(None);
        });
    }

    fn touch(&self) {
        let mut st = self.0.st.lock().unwrap();
        st.idle_at = None;
        if st.live.is_some() && st.in_flight == 0 {
            st.idle_at = Some(Instant::now() + self.0.cfg.idle);
        }
        drop(st);
        self.0.idle_cv.notify_all();
    }

    pub fn stop(&self) {
        let live = {
            let mut st = self.0.st.lock().unwrap();
            st.idle_at = None;
            st.live.clone()
        };
        let Some(live) = live.filter(|l| !l.stopping.swap(true, Ordering::SeqCst)) else { return };
        let _ = live.rpc.request("shutdown", Value::Null, Duration::from_secs(2));
        live.rpc.close_write();
        let mut exited = live.exited.lock().unwrap();
        if *exited {
            return;
        }
        let (g, res) = live.exited_cv.wait_timeout_while(exited, Duration::from_secs(3), |e| !*e).unwrap();
        exited = g;
        if res.timed_out() && !*exited {
            kill_pid(live.pid);
        }
    }

    pub fn kill_now(&self) {
        self.0.cancel_dump.store(true, Ordering::SeqCst);
        let (live, dumping) = {
            let mut st = self.0.st.lock().unwrap();
            st.idle_at = None;
            (st.live.clone(), st.dumping)
        };
        if let Some(pid) = dumping {
            kill_pid(pid);
        }
        if let Some(l) = live {
            l.stopping.store(true, Ordering::SeqCst);
            kill_pid(l.pid);
        }
    }
}

fn idle_watch(weak: Weak<Inner>) {
    loop {
        let Some(inner) = weak.upgrade() else { return };
        let due = {
            let st = inner.st.lock().unwrap();
            let wait = st.idle_at.map(|t| t.saturating_duration_since(Instant::now())).unwrap_or(Duration::from_millis(500)).min(Duration::from_millis(500));
            let (mut st, _) = inner.idle_cv.wait_timeout(st, wait).unwrap();
            match st.idle_at {
                Some(t) if t <= Instant::now() => {
                    st.idle_at = None;
                    true
                }
                _ => false,
            }
        };
        if due {
            LayoutDaemon(inner).stop();
        }
    }
}
