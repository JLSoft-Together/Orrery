use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::{Message, WebSocket};

const KEY: &str = "serve-test-key";

fn temp(tag: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ccm-serve-{tag}-{}-{}-{seq}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    for sub in ["claude/sessions", "claude/projects", "data", "codex", "codewhale", "web"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("web/index.html"), "<!doctype html>ok").unwrap();
    std::fs::write(dir.join("res.json"), r#"{"procs":[],"ports":{}}"#).unwrap();
    dir
}

struct Serve {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: mpsc::Receiver<Value>,
    backlog: Vec<Value>,
}

fn spawn(dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Serve {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_orrery-core"));
    cmd.arg("serve").args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for k in ["CCM_APP_KEY", "CCM_PORT", "CCM_HOST_RPC", "CCM_TOAST", "CCM_TOAST_BACKEND", "CCM_TOAST_LINK", "CCM_SERVE_STATS"] {
        cmd.env_remove(k);
    }
    let p = |s: &str| dir.join(s).to_string_lossy().into_owned();
    cmd.env("CLAUDE_CONFIG_DIR", p("claude")).env("CCM_DATA_DIR", p("data")).env("CODEX_HOME", p("codex")).env("CODEWHALE_HOME", p("codewhale"));
    cmd.env("CCM_WEB_DIST", p("web")).env("CCM_RES_FIXTURE", p("res.json")).env("CCM_VERIFY_PROCESSES", "0").env("CCM_PORT", "0");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    let out = child.stdout.take().unwrap();
    let (tx, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines() {
            let Ok(line) = line else { break };
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let _ = tx.send(v);
            }
        }
    });
    let mut err = child.stderr.take().unwrap();
    std::thread::spawn(move || {
        let mut sink = Vec::new();
        let _ = err.read_to_end(&mut sink);
    });
    let stdin = child.stdin.take();
    Serve { child, stdin, lines, backlog: Vec::new() }
}

impl Serve {
    fn next(&mut self, pred: impl Fn(&Value) -> bool, ms: u64) -> Option<Value> {
        if let Some(i) = self.backlog.iter().position(&pred) {
            return Some(self.backlog.remove(i));
        }
        let end = Instant::now() + Duration::from_millis(ms);
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            match self.lines.recv_timeout(left) {
                Ok(v) if pred(&v) => return Some(v),
                Ok(v) => self.backlog.push(v),
                Err(_) => return None,
            }
        }
        None
    }

    fn port(&mut self) -> u16 {
        let l = self.next(|v| v["type"] == "listening", 15_000).expect("listening");
        l["port"].as_u64().unwrap() as u16
    }

    fn send(&mut self, v: Value) {
        let s = self.stdin.as_mut().unwrap();
        writeln!(s, "{v}").unwrap();
        s.flush().unwrap();
    }

    fn wait_exit(&mut self, ms: u64) -> Option<i32> {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            if let Some(s) = self.child.try_wait().unwrap() {
                return s.code();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn ws(port: u16) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let mut req = format!("ws://127.0.0.1:{port}/ws").into_client_request().unwrap();
    req.headers_mut().insert("x-ccm-app", HeaderValue::from_static(KEY));
    let (socket, _) = tungstenite::client(req, stream).unwrap();
    socket
}

fn read_event(socket: &mut WebSocket<TcpStream>, kind: &str, ms: u64) -> Option<Value> {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        match socket.read() {
            Ok(Message::Text(t)) => {
                let v: Value = serde_json::from_str(&t).unwrap();
                if let Some(e) = v["events"].as_array().and_then(|es| es.iter().find(|e| e["type"] == kind)) {
                    return Some(e.clone());
                }
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => panic!("ws: {e}"),
        }
    }
    None
}

fn send_ws(socket: &mut WebSocket<TcpStream>, v: Value) {
    socket.send(Message::Text(v.to_string().into())).unwrap();
}

#[test]
fn without_an_app_key_serve_exits_1_unless_dev() {
    let dir = temp("nokey");
    let mut s = spawn(&dir, &[], &[]);
    assert_eq!(s.wait_exit(10_000), Some(1));
    assert!(s.next(|v| v["type"] == "listening", 100).is_none());
    let mut dev = spawn(&dir, &[], &["--dev"]);
    assert_ne!(dev.port(), 0);
}

#[test]
fn a_bad_port_is_an_error_and_port_0_picks_a_free_port() {
    let dir = temp("port");
    for bad in ["nope", "70000", "-1"] {
        let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY), ("CCM_PORT", bad)], &[]);
        assert_eq!(s.wait_exit(10_000), Some(2), "{bad}");
        assert!(s.next(|v| v["type"] == "listening", 100).is_none(), "{bad}");
    }
    let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY)], &[]);
    let port = s.port();
    assert!(port != 0 && port != 4317);
}

