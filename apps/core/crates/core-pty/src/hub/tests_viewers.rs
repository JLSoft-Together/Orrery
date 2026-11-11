use super::tests::{drain, spawn_long, Log};
use super::*;
use std::sync::Mutex;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

struct Peer {
    conn: u64,
    rx: UnboundedReceiver<Out>,
    meter: Arc<Meter>,
}

fn open(hub: &mut Hub<Log>, conn: u64) -> Peer {
    let (tx, rx) = unbounded_channel();
    let meter = Meter::new();
    hub.handle(Event::Open { conn, tx, meter: meter.clone() });
    Peer { conn, rx, meter }
}

fn say(hub: &mut Hub<Log>, conn: u64, v: Value) {
    hub.handle(Event::Frame { conn, frame: Frame::Text(v.to_string()), permit: None });
}

fn typed(hub: &mut Hub<Log>, conn: u64, b: &[u8]) {
    hub.handle(Event::Frame { conn, frame: Frame::Binary(b.to_vec()), permit: None });
}

fn data(hub: &mut Hub<Log>, key: u64, b: &[u8]) {
    hub.handle(Event::Pty { key, ev: PtyEvent::Data(b.to_vec()), credit: None });
}

fn texts(outs: &[Out]) -> Vec<Value> {
    outs.iter().filter_map(|o| if let Out::Text(t) = o { serde_json::from_str(t).ok() } else { None }).collect()
}

fn bytes(outs: &[Out]) -> Vec<u8> {
    outs.iter().filter_map(|o| if let Out::Binary(b) = o { Some(b.clone()) } else { None }).flatten().collect()
}

fn closes(outs: &[Out]) -> Vec<u16> {
    outs.iter().filter_map(|o| if let Out::Close(c, _) = o { Some(*c) } else { None }).collect()
}

fn resized(outs: &[Out]) -> Vec<(u64, u64)> {
    texts(outs).iter().filter(|m| m["t"] == "resized").map(|m| (m["cols"].as_u64().unwrap(), m["rows"].as_u64().unwrap())).collect()
}

fn setup(id: &str) -> (Hub<Log>, Log, Peer, u64) {
    let (tx, rx) = unbounded_channel::<Event>();
    std::mem::forget(rx);
    let log = Log::default();
    let mut hub = Hub::new(log.clone(), tx);
    let desk = open(&mut hub, 1);
    let key = spawn_long(&mut hub, &log, 1, id);
    (hub, log, desk, key)
}

fn view(hub: &mut Hub<Log>, conn: u64, id: &str, size: Option<(u16, u16)>) -> Peer {
    let p = open(hub, conn);
    let mut m = json!({ "t": "attach", "id": id, "mode": "view" });
    if let Some((c, r)) = size {
        m["cols"] = json!(c);
        m["rows"] = json!(r);
    }
    say(hub, conn, m);
    p
}

