use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

use super::phone_pty::wants_snapshot;
use super::pty_bridge::{spawn, FromHub, Slot, Spec, ToHub};
use super::pty_vt::{Vt, SCROLLBACK, SNAP_MAX};
use super::test_util::*;
use crate::state::lock;

fn lines(range: std::ops::RangeInclusive<usize>) -> Vec<u8> {
    range.map(|n| format!("line-{n}\r\n")).collect::<String>().into_bytes()
}

fn repaint(first: usize, rows: usize, filled: usize) -> Vec<u8> {
    let mut out = b"\x1b[?25l\x1b[H".to_vec();
    for r in 0..filled {
        out.extend(format!("line-{}\x1b[K\r\n", first + r).into_bytes());
    }
    for _ in filled..rows {
        out.extend_from_slice(b"\x1b[K");
    }
    out.extend(format!("\x1b[{};1H\x1b[?25h", filled + 1).into_bytes());
    out
}

struct Client {
    parser: vt100::Parser,
}

impl Client {
    fn new() -> Client {
        Client { parser: vt100::Parser::new(24, 80, 20_000) }
    }

    fn snapshot(&mut self, cols: u16, rows: u16, data: &str) {
        self.parser = vt100::Parser::new(rows, cols, 20_000);
        self.parser.process(data.as_bytes());
    }

    fn text(&mut self) -> Vec<String> {
        let (rows, cols) = self.parser.screen().size();
        self.parser.screen_mut().set_scrollback(usize::MAX);
        let mut offset = self.parser.screen().scrollback();
        let mut out = Vec::new();
        while offset > 0 {
            self.parser.screen_mut().set_scrollback(offset);
            let take = offset.min(usize::from(rows));
            out.extend(self.parser.screen().rows(0, cols).take(take));
            offset -= take;
        }
        self.parser.screen_mut().set_scrollback(0);
        out.extend(self.parser.screen().rows(0, cols));
        out.into_iter().map(|l| l.trim_end().to_owned()).filter(|l| !l.is_empty()).collect()
    }
}

fn assert_run(text: &[String], first: usize, last: usize) {
    let want: Vec<String> = (first..=last).map(|n| format!("line-{n}")).collect();
    assert_eq!(text, want.as_slice());
}

#[test]
fn a_snapshot_after_a_shrink_mid_burst_keeps_every_line() {
    let mut vt = Vt::new(80, 41);
    vt.feed(&lines(1..=1963));
    vt.resize(80, 19);
    assert_eq!(vt.size(), (80, 19));
    let snap = vt.snapshot();
    let mut phone = Client::new();
    phone.snapshot(80, 19, &snap);
    assert_run(&phone.text(), 1, 1963);
    let rest = [repaint(1946, 19, 18), lines(1964..=3000)].concat();
    vt.feed(&rest);
    phone.parser.process(&rest);
    assert_run(&phone.text(), 1, 3000);
    let mut again = Client::new();
    again.snapshot(80, 19, &vt.snapshot());
    assert_run(&again.text(), 3000 - SCROLLBACK - 17, 3000);
}

#[test]
fn a_snapshot_after_a_grow_keeps_the_cursor_row_like_conpty() {
    let mut vt = Vt::new(80, 19);
    vt.feed(&lines(1..=500));
    vt.resize(100, 41);
    let snap = vt.snapshot();
    let mut phone = Client::new();
    phone.snapshot(100, 41, &snap);
    let rest = [repaint(483, 41, 18), lines(501..=600)].concat();
    phone.parser.process(&rest);
    assert_run(&phone.text(), 1, 600);
}

#[test]
fn a_snapshot_restores_cursor_modes_and_the_alternate_screen() {
    let mut vt = Vt::new(40, 10);
    vt.feed(b"top\r\n\x1b[?1h\x1b[?2004h\x1b[?25l\x1b[3;5Hmid");
    let mut phone = Client::new();
    phone.snapshot(40, 10, &vt.snapshot());
    let s = phone.parser.screen();
    assert_eq!(s.cursor_position(), (2, 7));
    assert!(s.application_cursor() && s.bracketed_paste() && s.hide_cursor());
    vt.feed(b"\x1b[?1049h\x1b[2J\x1b[Hfull screen app");
    phone.snapshot(40, 10, &vt.snapshot());
    assert!(phone.parser.screen().alternate_screen());
    assert_eq!(phone.parser.screen().contents().trim_end(), "full screen app");
}

