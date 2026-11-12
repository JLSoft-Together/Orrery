use std::fmt;

use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::keys::KEY_LEN;
use crate::wire::{b64_decode, b64_encode, valid_id};
use crate::{Error, Result};

pub const CODE_LEN: usize = 16;
pub const PSK_LEN: usize = 32;
pub const QR_VERSION: u16 = 2;
pub const QR_V1: u16 = 1;
pub const MAX_QR_LEN: usize = 640;
pub const MAX_LAN: usize = 4;
pub const DEFAULT_LAN_PORT: u16 = 4320;
pub const TICKET_LEN: usize = 16;
pub const TICKET_TEXT_LEN: usize = 22;
pub const PSK_SALT: &[u8] = b"orrery-pair";
const MAX_TEXT_LEN: usize = 128;

#[derive(Clone)]
pub struct PairCode(Zeroizing<[u8; CODE_LEN]>);

impl PairCode {
    pub fn generate() -> Result<Self> {
        let mut code = Zeroizing::new([0u8; CODE_LEN]);
        getrandom::fill(code.as_mut_slice()).map_err(|_| Error::Rng)?;
        Ok(Self(code))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: [u8; CODE_LEN] = bytes.try_into().map_err(|_| Error::BadQr)?;
        Ok(Self(Zeroizing::new(raw)))
    }

    pub fn as_bytes(&self) -> &[u8; CODE_LEN] {
        &self.0
    }

    pub fn derive_psk(&self, machine_id: &str) -> Zeroizing<[u8; PSK_LEN]> {
        let mut out = Zeroizing::new([0u8; PSK_LEN]);
        let hk = Hkdf::<Sha256>::new(Some(PSK_SALT), self.0.as_slice());
        if hk.expand(machine_id.as_bytes(), out.as_mut_slice()).is_err() {
            out.zeroize();
        }
        out
    }
}

impl fmt::Debug for PairCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairCode(..)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PairTicket(Zeroizing<String>);

impl PairTicket {
    pub fn generate() -> Result<Self> {
        let mut raw = Zeroizing::new([0u8; TICKET_LEN]);
        getrandom::fill(raw.as_mut_slice()).map_err(|_| Error::Rng)?;
        Ok(Self(Zeroizing::new(b64_encode(raw.as_slice()))))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let ok = text.len() == TICKET_TEXT_LEN && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !ok {
            return Err(Error::BadQr);
        }
        Ok(Self(Zeroizing::new(text.to_owned())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.0.as_bytes()).into()
    }

    pub fn hash_text(&self) -> String {
        b64_encode(&self.hash())
    }
}

impl fmt::Debug for PairTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairTicket(..)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanAddr {
    pub host: String,
    pub port: u16,
}

impl LanAddr {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self { host: host.into(), port }
    }

    fn valid(&self) -> bool {
        !self.host.is_empty() && valid_text(&self.host) && !self.host.contains(':') && self.port != 0
    }

    fn wire(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn parse(text: &str) -> Option<Self> {
        let (host, port) = text.rsplit_once(':')?;
        if port.is_empty() || port.len() > 5 || !port.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let addr = Self::new(host, port.parse().ok()?);
        addr.valid().then_some(addr)
    }
}

#[derive(Clone)]
pub struct QrPayload {
    pub machine: String,
    pub name: String,
    pub lan: Vec<LanAddr>,
    pub relay: Option<String>,
    pub ticket: Option<PairTicket>,
    pub mkey: [u8; KEY_LEN],
    pub code: PairCode,
    pub exp: u64,
}

impl fmt::Debug for QrPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrPayload")
            .field("machine", &self.machine)
            .field("name", &self.name)
            .field("lan", &self.lan)
            .field("relay", &self.relay)
            .field("ticket", &self.ticket.is_some())
            .field("exp", &self.exp)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize)]
struct QrWire {
    v: u16,
    machine: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lan: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pt: Option<String>,
    mkey: String,
    code: String,
    exp: u64,
}

