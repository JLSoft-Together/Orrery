use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use snow::params::NoiseParams;
use snow::{Builder, HandshakeState};
use zeroize::Zeroizing;

use crate::keys::{parse_public, KEY_LEN};
use crate::{Error, Result, StaticKeypair, MIN_PROTO, PROTO};

pub(crate) const NOISE_MAX: usize = 65_535;
pub(crate) const TAG_LEN: usize = 16;
pub(crate) const MAX_ID_LEN: usize = 128;
pub(crate) const PAIR_PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s";
pub(crate) const SESSION_PATTERN: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";

pub(crate) struct Setup<'a> {
    pub pattern: &'static str,
    pub prologue: &'a [u8],
    pub local: &'a StaticKeypair,
    pub remote: Option<&'a [u8; KEY_LEN]>,
    pub psk: Option<&'a [u8; 32]>,
    pub ephemeral: Option<&'a [u8; KEY_LEN]>,
}

impl Setup<'_> {
    fn builder(&self) -> Result<Builder<'_>> {
        let params: NoiseParams = self.pattern.parse().map_err(|_| Error::Noise)?;
        let mut b = Builder::new(params)
            .prologue(self.prologue)?
            .local_private_key(self.local.private())?;
        if let Some(remote) = self.remote {
            b = b.remote_public_key(remote)?;
        }
        if let Some(psk) = self.psk {
            b = b.psk(3, psk)?;
        }
        if let Some(e) = self.ephemeral {
            b = b.fixed_ephemeral_key_for_testing_only(e);
        }
        Ok(b)
    }

    pub fn initiator(&self) -> Result<HandshakeState> {
        Ok(self.builder()?.build_initiator()?)
    }

    pub fn responder(&self) -> Result<HandshakeState> {
        Ok(self.builder()?.build_responder()?)
    }
}

pub(crate) fn write(hs: &mut HandshakeState, payload: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![0u8; NOISE_MAX];
    let n = hs.write_message(payload, &mut out)?;
    out.truncate(n);
    Ok(out)
}

pub(crate) fn read(hs: &mut HandshakeState, message: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if message.len() > NOISE_MAX {
        return Err(Error::BadMessage);
    }
    let mut out = Zeroizing::new(vec![0u8; message.len()]);
    let n = hs.read_message(message, &mut out)?;
    out.truncate(n);
    Ok(out)
}

pub(crate) fn remote_static(hs: &HandshakeState) -> Result<[u8; KEY_LEN]> {
    parse_public(hs.get_remote_static().ok_or(Error::Noise)?)
}

pub(crate) fn with_proto(proto: u16, noise: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(noise.len() + 2);
    out.extend_from_slice(&proto.to_be_bytes());
    out.extend_from_slice(noise);
    out
}

pub(crate) fn split_proto(message: &[u8]) -> Result<(u16, &[u8])> {
    if message.len() < 2 {
        return Err(Error::BadMessage);
    }
    let proto = u16::from_be_bytes([message[0], message[1]]);
    check_proto(proto)?;
    Ok((proto, &message[2..]))
}

pub(crate) fn check_proto(proto: u16) -> Result<()> {
    if proto < MIN_PROTO {
        return Err(Error::ProtoTooOld { min: MIN_PROTO });
    }
    if proto > PROTO {
        return Err(Error::ProtoTooNew { max: PROTO });
    }
    Ok(())
}

pub(crate) fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ID_LEN && !value.chars().any(char::is_control)
}

pub(crate) fn b64_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn b64_decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text.as_bytes()).ok()
}
