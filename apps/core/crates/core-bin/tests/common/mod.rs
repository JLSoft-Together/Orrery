#![allow(dead_code)]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

pub const ORIGIN: &str = "http://127.0.0.1:4317";
pub const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub struct Core {
    pub child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::Receiver<Value>,
    backlog: Vec<Value>,
    pub port: u16,
    pub stderr: Arc<Mutex<String>>,
}

impl Core {
    pub fn start() -> Core {
        let mut child = Command::new(env!("CARGO_BIN_EXE_orrery-core")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if tx.send(v).is_err() {
                        break;
                    }
                }
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let sink = stderr.clone();
        let mut err = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = err.read(&mut buf) {
                if n == 0 {
                    break;
                }
                sink.lock().unwrap().push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
        let stdin = child.stdin.take();
        let mut core = Core { child, stdin, rx, backlog: Vec::new(), port: 0, stderr };
        core.send(json!({ "t": "init", "token": TOKEN, "origins": [ORIGIN], "appVersion": "0.0.0-test" }));
        let l = core.next(|m| m["t"] == "listening", 10_000);
        core.port = l["port"].as_u64().unwrap() as u16;
        core
    }

    pub fn send(&mut self, v: Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{v}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    pub fn next(&mut self, pred: impl Fn(&Value) -> bool, ms: u64) -> Value {
        self.try_next(&pred, ms).unwrap_or_else(|| panic!("no control message in {ms} ms; backlog={:?} stderr={}", self.backlog, self.stderr.lock().unwrap()))
    }

    pub fn try_next(&mut self, pred: &dyn Fn(&Value) -> bool, ms: u64) -> Option<Value> {
        if let Some(i) = self.backlog.iter().position(pred) {
            return Some(self.backlog.remove(i));
        }
        let end = Instant::now() + Duration::from_millis(ms);
        loop {
            let left = end.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(v) if pred(&v) => return Some(v),
                Ok(v) => self.backlog.push(v),
                Err(_) => return None,
            }
        }
    }

    pub fn seen(&self, pred: impl Fn(&Value) -> bool) -> bool {
        self.backlog.iter().any(pred)
    }

    pub fn wait_exit(&mut self, ms: u64) -> ExitStatus {
        let end = Instant::now() + Duration::from_millis(ms);
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                return s;
            }
            if Instant::now() > end {
                let _ = self.child.kill();
                panic!("orrery-core did not exit; stderr={}", self.stderr.lock().unwrap());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn resolved(&mut self, rid: &Value, id: &str, file: &str, args: &[&str], cwd: &str) {
        self.send(json!({ "t": "resolved", "rid": rid, "ok": true, "id": id, "file": file, "args": args, "env": child_env(), "cwd": cwd }));
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn child_env() -> Value {
    let mut env = serde_json::Map::new();
    for (k, v) in std::env::vars() {
        if !k.starts_with("CCM_") && !k.starts_with("CARGO") && !k.starts_with("RUST") {
            env.insert(k, Value::String(v));
        }
    }
    env.insert("ORRERY_TOKEN".into(), Value::String("c".repeat(64)));
    Value::Object(env)
}

pub fn temp_dir(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("ccm-core-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap().to_string_lossy().trim_start_matches(r"\\?\").to_owned()
}

pub struct Client {
    pub ws: WebSocket<TcpStream>,
    pub texts: Vec<Value>,
    pub output: Vec<u8>,
    pub close: Option<u16>,
}

impl Client {
    pub fn connect(port: u16) -> Client {
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_nodelay(true).unwrap();
        let mut req = format!("ws://127.0.0.1:{port}/pty").into_client_request().unwrap();
        req.headers_mut().insert("origin", HeaderValue::from_static(ORIGIN));
        req.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_str(&format!("ccm.pty.{TOKEN}")).unwrap());
        let (ws, resp) = tungstenite::client::client_with_config(req, stream, Some(WebSocketConfig::default())).unwrap();
        assert_eq!(resp.headers().get("sec-websocket-protocol").unwrap(), &format!("ccm.pty.{TOKEN}"));
        ws.get_ref().set_read_timeout(Some(Duration::from_millis(25))).unwrap();
        Client { ws, texts: Vec::new(), output: Vec::new(), close: None }
    }

    pub fn send(&mut self, v: Value) {
        self.ws.send(Message::Text(v.to_string().into())).unwrap();
    }

    pub fn send_bytes(&mut self, b: &[u8]) {
        self.ws.send(Message::Binary(b.to_vec().into())).unwrap();
    }

    pub fn pump(&mut self) {
        if self.close.is_some() {
            return;
        }
        loop {
            match self.ws.read() {
                Ok(Message::Text(t)) => self.texts.push(serde_json::from_str(t.as_str()).unwrap()),
                Ok(Message::Binary(b)) => self.output.extend_from_slice(&b),
                Ok(Message::Close(f)) => {
                    self.close = Some(f.map(|f| u16::from(f.code)).unwrap_or(1005));
                    return;
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => return,
                Err(_) => {
                    self.close = Some(1006);
                    return;
                }
            }
        }
    }

    pub fn until<T>(&mut self, ms: u64, check: impl Fn(&Client) -> Option<T>) -> T {
        let end = Instant::now() + Duration::from_millis(ms);
        loop {
            self.pump();
            if let Some(v) = check(self) {
                return v;
            }
            if Instant::now() > end {
                panic!("timeout; texts={:?} output={:?} close={:?}", self.texts, String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(300)..]), self.close);
            }
        }
    }

    pub fn text(&mut self, t: &str) -> Value {
        self.until(10_000, |c| c.texts.iter().find(|m| m["t"] == t).cloned())
    }

    pub fn out_has(&mut self, needle: &str, ms: u64) {
        let n = needle.as_bytes().to_vec();
        self.until(ms, |c| c.output.windows(n.len()).any(|w| w == n.as_slice()).then_some(()));
    }

    pub fn out_str(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }
}

pub fn handshake_status(port: u16, host: &str, origin: Option<&str>, protocol: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let origin = origin.map(|o| format!("Origin: {o}\r\n")).unwrap_or_default();
    let req = format!(
        "GET /pty HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n{origin}Sec-WebSocket-Protocol: {protocol}\r\n\r\n"
    );
    s.write_all(req.as_bytes()).unwrap();
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).unwrap();
    line.trim().to_owned()
}

pub fn node() -> String {
    "node".to_owned()
}

pub fn pid_alive(pid: u64) -> bool {
    #[cfg(windows)]
    {
        let out = Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
    }
    #[cfg(unix)]
    {
        Command::new("kill").args(["-0", &pid.to_string()]).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    }
}

pub fn wait_dead(pid: u64, ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !pid_alive(pid)
}

pub fn kill_pid(pid: u64) {
    #[cfg(windows)]
    let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    #[cfg(unix)]
    let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
}
