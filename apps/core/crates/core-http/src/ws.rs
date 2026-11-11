use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_store::{MonitorEvent, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;

use crate::client::{parse_client_message, ClientMessage};
use crate::http::head_text;
use crate::server::{Hooks, StoreHandle};

pub const MAX_PAYLOAD: usize = 128 * 1024;

pub type Outbox = UnboundedSender<Arc<str>>;

pub type TapFn = Arc<dyn Fn(Arc<str>) + Send + Sync>;

#[derive(Clone)]
enum Sink {
    Channel(Outbox),
    Callback(TapFn),
}

#[derive(Clone)]
pub struct ClientContext {
    sink: Sink,
    pub clients: usize,
    pub client_id: u64,
}

impl ClientContext {
    pub(crate) fn channel(tx: Outbox, clients: usize, client_id: u64) -> ClientContext {
        ClientContext { sink: Sink::Channel(tx), clients, client_id }
    }

    pub(crate) fn callback(reply: TapFn, clients: usize, client_id: u64) -> ClientContext {
        ClientContext { sink: Sink::Callback(reply), clients, client_id }
    }

    pub fn reply(&self, event: MonitorEvent) {
        if let Ok(json) = serde_json::to_string(&ServerMessage::direct(event)) {
            let json: Arc<str> = json.into();
            match &self.sink {
                Sink::Channel(tx) => {
                    let _ = tx.send(json);
                }
                Sink::Callback(f) => f(json),
            }
        }
    }
}

#[derive(Default)]
pub struct Clients {
    map: Mutex<HashMap<u64, Outbox>>,
    taps: Mutex<HashMap<u64, TapFn>>,
    next: AtomicU64,
    next_tap: AtomicU64,
}

impl Clients {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Outbox>> {
        self.map.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn next_id(&self) -> u64 {
        self.next.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn add(&self, id: u64, tx: Outbox) -> usize {
        let mut map = self.lock();
        map.insert(id, tx);
        map.len()
    }

    pub fn remove(&self, id: u64) -> usize {
        let mut map = self.lock();
        map.remove(&id);
        map.len()
    }

    fn lock_taps(&self) -> std::sync::MutexGuard<'_, HashMap<u64, TapFn>> {
        self.taps.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn add_tap(&self, f: TapFn) -> u64 {
        let id = self.next_tap.fetch_add(1, Ordering::SeqCst) + 1;
        self.lock_taps().insert(id, f);
        id
    }

    pub(crate) fn remove_tap(&self, id: u64) {
        self.lock_taps().remove(&id);
    }

    pub fn broadcast(&self, message: &ServerMessage) -> Result<(), String> {
        let taps: Vec<TapFn> = self.lock_taps().values().cloned().collect();
        let map = self.lock();
        if map.is_empty() && taps.is_empty() {
            return Ok(());
        }
        let data: Arc<str> = serde_json::to_string(message).map_err(|e| e.to_string())?.into();
        for tx in map.values() {
            let _ = tx.send(data.clone());
        }
        drop(map);
        for f in taps {
            let data = data.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || f(data)));
        }
        Ok(())
    }
}

pub struct Session {
    pub store: StoreHandle,
    pub clients: Arc<Clients>,
    pub hooks: Hooks,
    pub heartbeat: Duration,
    pub shutdown: watch::Receiver<bool>,
}

fn config() -> WebSocketConfig {
    WebSocketConfig::default().max_message_size(Some(MAX_PAYLOAD)).max_frame_size(Some(MAX_PAYLOAD))
}

fn snapshot_json(store: &StoreHandle) -> Option<Arc<str>> {
    store.with(|s| serde_json::to_string(&s.snapshot_message()).ok()).map(Arc::from)
}

fn handle(session: &Session, tx: &Outbox, client_id: u64, text: &str) {
    let Ok(value) = serde_json::from_str::<Value>(text) else { return };
    match parse_client_message(&value) {
        Some(ClientMessage::Resync) => {
            let (store, tx) = (session.store.clone(), tx.clone());
            tokio::task::spawn_blocking(move || {
                if let Some(json) = snapshot_json(&store) {
                    let _ = tx.send(json);
                }
            });
        }
        Some(message) => {
            if let Some(hook) = &session.hooks.on_client_message {
                hook(message, &ClientContext::channel(tx.clone(), session.clients.len(), client_id));
            }
        }
        None => {}
    }
}

async fn linger(stream: &mut TcpStream) {
    let _ = stream.shutdown().await;
    let mut sink = [0u8; 8192];
    let drain = async { while matches!(stream.read(&mut sink).await, Ok(n) if n > 0) {} };
    let _ = tokio::time::timeout(Duration::from_secs(1), drain).await;
}

pub async fn run(mut stream: TcpStream, key: &str, protocol: Option<&str>, session: Session) {
    let mut shutdown = session.shutdown.clone();
    let mut headers = vec![("upgrade", "websocket".to_owned()), ("connection", "Upgrade".to_owned()), ("sec-websocket-accept", derive_accept_key(key.as_bytes()))];
    if let Some(p) = protocol {
        headers.push(("sec-websocket-protocol", p.to_owned()));
    }
    if stream.write_all(head_text(101, &headers, None).as_bytes()).await.is_err() {
        return;
    }
    let mut ws = WebSocketStream::from_raw_socket(stream, Role::Server, Some(config())).await;

    let (tx, mut rx) = unbounded_channel::<Arc<str>>();
    let client_id = session.clients.next_id();
    let (store, clients, first) = (session.store.clone(), session.clients.clone(), tx.clone());
    let joined = tokio::task::spawn_blocking(move || {
        store.with(|s| {
            s.flush();
            let count = clients.add(client_id, first.clone());
            match serde_json::to_string(&s.snapshot_message()) {
                Ok(json) => {
                    let _ = first.send(json.into());
                }
                Err(e) => eprintln!("[collector] snapshot: {e}"),
            }
            count
        })
    })
    .await;
    let Ok(count) = joined else { return };
    if let Some(hook) = &session.hooks.on_client_count {
        hook(count);
    }

    let mut alive = true;
    let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + session.heartbeat, session.heartbeat);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            incoming = ws.next() => match incoming {
                Some(Ok(Message::Text(text))) => handle(&session, &tx, client_id, text.as_str()),
                Some(Ok(Message::Binary(bytes))) => handle(&session, &tx, client_id, &String::from_utf8_lossy(&bytes)),
                Some(Ok(Message::Pong(_))) => alive = true,
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(WsError::Capacity(_))) => {
                    let _ = ws.close(Some(CloseFrame { code: CloseCode::Size, reason: "Max payload size exceeded".into() })).await;
                    linger(ws.get_mut()).await;
                    break;
                }
                Some(Err(_)) => break,
            },
            outgoing = rx.recv() => match outgoing {
                Some(data) => {
                    if ws.send(Message::text(&*data)).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
            _ = ticker.tick() => {
                if !alive {
                    break;
                }
                alive = false;
                if ws.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            },
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    let _ = ws.close(Some(CloseFrame { code: CloseCode::Away, reason: "".into() })).await;
                    break;
                }
            },
        }
    }
    let remaining = session.clients.remove(client_id);
    if let Some(hook) = &session.hooks.on_client_close {
        hook(client_id);
    }
    if let Some(hook) = &session.hooks.on_client_count {
        hook(remaining);
    }
}