#[test]
fn a_taken_port_exits_3_and_stdin_eof_exits_0() {
    let dir = temp("taken");
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port().to_string();
    let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY), ("CCM_PORT", &port)], &[]);
    assert_eq!(s.wait_exit(15_000), Some(3));
    let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY)], &[]);
    s.port();
    s.stdin.take();
    assert_eq!(s.wait_exit(15_000), Some(0));
}

#[test]
fn an_open_folder_dialog_does_not_block_other_requests() {
    let dir = temp("pick");
    let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY), ("CCM_HOST_RPC", "1")], &[]);
    let port = s.port();
    assert_eq!(s.next(|v| v["type"] == "busy", 10_000).unwrap(), json!({"type": "busy", "agents": 0, "jobs": 0}));
    let mut socket = ws(port);
    assert!(read_event(&mut socket, "snapshot", 10_000).is_some());
    send_ws(&mut socket, json!({"type": "folder.pick", "requestId": "r1"}));
    let req = s.next(|v| v["type"] == "host.req", 10_000).expect("host.req");
    assert_eq!(req["method"], "pickFolder");
    send_ws(&mut socket, json!({"type": "history.get"}));
    assert!(read_event(&mut socket, "history.data", 3_000).is_some(), "history.get waited for the dialog");
    let picked = dir.join("claude").to_string_lossy().into_owned();
    s.send(json!({"type": "host.res", "id": req["id"], "result": {"result": "ok", "dir": picked}}));
    let e = read_event(&mut socket, "folder.picked", 5_000).expect("folder.picked");
    assert_eq!(e["payload"], json!({"requestId": "r1", "result": "ok", "dir": picked}));
    s.send(json!({"type": "shutdown"}));
    assert_eq!(s.wait_exit(15_000), Some(0));
}

#[test]
fn toasts_and_main_messages_go_over_stdio() {
    let dir = temp("toast");
    let mut s = spawn(&dir, &[("CCM_APP_KEY", KEY), ("CCM_HOST_RPC", "1"), ("CCM_TOAST_BACKEND", "electron"), ("CCM_TOAST_LINK", "agent-monitor://")], &[]);
    let port = s.port();
    let mut socket = ws(port);
    assert!(read_event(&mut socket, "snapshot", 10_000).is_some());
    s.send(json!({"type": "device.used", "serial": "emulator-5554", "at": 5}));
    s.send(json!({"type": "toast.health", "health": "appOff"}));
    send_ws(&mut socket, json!({"type": "notify.test"}));
    let r = read_event(&mut socket, "notify.testResult", 5_000).expect("notify.testResult");
    assert_eq!(r["payload"]["result"], "ok");
    let shown = s.next(|v| v["type"] == "toast.show", 5_000).expect("toast.show");
    assert_eq!(shown["persistent"], false);
    assert_eq!(shown.get("body").is_some(), cfg!(windows));
    if cfg!(windows) {
        assert!(shown["url"].as_str().unwrap().starts_with("agent-monitor://focus?k="));
    }
    send_ws(&mut socket, json!({"type": "office.enable", "on": true}));
    send_ws(&mut socket, json!({"type": "history.get"}));
    assert!(read_event(&mut socket, "history.data", 3_000).is_some());
    s.stdin.take();
    assert_eq!(s.wait_exit(15_000), Some(0));
}
