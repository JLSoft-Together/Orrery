use crate::control::{CoreToMain, MainToCore, Resolved, RESOLVE_TIMEOUT_MS};
use crate::flow::{Credit, Gate, Meter, CONN_MAX};
use crate::owners::{self, Ask, Lookup, PidTable, TableCache};
use crate::ring::Ring;
use crate::session::{clamp_dim, Force, PtyEvent, PtyProc, Sink, SpawnSpec};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::OwnedSemaphorePermit;

pub const QUEUE_MAX: usize = 4 * 1024 * 1024;
pub const EARLY_EXIT: Duration = Duration::from_secs(15);
pub const MAX_LIVE: usize = 16;
pub const EXITED_TTL: Duration = Duration::from_secs(30 * 60);
pub const REAP_EVERY: Duration = Duration::from_secs(60);
pub const OVERFLOW_CLOSE: u16 = 4001;

pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
}

pub enum Out {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, &'static str),
}

pub enum Event {
    Control(Option<MainToCore>),
    StdinEof,
    Open { conn: u64, tx: UnboundedSender<Out>, meter: Arc<Meter> },
    Frame { conn: u64, frame: Frame, permit: Option<OwnedSemaphorePermit> },
    Closed { conn: u64 },
    Pty { key: u64, ev: PtyEvent, credit: Option<Credit> },
    ResolveTimeout { rid: String },
    Release(Credit),
    Reap,
    Owners { started: Instant, table: Option<PidTable> },
}

pub trait Control: Send {
    fn send(&self, msg: CoreToMain);
}

struct Session {
    id: String,
    preset: String,
    cwd: String,
    live_cwd: Option<String>,
    pid: u32,
    conn: Option<u64>,
    exit_code: Option<i64>,
    branch: Option<String>,
    si: Option<String>,
    started: Instant,
    exited_at: Option<Instant>,
    known: bool,
    input_dropped: bool,
    ring: Ring,
    gate: Arc<Gate>,
    proc: PtyProc,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.gate.close();
    }
}

#[derive(Default)]
struct FrameQueue {
    items: Vec<Frame>,
    bytes: usize,
    dropped: usize,
}

impl FrameQueue {
    fn push(&mut self, f: Frame) {
        let (size, droppable) = match &f {
            Frame::Text(t) => (t.len(), false),
            Frame::Binary(b) => (b.len(), true),
        };
        if droppable && self.bytes + size > QUEUE_MAX {
            self.dropped += 1;
            return;
        }
        self.bytes += size;
        self.items.push(f);
    }
}

struct Conn {
    tx: UnboundedSender<Out>,
    meter: Arc<Meter>,
    overflowed: AtomicBool,
    session: Option<u64>,
    queue: Option<FrameQueue>,
}

struct Resolving {
    conn: u64,
    preset: String,
    cols: u16,
    rows: u16,
}

pub struct Hub<C: Control> {
    control: C,
    events: UnboundedSender<Event>,
    sessions: HashMap<u64, Session>,
    ids: HashMap<String, u64>,
    conns: HashMap<u64, Conn>,
    resolving: HashMap<String, Resolving>,
    cwd_waits: HashMap<String, (u64, String)>,
    wt_waits: HashMap<String, u64>,
    closing: bool,
    to_force: Vec<Force>,
    next_key: u64,
    next_rid: u64,
    clock: Clock,
    tables: TableCache,
}

fn text(v: Value) -> Out {
    Out::Text(v.to_string())
}

pub fn early_exit_note(preset: &str, code: i64, signal: Option<&str>, ms: u128, killed: bool) -> String {
    let hex = if !(0..=0x7fff_ffff).contains(&code) { format!(" (0x{:X})", code as u32) } else { String::new() };
    let sig = signal.map(|s| format!(" signal={s}")).unwrap_or_default();
    let by_app = if killed { " killed-by-app" } else { "" };
    format!("pty early exit: preset={preset} code={code}{hex}{sig} after={ms}ms{by_app}")
}

