use core_remote_proto::frame::{Chunker, Frame, Kind, Message, Reassembler};
use core_remote_proto::pair::PairResponder;
use core_remote_proto::relay::issue_token;
use core_remote_proto::session::accept;
use core_remote_proto::{Error, Transport, PROTO};
use serde_json::{json, Value};

use crate::{now, Desk, NAME};

fn terminals(desk: &Desk) -> Value {
    json!([
        {"id": "t1", "status": desk.t1, "title": "api", "provider": "claude", "gitBranch": "feat/api", "cwd": "D:/Code/api", "statusSince": "2026-01-01T10:00:00.000Z"},
        {"id": "t2", "status": "working", "title": "web", "provider": "codex", "gitBranch": "main", "cwd": "D:/Code/web", "statusSince": "2026-01-01T10:00:00.000Z"},
        {"id": "t3", "status": "waiting", "title": "docs", "provider": "claude", "gitBranch": "docs", "cwd": "D:/Code/docs", "waitingFor": "npm test"},
        {"id": "t4", "status": "idle", "title": "infra", "provider": "shell", "gitBranch": "infra", "cwd": "D:/Code/infra"}
    ])
}

fn agents() -> Value {
    json!([
        {"id": "a2", "terminalId": "t2", "role": "main", "currentTool": "Edit", "currentToolSummary": "src/app.ts"},
        {"id": "a2s1", "terminalId": "t2", "role": "subagent", "parentId": "a2", "status": "working", "description": "Build the shell"},
        {"id": "a2s2", "terminalId": "t2", "role": "subagent", "parentId": "a2", "status": "completed", "description": "Read tokens"}
    ])
}

fn activity() -> Value {
    json!([
        {"id": "e1", "terminalId": "t2", "kind": "tool.started", "at": "2026-01-01T10:00:00.000Z", "data": {"tool": "Read"}},
        {"id": "e2", "terminalId": "t2", "kind": "tool.started", "at": "2026-01-01T10:01:00.000Z", "data": {"tool": "Edit"}},
        {"id": "e3", "terminalId": "t2", "kind": "tool.failed", "at": "2026-01-01T10:02:00.000Z", "data": {"tool": "Bash", "error": "exit 1"}}
    ])
}

fn snapshot_event(desk: &Desk) -> Value {
    json!({"t": "snapshot", "seq": desk.seq, "data": {"terminals": terminals(desk), "agents": agents(), "activity": activity()}})
}

fn ws_snapshot(desk: &Desk) -> Value {
    json!({
        "t": "ws.snapshot",
        "rev": desk.rev,
        "closedCount": 0,
        "tabs": [{
            "key": "tab1", "name": "api", "project": "api", "tree": {"pane": "p1"},
            "panes": [{"key": "p1", "kind": "agent", "title": desk.title, "state": "working", "rail": "working", "subagents": 0, "dirty": false, "busy": true}]
        }]
    })
}

const PAIR_MSG1_LEN: usize = 50;
const TOKEN_TTL: u64 = 30 * 24 * 3600;
const CLOSE_NORMAL: u16 = 1000;
const CLOSE_HANDSHAKE: u16 = 4400;
const CLOSE_DEVICE: u16 = 4401;
const CLOSE_PROTO: u16 = 4426;

pub enum Out {
    Send(Vec<u8>),
    Close(u16),
}

pub struct Live {
    transport: Transport,
    reassembler: Reassembler,
    device: String,
}

pub enum Conn {
    New,
    Pairing(Box<PairResponder>),
    Live(Box<Live>),
    Done,
}

fn sealed(transport: &mut Transport, kind: Kind, stream: u32, payload: &[u8]) -> Vec<Out> {
    let Ok(chunks) = Chunker::new(kind, stream, payload) else {
        return vec![Out::Close(CLOSE_HANDSHAKE)];
    };
    let mut outs = Vec::new();
    for frame in chunks {
        match frame.encode().and_then(|bytes| transport.seal(&bytes)) {
            Ok(ct) => outs.push(Out::Send(ct)),
            Err(_) => return vec![Out::Close(CLOSE_HANDSHAKE)],
        }
    }
    outs
}

fn json_out(transport: &mut Transport, value: &Value) -> Vec<Out> {
    sealed(transport, Kind::Json, 0, value.to_string().as_bytes())
}

fn close_code(e: &Error) -> u16 {
    match e {
        Error::Unauthorized => CLOSE_DEVICE,
        Error::ProtoTooOld { .. } | Error::ProtoTooNew { .. } => CLOSE_PROTO,
        _ => CLOSE_HANDSHAKE,
    }
}

impl Conn {
    pub fn push(&mut self, value: &Value) -> Vec<Out> {
        match self {
            Conn::Live(live) => live.push(value),
            _ => Vec::new(),
        }
    }

