use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use sha1::Sha1;

use crate::state::lock;

pub const PREFIX: &str = "o1:";
pub const MASTER: &str = "safe-storage-key";
pub const LEGACY: &str = "electron-safe-storage-key";

pub trait Secrets: Send + Sync {
    fn get(&self, name: &str) -> Option<String>;
    fn set(&self, name: &str, value: &str) -> bool;
}

pub struct Keyring {
    pub service: String,
}

impl Secrets for Keyring {
    fn get(&self, name: &str) -> Option<String> {
        keyring::Entry::new(&self.service, name).ok()?.get_password().ok()
    }

    fn set(&self, name: &str, value: &str) -> bool {
        keyring::Entry::new(&self.service, name).and_then(|e| e.set_password(value)).is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyKey {
    Gcm([u8; 32]),
    Cbc([u8; 16]),
}

impl LegacyKey {
    pub fn encode(&self) -> String {
        match self {
            LegacyKey::Gcm(k) => format!("gcm:{}", B64.encode(k)),
            LegacyKey::Cbc(k) => format!("cbc:{}", B64.encode(k)),
        }
    }

    pub fn decode(s: &str) -> Option<LegacyKey> {
        let (kind, raw) = s.split_once(':')?;
        let bytes = B64.decode(raw).ok()?;
        match kind {
            "gcm" => Some(LegacyKey::Gcm(bytes.try_into().ok()?)),
            "cbc" => Some(LegacyKey::Cbc(bytes.try_into().ok()?)),
            _ => None,
        }
    }
}

pub fn mac_key(password: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    key
}

pub fn legacy_decrypt(key: LegacyKey, data: &[u8]) -> Option<String> {
    let body = data.strip_prefix(b"v10")?;
    let plain = match key {
        LegacyKey::Gcm(k) => {
            if body.len() < 12 + 16 {
                return None;
            }
            let (nonce, ct) = body.split_at(12);
            Aes256Gcm::new(&k.into()).decrypt(Nonce::from_slice(nonce), ct).ok()?
        }
        LegacyKey::Cbc(k) => cbc::Decryptor::<aes::Aes128>::new(&k.into(), &[b' '; 16].into()).decrypt_padded_vec_mut::<Pkcs7>(body).ok()?,
    };
    String::from_utf8(plain).ok()
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnsealError {
    Broken,
    Legacy,
}

pub type LegacySource = Box<dyn Fn() -> Option<LegacyKey> + Send + Sync>;

pub struct Vault {
    secrets: Box<dyn Secrets>,
    legacy: LegacySource,
    master: Mutex<Option<[u8; 32]>>,
    old: Mutex<Option<LegacyKey>>,
    warned: AtomicBool,
}

fn random<const N: usize>() -> Option<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).ok()?;
    Some(buf)
}

fn key32(s: &str) -> Option<[u8; 32]> {
    B64.decode(s.trim()).ok()?.try_into().ok()
}

impl Vault {
    pub fn new(secrets: Box<dyn Secrets>, legacy: LegacySource) -> Vault {
        Vault { secrets, legacy, master: Mutex::new(None), old: Mutex::new(None), warned: AtomicBool::new(false) }
    }

    fn master(&self, create: bool) -> Option<[u8; 32]> {
        let mut cached = lock(&self.master);
        if let Some(k) = *cached {
            return Some(k);
        }
        let found = self.secrets.get(MASTER).and_then(|s| key32(&s));
        let key = match found {
            Some(k) => k,
            None if create => {
                let fresh = random::<32>()?;
                if !self.secrets.set(MASTER, &B64.encode(fresh)) {
                    return None;
                }
                self.secrets.get(MASTER).and_then(|s| key32(&s)).filter(|k| *k == fresh)?
            }
            None => return None,
        };
        *cached = Some(key);
        Some(key)
    }

    fn legacy_key(&self) -> Option<LegacyKey> {
        let mut cached = lock(&self.old);
        if let Some(k) = *cached {
            return Some(k);
        }
        let key = match self.secrets.get(LEGACY).and_then(|s| LegacyKey::decode(&s)) {
            Some(k) => k,
            None => {
                let k = (self.legacy)()?;
                self.secrets.set(LEGACY, &k.encode());
                k
            }
        };
        *cached = Some(key);
        Some(key)
    }

    pub fn available(&self) -> bool {
        self.master(true).is_some()
    }

    pub fn seal(&self, plain: &str) -> Option<String> {
        let key = self.master(true)?;
        let nonce = random::<12>()?;
        let ct = Aes256Gcm::new(&key.into()).encrypt(Nonce::from_slice(&nonce), plain.as_bytes()).ok()?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ct);
        Some(format!("{PREFIX}{}", B64.encode(out)))
    }

    pub fn unseal(&self, sealed: &str) -> Result<String, UnsealError> {
        if let Some(rest) = sealed.strip_prefix(PREFIX) {
            let key = self.master(false).ok_or(UnsealError::Broken)?;
            let raw = B64.decode(rest).map_err(|_| UnsealError::Broken)?;
            if raw.len() < 12 + 16 {
                return Err(UnsealError::Broken);
            }
            let (nonce, ct) = raw.split_at(12);
            let plain = Aes256Gcm::new(&key.into()).decrypt(Nonce::from_slice(nonce), ct).map_err(|_| UnsealError::Broken)?;
            return String::from_utf8(plain).map_err(|_| UnsealError::Broken);
        }
        let raw = B64.decode(sealed.trim()).map_err(|_| UnsealError::Broken)?;
        let key = self.legacy_key().ok_or(UnsealError::Legacy)?;
        legacy_decrypt(key, &raw).ok_or(UnsealError::Legacy)
    }

