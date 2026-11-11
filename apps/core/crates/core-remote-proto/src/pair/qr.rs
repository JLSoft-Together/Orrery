use std::fmt;

use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use crate::keys::KEY_LEN;
use crate::wire::{b64_decode, b64_encode, valid_id};
use crate::{Error, Result};

pub const CODE_LEN: usize = 16;
pub const PSK_LEN: usize = 32;
pub const QR_VERSION: u16 = 1;
pub const MAX_QR_LEN: usize = 640;
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

#[derive(Clone)]
pub struct QrPayload {
    pub machine: String,
    pub name: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub relay: Option<String>,
    pub mkey: [u8; KEY_LEN],
    pub code: PairCode,
    pub exp: u64,
}

impl fmt::Debug for QrPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrPayload")
            .field("machine", &self.machine)
            .field("name", &self.name)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("relay", &self.relay)
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
    relay: Option<String>,
    mkey: String,
    code: String,
    exp: u64,
}

impl Drop for QrWire {
    fn drop(&mut self) {
        self.code.zeroize();
    }
}

fn valid_text(value: &str) -> bool {
    value.len() <= MAX_TEXT_LEN && !value.chars().any(char::is_control)
}

impl QrPayload {
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.exp
    }

    fn validate(&self) -> Result<()> {
        let ok = valid_id(&self.machine)
            && valid_text(&self.name)
            && self.host.as_deref().is_none_or(|h| !h.is_empty() && valid_text(h))
            && self.relay.as_deref().is_none_or(|r| !r.is_empty() && valid_text(r))
            && self.port != Some(0);
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
            host: self.host.clone(),
            port: self.port,
            relay: self.relay.clone(),
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
        if wire.v != QR_VERSION {
            return Err(Error::BadQr);
        }
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
            host: wire.host.clone(),
            port: wire.port,
            relay: wire.relay.clone(),
            mkey,
            code,
            exp: wire.exp,
        };
        qr.validate()?;
        Ok(qr)
    }
}
