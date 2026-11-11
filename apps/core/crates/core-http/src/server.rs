use std::future::Future;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::Duration;

use core_store::event::FocusWindowResult;
use core_store::time::now_ms;
use core_store::Store;
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;

use crate::client::ClientMessage;
use crate::http::{read_body, read_head, write_response, HeadError, Request, Response};
use crate::js::len16;
use crate::cookie::{authorized, authorized_ws, login};
use crate::origin::is_local_request;
use crate::query::{js_number, Params};
use crate::static_files::{serve as serve_static, url_pathname};
use crate::tap::StoreTap;
use crate::ws::{self, ClientContext, Clients, Session};

pub const PORT_IN_USE_EXIT: i32 = 3;
const FOCUS_BODY_MAX: usize = 2048;
const BUSY_RETRY_MS: i64 = 10;
const DENIED: &str = "Orrery only opens in its desktop app.";

pub type MessageHook = Arc<dyn Fn(ClientMessage, &ClientContext) + Send + Sync>;
pub type SnoozeHook = Arc<dyn Fn(&str, Option<&str>, f64) -> bool + Send + Sync>;
pub type FocusFuture = Pin<Box<dyn Future<Output = Option<FocusWindowResult>> + Send>>;

#[derive(Clone, Default)]
pub struct Hooks {
    pub on_client_message: Option<MessageHook>,
    pub on_client_close: Option<Arc<dyn Fn(u64) + Send + Sync>>,
    pub on_client_count: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    pub on_focus_run: Option<Arc<dyn Fn(String, String) -> FocusFuture + Send + Sync>>,
    pub on_snooze_link: Option<SnoozeHook>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub web_dist: PathBuf,
    pub app_key: Option<String>,
    pub dev_origin_ports: Vec<u16>,
    pub heartbeat: Duration,
}

impl Config {
    pub fn new(port: u16, web_dist: impl Into<PathBuf>) -> Config {
        Config { host: "127.0.0.1".into(), port, web_dist: web_dist.into(), app_key: None, dev_origin_ports: Vec::new(), heartbeat: Duration::from_secs(30) }
    }
}

#[derive(Debug)]
pub enum StartError {
    PortInUse(std::io::Error),
    NotLoopback(String),
    Io(std::io::Error),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::PortInUse(e) => write!(f, "port in use: {e}"),
            StartError::NotLoopback(h) => write!(f, "refusing to bind a non-loopback host: {h}"),
            StartError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StartError {}

#[derive(Clone)]
pub struct StoreHandle {
    inner: Arc<Mutex<Store>>,
    wake: Arc<Notify>,
    seq: Arc<AtomicI64>,
}

impl StoreHandle {
    pub fn new(store: Store) -> StoreHandle {
        let seq = Arc::new(AtomicI64::new(store.seq()));
        StoreHandle { inner: Arc::new(Mutex::new(store)), wake: Arc::new(Notify::new()), seq }
    }

    fn lock(&self) -> MutexGuard<'_, Store> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut Store) -> R) -> R {
        let mut guard = self.lock();
        let result = f(&mut guard);
        self.seq.store(guard.seq(), Ordering::SeqCst);
        drop(guard);
        self.wake.notify_one();
        result
    }

