#![allow(dead_code)]

mod commands;
mod viewer;

use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use core_remote_proto::frame::{Chunker, Frame, Kind, Reassembler};
use core_remote_proto::pair::{LanAddr, PairCode, PairResponder, QrPayload};
use core_remote_proto::session;
use core_remote_proto::{Error, StaticKeypair, Transport};
use serde_json::{json, Value};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Message, WebSocket};

use commands::{ok, workspace_snapshot, Session};

pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct Gateway {
    pub desktop: StaticKeypair,
    pub machine_id: String,
    pub name: String,
    pub code: PairCode,
    pub exp: u64,
    pub devices: Mutex<Vec<(String, [u8; 32])>>,
    pub code_used: Mutex<bool>,
    pub deny_pairing: bool,
    pub port: Mutex<u16>,
}

impl Gateway {
    pub fn new(exp_in: i64) -> Arc<Self> {
        let exp = (now_seconds() as i64 + exp_in) as u64;
        Arc::new(Self {
            desktop: StaticKeypair::generate().unwrap(),
            machine_id: "machine-fake-0001".into(),
            name: "Fake Desktop".into(),
            code: PairCode::generate().unwrap(),
            exp,
            devices: Mutex::new(Vec::new()),
            code_used: Mutex::new(false),
            deny_pairing: false,
            port: Mutex::new(0),
        })
    }

    pub fn qr(&self, mkey_override: Option<[u8; 32]>) -> String {
        let port = *self.port.lock().unwrap();
        QrPayload {
            machine: self.machine_id.clone(),
            name: self.name.clone(),
            lan: vec![LanAddr::new("127.0.0.1", port)],
            relay: Some("https://relay.example.dev".into()),
            ticket: None,
            mkey: mkey_override.unwrap_or(*self.desktop.public()),
            code: self.code.clone(),
            exp: self.exp,
        }
        .encode()
        .unwrap()
    }

    pub fn device_count(&self) -> usize {
        self.devices.lock().unwrap().len()
    }