#[test]
fn a_huge_history_is_cut_from_the_oldest_lines() {
    let mut vt = Vt::new(200, 20);
    let wide = "x".repeat(190);
    let mut burst = Vec::new();
    for n in 1..=3000 {
        burst.extend(format!("\x1b[3{}m{n:05}{wide}\x1b[m\r\n", n % 8).into_bytes());
    }
    vt.feed(&burst);
    let snap = vt.snapshot();
    let json = serde_json::to_vec(&json!({ "data": snap })).unwrap();
    assert!(json.len() <= SNAP_MAX + 1024, "{}", json.len());
    assert!(json.len() <= core_remote_proto::frame::MAX_MESSAGE);
    let mut phone = Client::new();
    phone.snapshot(200, 20, &snap);
    let text = phone.text();
    assert!(text.last().unwrap().starts_with("03000"));
    let nums: Vec<usize> = text.iter().map(|l| l[..5].parse().unwrap()).collect();
    assert!(nums.windows(2).all(|w| w[1] == w[0] + 1));
}

fn wraps(parser: &mut vt100::Parser) -> Vec<(String, bool)> {
    let (rows, cols) = parser.screen().size();
    parser.screen_mut().set_scrollback(usize::MAX);
    let mut offset = parser.screen().scrollback();
    let mut out = Vec::new();
    while offset > 0 {
        parser.screen_mut().set_scrollback(offset);
        let take = offset.min(usize::from(rows));
        let s = parser.screen();
        out.extend(s.rows(0, cols).take(take).enumerate().map(|(r, t)| (t, s.row_wrapped(r as u16))));
        offset -= take;
    }
    parser.screen_mut().set_scrollback(0);
    let s = parser.screen();
    out.extend(s.rows(0, cols).enumerate().map(|(r, t)| (t, s.row_wrapped(r as u16))));
    out
}

#[test]
fn a_snapshot_keeps_soft_wrapped_rows_soft_so_a_reflow_can_join_them() {
    let mut vt = Vt::new(20, 6);
    let mut feed = Vec::new();
    for n in 0..8 {
        feed.extend(format!("\x1b[3{}m{}\x1b[m\r\n", n % 8, format!("{n}").repeat(45)).into_bytes());
    }
    feed.extend_from_slice(b"\x1b[44m");
    feed.extend("x".repeat(20).into_bytes());
    feed.extend_from_slice(b"\x1b[m   indented\r\n");
    feed.extend("y".repeat(18).into_bytes());
    feed.extend("\u{4e2d}".as_bytes());
    feed.extend_from_slice(b"tail\r\n$ ");
    vt.feed(&feed);
    let mut want = vt100::Parser::new(6, 20, 20_000);
    want.process(&feed);
    let mut phone = Client::new();
    phone.snapshot(20, 6, &vt.snapshot());
    let got = wraps(&mut phone.parser);
    let expect = wraps(&mut want);
    assert_eq!(got.len(), expect.len());
    assert_eq!(got, expect);
    assert!(got.iter().filter(|(_, w)| *w).count() >= 16, "{got:?}");
    assert_eq!(phone.parser.screen().cursor_position(), want.screen().cursor_position());
    assert_eq!(phone.parser.screen().cell(5, 0).map(|c| c.contents().to_owned()), want.screen().cell(5, 0).map(|c| c.contents().to_owned()));
}

#[test]
fn only_clients_that_ask_get_snapshots() {
    assert!(wants_snapshot(&json!({ "caps": ["x", "pty.snapshot"] })));
    assert!(!wants_snapshot(&json!({ "caps": [] })));
    assert!(!wants_snapshot(&json!({ "stream": 1 })));
    assert!(!wants_snapshot(&json!({ "caps": "pty.snapshot" })));
}

