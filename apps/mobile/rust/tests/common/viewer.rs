use std::net::TcpStream;

use core_remote_proto::Transport;
use serde_json::{json, Value};
use tungstenite::WebSocket;

use super::commands::{fail, Session};
use super::send_json;

type Ws = WebSocket<TcpStream>;

const ROOT: &str = "/work/demo";
const FILE: &str = "/work/demo/main.dart";

fn data(ws: &mut Ws, tr: &mut Transport, id: &Value, body: Value) {
    send_json(ws, tr, &json!({"re": id, "ok": true, "data": body}));
}

pub fn run(ws: &mut Ws, tr: &mut Transport, session: &mut Session, command: &Value) -> bool {
    let id = &command["id"];
    match command["t"].as_str().unwrap_or("") {
        "fs.list" => {
            if command["root"] != ROOT {
                return fail_denied(ws, tr, id);
            }
            data(
                ws,
                tr,
                id,
                json!([
                    {"name": "lib", "path": "/work/demo/lib", "directory": true, "outOfRoot": false},
                    {"name": "main.dart", "path": FILE, "directory": false, "outOfRoot": false},
                ]),
            );
        }
        "fs.read" => {
            if command["path"] != FILE {
                return fail_denied(ws, tr, id);
            }
            data(
                ws,
                tr,
                id,
                json!({
                    "kind": "text", "path": FILE, "size": session.fs_text.len(),
                    "mtimeMs": session.fs_mtime, "text": session.fs_text, "lang": "dart",
                }),
            );
        }
        "fs.write" => {
            if command["path"] != FILE {
                return fail_denied(ws, tr, id);
            }
            if command["base"].as_u64() != Some(session.fs_mtime) {
                data(ws, tr, id, json!({"ok": false, "error": "conflict"}));
                return true;
            }
            session.fs_text = command["text"].as_str().unwrap_or("").to_owned();
            session.fs_mtime += 1000;
            data(
                ws,
                tr,
                id,
                json!({"ok": true, "path": FILE, "mtimeMs": session.fs_mtime, "size": session.fs_text.len()}),
            );
            if session.fs_watching {
                send_json(ws, tr, &json!({"t": "fs.changed", "root": ROOT, "paths": [FILE]}));
            }
        }
        "fs.watch" => {
            session.fs_watching = true;
            data(ws, tr, id, json!(true));
        }
        "fs.unwatch" => {
            session.fs_watching = false;
            data(ws, tr, id, json!(false));
        }
        "git.watch" | "git.unwatch" => data(ws, tr, id, json!(true)),
        "git.status" => data(
            ws,
            tr,
            id,
            json!({
                "ok": true, "root": ROOT, "branch": "main", "upstream": null, "ahead": 0, "behind": 0,
                "truncated": false,
                "entries": [{"path": "main.dart", "x": ".", "y": "M", "kind": "modified", "staged": false}],
            }),
        ),
        _ => return false,
    }
    true
}

fn fail_denied(ws: &mut Ws, tr: &mut Transport, id: &Value) -> bool {
    fail(ws, tr, id, "denied");
    true
}
