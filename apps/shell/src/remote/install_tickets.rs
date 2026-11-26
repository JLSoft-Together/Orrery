use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};

use super::guard_ip::norm;
use super::install::{MAX_SLOTS, TTL_SECS};
use crate::state::lock;

const TOKEN_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketKind {
    ApkPage,
    ApkOffer,
}

impl TicketKind {
    pub const fn ttl(self) -> u64 {
        match self {
            Self::ApkPage | Self::ApkOffer => TTL_SECS,
        }
    }
}

#[derive(Clone)]
struct Ticket {
    kind: TicketKind,
    token: String,
    path: PathBuf,
    version: String,
    exp: u64,
    fetched: Option<IpAddr>,
    used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub kind: TicketKind,
    pub path: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deny {
    Unarmed,
    Missing,
    NotFetched,
    OtherIp,
    Used,
}

impl Deny {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unarmed => "install_unarmed",
            Self::Missing => "install_missing",
            Self::NotFetched => "install_fetch",
            Self::OtherIp => "install_ip",
            Self::Used => "install_used",
        }
    }
}

#[derive(Default)]
pub struct Tickets {
    slots: Mutex<Vec<Ticket>>,
    armed_until: Mutex<u64>,
    last_fetch: Mutex<Option<(IpAddr, u64)>>,
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn find<'a>(slots: &'a mut [Ticket], token: &str) -> Option<&'a mut Ticket> {
    let mut hit = None;
    for t in slots.iter_mut() {
        if same(t.token.as_bytes(), token.as_bytes()) {
            hit = Some(t);
        }
    }
    hit
}

impl Tickets {
    pub fn issue(&self, kind: TicketKind, path: PathBuf, now: u64) -> Option<(String, u64)> {
        self.issue_with(kind, path, String::new(), now)
    }

    pub fn issue_with(&self, kind: TicketKind, path: PathBuf, version: String, now: u64) -> Option<(String, u64)> {
        let mut raw = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut raw).ok()?;
        let token = URL_SAFE_NO_PAD.encode(raw);
        let exp = now + kind.ttl();
        let mut slots = lock(&self.slots);
        slots.retain(|t| now < t.exp);
        if slots.len() >= MAX_SLOTS {
            let victim = slots.iter().position(|t| t.kind == kind).unwrap_or(0);
            slots.remove(victim);
        }
        slots.push(Ticket { kind, token: token.clone(), path, version, exp, fetched: None, used: false });
        Some((token, exp))
    }

    pub fn revoke(&self) {
        lock(&self.slots).clear();
        *lock(&self.armed_until) = 0;
        *lock(&self.last_fetch) = None;
    }

    pub fn len(&self) -> usize {
        lock(&self.slots).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn hit(&self, token: &str, now: u64) -> Option<Hit> {
        let mut slots = lock(&self.slots);
        slots.retain(|t| now < t.exp);
        find(&mut slots, token).map(|t| Hit { kind: t.kind, path: t.path.clone(), version: t.version.clone() })
    }

    pub fn note_fetch(&self, token: &str, ip: IpAddr, now: u64) -> bool {
        let ip = norm(ip);
        let first = {
            let mut slots = lock(&self.slots);
            slots.retain(|t| now < t.exp);
            match find(&mut slots, token) {
                Some(t) if t.fetched.is_none() => {
                    t.fetched = Some(ip);
                    true
                }
                _ => false,
            }
        };
        if first {
            *lock(&self.last_fetch) = Some((ip, now));
        }
        first
    }

    pub fn arm(&self, on: bool, now: u64) {
        *lock(&self.armed_until) = if on { now + TTL_SECS } else { 0 };
    }

    pub fn armed(&self, now: u64) -> bool {
        now < *lock(&self.armed_until)
    }

    pub fn claim_pair(&self, token: &str, ip: IpAddr, now: u64) -> Result<(), Deny> {
        if !self.armed(now) {
            return Err(Deny::Unarmed);
        }
        let mut slots = lock(&self.slots);
        slots.retain(|t| now < t.exp);
        let t = find(&mut slots, token).filter(|t| t.kind == TicketKind::ApkPage).ok_or(Deny::Missing)?;
        match t.fetched {
            None => return Err(Deny::NotFetched),
            Some(f) if f != norm(ip) => return Err(Deny::OtherIp),
            Some(_) => {}
        }
        if t.used {
            return Err(Deny::Used);
        }
        t.used = true;
        Ok(())
    }

    pub fn fetched_json(&self) -> Value {
        lock(&self.last_fetch).map_or(Value::Null, |(ip, at)| json!({ "ip": ip.to_string(), "at": at }))
    }
}
