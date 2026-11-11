use std::fmt;

use snow::{HandshakeState, TransportState};

use crate::keys::{parse_public, KEY_LEN};
use crate::wire::{NOISE_MAX, TAG_LEN};
use crate::{Error, Result};

pub const MAX_PLAINTEXT: usize = NOISE_MAX - TAG_LEN;

pub struct Transport {
    inner: TransportState,
    dead: bool,
}

impl Transport {
    pub(crate) fn from_handshake(hs: HandshakeState) -> Result<Self> {
        Ok(Self {
            inner: hs.into_transport_mode()?,
            dead: false,
        })
    }

    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if self.dead {
            return Err(Error::Closed);
        }
        if plaintext.len() > MAX_PLAINTEXT {
            return Err(Error::TooLarge);
        }
        let mut out = vec![0u8; plaintext.len() + TAG_LEN];
        let n = self.inner.write_message(plaintext, &mut out)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn open(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if self.dead {
            return Err(Error::Closed);
        }
        if ciphertext.len() < TAG_LEN || ciphertext.len() > NOISE_MAX {
            self.dead = true;
            return Err(Error::BadMessage);
        }
        let mut out = vec![0u8; ciphertext.len()];
        match self.inner.read_message(ciphertext, &mut out) {
            Ok(n) => {
                out.truncate(n);
                Ok(out)
            }
            Err(_) => {
                self.dead = true;
                Err(Error::Noise)
            }
        }
    }

    pub fn is_closed(&self) -> bool {
        self.dead
    }

    pub fn remote_static(&self) -> Option<[u8; KEY_LEN]> {
        self.inner
            .get_remote_static()
            .and_then(|k| parse_public(k).ok())
    }
}

impl fmt::Debug for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transport").field("closed", &self.dead).finish()
    }
}
