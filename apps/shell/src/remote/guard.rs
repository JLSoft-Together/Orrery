use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use core_remote_proto::relay::{encode_block_list, encode_block_remove, encode_block_set, encode_trust_set, BlockRow, BlockRule, MAX_GUARD_RULES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Notify;

use super::config::write_json;
use super::guard_ip::{key, Cidr};
use crate::state::lock;

pub const GUARD_FILE: &str = "guard.json";
pub const STRIKE_MAX: usize = 20;
pub const STRIKE_WINDOW: u64 = 600;
pub const BLOCK_SECS: [u64; 3] = [3600, 86_400, 7 * 86_400];
pub const REPEAT_SECS: u64 = 7 * 86_400;
pub const SPARE_SECS: u64 = 600;
pub const LOG_MAX: usize = 200;
pub const LAN_ROWS: usize = 256;
pub const STRIKE_KEYS: usize = 1024;
pub const REMOVES_MAX: usize = 64;
pub const TTL_MIN: u64 = 60;
pub const TTL_MAX: u64 = 365 * 86_400;
const LOG_EMIT_GAP: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manual {
    pub c: String,
    #[serde(default)]
    pub until: Option<u64>,
    #[serde(default)]
    pub at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    trust: Vec<String>,
    #[serde(default)]
    manual: Vec<Manual>,
    #[serde(default)]
    lan: Vec<BlockRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub at: u64,
    pub ip: String,
    pub via: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    pub result: &'static str,
    pub why: &'static str,
}

#[derive(Default)]
struct State {
    file: File,
    strikes: HashMap<String, Vec<u64>>,
    good: HashMap<String, (usize, u64)>,
    relay: Vec<BlockRow>,
    log: VecDeque<Entry>,
    log_emit: u64,
    removes: Vec<Option<String>>,
    dirty: bool,
    alert_at: Option<u64>,
    missed: u32,
    place_at: HashMap<String, u64>,
}

pub struct Guard {
    path: PathBuf,
    state: Mutex<State>,
    rev: AtomicU64,
    pub wake: Notify,
}

fn rules(list: &[Manual], now: u64) -> Vec<BlockRule> {
    list.iter().filter(|m| m.until.is_none_or(|u| u > now)).map(|m| BlockRule { cidr: m.c.clone(), ttl: m.until.map(|u| u - now) }).collect()
}

fn covered(text: &str, ip: IpAddr) -> bool {
    Cidr::parse(text).is_some_and(|c| c.contains(ip))
}

fn row_cidr(row: &BlockRow) -> Option<Cidr> {
    Cidr::parse(&row.k)
}

impl Guard {
    pub fn load(dir: &Path) -> Guard {
        let path = dir.join(GUARD_FILE);
        let file = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<File>(&t).ok()).unwrap_or_default();
        Guard { path, state: Mutex::new(State { file, ..State::default() }), rev: AtomicU64::new(1), wake: Notify::new() }
    }

    fn save(&self, st: &State) {
        write_json(&self.path, &st.file);
        self.rev.fetch_add(1, Ordering::SeqCst);
    }

    pub fn rev(&self) -> u64 {
        self.rev.load(Ordering::SeqCst)
    }

    pub fn manual_blocked(&self, ip: IpAddr, now: u64) -> bool {
        lock(&self.state).file.manual.iter().any(|m| m.until.is_none_or(|u| u > now) && covered(&m.c, ip))
    }

    pub fn admits(&self, ip: IpAddr, now: u64) -> bool {
        if self.manual_blocked(ip, now) {
            return false;
        }
        let st = lock(&self.state);
        let k = key(ip);
        !st.file.lan.iter().any(|r| r.k == k && r.until > now)
    }

    pub fn good_open(&self, ip: IpAddr, now: u64) {
        let mut st = lock(&self.state);
        let e = st.good.entry(key(ip)).or_insert((0, now));
        e.0 += 1;
        e.1 = now;
    }

    pub fn good_close(&self, ip: IpAddr, now: u64) {
        let mut st = lock(&self.state);
        if let Some(e) = st.good.get_mut(&key(ip)) {
            e.0 = e.0.saturating_sub(1);
            e.1 = now;
        }
    }

    pub fn strike(&self, ip: IpAddr, why: &'static str, now: u64) -> Option<BlockRow> {
        let k = key(ip);
        let mut st = lock(&self.state);
        if st.file.trust.iter().any(|t| covered(t, ip)) {
            return None;
        }
        let list = st.strikes.entry(k.clone()).or_default();
        list.retain(|t| now.saturating_sub(*t) < STRIKE_WINDOW);
        list.push(now);
        let n = list.len();
        if st.strikes.len() > STRIKE_KEYS {
            st.strikes.retain(|_, l| l.iter().any(|t| now.saturating_sub(*t) < STRIKE_WINDOW));
        }
        let spared = st.good.get(&k).is_some_and(|(open, last)| *open > 0 || now.saturating_sub(*last) < SPARE_SECS);
        if n < STRIKE_MAX || spared {
            return None;
        }
        let old = st.file.lan.iter().find(|r| r.k == k).cloned();
        if old.as_ref().is_some_and(|r| r.until > now) {
            return None;
        }
        let level = match &old {
            Some(r) if now <= r.until + REPEAT_SECS => (r.level + 1).min(BLOCK_SECS.len() as u8),
            _ => 1,
        };
        let row = BlockRow { k: k.clone(), until: now + BLOCK_SECS[usize::from(level) - 1], level, n: n as u32, at: now, why: why.to_owned() };
        st.strikes.remove(&k);
        st.file.lan.retain(|r| r.k != k && r.until + REPEAT_SECS >= now);
        st.file.lan.push(row.clone());
        if st.file.lan.len() > LAN_ROWS {
            st.file.lan.sort_by_key(|r| std::cmp::Reverse(r.until));
            st.file.lan.truncate(LAN_ROWS);
        }
        self.save(&st);
        Some(row)
    }

    pub fn relay_list(&self, items: Vec<BlockRow>) {
        lock(&self.state).relay = items;
        self.rev.fetch_add(1, Ordering::SeqCst);
    }

    pub fn relay_blocked(&self, row: BlockRow) {
        let mut st = lock(&self.state);
        st.relay.retain(|r| r.k != row.k);
        st.relay.insert(0, row);
        st.relay.truncate(1024);
        self.rev.fetch_add(1, Ordering::SeqCst);
    }

    pub fn log(&self, entry: Entry) -> bool {
        let mut st = lock(&self.state);
        let now = entry.at;
        st.log.push_front(entry);
        st.log.truncate(LOG_MAX);
        self.rev.fetch_add(1, Ordering::SeqCst);
        if now.saturating_sub(st.log_emit) >= LOG_EMIT_GAP {
            st.log_emit = now;
            return true;
        }
        false
    }

    fn push_remove(st: &mut State, target: Option<String>) {
        if target.is_none() {
            st.removes.clear();
        }
        st.removes.push(target);
        if st.removes.len() > REMOVES_MAX {
            st.removes.remove(0);
        }
    }

    pub fn block(&self, text: &str, ttl: Option<u64>, now: u64) -> Result<(), &'static str> {
        let cidr = Cidr::parse(text).ok_or("invalid")?;
        if ttl.is_some_and(|t| !(TTL_MIN..=TTL_MAX).contains(&t)) {
            return Err("invalid");
        }
        let mut st = lock(&self.state);
        let live = st.good.iter().any(|(k, (n, _))| *n > 0 && Cidr::parse(k).is_some_and(|c| c.overlaps(&cidr)));
        if live {
            return Err("live");
        }
        let c = cidr.text();
        st.file.manual.retain(|m| m.c != c && m.until.is_none_or(|u| u > now));
        if st.file.manual.len() >= MAX_GUARD_RULES {
            return Err("full");
        }
        st.file.manual.push(Manual { c, until: ttl.map(|t| now + t), at: now });
        st.dirty = true;
        self.save(&st);
        drop(st);
        self.wake.notify_one();
        Ok(())
    }

    pub fn unblock(&self, target: Option<&str>) -> bool {
        let mut st = lock(&self.state);
        let before = (st.file.lan.len(), st.file.manual.len(), st.relay.len());
        match target {
            None => {
                st.file.lan.clear();
                st.file.manual.clear();
                st.relay.clear();
                st.strikes.clear();
                Self::push_remove(&mut st, None);
            }
            Some(t) => {
                let Some(cidr) = Cidr::parse(t) else { return false };
                let hit = |row: &BlockRow| row_cidr(row).is_some_and(|c| c.overlaps(&cidr));
                st.file.lan.retain(|r| !hit(r));
                st.relay.retain(|r| !hit(r));
                let exact = cidr.text();
                st.file.manual.retain(|m| m.c != exact);
                st.strikes.retain(|k, _| Cidr::parse(k).is_none_or(|c| !c.overlaps(&cidr)));
                Self::push_remove(&mut st, Some(cidr.text()));
            }
        }
        st.dirty = true;
        let changed = before != (st.file.lan.len(), st.file.manual.len(), st.relay.len());
        self.save(&st);
        drop(st);
        self.wake.notify_one();
        changed
    }

    pub fn set_trust(&self, list: &[String]) -> Result<(), &'static str> {
        if list.len() > MAX_GUARD_RULES {
            return Err("full");
        }
        let mut out: Vec<String> = Vec::new();
        for raw in list {
            let c = Cidr::parse(raw).ok_or("invalid")?.text();
            if !out.contains(&c) {
                out.push(c);
            }
        }
        let mut st = lock(&self.state);
        let trusted: Vec<Cidr> = out.iter().filter_map(|t| Cidr::parse(t)).collect();
        st.file.lan.retain(|r| row_cidr(r).is_none_or(|c| !trusted.iter().any(|t| t.overlaps(&c))));
        st.strikes.retain(|k, _| Cidr::parse(k).is_none_or(|c| !trusted.iter().any(|t| t.overlaps(&c))));
        st.file.trust = out;
        st.dirty = true;
        self.save(&st);
        drop(st);
        self.wake.notify_one();
        Ok(())
    }

    fn frames(st: &mut State, now: u64, all: bool) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        if all || std::mem::take(&mut st.dirty) {
            st.dirty = false;
            out.extend(encode_trust_set(&st.file.trust).ok());
            out.extend(encode_block_set(&rules(&st.file.manual, now)).ok());
        }
        for target in std::mem::take(&mut st.removes) {
            out.extend(encode_block_remove(target.as_deref()).ok());
        }
        if all {
            out.extend(encode_block_list().ok());
        }
        out
    }

    pub fn sync_frames(&self, now: u64) -> Vec<Vec<u8>> {
        Self::frames(&mut lock(&self.state), now, true)
    }

    pub fn take_ops(&self, now: u64) -> Vec<Vec<u8>> {
        Self::frames(&mut lock(&self.state), now, false)
    }

    pub fn alert_gate(&self, now: u64) -> Option<u32> {
        let mut st = lock(&self.state);
        if st.alert_at.is_some_and(|t| now.saturating_sub(t) < 3600) {
            st.missed += 1;
            return None;
        }
        st.alert_at = Some(now);
        Some(std::mem::take(&mut st.missed))
    }

    pub fn place_gate(&self, device: &str, now: u64) -> bool {
        let mut st = lock(&self.state);
        if st.place_at.get(device).is_some_and(|t| now.saturating_sub(*t) < 3600) {
            return false;
        }
        if st.place_at.len() >= 256 {
            st.place_at.clear();
        }
        st.place_at.insert(device.to_owned(), now);
        true
    }

    pub fn json(&self, now: u64) -> Value {
        let st = lock(&self.state);
        let mut blocked: Vec<Value> = Vec::new();
        for m in st.file.manual.iter().filter(|m| m.until.is_none_or(|u| u > now)) {
            blocked.push(json!({ "key": m.c, "source": "manual", "why": "manual", "count": 0, "at": m.at, "until": m.until, "level": 0 }));
        }
        for (source, rows) in [("lan", &st.file.lan), ("relay", &st.relay)] {
            for r in rows.iter().filter(|r| r.until > now) {
                blocked.push(json!({ "key": r.k, "source": source, "why": r.why, "count": r.n, "at": r.at, "until": r.until, "level": r.level }));
            }
        }
        json!({ "blocked": blocked, "trusted": st.file.trust, "log": st.log, "rev": self.rev() })
    }
}
