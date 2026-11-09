use crate::flow::{Meter, INBOUND_MAX};
use crate::hub::{Event, Frame, Out};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Semaphore;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;

pub const PROTOCOL_PREFIX: &str = "ccm.pty.";
pub const MAX_PAYLOAD: usize = 1024 * 1024;
pub const COALESCE_MAX: usize = 1024 * 1024;
pub const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub struct Auth {
    token: String,
    origins: HashSet<String>,
}

impl Auth {
    pub fn new(token: String, origins: Vec<String>) -> Auth {
        Auth { token, origins: origins.into_iter().filter(|o| !o.is_empty()).collect() }
    }

    pub fn token_ok(&self, candidate: &str) -> bool {
        let Some(rest) = candidate.strip_prefix(PROTOCOL_PREFIX) else { return false };
        !self.token.is_empty() && rest.len() == self.token.len() && bool::from(rest.as_bytes().ct_eq(self.token.as_bytes()))
    }

    pub fn check(&self, host: Option<&str>, origin: Option<&str>, protocols: &[&str]) -> Result<String, StatusCode> {
        if !host.is_some_and(host_ok) {
            return Err(StatusCode::FORBIDDEN);
        }
        if !origin.is_some_and(|o| self.origins.contains(o)) {
            return Err(StatusCode::FORBIDDEN);
        }
        protocols
            .iter()
            .flat_map(|h| h.split(','))
            .map(str::trim)
            .find(|p| self.token_ok(p))
            .map(str::to_owned)
            .ok_or(StatusCode::FORBIDDEN)
    }

    #[allow(clippy::result_large_err)]
    fn handshake(&self, req: &Request, mut resp: Response) -> Result<Response, ErrorResponse> {
        if req.uri().path() != "/pty" {
            return Err(refuse(StatusCode::BAD_REQUEST));
        }
        let header = |name: &str| req.headers().get(name).and_then(|v| v.to_str().ok());
        let protocols: Vec<&str> = req.headers().get_all("sec-websocket-protocol").iter().filter_map(|v| v.to_str().ok()).collect();
        let chosen = self.check(header("host"), header("origin"), &protocols).map_err(refuse)?;
        let value = HeaderValue::from_str(&chosen).map_err(|_| refuse(StatusCode::FORBIDDEN))?;
        resp.headers_mut().insert("sec-websocket-protocol", value);
        Ok(resp)
    }
}

fn host_ok(host: &str) -> bool {
    host.strip_prefix("127.0.0.1:").is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

fn refuse(status: StatusCode) -> ErrorResponse {
    let mut r = ErrorResponse::new(None);
    *r.status_mut() = status;
    r
}

pub fn config() -> WebSocketConfig {
    WebSocketConfig::default().max_message_size(Some(MAX_PAYLOAD)).max_frame_size(Some(MAX_PAYLOAD))
}

pub async fn serve(stream: TcpStream, conn: u64, auth: Arc<Auth>, hub: UnboundedSender<Event>) {
    let _ = stream.set_nodelay(true);
    #[allow(clippy::result_large_err)]
    let cb = move |req: &Request, resp: Response| auth.handshake(req, resp);
    let accepting = tokio_tungstenite::accept_hdr_async_with_config(stream, cb, Some(config()));
    let Ok(Ok(ws)) = tokio::time::timeout(HANDSHAKE_TIMEOUT, accepting).await else { return };
    let (mut sink, mut stream) = ws.split();
    let (tx, rx) = unbounded_channel::<Out>();
    let meter = Meter::new();
    if hub.send(Event::Open { conn, tx, meter: meter.clone() }).is_err() {
        return;
    }
    let inbound = Arc::new(Semaphore::new(INBOUND_MAX));
    let reading = async {
        while let Some(msg) = stream.next().await {
            let frame = match msg {
                Ok(Message::Text(t)) => Frame::Text(t.to_string()),
                Ok(Message::Binary(b)) => Frame::Binary(b.to_vec()),
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => continue,
            };
            let size = match &frame {
                Frame::Text(t) => t.len(),
                Frame::Binary(b) => b.len(),
            };
            let Ok(permit) = inbound.clone().acquire_many_owned(size.clamp(1, INBOUND_MAX) as u32).await else { break };
            if hub.send(Event::Frame { conn, frame, permit: Some(permit) }).is_err() {
                break;
            }
        }
    };
    let writing = write_loop(&mut sink, rx, &meter);
    tokio::select! {
        _ = reading => {}
        _ = writing => {}
    }
    let _ = hub.send(Event::Closed { conn });
}

async fn write_loop<S>(sink: &mut S, mut rx: UnboundedReceiver<Out>, meter: &Meter)
where
    S: SinkExt<Message> + Unpin,
{
    let mut pending: Option<Out> = None;
    loop {
        let item = match pending.take() {
            Some(i) => i,
            None => match rx.recv().await {
                Some(i) => i,
                None => return,
            },
        };
        let (msg, size) = match item {
            Out::Binary(mut buf) => {
                while buf.len() < COALESCE_MAX {
                    match rx.try_recv() {
                        Ok(Out::Binary(more)) => buf.extend_from_slice(&more),
                        Ok(other) => {
                            pending = Some(other);
                            break;
                        }
                        Err(_) => break,
                    }
                }
                let size = buf.len();
                (Message::Binary(buf.into()), size)
            }
            Out::Text(s) => {
                let size = s.len();
                (Message::Text(s.into()), size)
            }
            Out::Close(code, reason) => {
                let _ = sink.send(Message::Close(Some(CloseFrame { code: CloseCode::from(code), reason: reason.into() }))).await;
                return;
            }
        };
        let ok = sink.send(msg).await.is_ok();
        meter.sent(size);
        if !ok {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> Auth {
        Auth::new("s3cret".into(), vec!["http://127.0.0.1:4317".into(), "".into()])
    }

    #[test]
    fn token_compare() {
        let a = auth();
        assert!(a.token_ok("ccm.pty.s3cret"));
        assert!(!a.token_ok("ccm.pty.s3cre"));
        assert!(!a.token_ok("ccm.pty.s3cretX"));
        assert!(!a.token_ok("s3cret"));
        assert!(!Auth::new(String::new(), vec![]).token_ok("ccm.pty."));
    }

    #[test]
    fn handshake_rules() {
        let a = auth();
        let ok = a.check(Some("127.0.0.1:5000"), Some("http://127.0.0.1:4317"), &["chat, ccm.pty.s3cret"]);
        assert_eq!(ok, Ok("ccm.pty.s3cret".to_owned()));
        assert_eq!(a.check(Some("localhost:5000"), Some("http://127.0.0.1:4317"), &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:"), Some("http://127.0.0.1:4317"), &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(None, Some("http://127.0.0.1:4317"), &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:5000"), Some("http://evil.test"), &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:5000"), Some(""), &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:5000"), None, &["ccm.pty.s3cret"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:5000"), Some("http://127.0.0.1:4317"), &["ccm.pty.nope"]), Err(StatusCode::FORBIDDEN));
        assert_eq!(a.check(Some("127.0.0.1:5000"), Some("http://127.0.0.1:4317"), &[]), Err(StatusCode::FORBIDDEN));
    }
}
