mod conn;
mod data;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use core_remote_proto::pair::{PairCode, PairTicket, QrPayload};
use core_remote_proto::relay::{self, ControlKind, Envelope, RelayKey};
use core_remote_proto::{StaticKeypair, KEY_LEN};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use conn::{Conn, Out};

pub const NAME: &str = "Orrery Desk";

pub struct Desk {
    pub noise: StaticKeypair,
    pub relay_key: RelayKey,
    pub machine: String,
    pub relay_url: String,
    pub code: PairCode,
    pub devices: HashMap<String, [u8; KEY_LEN]>,
    pub seq: u64,
    pub rev: u64,
    pub t1: &'static str,
    pub title: String,
    pub view_only: bool,
    pub asked: bool,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn random<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).expect("rng");
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn post(relay_url: &str, path: &str, body: &str) -> Result<String, String> {
    let host = relay_url.strip_prefix("http://").ok_or("relay url must be http://")?.trim_end_matches('/');
    let mut stream = TcpStream::connect(host).map_err(|e| e.to_string())?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| e.to_string())?;
    if response.starts_with("HTTP/1.1 200") {
        Ok(response)
    } else {
        Err(response.lines().next().unwrap_or("no response").to_owned())
    }
}

fn arg(name: &str, fallback: &str) -> String {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| fallback.to_owned())
}

fn watch_stdin() -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            match stdin.read_line(&mut line) {
                Ok(0) | Err(_) => std::process::exit(0),
                Ok(_) => {
                    let _ = tx.send(line.trim().to_owned());
                }
            }
        }
    });
    rx
}

fn command(desk: &mut Desk, conns: &mut HashMap<u16, Conn>, line: &str) -> Vec<(u16, Out)> {
    let mut outs = Vec::new();
    match line {
        "events" => {
            desk.seq += 1;
            desk.t1 = "idle";
            let value = json!({"t": "events", "seq": desk.seq, "events": [{"type": "terminal.updated", "payload": {"id": "t1", "status": "idle"}}]});
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.push(&value).into_iter().map(|o| (*id, o)));
            }
        }
        "patch" => {
            let base = desk.rev;
            desk.rev += 1;
            desk.title = "claude patched".to_owned();
            let value = json!({"t": "ws.patch", "base": base, "rev": desk.rev, "ops": [{"op": "replace", "path": "/tabs/0/panes/0/title", "value": "claude patched"}]});
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.push(&value).into_iter().map(|o| (*id, o)));
            }
        }
        "big" => {
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.big().into_iter().map(|o| (*id, o)));
            }
        }
        "demo" => {
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.demo().into_iter().map(|o| (*id, o)));
            }
        }
        "confirm" => {
            let value = json!({"t": "confirm.ask", "cid": "cid1", "re": "git.discard", "title": "Bỏ thay đổi trong src/app.ts?", "detail": "git checkout -- src/app.ts", "ok": "Bỏ thay đổi", "cancel": "Giữ lại"});
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.push(&value).into_iter().map(|o| (*id, o)));
            }
        }
        "viewonly" => desk.view_only = true,
        "asked" => {
            desk.asked = true;
            desk.rev += 1;
            let value = data::ws_snapshot(desk);
            for (id, conn) in conns.iter_mut() {
                outs.extend(conn.push(&value).into_iter().map(|o| (*id, o)));
            }
        }
        "revoke" => {
            desk.devices.clear();
            for id in conns.keys() {
                outs.push((*id, Out::Close(4401)));
            }
        }
        _ => {}
    }
    outs
}

fn flush(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, conns: &mut HashMap<u16, Conn>, outs: Vec<(u16, Out)>) -> bool {
    for (conn, out) in outs {
        let frame = match out {
            Out::Send(msg) => relay::encode_data(conn, &msg),
            Out::Close(code) => {
                eprintln!("peer: close conn {conn} code {code}");
                conns.remove(&conn);
                relay::encode_close(conn, code)
            }
        };
        if let Ok(frame) = frame {
            if ws.send(Message::Binary(frame.into())).is_err() {
                eprintln!("peer: relay send failed");
                return false;
            }
        }
    }
    true
}