    pub fn try_with<R>(&self, f: impl FnOnce(&mut Store) -> R) -> Option<R> {
        let mut guard = match self.inner.try_lock() {
            Ok(g) => g,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        let result = f(&mut guard);
        self.seq.store(guard.seq(), Ordering::SeqCst);
        Some(result)
    }

    pub fn seq(&self) -> i64 {
        self.seq.load(Ordering::SeqCst)
    }
}

struct Shared {
    store: StoreHandle,
    clients: Arc<Clients>,
    hooks: Hooks,
    web_dist: PathBuf,
    app_key: Option<String>,
    ports: Vec<u16>,
    heartbeat: Duration,
    shutdown: watch::Receiver<bool>,
}

pub struct RunningServer {
    port: u16,
    shutdown: watch::Sender<bool>,
    store: StoreHandle,
    listener_id: u64,
    tasks: Vec<JoinHandle<()>>,
    tap: StoreTap,
}

impl RunningServer {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn tap(&self) -> StoreTap {
        self.tap.clone()
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        self.store.with(|s| s.unsubscribe(self.listener_id));
        for task in self.tasks {
            let _ = task.await;
        }
    }
}

fn bind_address(host: &str) -> Result<IpAddr, StartError> {
    let ip: IpAddr = if host == "localhost" { IpAddr::from([127, 0, 0, 1]) } else { host.parse().map_err(|_| StartError::NotLoopback(host.to_owned()))? };
    if ip.is_loopback() {
        Ok(ip)
    } else {
        Err(StartError::NotLoopback(host.to_owned()))
    }
}

pub async fn start(config: Config, store: StoreHandle, hooks: Hooks) -> Result<RunningServer, StartError> {
    let ip = bind_address(&config.host)?;
    let listener = TcpListener::bind((ip, config.port)).await.map_err(|e| if e.kind() == std::io::ErrorKind::AddrInUse { StartError::PortInUse(e) } else { StartError::Io(e) })?;
    let port = listener.local_addr().map_err(StartError::Io)?.port();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let clients = Arc::new(Clients::default());
    let sink = clients.clone();
    let listener_id = store.with(|s| s.subscribe(Box::new(move |m| sink.broadcast(m))));
    let tap = StoreTap::new(store.clone(), clients.clone(), hooks.on_client_message.clone());
    let mut ports = vec![port];
    ports.extend(config.dev_origin_ports.iter().copied());
    let shared = Arc::new(Shared {
        store: store.clone(),
        clients,
        hooks,
        web_dist: config.web_dist,
        app_key: config.app_key,
        ports,
        heartbeat: config.heartbeat,
        shutdown: shutdown_rx.clone(),
    });
    let accept = tokio::spawn(accept_loop(listener, shared, shutdown_rx.clone()));
    let flush = tokio::spawn(flush_loop(store.clone(), shutdown_rx));
    Ok(RunningServer { port, shutdown: shutdown_tx, store, listener_id, tasks: vec![accept, flush], tap })
}

async fn flush_loop(store: StoreHandle, mut shutdown: watch::Receiver<bool>) {
    loop {
        let next = store.try_with(|s| s.next_flush_at()).unwrap_or_else(|| Some(now_ms() + BUSY_RETRY_MS));
        let wait = next.map(|at| Duration::from_millis((at - now_ms()).max(0) as u64));
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            _ = store.wake.notified() => {}
            _ = async {
                match wait {
                    Some(d) => tokio::time::sleep(d).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                store.try_with(|s| s.poll(now_ms()));
            }
        }
    }
}

async fn accept_loop(listener: TcpListener, shared: Arc<Shared>, mut shutdown: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    tokio::spawn(connection(stream, shared.clone()));
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            },
        }
    }
}

async fn refuse(stream: &mut TcpStream, status: u16) {
    let _ = write_response(stream, Response::empty(status), false).await;
}

async fn connection(mut stream: TcpStream, shared: Arc<Shared>) {
    let _ = stream.set_nodelay(true);
    let request = match read_head(&mut stream).await {
        Ok(r) => r,
        Err(HeadError::TooLarge) => return refuse(&mut stream, 431).await,
        Err(HeadError::Bad) => return refuse(&mut stream, 400).await,
        Err(HeadError::Closed) => return,
    };
    if request.wants_upgrade() {
        return upgrade(stream, request, shared).await;
    }
    let head_only = request.method == "HEAD";
    if let Some(response) = route(&mut stream, &request, &shared).await {
        let _ = write_response(&mut stream, response, head_only).await;
    }
}

fn local(request: &Request, shared: &Shared) -> bool {
    is_local_request(request.header("host"), request.header("origin"), &shared.ports)
}