impl Drop for QrWire {
    fn drop(&mut self) {
        self.code.zeroize();
        if let Some(pt) = self.pt.as_mut() {
            pt.zeroize();
        }
    }
}

fn valid_text(value: &str) -> bool {
    value.len() <= MAX_TEXT_LEN && !value.chars().any(char::is_control)
}

fn v1_lan(host: Option<&String>, port: Option<u16>) -> Result<Vec<LanAddr>> {
    match host {
        None if port.is_none() => Ok(Vec::new()),
        None => Err(Error::BadQr),
        Some(h) => Ok(vec![LanAddr::new(h.clone(), port.unwrap_or(DEFAULT_LAN_PORT))]),
    }
}

fn v2_lan(list: Option<&Vec<String>>) -> Result<Vec<LanAddr>> {
    let list = list.map_or(&[][..], Vec::as_slice);
    if list.len() > MAX_LAN {
        return Err(Error::BadQr);
    }
    list.iter().map(|t| LanAddr::parse(t).ok_or(Error::BadQr)).collect()
}

impl QrPayload {
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.exp
    }

    pub fn first_lan(&self) -> Option<&LanAddr> {
        self.lan.first()
    }

    fn validate(&self) -> Result<()> {
        let ok = valid_id(&self.machine)
            && valid_text(&self.name)
            && self.lan.len() <= MAX_LAN
            && self.lan.iter().all(LanAddr::valid)
            && self.relay.as_deref().is_none_or(|r| !r.is_empty() && valid_text(r))
            && (self.ticket.is_none() || self.relay.is_some());
        if ok {
            Ok(())
        } else {
            Err(Error::BadQr)
        }
    }

    pub fn encode(&self) -> Result<String> {
        self.validate()?;
        let wire = QrWire {
            v: QR_VERSION,
            machine: self.machine.clone(),
            name: self.name.clone(),
            host: None,
            port: None,
            lan: (!self.lan.is_empty()).then(|| self.lan.iter().map(LanAddr::wire).collect()),
            relay: self.relay.clone(),
            pt: self.ticket.as_ref().map(|t| t.as_str().to_owned()),
            mkey: b64_encode(&self.mkey),
            code: b64_encode(self.code.as_bytes()),
            exp: self.exp,
        };
        let json = Zeroizing::new(serde_json::to_vec(&wire).map_err(|_| Error::BadQr)?);
        let out = b64_encode(&json);
        if out.len() > MAX_QR_LEN {
            return Err(Error::TooLarge);
        }
        Ok(out)
    }

    pub fn decode(text: &str) -> Result<Self> {
        if text.is_empty() || text.len() > MAX_QR_LEN {
            return Err(Error::BadQr);
        }
        let json = Zeroizing::new(b64_decode(text).ok_or(Error::BadQr)?);
        let wire: QrWire = serde_json::from_slice(&json).map_err(|_| Error::BadQr)?;
        let (lan, ticket) = match wire.v {
            QR_V1 if wire.lan.is_none() && wire.pt.is_none() => (v1_lan(wire.host.as_ref(), wire.port)?, None),
            QR_VERSION if wire.host.is_none() && wire.port.is_none() => {
                (v2_lan(wire.lan.as_ref())?, wire.pt.as_deref().map(PairTicket::parse).transpose()?)
            }
            _ => return Err(Error::BadQr),
        };
        let mkey: [u8; KEY_LEN] = b64_decode(&wire.mkey)
            .ok_or(Error::BadQr)?
            .as_slice()
            .try_into()
            .map_err(|_| Error::BadQr)?;
        let code_raw = Zeroizing::new(b64_decode(&wire.code).ok_or(Error::BadQr)?);
        let code = PairCode::from_bytes(&code_raw)?;
        let qr = QrPayload {
            machine: wire.machine.clone(),
            name: wire.name.clone(),
            lan,
            relay: wire.relay.clone(),
            ticket,
            mkey,
            code,
            exp: wire.exp,
        };
        qr.validate()?;
        Ok(qr)
    }
}
