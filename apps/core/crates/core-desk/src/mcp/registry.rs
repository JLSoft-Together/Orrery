use super::notes::{Clock, NoteQueue};
use super::paths::strip_verbatim;
use crate::util::path_str;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

pub const MAX_LIVE: usize = 16;
pub const TICKET_TTL_MS: f64 = 60_000.0;
pub const OPEN_REPLY_MS: u64 = 8_000;
pub const OPEN_RATE_MAX: usize = 20;
pub const OPEN_RATE_MS: f64 = 60_000.0;
pub const SPAWN_WAIT_MS: u64 = 10_000;
pub const RESERVE_MS: f64 = 30_000.0;
pub const PAGE_REPLY_MS: u64 = 8_000;

pub fn desk_reply_ms(tool: &str) -> u64 {
    match tool {
        "free_port" | "telegram_receive" => 600_000,
        "telegram_send" => 200_000,
        _ => 40_000,
    }
}

pub trait Bridge: Send + Sync {
    fn ask(&self, action: &str, args: Value, pane: &str, timeout_ms: u64) -> Value;
    fn desk(&self, channel: &str, call: Value) -> Value;
    fn note(&self, note: Value);
}

pub struct Pane {
    pub id: String,
    pub preset: String,
    pub cwd: String,
    pub live_cwd: Option<String>,
    pub pid: u32,
    pub spawned: bool,
    pub exited: bool,
    pub reserved_at: f64,
    pub token: String,
    pub ticket: Option<String>,
    pub agent_session: Option<String>,
    pub opens: Vec<f64>,
    pub rates: HashMap<String, Vec<f64>>,
    pub notes: Option<NoteQueue>,
}

pub struct Ticket {
    pub preset: String,
    pub cwd: String,
    pub session_id: Option<String>,
    pub name: Option<String>,
    pub expires: f64,
}

#[derive(Default)]
pub struct St {
    pub panes: HashMap<String, Pane>,
    pub tokens: HashMap<String, String>,
    pub tickets: HashMap<String, Ticket>,
    pub waits: HashMap<String, Option<String>>,
    pub disposed: bool,
}

pub struct Registry {
    pub(super) st: Mutex<St>,
    pub(super) spawn_cv: Condvar,
    pub(super) bridge: Arc<dyn Bridge>,
    pub(super) deepseek: bool,
    pub(super) home: String,
    pub(super) now: Clock,
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    let _ = getrandom::fill(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn unc(s: &str) -> bool {
    s.starts_with("\\\\") || s.starts_with("//") || s.starts_with("\\/") || s.starts_with("/\\")
}

pub fn resolve_dir(cwd: Option<&str>, home: &str) -> Option<String> {
    let Some(raw) = cwd.filter(|c| !c.is_empty()) else { return Some(home.to_string()) };
    if unc(raw) || !Path::new(raw).is_absolute() {
        return None;
    }
    let real = strip_verbatim(std::fs::canonicalize(raw).ok()?);
    let s = path_str(&real);
    (!unc(&s) && real.is_dir()).then_some(s)
}

impl Registry {
    pub fn new(bridge: Arc<dyn Bridge>, deepseek: bool, home: String, now: Clock) -> Arc<Registry> {
        Arc::new(Registry { st: Mutex::new(St::default()), spawn_cv: Condvar::new(), bridge, deepseek, home, now })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, St> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn now(&self) -> f64 {
        (self.now)()
    }

    pub fn preset_enabled(&self, preset: &str) -> bool {
        matches!(preset, "claude" | "codex" | "shell") || (preset == "deepseek" && self.deepseek)
    }

    pub fn live_count(&self) -> usize {
        let now = self.now();
        let mut st = self.lock();
        Self::count(&mut st, now)
    }

    pub(super) fn count(st: &mut St, now: f64) -> usize {
        let stale: Vec<String> = st.panes.values().filter(|p| !p.spawned && now - p.reserved_at > RESERVE_MS).map(|p| p.id.clone()).collect();
        for id in stale {
            if let Some(p) = st.panes.remove(&id) {
                st.tokens.remove(&p.token);
            }
        }
        st.panes.values().filter(|p| !p.exited).count()
    }

    pub fn take_ticket(&self, id: &str, preset: &str) -> Option<(Option<String>, Option<String>)> {
        let now = self.now();
        let mut st = self.lock();
        if st.tickets.get(id).is_none_or(|t| t.preset != preset) {
            return None;
        }
        let t = st.tickets.remove(id)?;
        (t.expires > now).then_some((t.session_id, t.name))
    }

    pub fn add(&self, id: &str, preset: &str, cwd: &str, agent_session: Option<&str>, ticket: Option<&str>) -> String {
        let token = random_hex(32);
        let now = self.now();
        let mut st = self.lock();
        st.panes.insert(
            id.to_string(),
            Pane {
                id: id.to_string(),
                preset: preset.to_string(),
                cwd: cwd.to_string(),
                live_cwd: None,
                pid: 0,
                spawned: false,
                exited: false,
                reserved_at: now,
                token: token.clone(),
                ticket: ticket.map(str::to_string),
                agent_session: agent_session.map(str::to_string),
                opens: Vec::new(),
                rates: HashMap::new(),
                notes: None,
            },
        );
        st.tokens.insert(token.clone(), id.to_string());
        token
    }

    pub fn spawned(&self, id: &str, pid: u32) {
        let mut st = self.lock();
        let Some(p) = st.panes.get_mut(id) else { return };
        if p.spawned {
            return;
        }
        p.spawned = true;
        p.pid = pid;
        let ticket = p.ticket.clone();
        if let Some(slot) = ticket.and_then(|t| st.waits.get_mut(&t)) {
            *slot = Some(id.to_string());
            self.spawn_cv.notify_all();
        }
    }

    pub fn set_pid(&self, id: &str, pid: u32) {
        if let Some(p) = self.lock().panes.get_mut(id) {
            p.pid = pid;
        }
    }

    pub fn exited(&self, id: &str) {
        if let Some(p) = self.lock().panes.get_mut(id) {
            p.exited = true;
        }
    }

    pub fn removed(&self, id: &str) {
        let mut st = self.lock();
        if let Some(p) = st.panes.remove(id) {
            st.tokens.remove(&p.token);
            if let Some(n) = p.notes {
                n.dispose();
            }
        }
    }

    pub fn set_live_cwd(&self, id: &str, cwd: &str) {
        if let Some(p) = self.lock().panes.get_mut(id) {
            p.live_cwd = Some(cwd.to_string());
        }
    }

    pub fn dispose(&self) {
        let mut st = self.lock();
        st.disposed = true;
        for p in st.panes.values() {
            if let Some(n) = &p.notes {
                n.dispose();
            }
        }
        st.panes.clear();
        st.tokens.clear();
        st.tickets.clear();
        self.spawn_cv.notify_all();
    }

    pub(super) fn wait_spawn(&self, ticket: &str, ms: u64) -> Option<String> {
        let deadline = std::time::Instant::now() + Duration::from_millis(ms);
        let mut st = self.lock();
        loop {
            if st.disposed {
                st.waits.remove(ticket);
                return None;
            }
            if let Some(Some(id)) = st.waits.get(ticket) {
                let id = id.clone();
                st.waits.remove(ticket);
                return Some(id);
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                st.waits.remove(ticket);
                return None;
            }
            st = self.spawn_cv.wait_timeout(st, left).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}
