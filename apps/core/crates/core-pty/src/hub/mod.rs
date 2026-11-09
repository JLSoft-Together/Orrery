mod frames;
mod from_main;
mod spawn;
#[cfg(test)]
mod tests;

use crate::control::{CoreToMain, MainToCore};
use crate::flow::{Credit, Gate, Meter, CONN_MAX};
use crate::owners::{self, PidTable, TableCache};
use crate::ring::Ring;
use crate::session::{Force, PtyEvent, PtyProc};
use serde_json::{json, Value};
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
}
