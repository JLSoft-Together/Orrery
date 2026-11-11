use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use core_remote_proto::relay::{decode, encode_data, Control as RelayControl, ControlKind, Envelope};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::time::{interval, timeout, Instant};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use ureq::tls::{RootCerts, TlsConfig};

use super::config::now_secs;
use super::ctx::Ctx;
use super::limits::HANDSHAKE;
use super::link::{Budget, Link, Out, Outbox, Peer, DRAIN_MAX, INBOX, OUTBOX};
use super::session;
use crate::state::lock;

pub const REGISTER_DOMAIN: &str = "orrery-register/1";
pub const MACHINE_DOMAIN: &str = "orrery-machine/1";
pub const PING_EVERY: Duration = Duration::from_secs(30);
pub const MAX_BACKOFF: Duration = Duration::from_secs(60);
pub const MAX_CONNS: usize = 64;
pub const REVOKE_BATCH: usize = 64;
pub const CLOSE_REPLACED: u16 = 4409;
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
pub const MAX_RELAY_MESSAGE: usize = 1 << 20;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub fn ws_url(base: &str, machine: &str, ts: u64, sig: &str) -> Option<String> {
    let base = base.trim_end_matches('/');
    let (scheme, rest) = if let Some(r) = base.strip_prefix("https://") {
        ("wss://", r)
    } else {
        ("ws://", base.strip_prefix("http://")?)
    };
    Some(format!("{scheme}{rest}/m/{machine}?ts={ts}&sig={sig}"))
}

fn tls() -> Option<Arc<rustls::ClientConfig>> {
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .ok()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Some(Arc::new(cfg))
}

fn post(url: &str, body: &Value) -> Option<u16> {
    let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(HTTP_TIMEOUT)).http_status_as_error(false).tls_config(tls).build().into();
    let res = agent.post(url).header("content-type", "application/json").send(body.to_string()).ok()?;
    Some(res.status().as_u16())
}

fn admin_body(ctx: &Ctx) -> (String, String, u64) {
    let mut cfg = lock(&ctx.config);
    let ts = cfg.next_admin_ts(now_secs());
    cfg.save(&ctx.dir);
    (cfg.relay.url.trim_end_matches('/').to_owned(), cfg.machine_id.clone(), ts)
}

pub fn register(ctx: &Ctx) -> Result<(), &'static str> {
    let (base, machine, ts) = admin_body(ctx);
    let body = json!({
        "machineId": machine,
        "pubkey": ctx.keys.sign_public(),
        "relayKey": ctx.keys.relay_key(),
        "ts": ts,
        "sig": ctx.keys.sign(REGISTER_DOMAIN, &machine, ts),
    });
    match post(&format!("{base}/register"), &body) {
        Some(200) => {
            let mut cfg = lock(&ctx.config);
            cfg.relay.registered = true;
            cfg.save(&ctx.dir);
            Ok(())
        }
        Some(403) => Err("taken"),
        Some(_) => Err("rejected"),
        None => Err("offline"),
    }
}

pub fn flush_revokes(ctx: &Ctx) {
    let batch: Vec<String> = lock(&ctx.config).relay.pending_revoke.iter().take(REVOKE_BATCH).cloned().collect();
    if batch.is_empty() {
        return;
    }
    let (base, machine, ts) = admin_body(ctx);
    let body = json!({ "machineId": machine, "deviceIds": batch, "ts": ts, "sig": ctx.keys.sign(REGISTER_DOMAIN, &machine, ts) });
    if matches!(post(&format!("{base}/revoke"), &body), Some(200 | 404)) {
        let mut cfg = lock(&ctx.config);
        cfg.relay.pending_revoke.retain(|id| !batch.contains(id));
        cfg.save(&ctx.dir);
    }
}

async fn wait(stop: &mut watch::Receiver<bool>, d: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(d) => !*stop.borrow(),
        _ = stop.changed() => false,
    }
}