async fn fake_hub(listener: TcpListener, script: Vec<Message>) {
    let (tcp, _) = listener.accept().await.unwrap();
    let echo = |req: &Request, mut res: Response| {
        if let Some(p) = req.headers().get("sec-websocket-protocol") {
            res.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_str(p.to_str().unwrap()).unwrap());
        }
        Ok(res)
    };
    let mut ws = tokio_tungstenite::accept_hdr_async(tcp, echo).await.unwrap();
    let first = ws.next().await.unwrap().unwrap();
    let attach: Value = serde_json::from_str(first.to_text().unwrap()).unwrap();
    assert_eq!(attach["t"], "attach");
    for msg in script {
        ws.send(msg).await.unwrap();
    }
    let _ = tokio::time::timeout(Duration::from_secs(10), ws.next()).await;
}

fn text_msg(v: Value) -> Message {
    Message::Text(v.to_string().into())
}

fn chunks(bytes: Vec<u8>, size: usize) -> Vec<Message> {
    bytes.chunks(size).map(|c| Message::Binary(c.to_vec().into())).collect()
}

fn run_bridge(tag: &str, snapshot: bool) -> Vec<FromHub> {
    let rig = rig(tag, "", false);
    let ctx = rig.remote.ctx().unwrap();
    runtime().block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        *lock(&rig.host.pty) = Some((format!("ws://{}", listener.local_addr().unwrap()), "ccm-pty".into()));
        let mut script = vec![text_msg(json!({ "t": "ready", "id": "s" })), text_msg(json!({ "t": "resized", "cols": 80, "rows": 41 })), Message::Binary(lines(1..=500).into())];
        script.extend(chunks(lines(501..=1963), 64 * 1024));
        script.push(text_msg(json!({ "t": "resized", "cols": 80, "rows": 19 })));
        script.extend(chunks([repaint(1946, 19, 18), lines(1964..=3000)].concat(), 4096));
        script.push(text_msg(json!({ "t": "exit", "code": 0 })));
        let hub = tokio::spawn(fake_hub(listener, script));
        let (back, mut rx) = tokio::sync::mpsc::channel(256);
        let spec = Spec { re: "a".into(), stream: 3, session: "s".into(), cols: 50, rows: 20, gen: 9, backlog: Arc::new(AtomicUsize::new(0)), snapshot };
        let _tx = spawn(ctx.host.clone(), spec, Slot::take(&ctx, "d").unwrap(), back);
        let mut got = Vec::new();
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.unwrap().unwrap();
            let end = matches!(msg, FromHub::Ended(..));
            got.push(msg);
            if end {
                break;
            }
        }
        hub.abort();
        got
    })
}

fn kinds(got: &[FromHub]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in got {
        let k = match m {
            FromHub::Json(_, _, v) => v["t"].as_str().unwrap_or("reply").to_owned(),
            FromHub::Out(..) => "out".into(),
            FromHub::Ended(..) => "ended".into(),
        };
        if k != "out" {
            out.push(k);
        }
    }
    out
}

#[test]
fn the_bridge_sends_a_snapshot_on_attach_and_after_a_resize_mid_burst() {
    let got = run_bridge("pty-snap", true);
    assert_eq!(kinds(&got), ["reply", "pty.ready", "pty.snapshot", "pty.resized", "pty.snapshot", "pty.exit", "ended"]);
    let first_snap = got.iter().position(|m| matches!(m, FromHub::Json(_, _, v) if v["t"] == "pty.snapshot")).unwrap();
    assert!(!got[..first_snap].iter().any(|m| matches!(m, FromHub::Out(..))));
    let mut phone = Client::new();
    let mut snaps = 0;
    for m in &got {
        match m {
            FromHub::Json(stream, gen, v) if v["t"] == "pty.snapshot" => {
                assert_eq!((*stream, *gen, v["stream"].as_u64(), v["gen"].as_u64()), (3, 9, Some(3), Some(9)));
                snaps += 1;
                let (cols, rows) = (v["cols"].as_u64().unwrap() as u16, v["rows"].as_u64().unwrap() as u16);
                assert_eq!((cols, rows), if snaps == 1 { (80, 41) } else { (80, 19) });
                phone.snapshot(cols, rows, v["data"].as_str().unwrap());
                if snaps == 2 {
                    assert_run(&phone.text(), 1, 1963);
                }
            }
            FromHub::Out(_, _, bytes) => phone.parser.process(bytes),
            _ => {}
        }
    }
    assert_run(&phone.text(), 1, 3000);
}

