use std::sync::Mutex;

use core_remote_proto::frame::{Chunker, Frame, Kind, Reassembler};
use core_remote_proto::pair::{PairInitiator, QrPayload};
use core_remote_proto::session::SessionInitiator;
use core_remote_proto::{Error, StaticKeypair, Transport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteErrorKind {
    Crypto,
    KeyMismatch,
    Unauthorized,
    ProtoTooOld,
    ProtoTooNew,
    BadMessage,
    BadPayload,
    BadKey,
    BadQr,
    Expired,
    Frame,
    TooLarge,
    Closed,
    Rng,
    Busy,
}

#[derive(Debug)]
pub struct RemoteError {
    pub kind: RemoteErrorKind,
    pub value: u32,
}

impl RemoteError {
    fn of(kind: RemoteErrorKind) -> Self {
        Self { kind, value: 0 }
    }
}

impl From<Error> for RemoteError {
    fn from(error: Error) -> Self {
        use RemoteErrorKind as K;
        match error {
            Error::ProtoTooOld { min } => Self { kind: K::ProtoTooOld, value: u32::from(min) },
            Error::ProtoTooNew { max } => Self { kind: K::ProtoTooNew, value: u32::from(max) },
            Error::Noise => Self::of(K::Crypto),
            Error::KeyMismatch => Self::of(K::KeyMismatch),
            Error::Unauthorized => Self::of(K::Unauthorized),
            Error::BadMessage => Self::of(K::BadMessage),
            Error::BadPayload => Self::of(K::BadPayload),
            Error::BadKey => Self::of(K::BadKey),
            Error::BadQr | Error::BadToken => Self::of(K::BadQr),
            Error::Expired => Self::of(K::Expired),
            Error::UnknownKind(_) | Error::BadFrame => Self::of(K::Frame),
            Error::TooLarge => Self::of(K::TooLarge),
            Error::Closed => Self::of(K::Closed),
            Error::Rng => Self::of(K::Rng),
            _ => Self::of(K::Crypto),
        }
    }
}

pub struct QrInfo {
    pub machine: String,
    pub name: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub relay: Option<String>,
    pub mkey: Vec<u8>,
    pub exp: u64,
}

#[flutter_rust_bridge::frb(sync)]
pub fn decode_qr(text: String) -> Result<QrInfo, RemoteError> {
    let qr = QrPayload::decode(&text)?;
    Ok(QrInfo {
        machine: qr.machine.clone(),
        name: qr.name.clone(),
        host: qr.host.clone(),
        port: qr.port,
        relay: qr.relay.clone(),
        mkey: qr.mkey.to_vec(),
        exp: qr.exp,
    })
}

#[flutter_rust_bridge::frb(sync)]
pub fn generate_static_key() -> Result<Vec<u8>, RemoteError> {
    let key = StaticKeypair::generate()?;
    Ok(key.export_private().to_vec())
}

#[flutter_rust_bridge::frb(sync)]
pub fn static_public_key(private: Vec<u8>) -> Result<Vec<u8>, RemoteError> {
    let key = StaticKeypair::from_private(&private)?;
    Ok(key.public().to_vec())
}

#[flutter_rust_bridge::frb(opaque)]
pub struct SecureChannel {
    inner: Mutex<ChannelState>,
}

struct ChannelState {
    transport: Transport,
    reassembler: Reassembler,
}

pub struct InboundMessage {
    pub kind: u8,
    pub stream: u32,
    pub payload: Vec<u8>,
}

impl SecureChannel {
    fn new(transport: Transport) -> Self {
        Self {
            inner: Mutex::new(ChannelState {
                transport,
                reassembler: Reassembler::new(),
            }),
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn seal(&self, kind: u8, stream: u32, payload: Vec<u8>) -> Result<Vec<Vec<u8>>, RemoteError> {
        let kind = Kind::try_from(kind)?;
        let mut state = self.inner.lock().map_err(|_| RemoteError::of(RemoteErrorKind::Closed))?;
        let mut out = Vec::new();
        for frame in Chunker::new(kind, stream, &payload)? {
            let bytes = frame.encode()?;
            out.push(state.transport.seal(&bytes)?);
        }
        Ok(out)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn open(&self, ciphertext: Vec<u8>) -> Result<Option<InboundMessage>, RemoteError> {
        let mut state = self.inner.lock().map_err(|_| RemoteError::of(RemoteErrorKind::Closed))?;
        let plain = state.transport.open(&ciphertext)?;
        let frame = Frame::decode(&plain)?;
        let message = state.reassembler.push(frame)?;
        Ok(message.map(|m| InboundMessage {
            kind: m.kind as u8,
            stream: m.stream,
            payload: m.payload,
        }))
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn is_closed(&self) -> bool {
        self.inner
            .lock()
            .map(|s| s.transport.is_closed())
            .unwrap_or(true)
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct PairingHandshake {
    inner: Mutex<Option<PairInitiator>>,
}

#[flutter_rust_bridge::frb(non_opaque)]
pub struct PairingStart {
    pub handshake: PairingHandshake,
    pub msg1: Vec<u8>,
}

#[flutter_rust_bridge::frb(non_opaque)]
pub struct PairingDone {
    pub machine_id: String,
    pub desktop_key: Vec<u8>,
    pub msg3: Vec<u8>,
    pub channel: SecureChannel,
}

#[flutter_rust_bridge::frb(sync)]
pub fn pair_start(
    private: Vec<u8>,
    qr_text: String,
    device_id: String,
    device_name: String,
    now: u64,
) -> Result<PairingStart, RemoteError> {
    let key = StaticKeypair::from_private(&private)?;
    let qr = QrPayload::decode(&qr_text)?;
    let (initiator, msg1) = PairInitiator::start(&key, &qr, &device_id, &device_name, now)?;
    Ok(PairingStart {
        handshake: PairingHandshake {
            inner: Mutex::new(Some(initiator)),
        },
        msg1,
    })
}

impl PairingHandshake {
    #[flutter_rust_bridge::frb(sync)]
    pub fn finish(&self, msg2: Vec<u8>) -> Result<PairingDone, RemoteError> {
        let initiator = self
            .inner
            .lock()
            .map_err(|_| RemoteError::of(RemoteErrorKind::Closed))?
            .take()
            .ok_or(RemoteError::of(RemoteErrorKind::Busy))?;
        let (desktop, msg3) = initiator.finish(&msg2)?;
        Ok(PairingDone {
            machine_id: desktop.machine_id,
            desktop_key: desktop.desktop_static.to_vec(),
            msg3,
            channel: SecureChannel::new(desktop.transport),
        })
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct SessionHandshake {
    inner: Mutex<Option<SessionInitiator>>,
}

#[flutter_rust_bridge::frb(non_opaque)]
pub struct SessionStart {
    pub handshake: SessionHandshake,
    pub msg1: Vec<u8>,
}

#[flutter_rust_bridge::frb(sync)]
pub fn session_start(
    private: Vec<u8>,
    desktop_key: Vec<u8>,
    device_id: String,
) -> Result<SessionStart, RemoteError> {
    let key = StaticKeypair::from_private(&private)?;
    let desktop: [u8; 32] = desktop_key.as_slice().try_into().map_err(|_| RemoteError::of(RemoteErrorKind::BadKey))?;
    let (initiator, msg1) = SessionInitiator::start(&key, &desktop, &device_id)?;
    Ok(SessionStart {
        handshake: SessionHandshake {
            inner: Mutex::new(Some(initiator)),
        },
        msg1,
    })
}

impl SessionHandshake {
    #[flutter_rust_bridge::frb(sync)]
    pub fn finish(&self, msg2: Vec<u8>) -> Result<SecureChannel, RemoteError> {
        let initiator = self
            .inner
            .lock()
            .map_err(|_| RemoteError::of(RemoteErrorKind::Closed))?
            .take()
            .ok_or(RemoteError::of(RemoteErrorKind::Busy))?;
        let transport = initiator.finish(&msg2)?;
        Ok(SecureChannel::new(transport))
    }
}
