use std::collections::HashMap;

use core_monitor::peers::{peer_link_id, PeerInbound, PeerLinkStore, PeerMessage, PeerSent, PeerSink};
use core_store::model::{PeerLink, PeerLinkDir};
use core_store::time::iso_of_ms;

const A: &str = "100:1";
const B: &str = "200:2";
const C: &str = "300:3";
const SUB: &str = "a0123456789abcdef";
const T0: i64 = 1_791_892_800_000;

fn iso(s: i64) -> String {
    iso_of_ms(T0 + s * 1000)
}

#[derive(Debug, Clone, PartialEq)]
enum Ev {
    Upsert(PeerLink),
    Remove(String),
}

struct Sink {
    events: Vec<Ev>,
    messages: Vec<PeerMessage>,
    keys: HashMap<String, String>,
}

impl Sink {
    fn new() -> Self {
        let keys = [("ka", A), ("kb", B), ("kc", C)].iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        Sink { events: Vec::new(), messages: Vec::new(), keys }
    }

    fn last(&self) -> Option<&PeerLink> {
        self.events.iter().rev().find_map(|e| match e {
            Ev::Upsert(l) => Some(l),
            Ev::Remove(_) => None,
        })
    }
}

impl PeerSink for Sink {
    fn upsert_link(&mut self, link: PeerLink) {
        self.events.push(Ev::Upsert(link));
    }
    fn remove_link(&mut self, id: &str) {
        self.events.push(Ev::Remove(id.to_owned()));
    }
    fn sender_of(&self, from_key: &str) -> Option<String> {
        self.keys.get(from_key).cloned()
    }
    fn on_message(&mut self, message: PeerMessage) {
        self.messages.push(message);
    }
}

fn received(receiver: &str, from_key: &str, msg_id: Option<&str>, s: i64, live: bool, dedupe: Option<&str>) -> PeerInbound {
    PeerInbound {
        receiver: receiver.into(),
        from_key: from_key.into(),
        msg_id: msg_id.map(Into::into),
        dedupe_key: dedupe.map_or_else(|| format!("u-{}", msg_id.map_or_else(|| s.to_string(), str::to_owned)), str::to_owned),
        at: Some(iso(s)),
        live,
    }
}

fn sent(terminal: &str, agent: &str, msg: &str, s: i64) -> PeerSent {
    PeerSent { terminal_id: terminal.into(), agent_id: agent.into(), msg_id: msg.into(), at: Some(iso(s)) }
}

const TTL: i64 = 60 * 60_000;

#[test]
fn uses_the_same_link_id_as_the_web_store() {
    assert_eq!(peer_link_id(B, A), format!("{A}>{B}"));
    assert_eq!(peer_link_id(A, B), format!("{A}>{B}"));
}

#[test]
fn counts_each_message_once_by_msg_id_in_both_directions() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.sent(vec![sent(B, &format!("{B}:main"), "msg-0001", 1)], now);
    store.inbound(vec![received(A, "kb", Some("msg-0001"), 1, true, None), received(A, "kb", Some("msg-0001"), 1, true, Some("src-copy"))], now);
    store.flush(&mut sink, now, now, false);
    let l = sink.last().unwrap();
    assert_eq!((l.id.as_str(), l.a.as_str(), l.b.as_str(), l.ab, l.ba, l.last_dir), (format!("{A}>{B}").as_str(), A, B, 0, 1, PeerLinkDir::Ba));
    assert_eq!((l.first_at.as_str(), l.last_at.as_str(), l.last_agent_id.as_deref()), (iso(1).as_str(), iso(1).as_str(), Some(format!("{B}:main").as_str())));
    store.sent(vec![sent(A, &format!("{A}:main"), "msg-0002", 5)], now);
    store.inbound(vec![received(B, "ka", Some("msg-0002"), 5, true, None)], now);
    store.flush(&mut sink, now, now, false);
    store.inbound(vec![received(A, "kb", Some("msg-0001"), 1, true, None)], now);
    store.flush(&mut sink, now, now, true);
    let l = sink.last().unwrap();
    assert_eq!((l.ab, l.ba, l.first_at.clone(), l.last_at.clone(), l.last_dir, l.last_agent_id.clone()), (1, 1, iso(1), iso(5), PeerLinkDir::Ab, Some(format!("{A}:main"))));
    assert_eq!(sink.events.len(), 2);
    assert_eq!(sink.messages, [PeerMessage { from: B.into(), to: A.into(), agent_id: format!("{B}:main"), at: iso(1) }, PeerMessage { from: A.into(), to: B.into(), agent_id: format!("{A}:main"), at: iso(5) }]);
    assert_eq!(store.list().len(), 1);
}

