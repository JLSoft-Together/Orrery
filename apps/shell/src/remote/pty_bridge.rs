use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep_until, timeout, Instant};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::ctx::{Ctx, Host};
use super::dashboard::error;

pub const ORIGIN: &str = "orrery-remote://gateway";
pub const HUB_TIMEOUT: Duration = Duration::from_secs(5);
pub const OUT_BATCH: Duration = Duration::from_millis(30);
pub const OUT_MAX: usize = 256 * 1024;
pub const SEND_WAIT: Duration = Duration::from_secs(2);
pub const MAX_BACKLOG: usize = 4 << 20;
pub const OVERFLOW: u16 = 4001;
const TO_HUB: usize = 64;

type Hub = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub enum ToHub {
    Input(Vec<u8>, bool),
    Resize(u16, u16),
}

pub enum FromHub {
    Json(u32, u64, Value),
    Out(u32, u64, Vec<u8>),
    Ended(u32, u64, bool),
}

pub struct Slot {
    ctx: Arc<Ctx>,
    device: String,
}

impl Slot {
    pub fn take(ctx: &Arc<Ctx>, device: &str) -> Option<Slot> {
        ctx.board.take_attach(device).then(|| Slot { ctx: ctx.clone(), device: device.to_owned() })
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.ctx.board.release_attach(&self.device);
    }
}

pub struct Attach {
    pub tx: mpsc::Sender<ToHub>,
    pub gen: u64,
    pub session: String,
    pub typed_at: Option<std::time::Instant>,
}

pub struct Spec {
    pub re: String,
    pub stream: u32,
    pub session: String,
    pub cols: u16,
    pub rows: u16,
    pub gen: u64,
    pub backlog: Arc<AtomicUsize>,
}

pub fn spawn(host: Arc<dyn Host>, spec: Spec, slot: Slot, back: mpsc::Sender<FromHub>) -> mpsc::Sender<ToHub> {
    let (tx, rx) = mpsc::channel(TO_HUB);
    tokio::spawn(async move {
        let (stream, gen) = (spec.stream, spec.gen);
        let stalled = run(host, spec, rx, &back).await;
        drop(slot);
        let _ = back.send(FromHub::Ended(stream, gen, stalled)).await;
    });
    tx
}

async fn connect(host: Arc<dyn Host>) -> Option<Hub> {
    let ep = tokio::task::spawn_blocking(move || host.pty()).await.ok()??;
    let mut req = ep.url.as_str().into_client_request().ok()?;
    req.headers_mut().insert("origin", HeaderValue::from_static(ORIGIN));
    req.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_str(&ep.protocol).ok()?);
    let (ws, _) = timeout(HUB_TIMEOUT, tokio_tungstenite::connect_async(req)).await.ok()?.ok()?;
    Some(ws)
}

fn text_of(msg: &Message) -> Option<Value> {
    match msg {
        Message::Text(t) => serde_json::from_str(t.as_str()).ok(),
        _ => None,
    }
}

async fn ready(ws: &mut Hub) -> Result<(), &'static str> {
    let deadline = Instant::now() + HUB_TIMEOUT;
    loop {
        let msg = match timeout(deadline.saturating_duration_since(Instant::now()), ws.next()).await {
            Ok(Some(Ok(m))) => m,
            Ok(_) => return Err("offline"),
            Err(_) => return Err("timeout"),
        };
        match text_of(&msg).as_ref().and_then(|v| v["t"].as_str().map(str::to_owned)).as_deref() {
            Some("ready") => return Ok(()),
            Some("gone") => return Err("notFound"),
            Some("error") => return Err("internal"),
            _ => {}
        }
    }
}

struct Pump<'a> {
    back: &'a mpsc::Sender<FromHub>,
    stream: u32,
    gen: u64,
    size: (u16, u16),
    announced: bool,
    buf: Vec<u8>,
    backlog: Arc<AtomicUsize>,
    stalled: bool,
}

