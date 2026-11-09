use crate::control::{encode, CoreToMain, LineSplitter, MainToCore};
use crate::flow::{Gate, CONTROL_INFLIGHT_MAX};
use crate::hub::{Control, Event, Hub, REAP_EVERY};
use crate::ws::{self, Auth};
use std::io::{Read, Write};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::Instant;

pub const SHUTDOWN_LINGER: Duration = Duration::from_millis(300);
pub const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

pub struct StdoutControl {
    tx: mpsc::Sender<String>,
}

impl Control for StdoutControl {
    fn send(&self, msg: CoreToMain) {
        let _ = self.tx.send(encode(&msg));
    }
}

fn start_stdout() -> (mpsc::Sender<String>, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel::<String>();
    let (done_tx, done) = mpsc::channel::<()>();
    let _ = std::thread::Builder::new().name("control-out".into()).spawn(move || {
        let mut out = std::io::stdout();
        for line in rx {
            if out.write_all(line.as_bytes()).and_then(|_| out.flush()).is_err() {
                break;
            }
        }
        drop(done_tx);
    });
    (tx, done)
}

fn start_stdin(events: UnboundedSender<Event>) {
    let _ = std::thread::Builder::new().name("control-in".into()).spawn(move || {
        let mut input = std::io::stdin();
        let mut splitter = LineSplitter::default();
        let mut buf = vec![0u8; 64 * 1024];
        let gate = Gate::with_max(CONTROL_INFLIGHT_MAX);
        loop {
            match input.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let credit = gate.acquire(n);
                    for msg in splitter.push(&buf[..n]) {
                        if events.send(Event::Control(msg)).is_err() {
                            return;
                        }
                    }
                    if events.send(Event::Release(credit)).is_err() {
                        return;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = events.send(Event::StdinEof);
    });
}

pub fn run() -> i32 {
    #[cfg(windows)]
    if let Err(e) = crate::jobs::windows::adopt_self() {
        eprintln!("orrery-core: own job not set, error={}", e.raw_os_error().unwrap_or(0));
    }
    let (out, out_done) = start_stdout();
    let (ev_tx, ev_rx) = unbounded_channel();
    start_stdin(ev_tx.clone());
    let code = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt.block_on(serve(ev_tx, ev_rx, out)),
        Err(e) => {
            eprintln!("orrery-core: runtime failed: {e}");
            1
        }
    };
    let _ = out_done.recv_timeout(Duration::from_secs(2));
    code
}

async fn serve(ev_tx: UnboundedSender<Event>, mut ev_rx: UnboundedReceiver<Event>, out: mpsc::Sender<String>) -> i32 {
    let mut warned = false;
    let (token, origins, app_version) = loop {
        match ev_rx.recv().await {
            Some(Event::Control(Some(MainToCore::Init { token, origins, app_version }))) => break (token, origins, app_version),
            Some(Event::Control(_)) => {
                if !warned {
                    warned = true;
                    eprintln!("orrery-core: control line before init ignored");
                }
            }
            Some(Event::StdinEof) | None => return 0,
            Some(_) => {}
        }
    };
    let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
        Ok(l) => l,
        Err(err) => {
            eprintln!("orrery-core: bind failed: {err}");
            return 1;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => {
            eprintln!("orrery-core: local_addr failed: {err}");
            return 1;
        }
    };
    let control = StdoutControl { tx: out };
    control.send(CoreToMain::Listening { port });
    eprintln!("orrery-core: listening on 127.0.0.1:{port} (app {app_version})");
    let auth = Arc::new(Auth::new(token, origins));
    let mut hub = Hub::new(control, ev_tx.clone());
    let reap_tx = ev_tx.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval_at(Instant::now() + REAP_EVERY, REAP_EVERY);
        loop {
            tick.tick().await;
            if reap_tx.send(Event::Reap).is_err() {
                return;
            }
        }
    });
    let mut next_conn = 1u64;
    let mut deadline: Option<Instant> = None;
    loop {
        let linger = async {
            match deadline {
                Some(d) => tokio::time::sleep_until(d).await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    let conn = next_conn;
                    next_conn += 1;
                    tokio::spawn(ws::serve(stream, conn, auth.clone(), ev_tx.clone()));
                }
                Err(_) => tokio::time::sleep(ACCEPT_BACKOFF).await,
            },
            ev = ev_rx.recv() => match ev {
                Some(ev) => hub.handle(ev),
                None => return 0,
            },
            _ = linger => {
                hub.force_kill();
                return 0;
            }
        }
        if hub.closing() && deadline.is_none() {
            deadline = Some(Instant::now() + SHUTDOWN_LINGER);
        }
    }
}
