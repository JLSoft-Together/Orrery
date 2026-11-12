mod conn;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use core_remote_proto::pair::{PairCode, PairTicket, QrPayload};
use core_remote_proto::relay::{self, ControlKind, Envelope, RelayKey};
use core_remote_proto::{StaticKeypair, KEY_LEN};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use tungstenite::Message;

use conn::{Conn, Out};

pub const NAME: &str = "Spike PC";

pub struct Desk {
    pub noise: StaticKeypair,
    pub relay_key: RelayKey,
    pub machine: String,
    pub relay_url: String,
    pub code: PairCode,
    pub devices: HashMap<String, [u8; KEY_LEN]>,
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

fn watch_stdin() {
    std::thread::spawn(|| {
        let mut sink = [0u8; 64];
        while matches!(std::io::stdin().read(&mut sink), Ok(n) if n > 0) {}
        std::process::exit(0);
    });
}

fn main() {
    watch_stdin();
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
    loop {
        let bytes = match ws.read() {
            Ok(Message::Binary(b)) => b,
            Ok(Message::Close(_)) | Err(_) => break,
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
        for out in outs {
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
                    return;
                }
            }
        }
    }
    eprintln!("peer: relay socket closed");
}
