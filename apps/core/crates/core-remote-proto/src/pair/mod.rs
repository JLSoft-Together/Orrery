mod qr;

use std::fmt;

use serde::{Deserialize, Serialize};
use snow::HandshakeState;

use crate::keys::KEY_LEN;
use crate::wire::{read, remote_static, split_proto, valid_id, with_proto, write, Setup, PAIR_PATTERN};
use crate::{prologue, Error, Result, StaticKeypair, Transport, PROTO};

pub use qr::{PairCode, QrPayload, CODE_LEN, MAX_QR_LEN, PSK_LEN, PSK_SALT, QR_VERSION};

#[derive(Serialize, Deserialize)]
struct DeviceHello {
    #[serde(rename = "deviceId")]
    device_id: String,
    name: String,
}

pub struct PairInitiator {
    hs: HandshakeState,
    mkey: [u8; KEY_LEN],
    machine: String,
    hello: Vec<u8>,
}

#[derive(Debug)]
pub struct PairedDesktop {
    pub machine_id: String,
    pub desktop_static: [u8; KEY_LEN],
    pub transport: Transport,
}

pub struct PairResponder {
    hs: HandshakeState,
}

#[derive(Debug)]
pub struct PairedDevice {
    pub device_id: String,
    pub name: String,
    pub device_static: [u8; KEY_LEN],
    pub transport: Transport,
}

fn valid_name(name: &str) -> bool {
    name.len() <= 128 && !name.chars().any(char::is_control)
}

impl PairInitiator {
    pub fn start(
        local: &StaticKeypair,
        qr: &QrPayload,
        device_id: &str,
        device_name: &str,
        now: u64,
    ) -> Result<(Self, Vec<u8>)> {
        Self::start_with(local, qr, device_id, device_name, now, None)
    }

    #[cfg(feature = "test-vectors")]
    pub fn start_fixed(
        local: &StaticKeypair,
        qr: &QrPayload,
        device_id: &str,
        device_name: &str,
        now: u64,
        ephemeral: &[u8; KEY_LEN],
    ) -> Result<(Self, Vec<u8>)> {
        Self::start_with(local, qr, device_id, device_name, now, Some(ephemeral))
    }

    pub(crate) fn start_with(
        local: &StaticKeypair,
        qr: &QrPayload,
        device_id: &str,
        device_name: &str,
        now: u64,
        ephemeral: Option<&[u8; KEY_LEN]>,
    ) -> Result<(Self, Vec<u8>)> {
        if qr.is_expired(now) {
            return Err(Error::Expired);
        }
        if !valid_id(device_id) || !valid_name(device_name) {
            return Err(Error::BadPayload);
        }
        let hello = serde_json::to_vec(&DeviceHello {
            device_id: device_id.to_owned(),
            name: device_name.to_owned(),
        })
        .map_err(|_| Error::BadPayload)?;
        let psk = qr.code.derive_psk(&qr.machine);
        let prologue = prologue(PROTO);
        let mut hs = Setup {
            pattern: PAIR_PATTERN,
            prologue: &prologue,
            local,
            remote: None,
            psk: Some(&psk),
            ephemeral,
        }
        .initiator()?;
        let msg1 = with_proto(PROTO, &write(&mut hs, &[])?);
        Ok((
            Self {
                hs,
                mkey: qr.mkey,
                machine: qr.machine.clone(),
                hello,
            },
            msg1,
        ))
    }

    pub fn finish(mut self, msg2: &[u8]) -> Result<(PairedDesktop, Vec<u8>)> {
        let payload = read(&mut self.hs, msg2)?;
        if !payload.is_empty() {
            return Err(Error::BadPayload);
        }
        let desktop_static = remote_static(&self.hs)?;
        if desktop_static != self.mkey {
            return Err(Error::KeyMismatch);
        }
        let msg3 = write(&mut self.hs, &self.hello)?;
        let transport = Transport::from_handshake(self.hs)?;
        Ok((
            PairedDesktop {
                machine_id: self.machine,
                desktop_static,
                transport,
            },
            msg3,
        ))
    }
}

impl PairResponder {
    pub fn start(
        local: &StaticKeypair,
        code: &PairCode,
        machine_id: &str,
        msg1: &[u8],
    ) -> Result<(Self, Vec<u8>)> {
        Self::start_with(local, code, machine_id, msg1, None)
    }

    pub(crate) fn start_with(
        local: &StaticKeypair,
        code: &PairCode,
        machine_id: &str,
        msg1: &[u8],
        ephemeral: Option<&[u8; KEY_LEN]>,
    ) -> Result<(Self, Vec<u8>)> {
        let (proto, noise) = split_proto(msg1)?;
        let psk = code.derive_psk(machine_id);
        let prologue = prologue(proto);
        let mut hs = Setup {
            pattern: PAIR_PATTERN,
            prologue: &prologue,
            local,
            remote: None,
            psk: Some(&psk),
            ephemeral,
        }
        .responder()?;
        let payload = read(&mut hs, noise)?;
        if !payload.is_empty() {
            return Err(Error::BadPayload);
        }
        let msg2 = write(&mut hs, &[])?;
        Ok((Self { hs }, msg2))
    }

    pub fn finish(mut self, msg3: &[u8]) -> Result<PairedDevice> {
        let payload = read(&mut self.hs, msg3)?;
        let hello: DeviceHello = serde_json::from_slice(&payload).map_err(|_| Error::BadPayload)?;
        if !valid_id(&hello.device_id) || !valid_name(&hello.name) {
            return Err(Error::BadPayload);
        }
        let device_static = remote_static(&self.hs)?;
        let transport = Transport::from_handshake(self.hs)?;
        Ok(PairedDevice {
            device_id: hello.device_id,
            name: hello.name,
            device_static,
            transport,
        })
    }
}

impl fmt::Debug for PairInitiator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairInitiator(..)")
    }
}

impl fmt::Debug for PairResponder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairResponder(..)")
    }
}