#[test]
fn credits_the_subagent_that_sent_it_waiting_up_to_the_hold() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.inbound(vec![received(A, "kb", Some("msg-0003"), 2, true, None)], now);
    store.flush(&mut sink, now, now, false);
    assert!(sink.events.is_empty());
    store.sent(vec![sent(B, SUB, "msg-0003", 2)], now);
    store.flush(&mut sink, now, now, false);
    assert_eq!((sink.last().unwrap().ba, sink.last().unwrap().last_agent_id.as_deref()), (1, Some(SUB)));
    assert_eq!(sink.messages.last().map(|m| (m.from.as_str(), m.agent_id.as_str())), Some((B, SUB)));
    store.inbound(vec![received(A, "kb", Some("msg-0004"), 3, true, None)], now);
    store.flush(&mut sink, now, now, false);
    assert_eq!(sink.events.len(), 1);
    assert_eq!(store.wake_at(), Some(now + 2_000));
    store.flush(&mut sink, now, now + 1_999, false);
    assert_eq!(sink.events.len(), 1);
    store.flush(&mut sink, now, now + 2_000, false);
    assert_eq!((sink.last().unwrap().ba, sink.last().unwrap().last_agent_id.clone()), (2, Some(format!("{B}:main"))));
}

#[test]
fn dedupes_a_message_without_msg_id_by_receiver_and_record_key() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.inbound(vec![received(A, "kb", None, 1, true, Some("u-1")), received(A, "kb", None, 1, true, Some("u-1")), received(A, "kb", None, 2, true, Some("u-2"))], now);
    store.flush(&mut sink, now, now, false);
    assert_eq!(sink.last().unwrap().ba, 2);
}

#[test]
fn builds_catch_up_links_without_messages_and_skips_ones_past_the_ttl() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 30 * 60_000);
    store.inbound(vec![received(A, "kb", Some("msg-0005"), 0, false, None), received(A, "kc", Some("msg-0006"), -3 * 60 * 60, false, None)], now);
    store.flush(&mut sink, now, now, true);
    assert_eq!(store.list().iter().map(|l| l.id.clone()).collect::<Vec<_>>(), [format!("{A}>{B}")]);
    assert_eq!((sink.last().unwrap().ba, sink.last().unwrap().last_agent_id.clone()), (1, Some(format!("{B}:main"))));
    assert!(sink.messages.is_empty());
}

#[test]
fn drops_a_link_once_quiet_past_the_ttl_and_when_one_of_its_terminals_goes() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.inbound(vec![received(A, "kb", Some("msg-0007"), 1, true, None), received(C, "ka", Some("msg-0008"), 1, true, None)], now);
    store.flush(&mut sink, now, now, false);
    store.flush(&mut sink, now + 2_000, now + 2_000, false);
    assert_eq!(store.list().len(), 2);
    store.drop_terminal(&mut sink, C);
    assert_eq!(sink.events.last(), Some(&Ev::Remove(format!("{A}>{C}"))));
    let later = T0 + 61 * 60_000;
    store.prune(&mut sink, later);
    assert_eq!(sink.events.last(), Some(&Ev::Remove(format!("{A}>{B}"))));
    assert!(store.list().is_empty());
    store.inbound(vec![received(A, "kb", Some("msg-0007"), 1, true, None)], later);
    store.flush(&mut sink, later, later, true);
    assert!(store.list().is_empty());
}

#[test]
fn holds_an_unknown_sender_until_the_final_flush_and_ignores_its_own_messages() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.inbound(vec![received(A, "kx", None, 1, false, None)], now);
    store.flush(&mut sink, now, now, false);
    store.flush(&mut sink, now, now, true);
    assert!(sink.events.is_empty());
    store.sent(vec![sent(C, &format!("{C}:main"), "msg-0009", 2)], now);
    store.inbound(vec![received(A, "kx", Some("msg-0009"), 2, true, None)], now);
    store.flush(&mut sink, now, now, false);
    let l = sink.last().unwrap();
    assert_eq!((l.id.clone(), l.ab, l.ba), (format!("{A}>{C}"), 0, 1));
    store.inbound(vec![received(A, "ka", Some("msg-0010"), 3, false, None)], now);
    store.flush(&mut sink, now, now, true);
    assert_eq!(sink.events.len(), 1);
}

#[test]
fn stops_its_hold_timer() {
    let (mut store, mut sink, now) = (PeerLinkStore::new(TTL, None), Sink::new(), T0 + 60_000);
    store.inbound(vec![received(A, "kb", Some("msg-0011"), 1, true, None)], now);
    store.flush(&mut sink, now, now, false);
    assert!(store.wake_at().is_some());
    store.stop();
    assert!(store.wake_at().is_none());
    assert!(sink.events.is_empty());
}