#[test]
fn an_old_client_still_gets_the_raw_tail_and_no_snapshot() {
    let got = run_bridge("pty-raw", false);
    assert_eq!(kinds(&got), ["reply", "pty.ready", "pty.resized", "pty.exit", "ended"]);
    let raw: Vec<u8> = got.iter().filter_map(|m| if let FromHub::Out(_, _, b) = m { Some(b.clone()) } else { None }).flatten().collect();
    assert!(raw.starts_with(b"line-1\r\nline-2\r\n"));
    assert!(String::from_utf8_lossy(&raw).ends_with("line-3000\r\n"));
}

#[test]
fn the_phone_never_resizes_the_pane_it_attaches_without_a_size_and_never_claims() {
    let rig = rig("pty-nosize", "", false);
    let ctx = rig.remote.ctx().unwrap();
    runtime().block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        *lock(&rig.host.pty) = Some((format!("ws://{}", listener.local_addr().unwrap()), "ccm-pty".into()));
        let (seen_tx, mut seen_rx) = tokio::sync::mpsc::channel::<Message>(16);
        let hub = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let echo = |req: &Request, mut res: Response| {
                if let Some(p) = req.headers().get("sec-websocket-protocol") {
                    res.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_str(p.to_str().unwrap()).unwrap());
                }
                Ok(res)
            };
            let mut ws = tokio_tungstenite::accept_hdr_async(tcp, echo).await.unwrap();
            let attach = ws.next().await.unwrap().unwrap();
            seen_tx.send(attach).await.unwrap();
            for msg in [text_msg(json!({ "t": "ready", "id": "s" })), text_msg(json!({ "t": "resized", "cols": 180, "rows": 40 })), Message::Binary(b"desk wide\r\n".to_vec().into())] {
                ws.send(msg).await.unwrap();
            }
            while let Some(Ok(m)) = ws.next().await {
                seen_tx.send(m).await.unwrap();
            }
        });
        let (back, mut rx) = tokio::sync::mpsc::channel(256);
        let spec = Spec { re: "a".into(), stream: 4, session: "s".into(), cols: 50, rows: 20, gen: 1, backlog: Arc::new(AtomicUsize::new(0)), snapshot: true };
        let tx = spawn(ctx.host.clone(), spec, Slot::take(&ctx, "d").unwrap(), back);
        let attach: Value = serde_json::from_str(tokio::time::timeout(Duration::from_secs(5), seen_rx.recv()).await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!((attach["t"].as_str(), attach["mode"].as_str()), (Some("attach"), Some("view")));
        assert!(attach.get("cols").is_none() && attach.get("rows").is_none(), "{attach}");
        let mut snap = None;
        while snap.is_none() {
            if let FromHub::Json(_, _, v) = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.unwrap().unwrap() {
                if v["t"] == "pty.snapshot" {
                    snap = Some(v);
                }
            }
        }
        assert_eq!(snap.unwrap()["cols"], 180);
        tx.send(ToHub::Input(b"ls\r".to_vec())).await.unwrap();
        let got = tokio::time::timeout(Duration::from_secs(5), seen_rx.recv()).await.unwrap().unwrap();
        assert!(matches!(&got, Message::Binary(b) if b.as_ref() == b"ls\r"), "{got:?}");
        assert!(tokio::time::timeout(Duration::from_millis(300), seen_rx.recv()).await.is_err());
        hub.abort();
    });
}
