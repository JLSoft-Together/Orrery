use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::state::{lock, take_slot};

pub const HANDSHAKE: Duration = Duration::from_secs(5);
pub const MAX_PENDING_PER_IP: usize = 8;
pub const MAX_PENDING_TOTAL: usize = 64;
pub const MAX_OPENS_PER_MINUTE: usize = 60;
pub const MAX_SESSIONS: usize = 16;
pub const MAX_COMMANDS_PER_MINUTE: usize = 240;
const MINUTE: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Entry {
    pending: usize,
    opens: Vec<Instant>,
}

#[derive(Default)]
pub struct Gate {
    map: Mutex<HashMap<IpAddr, Entry>>,
}

pub struct Pending {
    gate: Arc<Gate>,
    ip: IpAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    Total,
    Ip,
}

impl Gate {
    pub fn admit(self: &Arc<Self>, ip: IpAddr, now: Instant) -> Result<Pending, Refused> {
        let mut map = lock(&self.map);
        let total: usize = map.values().map(|e| e.pending).sum();
        let entry = map.entry(ip).or_default();
        if entry.pending >= MAX_PENDING_PER_IP {
            return Err(Refused::Ip);
        }
        if !take_slot(&mut entry.opens, now, MAX_OPENS_PER_MINUTE, MINUTE) {
            return Err(Refused::Ip);
        }
        if total >= MAX_PENDING_TOTAL {
            entry.opens.pop();
            return Err(Refused::Total);
        }
        entry.pending += 1;
        Ok(Pending { gate: self.clone(), ip })
    }

    #[cfg(test)]
    pub fn pending(&self, ip: IpAddr) -> usize {
        lock(&self.map).get(&ip).map_or(0, |e| e.pending)
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        let mut map = lock(&self.gate.map);
        let empty = match map.get_mut(&self.ip) {
            Some(e) => {
                e.pending = e.pending.saturating_sub(1);
                e.pending == 0 && e.opens.iter().all(|t| t.elapsed() >= MINUTE)
            }
            None => false,
        };
        if empty {
            map.remove(&self.ip);
        }
    }
}

#[derive(Default)]
pub struct Rate {
    times: Vec<Instant>,
}

impl Rate {
    pub fn take(&mut self, now: Instant, max: usize) -> bool {
        take_slot(&mut self.times, now, max, MINUTE)
    }
}