    pub fn first_warning(&self) -> bool {
        !self.warned.swap(true, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncryptMut;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Default, Clone)]
    struct Mem(Arc<Mutex<HashMap<String, String>>>, Arc<AtomicBool>);

    impl Secrets for Mem {
        fn get(&self, name: &str) -> Option<String> {
            lock(&self.0).get(name).cloned()
        }
        fn set(&self, name: &str, value: &str) -> bool {
            if self.1.load(Ordering::SeqCst) {
                return false;
            }
            lock(&self.0).insert(name.into(), value.into());
            true
        }
    }

    fn vault(mem: &Mem, legacy: Option<LegacyKey>) -> Vault {
        Vault::new(Box::new(mem.clone()), Box::new(move || legacy))
    }

    fn electron_win(key: [u8; 32], plain: &str) -> String {
        let nonce = [7u8; 12];
        let ct = Aes256Gcm::new(&key.into()).encrypt(Nonce::from_slice(&nonce), plain.as_bytes()).unwrap();
        let mut out = b"v10".to_vec();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        B64.encode(out)
    }

    fn electron_mac(password: &str, plain: &str) -> String {
        let key = mac_key(password);
        let ct = cbc::Encryptor::<aes::Aes128>::new(&key.into(), &[b' '; 16].into()).encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes());
        let mut out = b"v10".to_vec();
        out.extend_from_slice(&ct);
        B64.encode(out)
    }

    #[test]
    fn seal_and_unseal_round_trip_through_one_master_key() {
        let mem = Mem::default();
        let v = vault(&mem, None);
        assert!(v.available());
        let a = v.seal("ghp_secret").unwrap();
        let b = v.seal("ghp_secret").unwrap();
        assert!(a.starts_with(PREFIX));
        assert_ne!(a, b);
        assert!(!a.contains("ghp_secret"));
        assert_eq!(v.unseal(&a).unwrap(), "ghp_secret");
        let again = vault(&mem, None);
        assert_eq!(again.unseal(&b).unwrap(), "ghp_secret");
        assert_eq!(lock(&mem.0).len(), 1);
    }

    #[test]
    fn a_broken_or_foreign_value_is_an_error_not_a_panic() {
        let mem = Mem::default();
        let v = vault(&mem, None);
        let sealed = v.seal("x").unwrap();
        let mut cut = sealed.clone();
        cut.truncate(sealed.len() - 4);
        assert_eq!(v.unseal(&cut), Err(UnsealError::Broken));
        assert_eq!(v.unseal("o1:!!"), Err(UnsealError::Broken));
        assert_eq!(v.unseal("o1:"), Err(UnsealError::Broken));
        assert_eq!(vault(&Mem::default(), None).unseal(&sealed), Err(UnsealError::Broken));
        assert_eq!(v.unseal("not base64 ***"), Err(UnsealError::Broken));
        assert_eq!(v.unseal(&electron_win([1; 32], "t")), Err(UnsealError::Legacy));
    }

    #[test]
    fn electron_windows_ciphertext_is_read_and_its_key_kept_in_the_keyring() {
        let key = [9u8; 32];
        let mem = Mem::default();
        let v = vault(&mem, Some(LegacyKey::Gcm(key)));
        assert_eq!(v.unseal(&electron_win(key, "ghp_old")).unwrap(), "ghp_old");
        assert_eq!(lock(&mem.0).get(LEGACY).cloned(), Some(LegacyKey::Gcm(key).encode()));
        let later = vault(&mem, None);
        assert_eq!(later.unseal(&electron_win(key, "tg")).unwrap(), "tg");
        assert_eq!(later.unseal(&electron_win([2; 32], "tg")), Err(UnsealError::Legacy));
    }

    #[test]
    fn electron_mac_ciphertext_uses_the_keychain_password() {
        let key = mac_key("pa55word==");
        let v = vault(&Mem::default(), Some(LegacyKey::Cbc(key)));
        assert_eq!(v.unseal(&electron_mac("pa55word==", "ghp_mac")).unwrap(), "ghp_mac");
        assert_eq!(v.unseal(&electron_mac("other", "ghp_mac")), Err(UnsealError::Legacy));
    }

    #[test]
    fn a_keyring_that_refuses_writes_means_no_sealing() {
        let mem = Mem::default();
        mem.1.store(true, Ordering::SeqCst);
        let v = vault(&mem, None);
        assert!(!v.available());
        assert!(v.seal("x").is_none());
    }

    #[test]
    fn legacy_keys_encode_both_ways_and_reject_junk() {
        for k in [LegacyKey::Gcm([3; 32]), LegacyKey::Cbc([4; 16])] {
            assert_eq!(LegacyKey::decode(&k.encode()), Some(k));
        }
        assert_eq!(LegacyKey::decode("gcm:AAAA"), None);
        assert_eq!(LegacyKey::decode("xyz:AAAA"), None);
        assert_eq!(LegacyKey::decode("nothing"), None);
        assert!(legacy_decrypt(LegacyKey::Gcm([0; 32]), b"v10short").is_none());
        assert!(legacy_decrypt(LegacyKey::Cbc([0; 16]), b"v11whatever0000000").is_none());
    }

    #[test]
    fn warnings_fire_once() {
        let v = vault(&Mem::default(), None);
        assert!(v.first_warning());
        assert!(!v.first_warning());
    }
}