impl Pump<'_> {
    async fn put(&mut self, msg: FromHub) -> bool {
        match timeout(SEND_WAIT, self.back.send(msg)).await {
            Ok(Ok(())) => true,
            Ok(Err(_)) => false,
            Err(_) => {
                self.stalled = true;
                false
            }
        }
    }

    async fn json(&mut self, v: Value) -> bool {
        self.put(FromHub::Json(self.stream, self.gen, v)).await
    }

    async fn announce(&mut self, size: (u16, u16)) -> bool {
        self.announced = true;
        self.json(json!({ "t": "pty.ready", "stream": self.stream, "cols": size.0, "rows": size.1 })).await
    }

    async fn flush(&mut self) -> bool {
        if self.buf.is_empty() {
            return true;
        }
        if !self.announced && !self.announce(self.size).await {
            return false;
        }
        let len = self.buf.len();
        if self.backlog.fetch_add(len, Ordering::SeqCst) + len > MAX_BACKLOG {
            self.backlog.fetch_sub(len, Ordering::SeqCst);
            self.stalled = true;
            return false;
        }
        let bytes = std::mem::take(&mut self.buf);
        if self.put(FromHub::Out(self.stream, self.gen, bytes)).await {
            return true;
        }
        self.backlog.fetch_sub(len, Ordering::SeqCst);
        false
    }

    async fn text(&mut self, v: &Value) -> Option<bool> {
        match v["t"].as_str()? {
            "resized" => {
                let size = (v["cols"].as_u64().unwrap_or(80) as u16, v["rows"].as_u64().unwrap_or(24) as u16);
                if !self.flush().await {
                    return Some(false);
                }
                if !self.announced {
                    return Some(self.announce(size).await);
                }
                Some(self.json(json!({ "t": "pty.resized", "stream": self.stream, "cols": size.0, "rows": size.1 })).await)
            }
            "exit" => {
                if self.flush().await {
                    let _ = self.json(json!({ "t": "pty.exit", "stream": self.stream, "code": v["code"] })).await;
                }
                Some(false)
            }
            _ => None,
        }
    }

    async fn closed(&mut self, code: Option<u16>) {
        if !self.flush().await {
            return;
        }
        let event = if code == Some(OVERFLOW) { json!({ "t": "pty.overflow", "stream": self.stream }) } else { json!({ "t": "pty.exit", "stream": self.stream, "code": Value::Null }) };
        let _ = self.json(event).await;
    }
}

async fn fail(back: &mpsc::Sender<FromHub>, spec: &Spec, code: &str) {
    let _ = timeout(SEND_WAIT, back.send(FromHub::Json(spec.stream, spec.gen, error(Value::from(spec.re.as_str()), code)))).await;
}

async fn run(host: Arc<dyn Host>, spec: Spec, mut rx: mpsc::Receiver<ToHub>, back: &mpsc::Sender<FromHub>) -> bool {
    let Some(mut ws) = connect(host).await else {
        fail(back, &spec, "offline").await;
        return false;
    };
    let attach = json!({ "t": "attach", "id": spec.session, "mode": "view", "cols": spec.cols, "rows": spec.rows });
    if ws.send(Message::Text(attach.to_string().into())).await.is_err() {
        fail(back, &spec, "offline").await;
        return false;
    }
    if let Err(code) = ready(&mut ws).await {
        fail(back, &spec, code).await;
        return false;
    }
    if rx.is_closed() {
        let _ = ws.close(None).await;
        return false;
    }
    let mut pump = Pump { back, stream: spec.stream, gen: spec.gen, size: (spec.cols, spec.rows), announced: false, buf: Vec::new(), backlog: spec.backlog.clone(), stalled: false };
    if !pump.json(json!({ "re": spec.re, "ok": true, "stream": spec.stream })).await {
        return pump.stalled;
    }
    let mut flush_at: Option<Instant> = None;
    loop {
        tokio::select! {
            msg = ws.next() => match msg {
                Some(Ok(Message::Binary(b))) => {
                    pump.buf.extend_from_slice(&b);
                    if pump.buf.len() >= OUT_MAX {
                        flush_at = None;
                        if !pump.flush().await {
                            break;
                        }
                    } else if flush_at.is_none() {
                        flush_at = Some(Instant::now() + OUT_BATCH);
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    pump.closed(frame.map(|f| u16::from(f.code))).await;
                    break;
                }
                Some(Ok(m)) => {
                    let Some(v) = text_of(&m) else { continue };
                    if pump.text(&v).await == Some(false) {
                        break;
                    }
                }
                _ => {
                    pump.closed(None).await;
                    break;
                }
            },
            _ = async { sleep_until(flush_at.unwrap_or_else(Instant::now)).await }, if flush_at.is_some() => {
                flush_at = None;
                if !pump.flush().await {
                    break;
                }
            }
            cmd = rx.recv() => {
                let sent = match cmd {
                    Some(ToHub::Input(bytes, claim)) => {
                        let claimed = !claim || ws.send(Message::Text(json!({ "t": "claim" }).to_string().into())).await.is_ok();
                        claimed && ws.send(Message::Binary(bytes.into())).await.is_ok()
                    }
                    Some(ToHub::Resize(cols, rows)) => ws.send(Message::Text(json!({ "t": "resize", "cols": cols, "rows": rows }).to_string().into())).await.is_ok(),
                    None => break,
                };
                if !sent {
                    pump.closed(None).await;
                    break;
                }
            }
        }
    }
    let _ = timeout(SEND_WAIT, ws.close(None)).await;
    pump.stalled
}
