use core_store::model::{PeerLink, PeerLinkDir, PeerLinkKind};
use core_store::time::iso_of_ms;
use core_transcript::time::parse_iso;
use indexmap::IndexMap;
use serde::Deserialize;

pub const PEER_TTL_MS: i64 = 60 * 60_000;
pub const PEER_HOLD_MS: i64 = 2_000;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInbound {
    pub receiver: String,
    pub from_key: String,
    pub msg_id: Option<String>,
    pub dedupe_key: String,
    pub at: Option<String>,
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerSent {
    pub terminal_id: String,
    pub agent_id: String,
    pub msg_id: String,
    pub at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeerMessage {
    pub from: String,
    pub to: String,
    pub agent_id: String,
    pub at: String,
}

pub trait PeerSink {
    fn upsert_link(&mut self, link: PeerLink);
    fn remove_link(&mut self, id: &str);
    fn sender_of(&self, from_key: &str) -> Option<String>;
    fn on_message(&mut self, message: PeerMessage);
}

pub fn peer_link_id(a: &str, b: &str) -> String {
    if a < b {
        format!("{a}>{b}")
    } else {
        format!("{b}>{a}")
    }
}

fn time_of(at: Option<&str>, fallback: i64) -> i64 {
    at.filter(|s| !s.is_empty()).and_then(parse_iso).unwrap_or(fallback)
}

struct Pending {
    item: PeerInbound,
    queued_at: i64,
}

struct Sender {
    sent: PeerSent,
    at_ms: i64,
}

pub struct PeerLinkStore {
    ttl_ms: i64,
    hold_ms: i64,
    links: IndexMap<String, PeerLink>,
    seen: IndexMap<String, i64>,
    senders: IndexMap<String, Sender>,
    pending: Vec<Pending>,
    wake_at: Option<i64>,
}

impl PeerLinkStore {
    pub fn new(ttl_ms: i64, hold_ms: Option<i64>) -> Self {
        PeerLinkStore { ttl_ms, hold_ms: hold_ms.unwrap_or(PEER_HOLD_MS), links: IndexMap::new(), seen: IndexMap::new(), senders: IndexMap::new(), pending: Vec::new(), wake_at: None }
    }

    pub fn list(&self) -> Vec<PeerLink> {
        self.links.values().cloned().collect()
    }

    pub fn wake_at(&self) -> Option<i64> {
        self.wake_at
    }

    pub fn sent(&mut self, items: Vec<PeerSent>, now: i64) {
        let cutoff = now - self.ttl_ms;
        for s in items {
            let at_ms = time_of(s.at.as_deref(), now);
            if at_ms >= cutoff {
                self.senders.insert(s.msg_id.clone(), Sender { sent: s, at_ms });
            }
        }
    }

    pub fn inbound(&mut self, items: Vec<PeerInbound>, clock: i64) {
        self.pending.extend(items.into_iter().map(|item| Pending { item, queued_at: clock }));
    }

    pub fn flush(&mut self, sink: &mut dyn PeerSink, now: i64, clock: i64, final_pass: bool) {
        self.wake_at = None;
        let cutoff = now - self.ttl_ms;
        let mut keep = Vec::new();
        let mut wake: Option<i64> = None;
        for p in std::mem::take(&mut self.pending) {
            let at_ms = time_of(p.item.at.as_deref(), now);
            let key = p.item.msg_id.clone().unwrap_or_else(|| format!("{}|{}", p.item.receiver, p.item.dedupe_key));
            if at_ms < cutoff || self.seen.contains_key(&key) {
                continue;
            }
            let sent = p.item.msg_id.as_ref().and_then(|m| self.senders.get(m)).map(|s| s.sent.clone());
            let sender = sink.sender_of(&p.item.from_key).or_else(|| sent.as_ref().map(|s| s.terminal_id.clone()));
            let deadline = p.queued_at + self.hold_ms;
            let waiting = p.item.live && p.item.msg_id.is_some() && sent.is_none() && clock < deadline;
            if let (Some(sender), false) = (&sender, waiting) {
                self.seen.insert(key, at_ms);
                if *sender != p.item.receiver {
                    self.apply(sink, &p.item, sender, sent.as_ref(), at_ms);
                }
                continue;
            }
            if sender.is_none() && !waiting && final_pass {
                continue;
            }
            if waiting {
                wake = Some(wake.map_or(deadline, |w: i64| w.min(deadline)));
            }
            keep.push(p);
        }
        self.pending = keep;
        self.wake_at = wake;
    }

    fn apply(&mut self, sink: &mut dyn PeerSink, p: &PeerInbound, sender: &str, sent: Option<&PeerSent>, at_ms: i64) {
        let id = peer_link_id(sender, &p.receiver);
        let (a, b) = if sender < p.receiver.as_str() { (sender, p.receiver.as_str()) } else { (p.receiver.as_str(), sender) };
        let dir = if sender == a { PeerLinkDir::Ab } else { PeerLinkDir::Ba };
        let at = iso_of_ms(at_ms);
        let agent_id = match sent {
            Some(s) if s.terminal_id == sender => s.agent_id.clone(),
            _ => format!("{sender}:main"),
        };
        let mut link = self.links.get(&id).cloned().unwrap_or_else(|| PeerLink {
            id: id.clone(),
            kind: PeerLinkKind::Message,
            a: a.to_owned(),
            b: b.to_owned(),
            ab: 0,
            ba: 0,
            first_at: at.clone(),
            last_at: at.clone(),
            last_dir: dir,
            last_agent_id: Some(agent_id.clone()),
        });
        match dir {
            PeerLinkDir::Ab => link.ab += 1,
            PeerLinkDir::Ba => link.ba += 1,
        }
        if at < link.first_at {
            link.first_at = at.clone();
        }
        if at >= link.last_at {
            link.last_at = at.clone();
            link.last_dir = dir;
            link.last_agent_id = Some(agent_id.clone());
        }
        self.links.insert(id, link.clone());
        sink.upsert_link(link);
        if p.live {
            sink.on_message(PeerMessage { from: sender.to_owned(), to: p.receiver.clone(), agent_id, at });
        }
    }

    pub fn prune(&mut self, sink: &mut dyn PeerSink, now: i64) {
        let cutoff = now - self.ttl_ms;
        let stale: Vec<String> = self.links.iter().filter(|(_, l)| !parse_iso(&l.last_at).is_some_and(|t| t >= cutoff)).map(|(k, _)| k.clone()).collect();
        for id in stale {
            self.links.shift_remove(&id);
            sink.remove_link(&id);
        }
        self.seen.retain(|_, at| *at >= cutoff);
        self.senders.retain(|_, s| s.at_ms >= cutoff);
    }

    pub fn drop_terminal(&mut self, sink: &mut dyn PeerSink, terminal_id: &str) {
        let gone: Vec<String> = self.links.iter().filter(|(_, l)| l.a == terminal_id || l.b == terminal_id).map(|(k, _)| k.clone()).collect();
        for id in gone {
            self.links.shift_remove(&id);
            sink.remove_link(&id);
        }
        self.pending.retain(|p| p.item.receiver != terminal_id);
        self.senders.retain(|_, s| s.sent.terminal_id != terminal_id);
    }

    pub fn stop(&mut self) {
        self.wake_at = None;
    }
}