#[tokio::test]
async fn own_and_view_get_the_same_bytes_and_nobody_is_replaced() {
    let (mut hub, _log, mut desk, key) = setup("V1");
    data(&mut hub, key, b"before ");
    let mut phone = view(&mut hub, 2, "V1", Some((50, 20)));
    let first = drain(&mut phone.rx);
    let kinds: Vec<String> = texts(&first).iter().map(|m| m["t"].as_str().unwrap().to_owned()).collect();
    assert_eq!(kinds, ["ready", "resized"]);
    assert_eq!(texts(&first)[0]["replay"], true);
    assert_eq!(resized(&first), [(80, 24)]);
    assert_eq!(bytes(&first), b"before ");
    for chunk in [&b"one "[..], b"two ", b"three"] {
        data(&mut hub, key, chunk);
    }
    let d = drain(&mut desk.rx);
    let p = drain(&mut phone.rx);
    assert!(closes(&d).is_empty() && closes(&p).is_empty());
    assert!(texts(&d).iter().all(|m| m["t"] != "resized"), "desktop alone in size gets no resized");
    assert_eq!(bytes(&d), b"before one two three");
    assert_eq!(bytes(&p), b"one two three");
    hub.handle(Event::Control(Some(MainToCore::Branch { id: "V1".into(), branch: Some("main".into()) })));
    assert_eq!(texts(&drain(&mut desk.rx))[0]["branch"], "main");
    assert_eq!(texts(&drain(&mut phone.rx))[0]["branch"], "main");
    assert_eq!(hub.sessions[&key].input_owner, Some(desk.conn));
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn whoever_claims_owns_the_size_and_the_other_side_hears_resized() {
    let (mut hub, _log, mut desk, key) = setup("V2");
    say(&mut hub, 1, json!({ "t": "resize", "cols": 120, "rows": 40 }));
    let mut phone = view(&mut hub, 2, "V2", Some((50, 20)));
    drain(&mut desk.rx);
    assert_eq!(resized(&drain(&mut phone.rx)), [(120, 40)]);
    say(&mut hub, 2, json!({ "t": "resize", "cols": 52, "rows": 21 }));
    assert_eq!(hub.sessions[&key].size, (120, 40), "a viewer that is not the owner never resizes");
    assert!(drain(&mut desk.rx).is_empty());

    typed(&mut hub, 2, b"x");
    assert_eq!(hub.sessions[&key].input_owner, Some(1), "input alone never moves the owner");
    assert_eq!(hub.sessions[&key].size, (120, 40));
    assert!(drain(&mut desk.rx).is_empty());
    say(&mut hub, 2, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(2));
    assert_eq!(hub.sessions[&key].size, (52, 21));
    assert_eq!(resized(&drain(&mut desk.rx)), [(52, 21)]);
    assert!(resized(&drain(&mut phone.rx)).is_empty());
    assert!(phone.meter.bound_to(&hub.sessions[&key].gate));
    assert!(desk.meter.bound_to(&hub.sessions[&key].gate), "an own viewer keeps pacing the pane");
    say(&mut hub, 2, json!({ "t": "claim" }));
    assert!(drain(&mut desk.rx).is_empty(), "no resized when the owner did not change");

    say(&mut hub, 1, json!({ "t": "resize", "cols": 130, "rows": 41 }));
    assert_eq!(hub.sessions[&key].size, (52, 21));
    typed(&mut hub, 1, b"z");
    assert_eq!(hub.sessions[&key].input_owner, Some(2));
    say(&mut hub, 1, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(1));
    assert_eq!(hub.sessions[&key].size, (130, 41));
    assert_eq!(resized(&drain(&mut phone.rx)), [(130, 41)]);
    assert!(resized(&drain(&mut desk.rx)).is_empty());
    assert!(desk.meter.bound_to(&hub.sessions[&key].gate));
    assert!(!phone.meter.bound_to(&hub.sessions[&key].gate));

    say(&mut hub, 1, json!({ "t": "resize", "cols": 100, "rows": 30 }));
    assert_eq!(resized(&drain(&mut phone.rx)), [(100, 30)]);
    say(&mut hub, 2, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(2));
    assert_eq!(resized(&drain(&mut desk.rx)), [(52, 21)]);
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn owner_leaving_hands_input_and_size_to_the_next_viewer() {
    let (mut hub, _log, mut desk, key) = setup("V3");
    let mut phone = view(&mut hub, 2, "V3", Some((50, 20)));
    let mut tablet = view(&mut hub, 3, "V3", Some((70, 22)));
    say(&mut hub, 3, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].size, (70, 22));
    drain(&mut desk.rx);
    drain(&mut phone.rx);
    drain(&mut tablet.rx);

    hub.handle(Event::Closed { conn: 3 });
    assert_eq!(hub.sessions[&key].input_owner, Some(1), "an own viewer is preferred");
    assert_eq!(hub.sessions[&key].size, (80, 24));
    assert_eq!(resized(&drain(&mut phone.rx)), [(80, 24)]);
    assert!(resized(&drain(&mut desk.rx)).is_empty());
    assert!(desk.meter.bound_to(&hub.sessions[&key].gate));

    hub.handle(Event::Closed { conn: 1 });
    assert_eq!(hub.sessions[&key].input_owner, Some(2), "else the earliest viewer");
    assert_eq!(hub.sessions[&key].size, (50, 20));
    assert!(phone.meter.bound_to(&hub.sessions[&key].gate));
    hub.handle(Event::Closed { conn: 2 });
    assert_eq!(hub.sessions[&key].input_owner, None);
    assert!(hub.sessions[&key].viewers.is_empty());
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn own_attach_still_replaces_own_but_never_a_view() {
    let (mut hub, _log, mut desk, key) = setup("V4");
    let mut phone = view(&mut hub, 2, "V4", Some((50, 20)));
    say(&mut hub, 2, json!({ "t": "claim" }));
    let mut desk2 = open(&mut hub, 4);
    say(&mut hub, 4, json!({ "t": "attach", "id": "V4" }));
    assert_eq!(closes(&drain(&mut desk.rx)), [4000]);
    assert!(closes(&drain(&mut phone.rx)).is_empty());
    let d2 = drain(&mut desk2.rx);
    assert!(texts(&d2).iter().all(|m| m["t"] != "resized"), "own attach keeps the desktop byte stream");
    assert_eq!(hub.sessions[&key].input_owner, Some(4), "an own attach takes input back from a view owner");
    assert_eq!(hub.sessions[&key].viewers.iter().map(|v| v.conn).collect::<Vec<_>>(), [2, 4]);

    let mut desk3 = open(&mut hub, 5);
    say(&mut hub, 4, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(4));
    say(&mut hub, 5, json!({ "t": "attach", "id": "V4" }));
    assert_eq!(closes(&drain(&mut desk2.rx)), [4000]);
    assert_eq!(hub.sessions[&key].input_owner, Some(5), "a replaced owner hands input to its replacement");
    assert!(desk3.meter.bound_to(&hub.sessions[&key].gate));
    drain(&mut desk3.rx);
    say(&mut hub, 2, json!({ "t": "kill", "id": "V4" }));
    assert!(!hub.ids.contains_key("V4"));
    assert_eq!(hub.conns[&5].session, None, "every viewer of a killed pane is free to attach again");
    assert_eq!(hub.conns[&2].session, None);
    hub.handle(Event::Pty { key, ev: PtyEvent::Exit { code: 1, signal: None }, credit: None });
    assert_eq!(texts(&drain(&mut desk3.rx)).last().unwrap()["t"], "exit");
    assert_eq!(texts(&drain(&mut phone.rx)).last().unwrap()["t"], "exit");
}

#[tokio::test]
async fn slow_view_alone_is_closed_4001_and_never_pauses_the_pane() {
    let (mut hub, _log, mut desk, key) = setup("V5");
    let mut phone = view(&mut hub, 2, "V5", None);
    drain(&mut phone.rx);
    let gate = hub.sessions[&key].gate.clone();
    let chunk = vec![b'q'; 1024 * 1024];
    let mut got = 0;
    for _ in 0..10 {
        data(&mut hub, key, &chunk);
        assert!(!phone.meter.bound_to(&gate));
        let d = drain(&mut desk.rx);
        let n = bytes(&d).len();
        desk.meter.sent(n);
        assert!(!gate.paused(), "the phone never paces the pane");
        got += n;
        assert!(closes(&d).is_empty());
    }
    assert_eq!(got, 10 * chunk.len());
    let p = drain(&mut phone.rx);
    assert_eq!(closes(&p), [OVERFLOW_CLOSE]);
    assert!(matches!(p.last(), Some(Out::Close(OVERFLOW_CLOSE, "overflow"))));
    assert_eq!(bytes(&p).len(), 7 * chunk.len(), "the attach texts count in its backlog too");
    hub.handle(Event::Closed { conn: 2 });
    assert!(desk.meter.bound_to(&gate));
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn view_after_exit_gets_replay_and_exit_and_keeps_the_session_from_the_reaper() {
    let (tx, _rx) = unbounded_channel();
    let log = Log::default();
    let mut hub = Hub::new(log.clone(), tx);
    let t0 = Instant::now();
    let offset = Arc::new(Mutex::new(Duration::ZERO));
    let o = offset.clone();
    hub.set_clock(Arc::new(move || t0 + *o.lock().unwrap()));
    let _desk = open(&mut hub, 1);
    let key = spawn_long(&mut hub, &log, 1, "V6");
    data(&mut hub, key, b"tail");
    hub.handle(Event::Pty { key, ev: PtyEvent::Exit { code: 5, signal: None }, credit: None });
    hub.handle(Event::Closed { conn: 1 });
    let mut phone = view(&mut hub, 2, "V6", Some((40, 10)));
    let outs = drain(&mut phone.rx);
    let kinds: Vec<String> = outs
        .iter()
        .map(|o| match o {
            Out::Text(t) => serde_json::from_str::<Value>(t).unwrap()["t"].as_str().unwrap().to_owned(),
            Out::Binary(_) => "bin".into(),
            Out::Close(..) => "close".into(),
        })
        .collect();
    assert_eq!(kinds, ["ready", "resized", "bin", "exit"]);
    assert_eq!(bytes(&outs), b"tail");
    assert_eq!(texts(&outs)[2]["code"], 5);
    say(&mut hub, 2, json!({ "t": "resize", "cols": 30, "rows": 9 }));
    typed(&mut hub, 2, b"ignored");
    assert_eq!(hub.sessions[&key].size, (80, 24));
    *offset.lock().unwrap() = Duration::from_secs(31 * 60);
    hub.handle(Event::Reap);
    assert!(hub.sessions.contains_key(&key), "a viewer keeps the exited session");
    hub.handle(Event::Closed { conn: 2 });
    hub.handle(Event::Reap);
    assert!(!hub.sessions.contains_key(&key));
}

#[tokio::test]
async fn a_conn_moving_to_another_pane_leaves_the_first_one() {
    let (mut hub, log, _desk, a) = setup("V7");
    let mut phone = view(&mut hub, 2, "V7", Some((50, 20)));
    say(&mut hub, 2, json!({ "t": "claim" }));
    say(&mut hub, 2, json!({ "t": "kill" }));
    let other = open(&mut hub, 3);
    let b = spawn_long(&mut hub, &log, 3, "V8");
    drop(other);
    say(&mut hub, 2, json!({ "t": "attach", "id": "V8", "mode": "view" }));
    assert!(hub.sessions.get(&a).is_none_or(|s| s.viewers.iter().all(|v| v.conn != 2)));
    drain(&mut phone.rx);
    data(&mut hub, a, b"old");
    assert!(bytes(&drain(&mut phone.rx)).is_empty());
    data(&mut hub, b, b"new");
    assert_eq!(bytes(&drain(&mut phone.rx)), b"new");
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn an_overflowed_owner_leaves_at_once_and_input_goes_back_to_the_desktop() {
    let (mut hub, _log, mut desk, key) = setup("V9");
    let mut phone = view(&mut hub, 2, "V9", Some((50, 20)));
    say(&mut hub, 2, json!({ "t": "claim" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(2));
    let gate = hub.sessions[&key].gate.clone();
    let chunk = vec![b'w'; 1024 * 1024];
    for _ in 0..9 {
        data(&mut hub, key, &chunk);
        let n = bytes(&drain(&mut desk.rx)).len();
        desk.meter.sent(n);
    }
    assert_eq!(closes(&drain(&mut phone.rx)), [OVERFLOW_CLOSE]);
    assert!(hub.sessions[&key].viewers.iter().all(|v| v.conn != 2), "an overflowed conn is no viewer any more");
    assert_eq!(hub.sessions[&key].input_owner, Some(1));
    assert_eq!(hub.sessions[&key].size, (80, 24));
    assert!(desk.meter.bound_to(&gate));
    assert!(!phone.meter.bound_to(&gate));
    assert!(!gate.paused());
    say(&mut hub, 2, json!({ "t": "claim" }));
    say(&mut hub, 2, json!({ "t": "resize", "cols": 10, "rows": 10 }));
    assert_eq!(hub.sessions[&key].input_owner, Some(1));
    assert_eq!(hub.sessions[&key].size, (80, 24));
    hub.handle(Event::Pty { key, ev: PtyEvent::Exit { code: 0, signal: None }, credit: None });
    hub.handle(Event::Closed { conn: 1 });
    hub.set_clock(Arc::new(|| Instant::now() + EXITED_TTL + Duration::from_secs(5)));
    hub.handle(Event::Reap);
    assert!(!hub.sessions.contains_key(&key), "the reaper does not wait for the overflowed socket");
}

#[tokio::test]
async fn a_slow_desktop_pauses_the_pane_while_a_claimed_phone_reads_fast() {
    let (mut hub, _log, mut desk, key) = setup("V10");
    let mut phone = view(&mut hub, 2, "V10", None);
    say(&mut hub, 2, json!({ "t": "claim" }));
    let gate = hub.sessions[&key].gate.clone();
    assert!(desk.meter.bound_to(&gate) && phone.meter.bound_to(&gate));
    data(&mut hub, key, &vec![b'k'; 600 * 1024]);
    let n = bytes(&drain(&mut phone.rx)).len();
    phone.meter.sent(n);
    assert!(gate.paused(), "the desktop backlog still holds the pane");
    let n = bytes(&drain(&mut desk.rx)).len();
    desk.meter.sent(n);
    assert!(!gate.paused());
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}

#[tokio::test]
async fn desktop_remount_after_its_old_socket_closed_takes_input_and_size_back_from_the_phone() {
    let (mut hub, _log, _desk, key) = setup("V11");
    let mut phone = view(&mut hub, 2, "V11", Some((50, 20)));
    hub.handle(Event::Closed { conn: 1 });
    assert_eq!(hub.sessions[&key].input_owner, Some(2));
    assert_eq!(hub.sessions[&key].size, (50, 20));
    drain(&mut phone.rx);
    let mut desk2 = open(&mut hub, 4);
    say(&mut hub, 4, json!({ "t": "attach", "id": "V11" }));
    assert_eq!(hub.sessions[&key].input_owner, Some(4));
    assert!(desk2.meter.bound_to(&hub.sessions[&key].gate));
    assert!(!phone.meter.bound_to(&hub.sessions[&key].gate));
    assert!(texts(&drain(&mut desk2.rx)).iter().all(|m| m["t"] != "resized"));
    say(&mut hub, 4, json!({ "t": "resize", "cols": 80, "rows": 24 }));
    assert_eq!(hub.sessions[&key].size, (80, 24));
    assert_eq!(resized(&drain(&mut phone.rx)), [(80, 24)]);
    let mut desk3 = open(&mut hub, 5);
    say(&mut hub, 5, json!({ "t": "attach", "id": "V11", "cols": 90, "rows": 30 }));
    assert_eq!(hub.sessions[&key].input_owner, Some(5));
    assert_eq!(hub.sessions[&key].size, (90, 30));
    assert_eq!(resized(&drain(&mut phone.rx)), [(90, 30)]);
    drain(&mut desk3.rx);
    hub.handle(Event::Control(Some(MainToCore::KillAll)));
}