fn main() {
    let commands = watch_stdin();
    let relay_url = arg("--relay", "http://127.0.0.1:8787").trim_end_matches('/').to_owned();
    let ws_base = relay_url.replacen("http://", "ws://", 1);
    let sign = SigningKey::from_bytes(&random::<32>());
    let mut desk = Desk {
        noise: StaticKeypair::generate().expect("noise key"),
        relay_key: RelayKey::generate().expect("relay key"),
        machine: format!("spike-{}", hex(&random::<12>())),
        relay_url: relay_url.clone(),
        code: PairCode::generate().expect("pair code"),
        devices: HashMap::new(),
        seq: 1,
        rev: 1,
        t1: "working",
        title: "claude".to_owned(),
        view_only: false,
        asked: false,
    };

    let ts = now();
    let reg_sig = sign.sign(format!("orrery-register/1{}{ts}", desk.machine).as_bytes());
    let body = json!({
        "machineId": desk.machine,
        "pubkey": b64(sign.verifying_key().as_bytes()),
        "relayKey": b64(desk.relay_key.as_bytes()),
        "ts": ts,
        "sig": b64(&reg_sig.to_bytes()),
    });
    if let Err(e) = post(&relay_url, "/register", &body.to_string()) {
        eprintln!("peer: register failed: {e}");
        std::process::exit(2);
    }

    let ws_sig = sign.sign(format!("orrery-machine/1{}{ts}", desk.machine).as_bytes());
    let url = format!("{ws_base}/m/{}?ts={ts}&sig={}", desk.machine, b64(&ws_sig.to_bytes()));
    let (mut ws, _) = match tungstenite::connect(url.as_str()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("peer: desktop ws failed: {e}");
            std::process::exit(3);
        }
    };

    let exp = now() + 120;
    let ticket = PairTicket::generate().expect("ticket");
    let qr = QrPayload {
        machine: desk.machine.clone(),
        name: NAME.into(),
        lan: Vec::new(),
        relay: Some(relay_url.clone()),
        ticket: Some(ticket.clone()),
        mkey: *desk.noise.public(),
        code: desk.code.clone(),
        exp,
    }
    .encode()
    .expect("qr");
    let armed = relay::encode_pair_open(&ticket.hash(), exp - now()).expect("pair.open");
    if ws.send(Message::Binary(armed.into())).is_err() {
        eprintln!("peer: pair.open failed");
        std::process::exit(4);
    }
    println!("{}", json!({ "ready": true, "machine": desk.machine, "qr": qr }));
    let _ = std::io::stdout().flush();

    let mut conns: HashMap<u16, Conn> = HashMap::new();
    let mut pairing: HashSet<u16> = HashSet::new();
    if let MaybeTlsStream::Plain(stream) = ws.get_mut() {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    }
    loop {
        while let Ok(line) = commands.try_recv() {
            let outs = command(&mut desk, &mut conns, &line);
            if !flush(&mut ws, &mut conns, outs) {
                return;
            }
        }
        let bytes = match ws.read() {
            Ok(Message::Binary(b)) => b,
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => continue,
            Err(_) => break,
            Ok(_) => continue,
        };
        let (conn, outs) = match relay::decode(&bytes) {
            Ok(Envelope::Control(c)) => {
                match c.t {
                    ControlKind::Open => {
                        conns.insert(c.conn, Conn::New);
                        if c.pairing {
                            pairing.insert(c.conn);
                        } else {
                            pairing.remove(&c.conn);
                        }
                    }
                    ControlKind::Close => {
                        conns.remove(&c.conn);
                        pairing.remove(&c.conn);
                    }
                }
                eprintln!("peer: conn {} {:?} pairing={} ip={:?}", c.conn, c.t, c.pairing, c.ip);
                continue;
            }
            Ok(Envelope::Data { conn, bytes }) => {
                let state = conns.entry(conn).or_insert(Conn::New);
                (conn, state.on_data(&mut desk, bytes, pairing.contains(&conn)))
            }
            Err(_) => continue,
        };
        let outs = outs.into_iter().map(|o| (conn, o)).collect();
        if !flush(&mut ws, &mut conns, outs) {
            return;
        }
    }
    eprintln!("peer: relay socket closed");
}
