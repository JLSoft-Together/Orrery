use std::fmt;

use serde::{Deserialize, Serialize};
use snow::HandshakeState;

use crate::keys::KEY_LEN;
use crate::wire::{read, remote_static, split_proto, valid_id, with_proto, write, Setup, SESSION_PATTERN};
use crate::{prologue, Error, Result, StaticKeypair, Transport, PROTO};

#[derive(Serialize, Deserialize)]
struct Hello {
    #[serde(rename = "deviceId")]
    device_id: String,
    proto: u16,
}

#[derive(Serialize, Deserialize)]
struct Welcome {
    proto: u16,
}

pub struct SessionInitiator {
    hs: HandshakeState,
    proto: u16,
}

#[derive(Debug)]
pub struct SessionAccepted {
    pub device_id: String,
    pub device_static: [u8; KEY_LEN],
    pub proto: u16,
    pub transport: Transport,
}

impl SessionInitiator {
    pub fn start(
        local: &StaticKeypair,
        desktop_static: &[u8; KEY_LEN],
        device_id: &str,
    ) -> Result<(Self, Vec<u8>)> {
        Self::start_with(local, desktop_static, device_id, None)
    }

    pub(crate) fn start_with(
        local: &StaticKeypair,
        desktop_static: &[u8; KEY_LEN],
        device_id: &str,
        ephemeral: Option<&[u8; KEY_LEN]>,
    ) -> Result<(Self, Vec<u8>)> {
        if !valid_id(device_id) {
            return Err(Error::BadPayload);
        }
        let hello = serde_json::to_vec(&Hello {
            device_id: device_id.to_owned(),
            proto: PROTO,
        })
        .map_err(|_| Error::BadPayload)?;
        let prologue = prologue(PROTO);
        let mut hs = Setup {
            pattern: SESSION_PATTERN,
            prologue: &prologue,
            local,
            remote: Some(desktop_static),
            psk: None,
            ephemeral,
        }
        .initiator()?;
        let msg1 = with_proto(PROTO, &write(&mut hs, &hello)?);
        Ok((Self { hs, proto: PROTO }, msg1))
    }

    pub fn finish(mut self, msg2: &[u8]) -> Result<Transport> {
        let payload = read(&mut self.hs, msg2)?;
        let welcome: Welcome = serde_json::from_slice(&payload).map_err(|_| Error::BadPayload)?;
        if welcome.proto != self.proto {
            return Err(Error::BadPayload);
        }
        Transport::from_handshake(self.hs)
    }
}

pub fn accept<F>(local: &StaticKeypair, msg1: &[u8], authorize: F) -> Result<(SessionAccepted, Vec<u8>)>
where
    F: FnOnce(&str, &[u8; KEY_LEN]) -> bool,
{
    accept_with(local, msg1, authorize, None)
}

pub(crate) fn accept_with<F>(
    local: &StaticKeypair,
    msg1: &[u8],
    authorize: F,
    ephemeral: Option<&[u8; KEY_LEN]>,
) -> Result<(SessionAccepted, Vec<u8>)>
where
    F: FnOnce(&str, &[u8; KEY_LEN]) -> bool,
{
    let (proto, noise) = split_proto(msg1)?;
    let prologue = prologue(proto);
    let mut hs = Setup {
        pattern: SESSION_PATTERN,
        prologue: &prologue,
        local,
        remote: None,
        psk: None,
        ephemeral,
    }
    .responder()?;
    let payload = read(&mut hs, noise)?;
    let hello: Hello = serde_json::from_slice(&payload).map_err(|_| Error::BadPayload)?;
    if hello.proto != proto || !valid_id(&hello.device_id) {
        return Err(Error::BadPayload);
    }
    let device_static = remote_static(&hs)?;
    if !authorize(&hello.device_id, &device_static) {
        return Err(Error::Unauthorized);
    }
    let welcome = serde_json::to_vec(&Welcome { proto }).map_err(|_| Error::BadPayload)?;
    let msg2 = write(&mut hs, &welcome)?;
    let transport = Transport::from_handshake(hs)?;
    Ok((
        SessionAccepted {
            device_id: hello.device_id,
            device_static,
            proto,
            transport,
        },
        msg2,
    ))
}

impl fmt::Debug for SessionInitiator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionInitiator(..)")
    }
}