impl<C: Control> Hub<C> {
    pub fn new(control: C, events: UnboundedSender<Event>) -> Hub<C> {
        Hub {
            control,
            events,
            sessions: HashMap::new(),
            ids: HashMap::new(),
            conns: HashMap::new(),
            resolving: HashMap::new(),
            cwd_waits: HashMap::new(),
            wt_waits: HashMap::new(),
            closing: false,
            to_force: Vec::new(),
            next_key: 1,
            next_rid: 1,
            clock: Arc::new(Instant::now),
            tables: TableCache::default(),
        }
    }

    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }

    fn live_count(&self) -> usize {
        self.sessions.values().filter(|s| s.known && s.exit_code.is_none()).count()
    }

    pub fn closing(&self) -> bool {
        self.closing
    }

    pub fn force_kill(&mut self) {
        for f in self.to_force.drain(..) {
            f.fire();
        }
    }

    fn rid(&mut self, prefix: char) -> String {
        let n = self.next_rid;
        self.next_rid += 1;
        format!("{prefix}{n}")
    }

    fn send(&self, conn: Option<u64>, out: Out) {
        let Some((id, c)) = conn.and_then(|id| self.conns.get(&id).map(|c| (id, c))) else { return };
        if c.overflowed.load(Ordering::Relaxed) {
            return;
        }
        let (n, droppable) = match &out {
            Out::Binary(b) => (b.len(), true),
            Out::Text(t) => (t.len(), false),
            Out::Close(..) => (0, false),
        };
        if !c.meter.queued(n, droppable) {
            c.overflowed.store(true, Ordering::Relaxed);
            c.meter.bind(None);
            eprintln!("orrery-core: socket backlog over {CONN_MAX} bytes, closed {OVERFLOW_CLOSE} conn={id}");
            let _ = c.tx.send(Out::Close(OVERFLOW_CLOSE, "overflow"));
            return;
        }
        if c.tx.send(out).is_err() {
            c.meter.sent(n);
        }
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::Control(Some(msg)) => self.on_main(msg),
            Event::Control(None) => {}
            Event::StdinEof => self.shutdown(),
            Event::Open { conn, tx, meter } => {
                self.conns.insert(conn, Conn { tx, meter, overflowed: AtomicBool::new(false), session: None, queue: None });
            }
            Event::Frame { conn, frame, permit } => {
                self.on_frame(conn, frame);
                drop(permit);
            }
            Event::Closed { conn } => {
                if let Some(c) = self.conns.remove(&conn) {
                    c.meter.bind(None);
                }
                for s in self.sessions.values_mut() {
                    if s.conn == Some(conn) {
                        s.conn = None;
                    }
                }
            }
            Event::Pty { key, ev, credit } => {
                self.on_pty(key, ev);
                drop(credit);
            }
            Event::Release(credit) => drop(credit),
            Event::Reap => self.reap(),
            Event::Owners { started, table } => {
                let (table, asks) = self.tables.scanned(started, table);
                for ask in asks {
                    let owners = owners::answer(&ask, table.as_deref());
                    self.send(Some(ask.conn), text(json!({ "t": "owners", "owners": owners })));
                }
            }
            Event::ResolveTimeout { rid } => {
                if let Some(r) = self.resolving.remove(&rid) {
                    eprintln!("orrery-core: resolve timeout");
                    self.send(Some(r.conn), text(json!({ "t": "error", "message": "spawn" })));
                    self.settle(r.conn, None);
                }
            }
        }
    }

    fn on_main(&mut self, msg: MainToCore) {
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

    fn census(&self) -> Vec<Value> {
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

    fn kill(&mut self, key: u64, now: bool) {
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

    fn on_pty(&mut self, key: u64, ev: PtyEvent) {
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

    fn on_resolved(&mut self, r: Resolved) {
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
                conn: None,
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
        self.attach(conn, key, false);
        self.control.send(CoreToMain::Spawned { id: r.id.clone(), pid });
        self.settle(conn, Some(key));
    }

    fn attach(&mut self, conn: u64, key: u64, replay: bool) {
        if !self.sessions.contains_key(&key) {
            return;
        }
        for (k, other) in self.sessions.iter_mut() {
            if *k != key && other.conn == Some(conn) {
                other.conn = None;
            }
        }
        let Some(s) = self.sessions.get_mut(&key) else { return };
        let old = s.conn.filter(|c| *c != conn);
        s.conn = Some(conn);
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
        let gate = s.gate.clone();
        if let Some(o) = old.and_then(|o| self.conns.get(&o)) {
            if o.meter.bound_to(&gate) {
                o.meter.bind(None);
            }
        }
        self.send(old, Out::Close(4000, "replaced"));
        for o in outs {
            self.send(Some(conn), o);
        }
        if let Some(c) = self.conns.get(&conn) {
            c.meter.bind(Some(gate));
        }
    }

    fn settle(&mut self, conn: u64, key: Option<u64>) {
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

    fn on_frame(&mut self, conn: u64, frame: Frame) {
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

    fn request_spawn(&mut self, conn: u64, msg: Map<String, Value>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

    #[derive(Clone, Default)]
    struct Log(Arc<Mutex<Vec<CoreToMain>>>);

    impl Control for Log {
        fn send(&self, msg: CoreToMain) {
            self.0.lock().unwrap().push(msg);
        }
    }

    impl Log {
        fn removed(&self, id: &str) -> bool {
            self.0.lock().unwrap().iter().any(|m| matches!(m, CoreToMain::Removed { id: i } if i == id))
        }
    }

    fn quick_exit() -> (String, Vec<String>) {
        #[cfg(windows)]
        return ("cmd.exe".into(), vec!["/d".into(), "/c".into(), "exit 0".into()]);
        #[cfg(unix)]
        return ("/bin/sh".into(), vec!["-c".into(), "exit 0".into()]);
    }

    fn long_lived() -> (String, Vec<String>) {
        #[cfg(windows)]
        return ("cmd.exe".into(), vec!["/d".into(), "/q".into(), "/k".into()]);
        #[cfg(unix)]
        return ("/bin/sh".into(), vec!["-c".into(), "sleep 30".into()]);
    }

    fn spawn_long(hub: &mut Hub<Log>, log: &Log, conn: u64, id: &str) -> u64 {
        hub.handle(Event::Frame { conn, frame: Frame::Text(json!({ "t": "spawn", "preset": "shell" }).to_string()), permit: None });
        let rid = match log.0.lock().unwrap().last() {
            Some(CoreToMain::Resolve { rid, .. }) => rid.clone(),
            other => panic!("no resolve: {other:?}"),
        };
        let (file, args) = long_lived();
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        let env: Vec<(String, String)> = std::env::vars().collect();
        hub.handle(Event::Control(Some(MainToCore::Resolved(Resolved { rid, id: id.into(), file, args, env, cwd, agent_session: None, si: None }))));
        *hub.ids.get(id).expect("spawned")
    }

    fn drain(rx: &mut UnboundedReceiver<Out>) -> Vec<Out> {
        let mut out = Vec::new();
        while let Ok(o) = rx.try_recv() {
            out.push(o);
        }
        out
    }

    #[tokio::test]
    async fn backlog_over_conn_max_closes_4001_stops_output_and_reattach_replays() {
        let (tx, _rx) = unbounded_channel();
        let log = Log::default();
        let mut hub = Hub::new(log.clone(), tx);
        let (otx, mut orx) = unbounded_channel();
        hub.handle(Event::Open { conn: 1, tx: otx, meter: Meter::new() });
        let key = spawn_long(&mut hub, &log, 1, "O1");
        let chunk = vec![b'z'; 1024 * 1024];
        for _ in 0..9 {
            hub.handle(Event::Pty { key, ev: PtyEvent::Data(chunk.clone()), credit: None });
        }
        let gate = hub.sessions[&key].gate.clone();
        assert!(!gate.paused(), "an overflowed socket must not keep the pane paused");
        let outs = drain(&mut orx);
        let close_at = outs.iter().position(|o| matches!(o, Out::Close(OVERFLOW_CLOSE, "overflow"))).expect("closed 4001");
        assert_eq!(close_at, outs.len() - 1, "nothing after the 4001 close");
        assert!(outs.iter().filter(|o| matches!(o, Out::Binary(_))).count() >= 7);
        hub.handle(Event::Pty { key, ev: PtyEvent::Data(b"later".to_vec()), credit: None });
        assert!(drain(&mut orx).is_empty());

        let (otx2, mut orx2) = unbounded_channel();
        hub.handle(Event::Open { conn: 2, tx: otx2, meter: Meter::new() });
        hub.handle(Event::Frame { conn: 2, frame: Frame::Text(json!({ "t": "attach", "id": "O1" }).to_string()), permit: None });
        let outs = drain(&mut orx2);
        assert!(matches!(&outs[0], Out::Text(t) if t.contains("\"replay\":true")));
        let replay = outs.iter().find_map(|o| if let Out::Binary(b) = o { Some(b.clone()) } else { None }).expect("replay");
        assert_eq!(replay.len(), crate::ring::RING_BYTES);
        assert!(replay.ends_with(b"later"));
        assert!(drain(&mut orx).is_empty(), "no 4000 on a socket already closed 4001");
        hub.handle(Event::Control(Some(MainToCore::KillAll)));
    }

    #[tokio::test]
    async fn reused_conn_stops_getting_output_and_exit_of_its_previous_session() {
        let (tx, _rx) = unbounded_channel();
        let log = Log::default();
        let mut hub = Hub::new(log.clone(), tx);
        let (otx, mut orx) = unbounded_channel();
        hub.handle(Event::Open { conn: 1, tx: otx, meter: Meter::new() });
        let a = spawn_long(&mut hub, &log, 1, "A1");
        hub.handle(Event::Frame { conn: 1, frame: Frame::Text(json!({ "t": "kill", "id": "A1" }).to_string()), permit: None });
        assert!(log.removed("A1"));
        let b = spawn_long(&mut hub, &log, 1, "B1");
        assert_eq!(hub.sessions.get(&a).map(|s| s.conn), Some(None));
        drain(&mut orx);
        hub.handle(Event::Pty { key: a, ev: PtyEvent::Data(b"old".to_vec()), credit: None });
        hub.handle(Event::Pty { key: a, ev: PtyEvent::Exit { code: 1, signal: None }, credit: None });
        assert!(drain(&mut orx).is_empty(), "old pane output / exit leaked into the new pane's socket");
        hub.handle(Event::Pty { key: b, ev: PtyEvent::Data(b"new".to_vec()), credit: None });
        assert!(matches!(drain(&mut orx).as_slice(), [Out::Binary(d)] if d == b"new"));
        hub.handle(Event::Control(Some(MainToCore::KillAll)));
    }

    async fn spawn_and_wait_exit(hub: &mut Hub<Log>, log: &Log, rx: &mut UnboundedReceiver<Event>, conn: u64, id: &str) {
        let (otx, _orx) = unbounded_channel();
        hub.handle(Event::Open { conn, tx: otx, meter: Meter::new() });
        hub.handle(Event::Frame { conn, frame: Frame::Text(json!({ "t": "spawn", "preset": "shell" }).to_string()), permit: None });
        let rid = match log.0.lock().unwrap().last() {
            Some(CoreToMain::Resolve { rid, .. }) => rid.clone(),
            other => panic!("no resolve: {other:?}"),
        };
        let (file, args) = quick_exit();
        let cwd = std::env::temp_dir().to_string_lossy().into_owned();
        let env: Vec<(String, String)> = std::env::vars().collect();
        hub.handle(Event::Control(Some(MainToCore::Resolved(Resolved { rid, id: id.into(), file, args, env, cwd, agent_session: None, si: None }))));
        loop {
            let ev = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.expect("exit within 10 s").expect("channel open");
            let exit = matches!(&ev, Event::Pty { ev: PtyEvent::Exit { .. }, .. });
            hub.handle(ev);
            if exit {
                break;
            }
        }
    }

    #[tokio::test]
    async fn reaper_drops_exited_unattached_sessions_after_30_minutes_by_fake_clock() {
        let (tx, mut rx) = unbounded_channel();
        let log = Log::default();
        let mut hub = Hub::new(log.clone(), tx);
        let t0 = Instant::now();
        let offset = Arc::new(Mutex::new(Duration::ZERO));
        let o = offset.clone();
        hub.set_clock(Arc::new(move || t0 + *o.lock().unwrap()));
        spawn_and_wait_exit(&mut hub, &log, &mut rx, 1, "R1").await;
        assert_eq!(hub.census().len(), 1);

        *offset.lock().unwrap() = Duration::from_secs(31 * 60);
        hub.handle(Event::Reap);
        assert!(!log.removed("R1"), "an attached socket keeps the exited session");

        hub.handle(Event::Closed { conn: 1 });
        *offset.lock().unwrap() = Duration::from_secs(29 * 60);
        hub.handle(Event::Reap);
        assert!(!log.removed("R1"), "29 minutes after exit is too early");
        assert_eq!(hub.census().len(), 1);

        *offset.lock().unwrap() = Duration::from_secs(30 * 60 + 1);
        hub.handle(Event::Reap);
        assert!(log.removed("R1"));
        assert!(hub.census().is_empty());
        assert!(hub.sessions.is_empty());

        let (otx, mut orx) = unbounded_channel();
        hub.handle(Event::Open { conn: 9, tx: otx, meter: Meter::new() });
        hub.handle(Event::Frame { conn: 9, frame: Frame::Text(json!({ "t": "attach", "id": "R1" }).to_string()), permit: None });
        match orx.try_recv() {
            Ok(Out::Text(t)) => assert_eq!(t, r#"{"t":"gone"}"#),
            _ => panic!("expected gone"),
        }
    }

    #[tokio::test]
    async fn dropping_a_session_closes_its_gate() {
        let (tx, mut rx) = unbounded_channel();
        let log = Log::default();
        let mut hub = Hub::new(log.clone(), tx);
        spawn_and_wait_exit(&mut hub, &log, &mut rx, 1, "R2").await;
        let gate = hub.sessions.values().next().unwrap().gate.clone();
        let full = gate.acquire(crate::flow::INFLIGHT_MAX);
        hub.handle(Event::Control(Some(MainToCore::KillAll)));
        assert!(hub.sessions.is_empty());
        assert!(log.removed("R2"));
        let more = gate.acquire(1);
        drop((full, more));
    }

    #[test]
    fn early_exit_note_matches_node() {
        assert_eq!(early_exit_note("shell", 3, None, 120, false), "pty early exit: preset=shell code=3 after=120ms");
        assert_eq!(early_exit_note("claude", 3221225794, None, 5, true), "pty early exit: preset=claude code=3221225794 (0xC0000142) after=5ms killed-by-app");
        assert_eq!(early_exit_note("codex", -1073741502, Some("SIGHUP"), 7, false), "pty early exit: preset=codex code=-1073741502 (0xC0000142) signal=SIGHUP after=7ms");
    }
}
