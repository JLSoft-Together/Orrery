use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_remote_proto::frame::{Frame, Kind};
use core_remote_proto::pair::{PairInitiator, QrPayload};
use core_remote_proto::session::SessionInitiator;
use core_remote_proto::{StaticKeypair, Transport};
use core_serve::StoreTap;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::config::{new_machine_id, now_secs, Config};
use super::ctx::{Host, Welcome};
use super::{Options, Remote};
use crate::state::lock;
use super::keys::{KeyStore, Read};
use zeroize::Zeroizing;

pub type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
pub struct TestHost {
    pub answer: AtomicBool,
    pub env_confirm: AtomicBool,
    pub asked: Mutex<Vec<String>>,
    pub tap: Mutex<Option<StoreTap>>,
    pub notes: Mutex<Vec<String>>,
    pub awake: Mutex<Vec<bool>>,
    pub changes: AtomicUsize,
}

impl Host for TestHost {
    fn confirm_pair(&self, name: &str) -> bool {
        lock(&self.asked).push(name.to_owned());
        if self.env_confirm.load(Ordering::SeqCst) {
            return std::env::var("CCM_E2E_CONFIRM").is_ok_and(|v| v == "ok");
        }
        self.answer.load(Ordering::SeqCst)
    }
    fn notify(&self, title: &str, body: &str) {
        lock(&self.notes).push(format!("{title}: {body}"));
    }
    fn tap(&self) -> Option<StoreTap> {
        lock(&self.tap).clone()
    }
    fn welcome(&self) -> Welcome {
        Welcome { version: "9.9.9".into(), theme: json!({ "mode": "dark", "palette": { "accent": "#0a84ff" } }), locale: "en".into() }
    }
    fn changed(&self, _status: Value) {
        self.changes.fetch_add(1, Ordering::SeqCst);
    }
    fn awake(&self, on: bool) {
        lock(&self.awake).push(on);
    }
}

#[derive(Default, Clone)]
pub struct MemSecrets(pub Arc<Mutex<HashMap<String, String>>>, pub Arc<AtomicBool>);

impl KeyStore for MemSecrets {
    fn read(&self, name: &str) -> Read {
        if self.1.load(Ordering::SeqCst) {
            return Read::Failed;
        }
        match lock(&self.0).get(name) {
            Some(v) => Read::Found(Zeroizing::new(v.clone())),
            None => Read::Missing,
        }
    }
    fn write(&self, name: &str, value: &str) -> bool {
        lock(&self.0).insert(name.into(), value.into());
        true
    }
}

