use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use core_remote_proto::frame::{Chunker, Kind};
use core_remote_proto::Transport;
use serde_json::Value;
use tokio::sync::mpsc;

pub const INBOX: usize = 64;
pub const OUTBOX: usize = 256;
pub const PRE_AUTH_FRAMES: usize = 4;
pub const PRE_AUTH_BYTES: usize = 64 * 1024;
pub const DRAIN_MAX: usize = 64;
pub const CLOSE_NORMAL: u16 = 1000;
pub const CLOSE_HANDSHAKE: u16 = 4400;
pub const CLOSE_DEVICE: u16 = 4401;
pub const CLOSE_PROTO: u16 = 4426;
pub const CLOSE_RATE: u16 = 4429;
pub const CLOSE_OFF: u16 = 4503;

#[derive(Debug)]
pub enum Out {
    Msg(Vec<u8>),
    Strike(&'static str),
    Good,
    Close(u16, &'static str),
}

#[derive(Clone)]
pub enum Outbox {
    Lan(mpsc::Sender<Out>),
    Relay(u16, u64, mpsc::Sender<(u16, u64, Out)>),
}

impl Outbox {
    pub async fn send(&self, out: Out) -> bool {
        match self {
            Outbox::Lan(tx) => tx.send(out).await.is_ok(),
            Outbox::Relay(conn, gen, tx) => tx.send((*conn, *gen, out)).await.is_ok(),
        }
    }

    pub async fn close(&self, code: u16, reason: &'static str) {
        let _ = self.send(Out::Close(code, reason)).await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Peer {
    Lan(IpAddr),
    Relay(u16),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origin {
    pub ip: Option<String>,
    pub country: Option<String>,
    pub pairing: bool,
}

pub struct Link {
    pub rx: mpsc::Receiver<Vec<u8>>,
    pub out: Outbox,
    pub peer: Peer,
    pub origin: Origin,
    pub budget: Arc<Budget>,
}

#[derive(Default)]
pub struct Budget {
    authed: AtomicBool,
    frames: AtomicUsize,
    bytes: AtomicUsize,
}

impl Budget {
    pub fn take(&self, len: usize) -> bool {
        if self.authed.load(Ordering::SeqCst) {
            return true;
        }
        let frames = self.frames.fetch_add(1, Ordering::SeqCst) + 1;
        let bytes = self.bytes.fetch_add(len, Ordering::SeqCst) + len;
        frames <= PRE_AUTH_FRAMES && bytes <= PRE_AUTH_BYTES
    }

    pub fn open(&self) {
        self.authed.store(true, Ordering::SeqCst);
    }
}

pub struct Secure {
    pub transport: Transport,
    pub out: Outbox,
}

impl Secure {
    pub async fn frame(&mut self, kind: Kind, stream: u32, payload: &[u8]) -> bool {
        let Ok(chunks) = Chunker::new(kind, stream, payload) else { return false };
        for frame in chunks {
            let Ok(bytes) = frame.encode() else { return false };
            let Ok(sealed) = self.transport.seal(&bytes) else { return false };
            if !self.out.send(Out::Msg(sealed)).await {
                return false;
            }
        }
        true
    }

    pub async fn json(&mut self, value: &Value) -> bool {
        match serde_json::to_vec(value) {
            Ok(bytes) => self.frame(Kind::Json, 0, &bytes).await,
            Err(_) => false,
        }
    }

    pub async fn close(&self, code: u16, reason: &'static str) {
        self.out.close(code, reason).await;
    }
}
