use std::fmt;

use snow::params::DHChoice;
use snow::resolvers::{CryptoResolver, DefaultResolver};
use zeroize::Zeroizing;

use crate::{Error, Result};

pub const KEY_LEN: usize = 32;

#[derive(Clone)]
pub struct StaticKeypair {
    private: Zeroizing<[u8; KEY_LEN]>,
    public: [u8; KEY_LEN],
}

impl StaticKeypair {
    pub fn generate() -> Result<Self> {
        let mut private = Zeroizing::new([0u8; KEY_LEN]);
        getrandom::fill(private.as_mut_slice()).map_err(|_| Error::Rng)?;
        Self::from_private(private.as_slice())
    }

    pub fn from_private(bytes: &[u8]) -> Result<Self> {
        let raw: [u8; KEY_LEN] = bytes.try_into().map_err(|_| Error::BadKey)?;
        let private = Zeroizing::new(raw);
        let mut dh = DefaultResolver
            .resolve_dh(&DHChoice::Curve25519)
            .ok_or(Error::Noise)?;
        dh.set(private.as_slice());
        let public: [u8; KEY_LEN] = dh.pubkey().try_into().map_err(|_| Error::BadKey)?;
        Ok(Self { private, public })
    }

    pub fn public(&self) -> &[u8; KEY_LEN] {
        &self.public
    }

    pub fn export_private(&self) -> Zeroizing<[u8; KEY_LEN]> {
        self.private.clone()
    }

    pub(crate) fn private(&self) -> &[u8; KEY_LEN] {
        &self.private
    }

    pub fn agree(&self, peer: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>> {
        let peer = parse_public(peer)?;
        let mut dh = DefaultResolver
            .resolve_dh(&DHChoice::Curve25519)
            .ok_or(Error::Noise)?;
        dh.set(self.private.as_slice());
        let mut out = Zeroizing::new([0u8; KEY_LEN]);
        dh.dh(&peer, out.as_mut_slice()).map_err(|_| Error::BadKey)?;
        if out.iter().all(|b| *b == 0) {
            return Err(Error::BadKey);
        }
        Ok(out)
    }
}

impl fmt::Debug for StaticKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticKeypair(..)")
    }
}

pub(crate) fn parse_public(bytes: &[u8]) -> Result<[u8; KEY_LEN]> {
    bytes.try_into().map_err(|_| Error::BadKey)
}
