use super::{text, Control, Event, FrameQueue, Hub, Mode, Out, Resolving, Session, MAX_LIVE, QUEUE_MAX};
use crate::control::{CoreToMain, Resolved, RESOLVE_TIMEOUT_MS};
use crate::flow::Gate;
use crate::ring::Ring;
use crate::session::{clamp_dim, PtyEvent, PtyProc, Sink, SpawnSpec};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

impl<C: Control> Hub<C> {
    pub(super) fn on_resolved(&mut self, r: Resolved) {
        let pending = self.resolving.remove(&r.rid);
        let Some(req) = pending.filter(|_| !self.closing) else {
            self.control.send(CoreToMain::Removed { id: r.id.clone() });
            return;
        };
        let conn = req.conn;
        let refuse = |hub: &mut Hub<C>, message: &str, removed: bool| {
            hub.send(Some(conn), text(json!({ "t": "error", "message": message })));
            if removed {
                hub.control.send(CoreToMain::Removed { id: r.id.clone() });
            }
            hub.settle(conn, None);
        };
        if !self.conns.contains_key(&conn) {
            return refuse(self, "spawn", true);
        }
        if self.live_count() >= MAX_LIVE {
            return refuse(self, "limit", true);
        }
        if self.ids.contains_key(&r.id) {
            eprintln!("orrery-core: resolved id already live");
            return refuse(self, "spawn", false);
        }
        let key = self.next_key;
        self.next_key += 1;
        let events = self.events.clone();
        let gate = Gate::new();
        let reader_gate = gate.clone();
        let sink: Sink = Arc::new(move |ev| {
            let credit = match &ev {
                PtyEvent::Draining => {
                    reader_gate.drain();
                    return;
                }
                PtyEvent::Data(d) => Some(reader_gate.acquire(d.len())),
                PtyEvent::Exit { .. } => None,
            };
            let _ = events.send(Event::Pty { key, ev, credit });
        });
        let spec = SpawnSpec { file: r.file.clone(), args: r.args.clone(), env: r.env.clone(), cwd: r.cwd.clone(), cols: req.cols, rows: req.rows };
        let proc = match PtyProc::spawn(&spec, sink) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("orrery-core: spawn failed preset={} kind={:?} os={}", req.preset, e.kind(), e.raw_os_error().unwrap_or(0));
                return refuse(self, "spawn", true);
            }
        };
        let pid = proc.pid;
        eprintln!("orrery-core: spawn id={} preset={} pid={pid}", r.id, req.preset);
        self.sessions.insert(
            key,
            Session {
                id: r.id.clone(),
                preset: req.preset,
                cwd: r.cwd.clone(),
                live_cwd: None,
                pid,
                viewers: Vec::new(),
                input_owner: None,
                size: (req.cols, req.rows),
                exit_code: None,
                branch: None,
                si: r.si.clone(),
                started: Instant::now(),
                exited_at: None,
                known: true,
                input_dropped: false,
                ring: Ring::default(),
                gate,
                proc,
            },
        );
        self.ids.insert(r.id.clone(), key);
        self.attach(conn, key, false, Mode::Own, Some((req.cols, req.rows)));
        self.control.send(CoreToMain::Spawned { id: r.id.clone(), pid });
        self.settle(conn, Some(key));
    }

    pub(super) fn attach(&mut self, conn: u64, key: u64, replay: bool, mode: Mode, size: Option<(u16, u16)>) {
        if !self.sessions.contains_key(&key) {
            return;
        }
        self.leave_all(conn, Some(key));
        let replaced = self.join(key, conn, mode, size);
        let Some(s) = self.sessions.get(&key) else { return };
        let mut ready = Map::new();
        ready.insert("t".into(), json!("ready"));
        ready.insert("id".into(), json!(s.id));
        ready.insert("preset".into(), json!(s.preset));
        ready.insert("pid".into(), json!(s.pid));
        ready.insert("cwd".into(), json!(s.cwd));
        ready.insert("replay".into(), json!(replay));
        if let Some(si) = &s.si {
            ready.insert("si".into(), json!(si));
        }
        let mut outs = vec![text(Value::Object(ready))];
        if mode == Mode::View {
            outs.extend(self.size_note(key));
        }
        if let Some(l) = &s.live_cwd {
            outs.push(text(json!({ "t": "cwd", "cwd": l })));
        }
        if replay && !s.ring.is_empty() {
            outs.push(Out::Binary(s.ring.snapshot()));
        }
        if let Some(b) = &s.branch {
            outs.push(text(json!({ "t": "branch", "branch": b })));
        }
        if let Some(code) = s.exit_code {
            outs.push(text(json!({ "t": "exit", "code": code })));
        }
        for old in replaced {
            self.send(Some(old), Out::Close(4000, "replaced"));
        }
        for o in outs {
            self.send(Some(conn), o);
        }
        self.rebind(key);
    }

    pub(super) fn settle(&mut self, conn: u64, key: Option<u64>) {
        let Some(c) = self.conns.get_mut(&conn) else { return };
        let queue = c.queue.take().unwrap_or_default();
        c.session = key;
        if queue.dropped > 0 {
            eprintln!("pty input dropped while spawning: {} frames over {} bytes", queue.dropped, QUEUE_MAX);
        }
        for f in queue.items {
            self.on_frame(conn, f);
        }
    }

    pub(super) fn request_spawn(&mut self, conn: u64, msg: Map<String, Value>) {
        if self.closing {
            return self.send(Some(conn), text(json!({ "t": "error", "message": "spawn" })));
        }
        let preset = match msg.get("preset") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        let cols = clamp_dim(msg.get("cols"), 80);
        let rows = clamp_dim(msg.get("rows"), 24);
        if let Some(c) = self.conns.get_mut(&conn) {
            c.queue = Some(FrameQueue::default());
        }
        let rid = self.rid('r');
        self.resolving.insert(rid.clone(), Resolving { conn, preset, cols, rows });
        let events = self.events.clone();
        let timer_rid = rid.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(RESOLVE_TIMEOUT_MS)).await;
            let _ = events.send(Event::ResolveTimeout { rid: timer_rid });
        });
        self.control.send(CoreToMain::Resolve { rid, msg: Value::Object(msg) });
    }
}