    pub fn on_data(&mut self, desk: &mut Desk, data: &[u8], pairing_only: bool) -> Vec<Out> {
        match std::mem::replace(self, Conn::Done) {
            Conn::New if data.len() == PAIR_MSG1_LEN => self.pair_start(desk, data),
            Conn::New if pairing_only => vec![Out::Close(CLOSE_HANDSHAKE)],
            Conn::New => self.session_start(desk, data),
            Conn::Pairing(responder) => pair_finish(desk, *responder, data),
            Conn::Live(mut live) => {
                let outs = live.on_data(desk, data);
                if !outs.iter().any(|o| matches!(o, Out::Close(_))) {
                    *self = Conn::Live(live);
                }
                outs
            }
            Conn::Done => Vec::new(),
        }
    }

    fn pair_start(&mut self, desk: &Desk, data: &[u8]) -> Vec<Out> {
        match PairResponder::start(&desk.noise, &desk.code, &desk.machine, data) {
            Ok((responder, msg2)) => {
                *self = Conn::Pairing(Box::new(responder));
                vec![Out::Send(msg2)]
            }
            Err(e) => {
                eprintln!("peer: pair msg1 rejected: {e}");
                vec![Out::Close(close_code(&e))]
            }
        }
    }

    fn session_start(&mut self, desk: &Desk, data: &[u8]) -> Vec<Out> {
        let devices = &desk.devices;
        match accept(&desk.noise, data, |id, key| devices.get(id) == Some(key)) {
            Ok((accepted, msg2)) => {
                eprintln!("peer: session up for {}", accepted.device_id);
                *self = Conn::Live(Box::new(Live {
                    transport: accepted.transport,
                    reassembler: Reassembler::new(),
                    device: accepted.device_id,
                }));
                vec![Out::Send(msg2)]
            }
            Err(e) => {
                eprintln!("peer: session msg1 rejected: {e}");
                vec![Out::Close(close_code(&e))]
            }
        }
    }
}

fn pair_finish(desk: &mut Desk, responder: PairResponder, data: &[u8]) -> Vec<Out> {
    let mut paired = match responder.finish(data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("peer: pair msg3 rejected: {e}");
            return vec![Out::Close(close_code(&e))];
        }
    };
    eprintln!("peer: paired {} ({})", paired.device_id, paired.name);
    desk.devices.insert(paired.device_id.clone(), paired.device_static);
    let token = match issue_token(&desk.relay_key, &desk.machine, &paired.device_id, now() + TOKEN_TTL) {
        Ok(t) => t,
        Err(_) => return vec![Out::Close(CLOSE_HANDSHAKE)],
    };
    let reply = json!({
        "t": "pair.ok",
        "machine": desk.machine,
        "name": NAME,
        "relay": desk.relay_url,
        "relayToken": token,
    });
    let mut outs = json_out(&mut paired.transport, &reply);
    outs.push(Out::Close(CLOSE_NORMAL));
    outs
}

impl Live {
    pub fn push(&mut self, value: &Value) -> Vec<Out> {
        json_out(&mut self.transport, value)
    }

    fn on_data(&mut self, desk: &Desk, data: &[u8]) -> Vec<Out> {
        let Ok(plain) = self.transport.open(data) else {
            eprintln!("peer: decrypt failed for {}", self.device);
            return vec![Out::Close(CLOSE_HANDSHAKE)];
        };
        let message = match Frame::decode(&plain).and_then(|f| self.reassembler.push(f)) {
            Ok(Some(m)) => m,
            Ok(None) => return Vec::new(),
            Err(_) => return vec![Out::Close(CLOSE_HANDSHAKE)],
        };
        self.respond(desk, message)
    }

    fn respond(&mut self, desk: &Desk, message: Message) -> Vec<Out> {
        match message.kind {
            Kind::Json => {
                let Ok(value) = serde_json::from_slice::<Value>(&message.payload) else {
                    return Vec::new();
                };
                let id = value.get("id").cloned().unwrap_or(Value::Null);
                let kind = value.get("t").and_then(Value::as_str);
                let mut extra: Vec<Value> = Vec::new();
                let reply = match kind {
                    Some("hello") => {
                        extra.push(snapshot_event(desk));
                        extra.push(ws_snapshot(desk));
                        json!({
                            "re": id, "ok": true, "machine": desk.machine, "name": NAME,
                            "version": "2.3.0", "minProto": PROTO, "locale": "vi",
                        })
                    }
                    Some("ws.get") => {
                        extra.push(ws_snapshot(desk));
                        json!({ "re": id, "ok": true })
                    }
                    Some("resync") => {
                        extra.push(snapshot_event(desk));
                        json!({ "re": id, "ok": true })
                    }
                    Some("echo") => json!({ "re": id, "ok": true, "data": value.get("data") }),
                    _ => json!({ "re": id, "ok": false, "error": "invalid" }),
                };
                let mut outs = json_out(&mut self.transport, &reply);
                for event in extra {
                    outs.extend(json_out(&mut self.transport, &event));
                }
                outs
            }
            Kind::PtyIn => sealed(&mut self.transport, Kind::PtyOut, message.stream, &message.payload),
            Kind::Ping => sealed(&mut self.transport, Kind::Ping, 0, &[]),
            Kind::PtyOut => Vec::new(),
        }
    }
}
