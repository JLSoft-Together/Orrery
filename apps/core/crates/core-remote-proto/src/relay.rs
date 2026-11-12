use std::fmt;

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::wire::{b64_decode, b64_encode, valid_id, NOISE_MAX};
use crate::{Error, Result};

pub const CONTROL_CONN: u16 = 0;
pub const ENVELOPE_HEADER: usize = 2;
pub const MAX_ENVELOPE: usize = ENVELOPE_HEADER + NOISE_MAX;
pub const MAX_CONTROL: usize = 256;
pub const RELAY_KEY_LEN: usize = 32;
pub const TOKEN_DOMAIN: &[u8] = b"orrery-relay/1";
pub const MAX_TOKEN_LEN: usize = 512;
const EXP_LEN: usize = 8;
const MAC_LEN: usize = 32;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlKind {
    Open,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    pub t: ControlKind,
    pub conn: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pairing: bool,
}

impl Control {
    pub fn new(t: ControlKind, conn: u16) -> Self {
        Self { t, conn, ip: None, country: None, pairing: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Envelope<'a> {
    Control(Control),
    Data { conn: u16, bytes: &'a [u8] },
}

pub fn encode_data(conn: u16, bytes: &[u8]) -> Result<Vec<u8>> {
    if conn == CONTROL_CONN {
        return Err(Error::BadMessage);
    }
    if bytes.len() > NOISE_MAX {
        return Err(Error::TooLarge);
    }
    let mut out = Vec::with_capacity(ENVELOPE_HEADER + bytes.len());
    out.extend_from_slice(&conn.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(out)
}

pub fn encode_control(control: &Control) -> Result<Vec<u8>> {
    if control.conn == CONTROL_CONN {
        return Err(Error::BadMessage);
    }
    let json = serde_json::to_vec(control).map_err(|_| Error::BadMessage)?;
    let mut out = Vec::with_capacity(ENVELOPE_HEADER + json.len());
    out.extend_from_slice(&CONTROL_CONN.to_be_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

pub fn encode_close(conn: u16, code: u16) -> Result<Vec<u8>> {
    if conn == CONTROL_CONN {
        return Err(Error::BadMessage);
    }
    let json = serde_json::to_vec(&serde_json::json!({ "t": "close", "conn": conn, "code": code })).map_err(|_| Error::BadMessage)?;
    let mut out = Vec::with_capacity(ENVELOPE_HEADER + json.len());
    out.extend_from_slice(&CONTROL_CONN.to_be_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

fn control_bytes(value: &serde_json::Value) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(value).map_err(|_| Error::BadMessage)?;
    if json.len() > MAX_CONTROL {
        return Err(Error::TooLarge);
    }
    let mut out = Vec::with_capacity(ENVELOPE_HEADER + json.len());
    out.extend_from_slice(&CONTROL_CONN.to_be_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

pub const MAX_PAIR_TTL: u64 = 300;

pub fn encode_pair_open(ticket_hash: &[u8; 32], ttl: u64) -> Result<Vec<u8>> {
    if ttl == 0 || ttl > MAX_PAIR_TTL {
        return Err(Error::BadMessage);
    }
    control_bytes(&serde_json::json!({ "t": "pair.open", "h": b64_encode(ticket_hash), "ttl": ttl }))
}

pub fn encode_pair_close() -> Result<Vec<u8>> {
    control_bytes(&serde_json::json!({ "t": "pair.close" }))
}

pub fn decode(bytes: &[u8]) -> Result<Envelope<'_>> {
    if bytes.len() < ENVELOPE_HEADER {
        return Err(Error::BadMessage);
    }
    if bytes.len() > MAX_ENVELOPE {
        return Err(Error::TooLarge);
    }
    let conn = u16::from_be_bytes([bytes[0], bytes[1]]);
    let body = &bytes[ENVELOPE_HEADER..];
    if conn != CONTROL_CONN {
        return Ok(Envelope::Data { conn, bytes: body });
    }
    if body.len() > MAX_CONTROL {
        return Err(Error::TooLarge);
    }
    let control: Control = serde_json::from_slice(body).map_err(|_| Error::BadMessage)?;
    if control.conn == CONTROL_CONN {
        return Err(Error::BadMessage);
    }
    Ok(Envelope::Control(control))
}

#[derive(Clone)]
pub struct RelayKey(Zeroizing<[u8; RELAY_KEY_LEN]>);

impl RelayKey {
    pub fn generate() -> Result<Self> {
        let mut key = Zeroizing::new([0u8; RELAY_KEY_LEN]);
        getrandom::fill(key.as_mut_slice()).map_err(|_| Error::Rng)?;
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: [u8; RELAY_KEY_LEN] = bytes.try_into().map_err(|_| Error::BadKey)?;
        Ok(Self(Zeroizing::new(raw)))
    }

    pub fn as_bytes(&self) -> &[u8; RELAY_KEY_LEN] {
        &self.0
    }

    fn mac(&self, machine_id: &str, device_id: &str, exp: u64) -> Result<HmacSha256> {
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(self.0.as_slice()).map_err(|_| Error::BadKey)?;
        mac.update(TOKEN_DOMAIN);
        mac.update(machine_id.as_bytes());
        mac.update(device_id.as_bytes());
        mac.update(&exp.to_be_bytes());
        Ok(mac)
    }
}

impl fmt::Debug for RelayKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RelayKey(..)")
    }
}

pub fn issue_token(key: &RelayKey, machine_id: &str, device_id: &str, exp: u64) -> Result<String> {
    if !valid_id(machine_id) || !valid_id(device_id) {
        return Err(Error::BadToken);
    }
    let tag = key.mac(machine_id, device_id, exp)?.finalize().into_bytes();
    let mut raw = Vec::with_capacity(device_id.len() + EXP_LEN + MAC_LEN);
    raw.extend_from_slice(device_id.as_bytes());
    raw.extend_from_slice(&exp.to_be_bytes());
    raw.extend_from_slice(&tag);
    Ok(b64_encode(&raw))
}

pub fn verify_token(key: &RelayKey, machine_id: &str, token: &str, now: u64) -> Result<String> {
    if token.is_empty() || token.len() > MAX_TOKEN_LEN {
        return Err(Error::BadToken);
    }
    let raw = b64_decode(token).ok_or(Error::BadToken)?;
    if raw.len() <= EXP_LEN + MAC_LEN {
        return Err(Error::BadToken);
    }
    let (body, tag) = raw.split_at(raw.len() - MAC_LEN);
    let (device_raw, exp_raw) = body.split_at(body.len() - EXP_LEN);
    let device_id = std::str::from_utf8(device_raw).map_err(|_| Error::BadToken)?;
    let exp = u64::from_be_bytes(exp_raw.try_into().map_err(|_| Error::BadToken)?);
    if !valid_id(device_id) {
        return Err(Error::BadToken);
    }
    key.mac(machine_id, device_id, exp)?
        .verify_slice(tag)
        .map_err(|_| Error::BadToken)?;
    if now >= exp {
        return Err(Error::Expired);
    }
    Ok(device_id.to_owned())
}