    pub fn serve(self: &Arc<Self>, listener: TcpListener) {
        *self.port.lock().unwrap() = listener.local_addr().unwrap().port();
        let gateway = Arc::clone(self);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&gateway);
                thread::spawn(move || shared.handle(stream));
            }
        });
    }

    fn authorized(&self, id: &str, key: &[u8; 32]) -> bool {
        self.devices
            .lock()
            .unwrap()
            .iter()
            .any(|(known_id, known_key)| known_id == id && known_key == key)
    }

    fn handle(&self, stream: TcpStream) {
        let Ok(mut ws) = tungstenite::accept(stream) else {
            return;
        };
        let msg1 = loop {
            match ws.read() {
                Ok(Message::Binary(bytes)) => break bytes.to_vec(),
                Ok(Message::Text(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                _ => return,
            }
        };
        let attempt = session::accept(&self.desktop, &msg1, |id, key| self.authorized(id, key));
        match attempt {
            Ok((accepted, msg2)) => {
                if ws.send(Message::Binary(msg2.into())).is_err() {
                    return;
                }
                self.run_session(ws, accepted.transport);
            }
            Err(Error::Unauthorized) => close(&mut ws, 4401),
            Err(_) => self.try_pair(ws, &msg1),
        }
    }

    fn try_pair(&self, mut ws: WebSocket<TcpStream>, msg1: &[u8]) {
        if now_seconds() >= self.exp || *self.code_used.lock().unwrap() {
            return close(&mut ws, 4400);
        }
        let Ok((responder, msg2)) =
            PairResponder::start(&self.desktop, &self.code, &self.machine_id, msg1)
        else {
            return close(&mut ws, 4400);
        };
        if ws.send(Message::Binary(msg2.into())).is_err() {
            return;
        }
        let msg3 = loop {
            match ws.read() {
                Ok(Message::Binary(bytes)) => break bytes.to_vec(),
                Ok(_) => continue,
                Err(_) => return,
            }
        };
        let Ok(mut device) = responder.finish(&msg3) else {
            return close(&mut ws, 4400);
        };
        *self.code_used.lock().unwrap() = true;
        let reply = if self.deny_pairing {
            json!({"t": "pair.denied"})
        } else {
            self.devices
                .lock()
                .unwrap()
                .push((device.device_id.clone(), device.device_static));
            json!({
                "t": "pair.ok",
                "machine": self.machine_id,
                "name": self.name,
                "relay": "https://relay.example.dev",
            })
        };
        send_json(&mut ws, &mut device.transport, &reply);
        let _ = ws.flush();
        loop {
            if ws.read().is_err() {
                return;
            }
        }
    }

    fn run_session(&self, mut ws: WebSocket<TcpStream>, mut transport: Transport) {
        let mut reassembler = Reassembler::new();
        let mut session = Session::new();
        loop {
            let cipher = match ws.read() {
                Ok(Message::Binary(bytes)) => bytes.to_vec(),
                Ok(Message::Close(_)) | Err(_) => return,
                Ok(_) => continue,
            };
            let Ok(plain) = transport.open(&cipher) else {
                return close(&mut ws, 4400);
            };
            let Ok(frame) = Frame::decode(&plain) else {
                return close(&mut ws, 4400);
            };
            let Ok(Some(message)) = reassembler.push(frame) else {
                continue;
            };
            match message.kind {
                Kind::Ping => send_frame(&mut ws, &mut transport, Kind::Ping, 0, &[]),
                Kind::PtyIn => {
                    let mut echoed = if message.claim { b"claim+echo:".to_vec() } else { b"echo:".to_vec() };
                    echoed.extend_from_slice(&message.payload);
                    send_frame(&mut ws, &mut transport, Kind::PtyOut, message.stream, &echoed);
                }
                Kind::PtyOut => {}
                Kind::Json => {
                    let Ok(command) = serde_json::from_slice::<Value>(&message.payload) else {
                        continue;
                    };
                    self.command(&mut ws, &mut transport, &command, &mut session);
                }
            }
        }
    }

    fn command(
        &self,
        ws: &mut WebSocket<TcpStream>,
        transport: &mut Transport,
        command: &Value,
        session: &mut Session,
    ) {
        let id = command["id"].clone();
        let kind = command["t"].as_str().unwrap_or("");
        match kind {
            "hello" => {
                send_json(
                    ws,
                    transport,
                    &json!({
                        "re": id, "ok": true, "machine": self.machine_id, "name": self.name,
                        "version": "fake-1", "minProto": 1,
                        "theme": {"mode": "dark", "palette": {}}, "locale": "en",
                    }),
                );
                self.snapshot_events(ws, transport, session);
                send_json(
                    ws,
                    transport,
                    &json!({"t": "push.config", "apiKey": "fake-api-key", "appId": "1:123456789:android:fake", "senderId": "123456789", "projectId": "fake-project"}),
                );
            }
            "resync" => {
                ok(ws, transport, &id);
                self.snapshot_events(ws, transport, session);
            }
            _ => {
                if !commands::run(ws, transport, session, command) {
                    send_json(
                        ws,
                        transport,
                        &json!({"re": id, "ok": false, "error": "notFound"}),
                    );
                }
            }
        }
    }

    fn snapshot_events(
        &self,
        ws: &mut WebSocket<TcpStream>,
        transport: &mut Transport,
        session: &mut Session,
    ) {
        send_json(
            ws,
            transport,
            &json!({
                "t": "snapshot", "seq": session.seq,
                "data": {
                    "protocolVersion": 1,
                    "terminals": [{"id": "t1", "title": "main", "status": "idle"}],
                    "agents": [], "activity": [],
                },
            }),
        );
        send_json(ws, transport, &workspace_snapshot(session.rev));
    }
}

pub fn close(ws: &mut WebSocket<TcpStream>, code: u16) {
    let _ = ws.close(Some(CloseFrame {
        code: CloseCode::Library(code),
        reason: "".into(),
    }));
    let _ = ws.flush();
    let _ = ws.read();
}

pub fn send_frame(
    ws: &mut WebSocket<TcpStream>,
    transport: &mut Transport,
    kind: Kind,
    stream: u32,
    payload: &[u8],
) {
    for frame in Chunker::new(kind, stream, payload).unwrap() {
        let sealed = transport.seal(&frame.encode().unwrap()).unwrap();
        let _ = ws.send(Message::Binary(sealed.into()));
    }
}

pub fn send_json(ws: &mut WebSocket<TcpStream>, transport: &mut Transport, value: &Value) {
    send_frame(ws, transport, Kind::Json, 0, value.to_string().as_bytes());
}