pub fn temp_dir(tag: &str) -> PathBuf {
    let mut raw = [0u8; 6];
    getrandom::fill(&mut raw).unwrap();
    let dir = std::env::temp_dir().join(format!("ccm-remote-{tag}-{}", raw.iter().map(|b| format!("{b:02x}")).collect::<String>()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn prepare(dir: &Path, relay: &str, registered: bool) {
    let mut cfg = Config { machine_id: new_machine_id().unwrap(), ..Config::default() };
    cfg.relay.url = relay.to_owned();
    cfg.relay.registered = registered;
    assert!(cfg.save(dir));
}

pub struct Rig {
    pub host: Arc<TestHost>,
    pub secrets: MemSecrets,
    pub dir: PathBuf,
    pub remote: Remote,
}

pub fn rig(tag: &str, relay: &str, registered: bool) -> Rig {
    let dir = temp_dir(tag);
    prepare(&dir, relay, registered);
    let host = Arc::new(TestHost::default());
    let secrets = MemSecrets::default();
    let opts = Options { addrs: Some(vec![Ipv4Addr::LOCALHOST]), port: Some(0), mdns: false, name: Some("Test PC".into()) };
    let remote = Remote::new(host.clone(), Box::new(secrets.clone()), dir.clone(), opts);
    Rig { host, secrets, dir, remote }
}

impl Rig {
    pub fn addr(&self) -> SocketAddr {
        self.remote.bound()[0]
    }

    pub fn desktop_key(&self) -> [u8; 32] {
        *self.remote.ctx().unwrap().keys.noise.public()
    }
}

pub struct Phone {
    pub key: StaticKeypair,
    pub id: String,
}

impl Phone {
    pub fn new(id: &str) -> Phone {
        Phone { key: StaticKeypair::generate().unwrap(), id: id.to_owned() }
    }
}

pub async fn connect(addr: SocketAddr) -> Ws {
    tokio_tungstenite::connect_async(format!("ws://{addr}/remote")).await.unwrap().0
}

pub async fn next_binary(ws: &mut Ws) -> Result<Vec<u8>, Option<u16>> {
    loop {
        match tokio::time::timeout(Duration::from_secs(10), ws.next()).await {
            Ok(Some(Ok(Message::Binary(b)))) => return Ok(b.to_vec()),
            Ok(Some(Ok(Message::Close(f)))) => return Err(f.map(|f| u16::from(f.code))),
            Ok(Some(Ok(_))) => continue,
            _ => return Err(None),
        }
    }
}

pub async fn close_code(ws: &mut Ws) -> Option<u16> {
    loop {
        match next_binary(ws).await {
            Ok(_) => continue,
            Err(code) => return code,
        }
    }
}

pub fn open_json(t: &mut Transport, bytes: &[u8]) -> Option<Value> {
    let frame = Frame::decode(&t.open(bytes).ok()?).ok()?;
    (frame.kind == Kind::Json).then(|| serde_json::from_slice(&frame.payload).ok()).flatten()
}

pub fn seal_json(t: &mut Transport, v: &Value) -> Vec<u8> {
    t.seal(&Frame::new(Kind::Json, 0, serde_json::to_vec(v).unwrap()).encode().unwrap()).unwrap()
}

pub async fn recv_json(ws: &mut Ws, t: &mut Transport) -> Result<Value, Option<u16>> {
    loop {
        let bytes = next_binary(ws).await?;
        if let Some(v) = open_json(t, &bytes) {
            return Ok(v);
        }
    }
}

pub async fn send_json(ws: &mut Ws, t: &mut Transport, v: Value) {
    ws.send(Message::Binary(seal_json(t, &v).into())).await.unwrap();
}

pub async fn pair(addr: SocketAddr, qr: &QrPayload, phone: &Phone) -> Result<Value, Option<u16>> {
    let mut ws = connect(addr).await;
    let (init, msg1) = PairInitiator::start(&phone.key, qr, &phone.id, "Pixel Test", now_secs()).unwrap();
    ws.send(Message::Binary(msg1.into())).await.unwrap();
    let msg2 = next_binary(&mut ws).await?;
    let (mut desk, msg3) = init.finish(&msg2).map_err(|_| None)?;
    ws.send(Message::Binary(msg3.into())).await.unwrap();
    let reply = recv_json(&mut ws, &mut desk.transport).await;
    let _ = close_code(&mut ws).await;
    reply
}

pub async fn session(addr: SocketAddr, desktop: &[u8; 32], phone: &Phone) -> Result<(Ws, Transport), Option<u16>> {
    let mut ws = connect(addr).await;
    let (init, msg1) = SessionInitiator::start(&phone.key, desktop, &phone.id).unwrap();
    ws.send(Message::Binary(msg1.into())).await.unwrap();
    let msg2 = next_binary(&mut ws).await?;
    let t = init.finish(&msg2).map_err(|_| None)?;
    Ok((ws, t))
}

pub async fn hello(ws: &mut Ws, t: &mut Transport) -> Value {
    send_json(ws, t, json!({ "id": "h1", "t": "hello", "proto": 1, "app": "test", "platform": "android", "caps": [] })).await;
    recv_json(ws, t).await.unwrap()
}

pub fn decode_qr(v: &Value) -> QrPayload {
    QrPayload::decode(v["qr"].as_str().unwrap()).unwrap()
}

pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap()
}
