use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_128_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde_json::json;
use zeroize::Zeroizing;

use super::keys::{KeyStore, Read};

pub const VAPID_SECRET: &str = "orrery-vapid";
pub const RECORD_SIZE: u32 = 4096;
pub const MAX_PLAIN: usize = 3993;
pub const PAD_STEP: usize = 64;
pub const JWT_TTL: u64 = 12 * 3600;
pub const PUBLIC_LEN: usize = 65;
pub const AUTH_LEN: usize = 16;
pub const SALT_LEN: usize = 16;
const KEY_INFO: &[u8] = b"WebPush: info\0";
const CEK_INFO: &[u8] = b"Content-Encoding: aes128gcm\0";
const NONCE_INFO: &[u8] = b"Content-Encoding: nonce\0";

pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, salt), ikm).as_ref().to_vec())
}

pub fn hkdf_expand(prk: &[u8], info: &[u8], len: usize) -> Zeroizing<Vec<u8>> {
    let mut input = info.to_vec();
    input.push(1);
    let full = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, prk), &input);
    Zeroizing::new(full.as_ref()[..len.min(32)].to_vec())
}

pub fn key_info(ua_public: &[u8], as_public: &[u8]) -> Vec<u8> {
    [KEY_INFO, ua_public, as_public].concat()
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct Derived {
    pub prk_key: Zeroizing<Vec<u8>>,
    pub ikm: Zeroizing<Vec<u8>>,
    pub prk: Zeroizing<Vec<u8>>,
    pub cek: Zeroizing<Vec<u8>>,
    pub nonce: Zeroizing<Vec<u8>>,
}

pub fn derive(ecdh_secret: &[u8], auth: &[u8], ua_public: &[u8], as_public: &[u8], salt: &[u8]) -> Derived {
    let prk_key = hkdf_extract(auth, ecdh_secret);
    let ikm = hkdf_expand(&prk_key, &key_info(ua_public, as_public), 32);
    let prk = hkdf_extract(salt, &ikm);
    let cek = hkdf_expand(&prk, CEK_INFO, 16);
    let nonce = hkdf_expand(&prk, NONCE_INFO, 12);
    Derived { prk_key, ikm, prk, cek, nonce }
}

pub fn header(salt: &[u8], rs: u32, as_public: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SALT_LEN + 5 + as_public.len());
    out.extend_from_slice(salt);
    out.extend_from_slice(&rs.to_be_bytes());
    out.push(as_public.len() as u8);
    out.extend_from_slice(as_public);
    out
}

pub fn pad_len(plain: usize) -> usize {
    let used = plain + 1;
    let target = used.div_ceil(PAD_STEP) * PAD_STEP;
    target.min(MAX_PLAIN + 1).saturating_sub(used)
}

pub fn seal(cek: &[u8], nonce: &[u8], plain: &[u8], pad: usize) -> Option<Vec<u8>> {
    let key = LessSafeKey::new(UnboundKey::new(&AES_128_GCM, cek).ok()?);
    let nonce = Nonce::try_assume_unique_for_key(nonce).ok()?;
    let mut record = Vec::with_capacity(plain.len() + 1 + pad + 16);
    record.extend_from_slice(plain);
    record.push(2);
    record.resize(plain.len() + 1 + pad, 0);
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut record).ok()?;
    Some(record)
}

pub fn valid_public(raw: &[u8]) -> bool {
    if raw.len() != PUBLIC_LEN || raw[0] != 4 {
        return false;
    }
    let Ok(probe) = EphemeralPrivateKey::generate(&ECDH_P256, &SystemRandom::new()) else { return false };
    agree_ephemeral(probe, &UnparsedPublicKey::new(&ECDH_P256, raw), |_| ()).is_ok()
}

