use std::io::{BufRead, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rust_lib_orrery_remote::api::remote::{
    decode_qr, generate_static_key, pair_start, session_start, SecureChannel,
};
use serde_json::{json, Value};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;
struct Fail(String);

impl<E: std::fmt::Debug> From<E> for Fail {
    fn from(error: E) -> Self {
        let text = format!("{error:?}");
        Self(text.trim_matches('"').to_owned())
    }
}

const POLL: Duration = Duration::from_millis(40);
const PING_EVERY: Duration = Duration::from_secs(20);

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Result<Vec<u8>, Fail> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(Into::into))
        .collect()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn state_file(dir: &Path) -> PathBuf {
    dir.join("phone.json")
}

fn load_state(dir: &Path) -> Result<Value, Fail> {
    Ok(serde_json::from_slice(&std::fs::read(state_file(dir))?)?)
}

fn connect(url: &str, timeout: Duration) -> Result<Ws, Fail> {
    let (ws, _) = tungstenite::connect(url)?;
    if let MaybeTlsStream::Plain(stream) = ws.get_ref() {
        stream.set_read_timeout(Some(timeout))?;
        stream.set_nodelay(true)?;
    }
    Ok(ws)
}

enum Inbound {
    Binary(Vec<u8>),
    Closed(Option<u16>),
    Idle,
}

fn read_one(ws: &mut Ws) -> Result<Inbound, Fail> {
    match ws.read() {
        Ok(Message::Binary(bytes)) => Ok(Inbound::Binary(bytes.to_vec())),
        Ok(Message::Close(frame)) => Ok(Inbound::Closed(frame.map(|f| u16::from(f.code)))),
        Ok(_) => Ok(Inbound::Idle),
        Err(tungstenite::Error::Io(e))
            if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) =>
        {
            Ok(Inbound::Idle)
        }
        Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => Ok(Inbound::Closed(None)),
        Err(e) => Err(e.into()),
    }
}

fn send_sealed(ws: &mut Ws, channel: &SecureChannel, kind: u8, stream: u32, payload: Vec<u8>, claim: bool) -> Result<(), Fail> {
    for sealed in channel.seal(kind, stream, payload, claim)? {
        ws.send(Message::Binary(sealed.into()))?;
    }
    Ok(())
}

fn url_of(state: &Value, args: &[String]) -> Result<String, Fail> {
    if let Some(url) = option(args, "--url") {
        return Ok(url);
    }
    let host = state["host"].as_str().ok_or("no host in state, pass --url")?;
    let port = state["port"].as_u64().ok_or("no port in state, pass --url")?;
    Ok(format!("ws://{host}:{port}/remote"))
}

fn pair(args: &[String]) -> Result<(), Fail> {
    let qr_text = args.get(1).ok_or("usage: pair <qr> --state <dir> [--url <ws>] [--name <n>]")?.clone();
    let dir = PathBuf::from(option(args, "--state").ok_or("--state required")?);
    std::fs::create_dir_all(&dir)?;
    let name = option(args, "--name").unwrap_or_else(|| "Phone CLI".into());
    let qr = decode_qr(qr_text.clone())?;
    let url = match option(args, "--url") {
        Some(url) => url,
        None => format!(
            "ws://{}:{}/remote",
            qr.host.clone().ok_or("QR has no host")?,
            qr.port.ok_or("QR has no port")?
        ),
    };
    let private = generate_static_key()?;
    let device_id = format!("e2e-{}", hex(&private[..6]));
    let started = pair_start(private.clone(), qr_text, device_id.clone(), name.clone(), now())?;
    let mut ws = connect(&url, Duration::from_secs(30))?;
    ws.send(Message::Binary(started.msg1.into()))?;
    let deadline = Instant::now() + Duration::from_secs(150);
    let msg2 = loop {
        match read_one(&mut ws)? {
            Inbound::Binary(bytes) => break bytes,
            Inbound::Closed(code) => {
                emit(json!({"ev": "closed", "stage": "msg2", "code": code}));
                return Err("closed before msg2".into());
            }
            Inbound::Idle if Instant::now() > deadline => return Err("msg2 timeout".into()),
            Inbound::Idle => {}
        }
    };
    let done = started.handshake.finish(msg2)?;
    ws.send(Message::Binary(done.msg3.into()))?;
    let reply = loop {
        match read_one(&mut ws)? {
            Inbound::Binary(bytes) => {
                if let Some(message) = done.channel.open(bytes)? {
                    if message.kind == 0 {
                        break serde_json::from_slice::<Value>(&message.payload)?;
                    }
                }
            }
            Inbound::Closed(code) => {
                emit(json!({"ev": "closed", "stage": "pair", "code": code}));
                return Err("closed before pair reply".into());
            }
            Inbound::Idle if Instant::now() > deadline => return Err("pair reply timeout".into()),
            Inbound::Idle => {}
        }
    };
    if reply["t"] == "pair.ok" {
        let state = json!({
            "private": hex(&private),
            "deviceId": device_id,
            "name": name,
            "machine": done.machine_id,
            "desktopKey": hex(&done.desktop_key),
            "host": qr.host,
            "port": qr.port,
            "relay": reply["relay"].as_str().map(str::to_owned).or(qr.relay),
            "relayToken": reply["relayToken"],
            "relayExp": reply["relayExp"],
        });
        std::fs::write(state_file(&dir), serde_json::to_vec_pretty(&state)?)?;
        emit(json!({"ev": "paired", "reply": reply, "deviceId": device_id, "machine": state["machine"]}));
        Ok(())
    } else {
        emit(json!({"ev": "pair-failed", "reply": reply}));
        Err("pair not accepted".into())
    }
}

