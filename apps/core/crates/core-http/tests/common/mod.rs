#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_http::{start, ClientMessage, Config, Hooks, RunningServer, StoreHandle};
use core_store::{Store, StoreOptions, TerminalSession};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub type Ws = WebSocketStream<TcpStream>;

pub fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ccm-http-{tag}-{}-{nanos}-{seq}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn dist_with_index(html: &str) -> PathBuf {
    let dist = temp_dir("dist");
    std::fs::write(dist.join("index.html"), html).unwrap();
    dist
}

pub fn new_store(batch_ms: i64) -> StoreHandle {
    StoreHandle::new(Store::new(StoreOptions { activity_limit: 10, batch_ms }))
}

pub fn terminal(id: &str, title: &str) -> TerminalSession {
    serde_json::from_value(serde_json::json!({ "id": id, "title": title, "status": "working" })).unwrap()
}

pub async fn serve(dist: PathBuf, tweak: impl FnOnce(&mut Config), hooks: Hooks) -> (RunningServer, StoreHandle) {
    let store = new_store(10);
    let mut config = Config::new(0, dist);
    tweak(&mut config);
    let server = start(config, store.clone(), hooks).await.unwrap();
    (server, store)
}

pub fn collecting() -> (Hooks, Arc<Mutex<Vec<ClientMessage>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let hooks = Hooks { on_client_message: Some(Arc::new(move |m, _| sink.lock().unwrap().push(m))), ..Hooks::default() };
    (hooks, got)
}

pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub async fn http(port: u16, method: &str, target: &str, headers: &[(&str, &str)], body: Option<&str>) -> Option<Reply> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let mut request = format!("{method} {target} HTTP/1.1\r\n");
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        request.push_str(&format!("Host: 127.0.0.1:{port}\r\n"));
    }
    for (k, v) in headers {
        request.push_str(&format!("{k}: {v}\r\n"));
    }
    if let Some(b) = body {
        request.push_str(&format!("Content-Length: {}\r\n", b.len()));
    }
    request.push_str("Connection: close\r\n\r\n");
    if let Some(b) = body {
        request.push_str(b);
    }
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw)).await;
    if raw.is_empty() {
        return None;
    }
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split(' ').nth(1)?.parse().ok()?;
    let headers = lines.filter_map(|l| l.split_once(": ")).map(|(k, v)| (k.to_owned(), v.to_owned())).collect();
    Some(Reply { status, headers, body: raw[split + 4..].to_vec() })
}

pub async fn get(port: u16, target: &str, headers: &[(&str, &str)]) -> Reply {
    http(port, "GET", target, headers, None).await.expect("a reply")
}

pub async fn ws_connect(port: u16, headers: &[(&str, &str)]) -> Result<Ws, String> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.map_err(|e| e.to_string())?;
    let mut request = format!("ws://127.0.0.1:{port}/ws").into_client_request().map_err(|e| e.to_string())?;
    for (k, v) in headers {
        request.headers_mut().insert(tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(k.as_bytes()).unwrap(), HeaderValue::from_str(v).unwrap());
    }
    let (ws, _) = tokio_tungstenite::client_async(request, stream).await.map_err(|e| e.to_string())?;
    Ok(ws)
}

pub async fn next_json(ws: &mut Ws) -> Value {
    use futures_util::StreamExt;
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next()).await.expect("a message in time").expect("open").expect("no error");
        if let Message::Text(t) = msg {
            return serde_json::from_str(t.as_str()).unwrap();
        }
    }
}

pub async fn send_json(ws: &mut Ws, value: Value) {
    use futures_util::SinkExt;
    ws.send(Message::text(value.to_string())).await.unwrap();
}

pub async fn wait_for(mut done: impl FnMut() -> bool) {
    for _ in 0..200 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition not met in time");
}