pub async fn run(ctx: Arc<Ctx>, mut stop: watch::Receiver<bool>) {
    let mut backoff = Duration::from_secs(1);
    while !*stop.borrow() {
        let (url, registered) = {
            let cfg = lock(&ctx.config);
            (cfg.relay.url.clone(), cfg.relay.registered)
        };
        if url.is_empty() {
            break;
        }
        if !registered {
            ctx.set_relay_state("registering");
            let c = ctx.clone();
            let result = tokio::task::spawn_blocking(move || register(&c)).await.unwrap_or(Err("offline"));
            if let Err(state) = result {
                ctx.set_relay_state(state);
                if !wait(&mut stop, backoff).await {
                    break;
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        }
        let c = ctx.clone();
        let _ = tokio::task::spawn_blocking(move || flush_revokes(&c)).await;
        ctx.set_relay_state("connecting");
        let code = match connect(&ctx, &url).await {
            Some(ws) => {
                backoff = Duration::from_secs(1);
                ctx.set_relay_state("online");
                connection(&ctx, ws, &mut stop).await
            }
            None => None,
        };
        ctx.set_relay_state(if code == Some(CLOSE_REPLACED) { "replaced" } else { "offline" });
        if code == Some(CLOSE_REPLACED) {
            backoff = MAX_BACKOFF;
        }
        if !wait(&mut stop, backoff).await {
            break;
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    ctx.set_relay_state("off");
}

fn ws_config() -> WebSocketConfig {
    WebSocketConfig::default().max_message_size(Some(MAX_RELAY_MESSAGE)).max_frame_size(Some(MAX_RELAY_MESSAGE))
}

async fn connect(ctx: &Ctx, base: &str) -> Option<Ws> {
    let (machine, ts) = {
        let mut cfg = lock(&ctx.config);
        let ts = cfg.next_machine_ts(now_secs());
        cfg.save(&ctx.dir);
        (cfg.machine_id.clone(), ts)
    };
    let url = ws_url(base, &machine, ts, &ctx.keys.sign(MACHINE_DOMAIN, &machine, ts))?;
    let connector = if url.starts_with("wss://") { Connector::Rustls(tls()?) } else { Connector::Plain };
    let fut = tokio_tungstenite::connect_async_tls_with_config(url, Some(ws_config()), true, Some(connector));
    match timeout(HTTP_TIMEOUT, fut).await {
        Ok(Ok((ws, _))) => Some(ws),
        _ => None,
    }
}

pub struct Conn {
    pub gen: u64,
    pub tx: mpsc::Sender<Vec<u8>>,
    pub budget: Arc<Budget>,
}

#[derive(Default)]
pub struct Conns {
    map: HashMap<u16, Conn>,
    next_gen: u64,
}

impl Conns {
    pub fn open(&mut self, conn: u16) -> Option<(u64, mpsc::Receiver<Vec<u8>>, Arc<Budget>)> {
        if self.map.len() >= MAX_CONNS && !self.map.contains_key(&conn) {
            return None;
        }
        self.map.remove(&conn);
        self.next_gen += 1;
        let (tx, rx) = mpsc::channel(INBOX);
        let budget = Arc::new(Budget::default());
        self.map.insert(conn, Conn { gen: self.next_gen, tx, budget: budget.clone() });
        Some((self.next_gen, rx, budget))
    }

    pub fn close(&mut self, conn: u16, gen: Option<u64>) {
        if self.map.get(&conn).is_some_and(|c| gen.is_none_or(|g| g == c.gen)) {
            self.map.remove(&conn);
        }
    }

    pub fn current(&self, conn: u16, gen: u64) -> bool {
        self.map.get(&conn).is_some_and(|c| c.gen == gen)
    }

    pub fn data(&mut self, conn: u16, bytes: &[u8]) {
        let ok = match self.map.get(&conn) {
            Some(c) => c.budget.take(bytes.len()) && c.tx.try_send(bytes.to_vec()).is_ok(),
            None => return,
        };
        if !ok {
            self.map.remove(&conn);
        }
    }
}

fn route(ctx: &Arc<Ctx>, conns: &mut Conns, out: &mpsc::Sender<(u16, u64, Out)>, bytes: &[u8]) {
    match decode(bytes) {
        Ok(Envelope::Control(RelayControl { t: ControlKind::Open, conn })) => {
            let Some((gen, rx, budget)) = conns.open(conn) else { return };
            let link = Link { rx, out: Outbox::Relay(conn, gen, out.clone()), peer: Peer::Relay(conn), budget };
            tokio::spawn(session::serve(ctx.clone(), link, None, Instant::now() + HANDSHAKE));
        }
        Ok(Envelope::Control(RelayControl { t: ControlKind::Close, conn })) => conns.close(conn, None),
        Ok(Envelope::Data { conn, bytes }) => conns.data(conn, bytes),
        Err(_) => {}
    }
}

async fn connection(ctx: &Arc<Ctx>, ws: Ws, stop: &mut watch::Receiver<bool>) -> Option<u16> {
    let (mut sink, mut stream) = ws.split();
    let (out_tx, mut out_rx) = mpsc::channel::<(u16, u64, Out)>(OUTBOX);
    let mut conns = Conns::default();
    let mut ping = interval(PING_EVERY);
    ping.tick().await;
    loop {
        tokio::select! {
            _ = stop.changed() => {
                let _ = sink.close().await;
                return None;
            }
            _ = ctx.relay_wake.notified() => {
                let c = ctx.clone();
                tokio::task::spawn_blocking(move || flush_revokes(&c));
            }
            _ = ping.tick() => {
                if sink.send(Message::Text("ping".into())).await.is_err() {
                    return None;
                }
            }
            Some((conn, gen, out)) = out_rx.recv() => {
                let mut next = Some((conn, gen, out));
                let mut drained = 0;
                while let Some((conn, gen, item)) = next.take() {
                    drained += 1;
                    if !conns.current(conn, gen) {
                        if drained < DRAIN_MAX {
                            next = out_rx.try_recv().ok();
                        }
                        continue;
                    }
                    match item {
                        Out::Msg(bytes) => {
                            if let Ok(env) = encode_data(conn, &bytes) {
                                if sink.feed(Message::Binary(env.into())).await.is_err() {
                                    return None;
                                }
                            }
                        }
                        Out::Close(..) => conns.close(conn, Some(gen)),
                    }
                    if drained < DRAIN_MAX {
                        next = out_rx.try_recv().ok();
                    }
                }
                if sink.flush().await.is_err() {
                    return None;
                }
            }
            msg = stream.next() => match msg {
                Some(Ok(Message::Binary(b))) => route(ctx, &mut conns, &out_tx, &b),
                Some(Ok(Message::Close(frame))) => return frame.map(|f| u16::from(f.code)),
                Some(Ok(_)) => {}
                _ => return None,
            },
        }
    }
}