fn feed_stdin(tx: mpsc::Sender<Value>) {
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(line.trim()) {
                if tx.send(value).is_err() {
                    return;
                }
            }
        }
        let _ = tx.send(json!({"op": "quit"}));
    });
}

fn feed_script(tx: mpsc::Sender<Value>, path: String) -> Result<(), Fail> {
    let text = if path == "-" {
        std::io::read_to_string(std::io::stdin())?
    } else {
        std::fs::read_to_string(path)?
    };
    let ops: Vec<Value> = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(list)) => list,
        _ => text.lines().filter_map(|l| serde_json::from_str(l.trim()).ok()).collect(),
    };
    thread::spawn(move || {
        for op in ops {
            if op["op"] == "sleep" {
                thread::sleep(Duration::from_millis(op["ms"].as_u64().unwrap_or(0)));
                continue;
            }
            if tx.send(op).is_err() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(400));
        let _ = tx.send(json!({"op": "quit"}));
    });
    Ok(())
}

fn apply(ws: &mut Ws, channel: &SecureChannel, op: &Value) -> Result<bool, Fail> {
    match op["op"].as_str().unwrap_or("") {
        "send" => {
            send_sealed(ws, channel, 0, 0, op["msg"].to_string().into_bytes(), false)?;
        }
        "pty_in" => {
            let stream = op["stream"].as_u64().unwrap_or(0) as u32;
            let text = op["text"].as_str().unwrap_or("").to_owned();
            send_sealed(ws, channel, 2, stream, text.into_bytes(), op["claim"].as_bool().unwrap_or(false))?;
        }
        "ping" => send_sealed(ws, channel, 3, 0, Vec::new(), false)?,
        "close" => {
            let _ = ws.close(None);
            return Ok(false);
        }
        "quit" => return Ok(false),
        other => emit(json!({"ev": "error", "error": format!("unknown op {other}")})),
    }
    Ok(true)
}

fn handle_inbound(channel: &SecureChannel, bytes: Vec<u8>) -> Result<(), Fail> {
    let Some(message) = channel.open(bytes)? else {
        return Ok(());
    };
    match message.kind {
        0 => emit(json!({"ev": "json", "msg": serde_json::from_slice::<Value>(&message.payload)?})),
        1 => emit(json!({
            "ev": "pty",
            "stream": message.stream,
            "text": String::from_utf8_lossy(&message.payload),
        })),
        _ => {}
    }
    Ok(())
}

fn session(args: &[String], script: Option<String>) -> Result<(), Fail> {
    let dir = PathBuf::from(option(args, "--state").ok_or("--state required")?);
    let state = load_state(&dir)?;
    let url = url_of(&state, args)?;
    let private = unhex(state["private"].as_str().ok_or("bad state")?)?;
    let desktop_key = unhex(state["desktopKey"].as_str().ok_or("bad state")?)?;
    let device_id = state["deviceId"].as_str().ok_or("bad state")?.to_owned();
    let started = session_start(private, desktop_key, device_id)?;
    let mut ws = connect(&url, POLL)?;
    ws.send(Message::Binary(started.msg1.into()))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let msg2 = loop {
        match read_one(&mut ws)? {
            Inbound::Binary(bytes) => break bytes,
            Inbound::Closed(code) => {
                emit(json!({"ev": "closed", "stage": "msg2", "code": code}));
                return Ok(());
            }
            Inbound::Idle if Instant::now() > deadline => return Err("msg2 timeout".into()),
            Inbound::Idle => {}
        }
    };
    let channel = started.handshake.finish(msg2)?;
    let (tx, rx): (_, Receiver<Value>) = mpsc::channel();
    match script {
        Some(path) => feed_script(tx, path)?,
        None => feed_stdin(tx),
    }
    emit(json!({"ev": "ready"}));
    let mut last_ping = Instant::now();
    loop {
        match read_one(&mut ws)? {
            Inbound::Binary(bytes) => handle_inbound(&channel, bytes)?,
            Inbound::Closed(code) => {
                emit(json!({"ev": "closed", "stage": "session", "code": code}));
                return Ok(());
            }
            Inbound::Idle => {}
        }
        if last_ping.elapsed() > PING_EVERY {
            send_sealed(&mut ws, &channel, 3, 0, Vec::new(), false)?;
            last_ping = Instant::now();
        }
        loop {
            match rx.recv_timeout(Duration::from_millis(1)) {
                Ok(op) => {
                    if !apply(&mut ws, &channel, &op)? {
                        emit(json!({"ev": "bye"}));
                        return Ok(());
                    }
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("pair") => pair(&args),
        Some("run") => match args.get(1) {
            Some(path) => session(&args, Some(path.clone())),
            None => Err("usage: run <script.json|-> --state <dir> [--url <ws>]".into()),
        },
        Some("repl") => session(&args, None),
        _ => Err("usage: phone_cli pair <qr> | run <script> | repl  --state <dir> [--url <ws>]".into()),
    };
    if let Err(error) = result {
        emit(json!({"ev": "fatal", "error": error.0}));
        std::process::exit(1);
    }
}