pub fn encrypt_with(ephemeral: EphemeralPrivateKey, salt: &[u8], ua_public: &[u8], auth: &[u8], plain: &[u8], pad: usize) -> Option<Vec<u8>> {
    if plain.len() + pad > MAX_PLAIN || salt.len() != SALT_LEN || auth.len() != AUTH_LEN || ua_public.len() != PUBLIC_LEN {
        return None;
    }
    let as_public = ephemeral.compute_public_key().ok()?;
    let as_public = as_public.as_ref().to_vec();
    let derived = agree_ephemeral(ephemeral, &UnparsedPublicKey::new(&ECDH_P256, ua_public), |secret| derive(secret, auth, ua_public, &as_public, salt)).ok()?;
    let mut body = header(salt, RECORD_SIZE, &as_public);
    body.extend_from_slice(&seal(&derived.cek, &derived.nonce, plain, pad)?);
    Some(body)
}

pub fn encrypt(ua_public: &[u8], auth: &[u8], plain: &[u8]) -> Option<Vec<u8>> {
    let rng = SystemRandom::new();
    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt).ok()?;
    let ephemeral = EphemeralPrivateKey::generate(&ECDH_P256, &rng).ok()?;
    encrypt_with(ephemeral, &salt, ua_public, auth, plain, pad_len(plain.len()))
}

pub fn audience(endpoint: &str) -> Option<String> {
    let rest = endpoint.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next().filter(|h| !h.is_empty())?;
    let host = authority.strip_suffix(":443").unwrap_or(authority);
    Some(format!("https://{host}"))
}

pub fn b64(raw: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(raw)
}

pub fn unb64(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text.trim_end_matches('=')).ok()
}

pub struct Vapid {
    pair: EcdsaKeyPair,
}

impl Vapid {
    pub fn from_pkcs8(der: &[u8]) -> Option<Vapid> {
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, der, &SystemRandom::new()).ok().map(|pair| Vapid { pair })
    }

    pub fn generate() -> Option<(Vapid, Zeroizing<Vec<u8>>)> {
        let doc = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &SystemRandom::new()).ok()?;
        let der = Zeroizing::new(doc.as_ref().to_vec());
        Some((Vapid::from_pkcs8(&der)?, der))
    }

    pub fn load_or_create(store: &dyn KeyStore) -> Option<Vapid> {
        match store.read(VAPID_SECRET) {
            Read::Found(text) => return B64.decode(text.trim()).ok().map(Zeroizing::new).and_then(|der| Vapid::from_pkcs8(&der)),
            Read::Failed => return None,
            Read::Missing => {}
        }
        let (_, der) = Vapid::generate()?;
        let text = Zeroizing::new(B64.encode(der.as_slice()));
        if !store.write(VAPID_SECRET, &text) {
            return None;
        }
        match store.read(VAPID_SECRET) {
            Read::Found(back) if back.trim() == text.as_str() => Vapid::from_pkcs8(&der),
            _ => None,
        }
    }

    pub fn public_key(&self) -> String {
        b64(self.pair.public_key().as_ref())
    }

    pub fn jwt(&self, aud: &str, sub: &str, exp: u64) -> Option<String> {
        let head = b64(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = b64(json!({ "aud": aud, "exp": exp, "sub": sub }).to_string().as_bytes());
        let input = format!("{head}.{claims}");
        let sig = self.pair.sign(&SystemRandom::new(), input.as_bytes()).ok()?;
        Some(format!("{input}.{}", b64(sig.as_ref())))
    }

    pub fn authorization(&self, endpoint: &str, sub: &str, now: u64) -> Option<String> {
        let aud = audience(endpoint)?;
        Some(format!("vapid t={}, k={}", self.jwt(&aud, sub, now + JWT_TTL)?, self.public_key()))
    }
}

pub fn valid_subject(sub: &str) -> bool {
    let ok_scheme = sub.strip_prefix("mailto:").is_some_and(|m| m.contains('@') && !m.starts_with('@') && !m.ends_with('@')) || sub.strip_prefix("https://").is_some_and(|h| !h.is_empty());
    ok_scheme && sub.len() <= 200 && sub.bytes().all(|b| b.is_ascii_graphic() && !matches!(b, b'"' | b'\\' | b'<' | b'>'))
}
