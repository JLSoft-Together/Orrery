use super::{text, Control, Event, Frame, Hub};
use crate::control::CoreToMain;
use crate::owners::{self, Ask, Lookup};
use crate::session::clamp_dim;
use serde_json::{json, Value};
use std::collections::HashMap;

impl<C: Control> Hub<C> {
    pub(super) fn on_frame(&mut self, conn: u64, frame: Frame) {
        let Some(c) = self.conns.get_mut(&conn) else { return };
        if let Some(q) = c.queue.as_mut() {
            q.push(frame);
            return;
        }
        match frame {
            Frame::Binary(data) => {
                if let Some(s) = c.session.and_then(|k| self.sessions.get_mut(&k)) {
                    if s.exit_code.is_none() && !s.proc.write(&data) && !s.input_dropped {
                        s.input_dropped = true;
                        eprintln!("orrery-core: input dropped, write queue full id={}", s.id);
                    }
                }
            }
            Frame::Text(raw) => self.on_text(conn, &raw),
        }
    }

    fn on_text(&mut self, conn: u64, raw: &str) {
        let Ok(Value::Object(msg)) = serde_json::from_str::<Value>(raw) else { return };
        let Some(t) = msg.get("t").and_then(Value::as_str).map(str::to_owned) else { return };
        let current = self.conns.get(&conn).and_then(|c| c.session);
        match t.as_str() {
            "wt.list" | "wt.add" | "wt.remove" => {
                let rid = self.rid('w');
                self.wt_waits.insert(rid.clone(), conn);
                self.control.send(CoreToMain::Wt { rid, msg: Value::Object(msg) });
                return;
            }
            "owners" => return self.ask_owners(conn, msg.get("pids")),
            "kill" => {
                if let Some(id) = msg.get("id").and_then(Value::as_str) {
                    let target = self.ids.get(id).copied();
                    if let Some(k) = target {
                        self.kill(k, false);
                    }
                    if target.is_some() && target == current {
                        if let Some(c) = self.conns.get_mut(&conn) {
                            c.session = None;
                        }
                    }
                    return;
                }
            }
            "spawn" if current.is_none() => return self.request_spawn(conn, msg),
            "attach" if current.is_none() => {
                let id = match msg.get("id") {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => "undefined".to_owned(),
                };
                match self.ids.get(&id).copied() {
                    Some(k) => {
                        self.attach(conn, k, true);
                        if let Some(c) = self.conns.get_mut(&conn) {
                            c.session = Some(k);
                        }
                    }
                    None => self.send(Some(conn), text(json!({ "t": "gone" }))),
                }
                return;
            }
            _ => {}
        }
        let Some(key) = current else { return };
        match t.as_str() {
            "resize" => {
                if let Some(s) = self.sessions.get(&key).filter(|s| s.exit_code.is_none()) {
                    s.proc.resize(clamp_dim(msg.get("cols"), 80), clamp_dim(msg.get("rows"), 24));
                }
            }
            "kill" => {
                self.kill(key, false);
                if let Some(c) = self.conns.get_mut(&conn) {
                    c.session = None;
                }
            }
            "cwd" => match msg.get("cwd").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                None => self.send(Some(conn), text(json!({ "t": "cwd", "cwd": null }))),
                Some(cwd) => {
                    let Some(id) = self.sessions.get(&key).map(|s| s.id.clone()) else { return };
                    let rid = self.rid('c');
                    self.cwd_waits.insert(rid.clone(), (conn, id.clone()));
                    self.control.send(CoreToMain::Cwd { rid, id, cwd: cwd.to_owned() });
                }
            },
            _ => {}
        }
    }

    fn ask_owners(&mut self, conn: u64, pids: Option<&Value>) {
        let pids = owners::wanted_pids(pids);
        let roots: HashMap<u64, String> = self.sessions.values().filter(|s| s.known && s.exit_code.is_none() && s.pid > 0).map(|s| (s.pid as u64, s.id.clone())).collect();
        if pids.is_empty() || roots.is_empty() {
            return self.send(Some(conn), text(json!({ "t": "owners", "owners": {} })));
        }
        let now = self.now();
        match self.tables.lookup(now, Ask { conn, pids, roots }) {
            (Lookup::Ready(table), Some(ask)) => {
                let owners = owners::answer(&ask, table.as_deref());
                self.send(Some(conn), text(json!({ "t": "owners", "owners": owners })));
            }
            (Lookup::Scan, _) => {
                let events = self.events.clone();
                tokio::task::spawn_blocking(move || {
                    let table = owners::read_pid_table();
                    let _ = events.send(Event::Owners { started: now, table });
                });
            }
            _ => {}
        }
    }
}