fn keyed(request: &Request, shared: &Shared) -> bool {
    authorized(request.header("x-ccm-app"), request.header("sec-fetch-site"), request.headers_all("cookie"), shared.app_key.as_deref())
}

fn ws_keyed(request: &Request, shared: &Shared) -> bool {
    authorized_ws(request.header("x-ccm-app"), request.header("sec-fetch-site"), request.header("origin"), request.headers_all("cookie"), shared.app_key.as_deref())
}

async fn upgrade(mut stream: TcpStream, request: Request, shared: Arc<Shared>) {
    let allowed = url_pathname(request.path()) == "/ws" && local(&request, &shared) && ws_keyed(&request, &shared);
    if !allowed {
        return;
    }
    let is_websocket = request.header("upgrade").is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
    let key = request.header("sec-websocket-key").filter(|k| !k.is_empty());
    let (true, Some(key), Some("13")) = (is_websocket && request.method == "GET", key, request.header("sec-websocket-version")) else {
        return refuse(&mut stream, 400).await;
    };
    let protocol = request.headers_all("sec-websocket-protocol").flat_map(|v| v.split(',')).map(str::trim).find(|p| !p.is_empty());
    let session = Session { store: shared.store.clone(), clients: shared.clients.clone(), hooks: shared.hooks.clone(), heartbeat: shared.heartbeat, shutdown: shared.shutdown.clone() };
    ws::run(stream, key, protocol, session).await;
}

async fn route(stream: &mut TcpStream, request: &Request, shared: &Shared) -> Option<Response> {
    if !local(request, shared) {
        return Some(Response::empty(403));
    }
    if request.target == "/health" {
        return Some(Response::json(200, json!({ "ok": true, "seq": shared.store.seq() }).to_string()));
    }
    if request.method == "GET" && request.path() == "/" {
        if let Some(response) = login(request.query(), shared.app_key.as_deref()) {
            return Some(response);
        }
    }
    if !keyed(request, shared) {
        return Some(Response::text(403, "text/plain; charset=utf-8", DENIED));
    }
    if request.target.starts_with("/snooze?") && request.method == "GET" {
        let ok = snooze(request, shared);
        return Some(Response::empty(if ok { 204 } else { 404 }).header("cache-control", "no-store"));
    }
    if request.target == "/focus" && request.method == "POST" {
        let body = read_body(stream, request, FOCUS_BODY_MAX).await?;
        return Some(focus(&body, shared).await);
    }
    Some(serve_static(&shared.web_dist, request.path()).await)
}

fn snooze(request: &Request, shared: &Shared) -> bool {
    let params = Params::parse(request.query());
    let Some(terminal_id) = params.get("t").filter(|t| !t.is_empty() && len16(t) <= 256) else { return false };
    let minutes = params.get("m").and_then(js_number).unwrap_or(f64::NAN);
    if !(minutes.is_finite() && minutes.fract() == 0.0 && minutes > 0.0) {
        return false;
    }
    shared.hooks.on_snooze_link.as_ref().is_some_and(|hook| hook(terminal_id, params.get("k"), minutes))
}

async fn focus(body: &[u8], shared: &Shared) -> Response {
    let parsed: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
    let terminal = parsed.get("t").and_then(|v| v.as_str()).filter(|t| len16(t) <= 256);
    let key = parsed.get("k").and_then(|v| v.as_str());
    let result = match (terminal, key, &shared.hooks.on_focus_run) {
        (Some(t), Some(k), Some(hook)) => hook(t.to_owned(), k.to_owned()).await,
        _ => None,
    };
    let value = result.and_then(|r| serde_json::to_value(r).ok());
    let status = if value.is_some() { 200 } else { 404 };
    let payload = json!({ "result": value.unwrap_or_else(|| json!("rejected")) });
    Response::json(status, payload.to_string()).header("cache-control", "no-store")
}
