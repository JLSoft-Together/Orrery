use std::fmt;

use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD};
use base64::Engine;
use core_remote_proto::relay::RelayKey;
use core_remote_proto::StaticKeypair;
use ring::signature::{Ed25519KeyPair, KeyPair};
use zeroize::Zeroizing;

pub const NOISE_SECRET: &str = "orrery-remote-noise";
pub const RELAY_SECRET: &str = "orrery-remote-relay";
pub const SIGN_SECRET: &str = "orrery-remote-sign";
const KEY_LEN: usize = 32;

pub enum Read {
    Found(Zeroizing<String>),
    Missing,
    Failed,
}

pub trait KeyStore: Send + Sync {
    fn read(&self, name: &str) -> Read;
    fn write(&self, name: &str, value: &str) -> bool;
}

pub struct OsKeyring {
    pub service: String,
}

impl KeyStore for OsKeyring {
    fn read(&self, name: &str) -> Read {
        let Ok(entry) = keyring::Entry::new(&self.service, name) else { return Read::Failed };
        match entry.get_password() {
            Ok(v) => Read::Found(Zeroizing::new(v)),
            Err(keyring::Error::NoEntry) => Read::Missing,
            Err(_) => Read::Failed,
        }
    }

    fn write(&self, name: &str, value: &str) -> bool {
        keyring::Entry::new(&self.service, name).and_then(|e| e.set_password(value)).is_ok()
    }
}

pub struct Keys {
    pub noise: StaticKeypair,
    pub relay: RelayKey,
    sign: Ed25519KeyPair,
}

impl fmt::Debug for Keys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Keys(..)")
    }
}

fn random32() -> Option<Zeroizing<[u8; KEY_LEN]>> {
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::fill(out.as_mut_slice()).ok()?;
    Some(out)
}

fn decode(text: &str) -> Option<Zeroizing<Vec<u8>>> {
    let raw = Zeroizing::new(B64.decode(text.trim()).ok()?);
    (raw.len() == KEY_LEN).then_some(raw)
}

fn stored(store: &dyn KeyStore, name: &str) -> Option<Zeroizing<Vec<u8>>> {
    match store.read(name) {
        Read::Found(text) => return decode(&text),
        Read::Failed => return None,
        Read::Missing => {}
    }
    let fresh = random32()?;
    let text = Zeroizing::new(B64.encode(fresh.as_slice()));
    if !store.write(name, &text) {
        return None;
    }
    match store.read(name) {
        Read::Found(back) => decode(&back).filter(|raw| raw.as_slice() == fresh.as_slice()),
        _ => None,
    }
}

impl Keys {
    pub fn load_or_create(store: &dyn KeyStore) -> Option<Keys> {
        let noise = StaticKeypair::from_private(&stored(store, NOISE_SECRET)?).ok()?;
        let relay = RelayKey::from_bytes(&stored(store, RELAY_SECRET)?).ok()?;
        let sign = Ed25519KeyPair::from_seed_unchecked(&stored(store, SIGN_SECRET)?).ok()?;
        Some(Keys { noise, relay, sign })
    }

    pub fn sign_public(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.sign.public_key().as_ref())
    }

    pub fn relay_key(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.relay.as_bytes())
    }

    pub fn sign(&self, domain: &str, machine_id: &str, ts: u64) -> String {
        let message = format!("{domain}{machine_id}{ts}");
        URL_SAFE_NO_PAD.encode(self.sign.sign(message.as_bytes()).as_ref())
    }
}
