use super::*;
use crate::control::Resolved;
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
