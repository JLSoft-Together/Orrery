use super::{early_exit_note, text, Control, Hub, Out, Session, EARLY_EXIT, EXITED_TTL};
use crate::control::{CoreToMain, MainToCore};
use crate::session::PtyEvent;
use serde_json::{json, Map, Value};

impl<C: Control> Hub<C> {
    pub(super) fn on_main(&mut self, msg: MainToCore) {
        match msg {
            MainToCore::Init { .. } => {}
            MainToCore::Resolved(r) => self.on_resolved(r),
            MainToCore::ResolveFailed { rid, message } => {
                let Some(r) = self.resolving.remove(&rid) else { return };
                self.send(Some(r.conn), text(json!({ "t": "error", "message": if self.closing { "spawn" } else { message.as_str() } })));
                self.settle(r.conn, None);
            }
            MainToCore::CwdOk { rid, cwd } => {
                let Some((conn, id)) = self.cwd_waits.remove(&rid) else { return };
                self.send(Some(conn), text(json!({ "t": "cwd", "cwd": cwd })));
                if let (Some(cwd), Some(s)) = (cwd, self.ids.get(&id).and_then(|k| self.sessions.get_mut(k))) {
                    if Some(&cwd) != s.live_cwd.as_ref().or(Some(&s.cwd)) {
                        s.live_cwd = Some(cwd);
                    }
                }
            }
            MainToCore::WtReply { rid, reply } => {
                if let Some(conn) = self.wt_waits.remove(&rid) {
                    self.send(Some(conn), text(reply));
                }
            }
            MainToCore::Branch { id, branch } => {
                let Some(s) = self.ids.get(&id).and_then(|k| self.sessions.get_mut(k)) else { return };
                s.branch = branch.clone();
                let conn = s.conn;
                self.send(conn, text(json!({ "t": "branch", "branch": branch })));
            }
            MainToCore::Census => {
                let sessions = self.census();
                self.control.send(CoreToMain::CensusReply { sessions });
            }
            MainToCore::KillAll => {
                let keys: Vec<u64> = self.ids.values().copied().collect();
                for k in keys {
                    self.kill(k, false);
                }
            }
            MainToCore::Shutdown => self.shutdown(),
        }
    }

    pub(super) fn census(&self) -> Vec<Value> {
        let mut rows: Vec<(&u64, &Session)> = self.sessions.iter().filter(|(_, s)| s.known).collect();
        rows.sort_by_key(|(k, _)| **k);
        rows.into_iter()
            .map(|(_, s)| {
                let mut o = Map::new();
                o.insert("id".into(), json!(s.id));
                o.insert("preset".into(), json!(s.preset));
                o.insert("cwd".into(), json!(s.cwd));
                if let Some(l) = &s.live_cwd {
                    o.insert("liveCwd".into(), json!(l));
                }
                o.insert("pid".into(), json!(s.pid));
                o.insert("alive".into(), json!(s.exit_code.is_none()));
                Value::Object(o)
            })
            .collect()
    }

    pub fn shutdown(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        eprintln!("orrery-core: shutdown, {} sessions", self.ids.len());
        let pending: Vec<String> = self.resolving.keys().cloned().collect();
        for rid in pending {
            if let Some(r) = self.resolving.remove(&rid) {
                self.send(Some(r.conn), text(json!({ "t": "error", "message": "spawn" })));
                if let Some(c) = self.conns.get_mut(&r.conn) {
                    c.queue = None;
                }
            }
        }
        let keys: Vec<u64> = self.ids.values().copied().collect();
        for k in keys {
            self.kill(k, true);
        }
    }

    pub(super) fn kill(&mut self, key: u64, now: bool) {
        let Some(s) = self.sessions.get_mut(&key) else { return };
        let was_known = s.known;
        if was_known {
            s.known = false;
            self.ids.remove(&s.id);
        }
        let id = s.id.clone();
        if s.exit_code.is_none() {
            eprintln!("orrery-core: kill id={} pid={}", s.id, s.pid);
            if now {
                let force = s.proc.hangup();
                self.to_force.push(force);
            } else {
                s.proc.kill();
            }
        } else {
            self.sessions.remove(&key);
        }
        if was_known {
            self.control.send(CoreToMain::Removed { id });
        }
    }

    pub fn reap(&mut self) {
        let now = self.now();
        let expired: Vec<u64> =
            self.sessions.iter().filter(|(_, s)| s.conn.is_none() && s.exited_at.is_some_and(|t| now.saturating_duration_since(t) > EXITED_TTL)).map(|(k, _)| *k).collect();
        for key in expired {
            let Some(s) = self.sessions.remove(&key) else { continue };
            if s.known {
                self.ids.remove(&s.id);
                self.control.send(CoreToMain::Removed { id: s.id.clone() });
            }
        }
    }

    pub(super) fn on_pty(&mut self, key: u64, ev: PtyEvent) {
        let now = self.now();
        let Some(s) = self.sessions.get_mut(&key) else { return };
        match ev {
            PtyEvent::Data(d) => {
                if s.exit_code.is_some() {
                    return;
                }
                s.ring.push(&d);
                let conn = s.conn;
                self.send(conn, Out::Binary(d));
            }
            PtyEvent::Draining => s.gate.drain(),
            PtyEvent::Exit { code, signal } => {
                if s.exit_code.is_some() {
                    return;
                }
                s.exited_at = Some(now);
                let ms = s.started.elapsed();
                if ms < EARLY_EXIT {
                    eprintln!("{}", early_exit_note(&s.preset, code, signal.as_deref(), ms.as_millis(), !s.known));
                } else {
                    eprintln!("orrery-core: exit id={} code={code}", s.id);
                }
                s.exit_code = Some(code);
                let (conn, known, id) = (s.conn, s.known, s.id.clone());
                self.send(conn, text(json!({ "t": "exit", "code": code })));
                if known {
                    self.control.send(CoreToMain::Exited { id, code });
                } else {
                    self.sessions.remove(&key);
                }
            }
        }
    }
}
