use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::time::{timeout_at, Instant};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::ctx::Ctx;
use super::limits::{Pending, HANDSHAKE};
use super::link::{Budget, Link, Out, Outbox, Peer, DRAIN_MAX, INBOX, OUTBOX};
use super::session;

pub const PATH: &str = "/remote";
pub const MAX_WS_MESSAGE: usize = 65_535;

pub fn is_private_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

pub fn select(ips: impl IntoIterator<Item = IpAddr>) -> Vec<Ipv4Addr> {
    let mut out: Vec<Ipv4Addr> = ips
        .into_iter()
        .filter_map(|ip| match ip {
            IpAddr::V4(v4) if is_private_v4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

pub fn lan_addrs() -> Vec<Ipv4Addr> {
    let ifaces = if_addrs::get_if_addrs().unwrap_or_default();
    select(ifaces.into_iter().filter(|i| i.is_oper_up() && !i.is_loopback()).map(|i| i.ip()))
}

pub fn bind(addrs: &[Ipv4Addr], port: u16) -> Vec<std::net::TcpListener> {
    addrs
        .iter()
        .filter(|ip| !ip.is_unspecified())
        .filter_map(|ip| bind_one(SocketAddr::from((*ip, port))))
        .collect()
}

pub fn bind_one(addr: SocketAddr) -> Option<std::net::TcpListener> {
    let s = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).ok()?;
    exclusive(&s);
    s.bind(&addr.into()).ok()?;
    s.listen(128).ok()?;
    s.set_nonblocking(true).ok()?;
    Some(s.into())
}

#[cfg(windows)]
fn exclusive(s: &Socket) -> bool {
    use std::os::windows::io::AsRawSocket;
    use windows_sys::Win32::Networking::WinSock::{setsockopt, SOL_SOCKET, SO_EXCLUSIVEADDRUSE};
    let on: i32 = 1;
    unsafe { setsockopt(s.as_raw_socket() as usize, SOL_SOCKET, SO_EXCLUSIVEADDRUSE, (&on as *const i32).cast(), 4) == 0 }
}

#[cfg(not(windows))]
fn exclusive(_: &Socket) -> bool {
    true
}

fn ws_config() -> WebSocketConfig {
    WebSocketConfig::default().max_message_size(Some(MAX_WS_MESSAGE)).max_frame_size(Some(MAX_WS_MESSAGE))
}

fn reject(status: StatusCode) -> ErrorResponse {
    let mut res = ErrorResponse::new(None);
    *res.status_mut() = status;
    res
}

#[allow(clippy::result_large_err)]
fn check(req: &Request, res: Response) -> Result<Response, ErrorResponse> {
    if req.uri().path() != PATH {
        return Err(reject(StatusCode::NOT_FOUND));
    }
    if req.headers().contains_key("origin") {
        return Err(reject(StatusCode::FORBIDDEN));
    }
    Ok(res)
}

pub async fn accept_loop(ctx: Arc<Ctx>, listener: std::net::TcpListener, mut stop: watch::Receiver<bool>) {
    let Ok(listener) = TcpListener::from_std(listener) else { return };
    loop {
        tokio::select! {
            got = listener.accept() => {
                let Ok((stream, peer)) = got else { continue };
                let Some(pending) = ctx.gate.admit(peer.ip(), std::time::Instant::now()) else {
                    drop(stream);
                    continue;
                };
                let _ = stream.set_nodelay(true);
                tokio::spawn(handle(ctx.clone(), stream, peer.ip(), pending));
            }
            _ = stop.changed() => break,
        }
    }
}

async fn handle(ctx: Arc<Ctx>, stream: TcpStream, ip: IpAddr, pending: Pending) {
    let deadline = Instant::now() + HANDSHAKE;
    let accepted = timeout_at(deadline, tokio_tungstenite::accept_hdr_async_with_config(stream, check, Some(ws_config()))).await;
    let Ok(Ok(ws)) = accepted else { return };
    let (in_tx, in_rx) = mpsc::channel(INBOX);
    let (out_tx, out_rx) = mpsc::channel(OUTBOX);
    let budget = Arc::new(Budget::default());
    let link = Link { rx: in_rx, out: Outbox::Lan(out_tx), peer: Peer::Lan(ip), budget: budget.clone() };
    tokio::spawn(pump(ws, in_tx, out_rx, budget));
    session::serve(ctx, link, Some(pending), deadline).await;
}

fn close_message(code: u16, reason: &'static str) -> Message {
    Message::Close(Some(CloseFrame { code: CloseCode::from(code), reason: reason.into() }))
}

pub async fn pump(ws: WebSocketStream<TcpStream>, in_tx: mpsc::Sender<Vec<u8>>, mut out_rx: mpsc::Receiver<Out>, budget: Arc<Budget>) {
    let (mut sink, mut stream) = ws.split();
    let reader = async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Binary(b) => {
                    if !budget.take(b.len()) || in_tx.send(b.to_vec()).await.is_err() {
                        break;
                    }
                }
                Message::Ping(_) | Message::Pong(_) => {}
                _ => break,
            }
        }
    };
    let writer = async move {
        while let Some(out) = out_rx.recv().await {
            let mut next = Some(out);
            let mut drained = 0;
            while let Some(item) = next.take() {
                drained += 1;
                match item {
                    Out::Msg(bytes) => {
                        if sink.feed(Message::Binary(bytes.into())).await.is_err() {
                            return;
                        }
                    }
                    Out::Close(code, reason) => {
                        let _ = sink.send(close_message(code, reason)).await;
                        return;
                    }
                }
                if drained < DRAIN_MAX {
                    next = out_rx.try_recv().ok();
                }
            }
            if sink.flush().await.is_err() {
                return;
            }
        }
        let _ = sink.close().await;
    };
    tokio::select! {
        _ = reader => {}
        _ = writer => {}
    }
}
