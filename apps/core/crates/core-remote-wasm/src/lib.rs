#![forbid(unsafe_code)]

mod channel;
mod handshake;

use core_remote_proto::pair::QrPayload;
use core_remote_proto::{Error, StaticKeypair, KEY_LEN};
use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use channel::{encode_frames, Message, Reassembler, Transport};
pub use handshake::{Pairing, Session};

pub(crate) fn error_code(e: &Error) -> &'static str {
    match e {
        Error::Noise => "noise",
        Error::KeyMismatch => "keyMismatch",
        Error::Unauthorized => "unauthorized",
        Error::ProtoTooOld { .. } => "protoTooOld",
        Error::ProtoTooNew { .. } => "protoTooNew",
        Error::BadMessage => "badMessage",
        Error::BadPayload => "badPayload",
        Error::BadKey => "badKey",
        Error::BadQr => "badQr",
        Error::BadToken => "badToken",
        Error::Expired => "expired",
        Error::UnknownKind(_) => "unknownKind",
        Error::BadFrame => "badFrame",
        Error::TooLarge => "tooLarge",
        Error::Closed => "closed",
        Error::Rng => "rng",
        _ => "unknown",
    }
}

pub(crate) fn fail(e: Error) -> JsError {
    JsError::new(error_code(&e))
}

pub(crate) fn key(bytes: &[u8]) -> Result<[u8; KEY_LEN], JsError> {
    bytes.try_into().map_err(|_| fail(Error::BadKey))
}

pub(crate) fn secs(value: f64) -> Result<u64, JsError> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= 9_007_199_254_740_991.0 {
        Ok(value as u64)
    } else {
        Err(JsError::new("badTime"))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[wasm_bindgen]
pub struct Identity(StaticKeypair);

#[wasm_bindgen]
impl Identity {
    pub fn generate() -> Result<Identity, JsError> {
        StaticKeypair::generate().map(Identity).map_err(fail)
    }

    #[wasm_bindgen(js_name = fromPrivate)]
    pub fn from_private(bytes: &[u8]) -> Result<Identity, JsError> {
        StaticKeypair::from_private(bytes).map(Identity).map_err(fail)
    }

    #[wasm_bindgen(getter, js_name = publicKey)]
    pub fn public_key(&self) -> Vec<u8> {
        self.0.public().to_vec()
    }

    #[wasm_bindgen(js_name = exportPrivate)]
    pub fn export_private(&self) -> Vec<u8> {
        self.0.export_private().to_vec()
    }
}

impl Identity {
    pub(crate) fn keypair(&self) -> &StaticKeypair {
        &self.0
    }
}

#[derive(Serialize)]
struct LanView<'a> {
    host: &'a str,
    port: u16,
}

#[derive(Serialize)]
struct QrView<'a> {
    v: u16,
    machine: &'a str,
    name: &'a str,
    host: Option<&'a str>,
    port: Option<u16>,
    lan: Vec<LanView<'a>>,
    relay: Option<&'a str>,
    pt: Option<&'a str>,
    mkey: String,
    exp: u64,
}

#[wasm_bindgen(js_name = decodeQr)]
pub fn decode_qr(text: &str) -> Result<String, JsError> {
    let qr = QrPayload::decode(text).map_err(fail)?;
    let view = QrView {
        v: core_remote_proto::pair::QR_VERSION,
        machine: &qr.machine,
        name: &qr.name,
        host: qr.first_lan().map(|a| a.host.as_str()),
        port: qr.first_lan().map(|a| a.port),
        lan: qr.lan.iter().map(|a| LanView { host: &a.host, port: a.port }).collect(),
        relay: qr.relay.as_deref(),
        pt: qr.ticket.as_ref().map(|t| t.as_str()),
        mkey: hex(&qr.mkey),
        exp: qr.exp,
    };
    serde_json::to_string(&view).map_err(|_| fail(Error::BadQr))
}

#[wasm_bindgen(js_name = protoVersion)]
pub fn proto_version() -> u16 {
    core_remote_proto::PROTO
}
