use std::collections::{HashMap, HashSet};
use std::net::TcpStream;

use core_remote_proto::frame::Kind;
use core_remote_proto::Transport;
use serde_json::{json, Value};
use tungstenite::WebSocket;

use super::{send_frame, send_json};

pub const MAX_ATTACH: usize = 4;

pub struct Session {
    pub seq: u64,
    pub rev: u64,
    pub streams: HashSet<u32>,
    pub confirms: HashMap<String, String>,
    pub next_cid: u32,
    pub fs_text: String,
    pub fs_mtime: u64,
    pub fs_watching: bool,
    pub push_token: Option<String>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            seq: 1,
            rev: 1,
            streams: HashSet::new(),
            confirms: HashMap::new(),
            next_cid: 1,
            fs_text: "void main() {}
".into(),
            fs_mtime: 1000,
            fs_watching: false,
            push_token: None,
        }
    }
}

type Ws = WebSocket<TcpStream>;

pub fn ok(ws: &mut Ws, tr: &mut Transport, id: &Value) {
    send_json(ws, tr, &json!({"re": id, "ok": true}));
}

pub fn fail(ws: &mut Ws, tr: &mut Transport, id: &Value, error: &str) {
    send_json(ws, tr, &json!({"re": id, "ok": false, "error": error}));
}

pub fn workspace_snapshot(rev: u64) -> Value {
    json!({
        "t": "ws.snapshot", "rev": rev, "closedCount": 0,
        "tabs": [{
            "key": "tab1",
            "project": "/work/demo",
            "panes": [{
                "key": "pane1", "kind": "term", "title": "claude",
                "sessionId": "s1", "terminalId": "t1", "rail": "idle",
            }],
        }],
    })
}

fn patch(ws: &mut Ws, tr: &mut Transport, session: &mut Session, ops: Value) {
    let base = session.rev;
    session.rev += 1;
    send_json(
        ws,
        tr,
        &json!({"t": "ws.patch", "base": base, "rev": session.rev, "ops": ops}),
    );
}

pub fn attach(ws: &mut Ws, tr: &mut Transport, session: &mut Session, command: &Value) {
    let id = &command["id"];
    let Some(stream) = command["stream"].as_u64().filter(|s| *s >= 1) else {
        return fail(ws, tr, id, "invalid");
    };
    let stream = stream as u32;
    if session.streams.contains(&stream) {
        return fail(ws, tr, id, "invalid");
    }
    if session.streams.len() >= MAX_ATTACH {
        return fail(ws, tr, id, "limit");
    }
    session.streams.insert(stream);
    send_json(ws, tr, &json!({"re": id, "ok": true, "stream": stream}));
    send_json(
        ws,
        tr,
        &json!({"t": "pty.ready", "stream": stream, "cols": 80, "rows": 24}),
    );
    send_frame(ws, tr, Kind::PtyOut, stream, b"replay\r\n");
    if command["sessionId"] == "vanishes" {
        send_json(ws, tr, &json!({"t": "pty.exit", "stream": stream, "code": null}));
        session.streams.remove(&stream);
    }
}

pub fn run(ws: &mut Ws, tr: &mut Transport, session: &mut Session, command: &Value) -> bool {
    let id = &command["id"];
    match command["t"].as_str().unwrap_or("") {
        "pty.attach" => attach(ws, tr, session, command),
        "pty.detach" => {
            if let Some(stream) = command["stream"].as_u64() {
                session.streams.remove(&(stream as u32));
            }
            ok(ws, tr, id);
        }
        "pty.resize" => ok(ws, tr, id),
        "push.register" => {
            let token = command["token"].as_str().unwrap_or("");
            let plain = (16..=512).contains(&token.len())
                && token.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'.' | b'-'));
            let locale = command["locale"].as_str().unwrap_or("");
            if !plain || !matches!(locale, "vi" | "en") || !matches!(command["platform"].as_str(), Some("android" | "ios")) {
                fail(ws, tr, id, "invalid");
                return true;
            }
            session.push_token = Some(token.to_owned());
            send_json(ws, tr, &json!({"re": id, "ok": true, "stored": token}));
        }
        "push.unregister" => {
            session.push_token = None;
            send_json(ws, tr, &json!({"re": id, "ok": true, "stored": null}));
        }
        "ws.get" => {
            ok(ws, tr, id);
            send_json(ws, tr, &workspace_snapshot(session.rev));
        }
        "ws.rename" => {
            ok(ws, tr, id);
            let name = command["name"].clone();
            patch(ws, tr, session, json!([{"op": "add", "path": "/tabs/0/name", "value": name}]));
        }
        "ws.close" => {
            let cid = format!("cid{}", session.next_cid);
            session.next_cid += 1;
            session.confirms.insert(cid.clone(), "close".into());
            send_json(ws, tr, &json!({"re": id, "ok": true, "confirm": cid}));
            send_json(
                ws,
                tr,
                &json!({
                    "t": "confirm.ask", "cid": cid, "re": id,
                    "title": "Close this pane?", "detail": "The agent in it stops.",
                    "ok": "Close", "cancel": "Keep",
                }),
            );
        }
        "confirm.answer" => {
            let cid = command["cid"].as_str().unwrap_or("").to_owned();
            if session.confirms.remove(&cid).is_none() {
                fail(ws, tr, id, "notFound");
                return true;
            }
            if command["ok"] == true {
                ok(ws, tr, id);
                patch(ws, tr, session, json!([{"op": "remove", "path": "/tabs/0/panes/0"}]));
            } else {
                send_json(ws, tr, &json!({"re": id, "ok": true, "cancelled": true}));
            }
        }
        _ => return super::viewer::run(ws, tr, session, command),
    }
    true
}
