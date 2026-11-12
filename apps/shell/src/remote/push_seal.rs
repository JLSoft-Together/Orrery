use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, X25519};
use ring::hkdf::{KeyType, Salt, HKDF_SHA256};
use core_remote_proto::StaticKeypair;
use ring::rand::SystemRandom;
use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::ctx::Ctx;
use super::devices::Device;
use super::push::{message, Job};
use super::webpush::{b64, unb64};

pub const SEAL_VERSION: &str = "2";
pub const SEAL_LEVEL: u64 = 2;
pub const DOMAIN: &[u8] = b"orrery-push/2";
pub const KEY_LEN: usize = 32;
pub const PAD_STEP: usize = 64;
pub const DATA_MAX: usize = 4096;
pub const SEAL_OK_TTL: u64 = 48 * 3600;
const AEAD_KEY_LEN: usize = 32;
const OKM_LEN: usize = AEAD_KEY_LEN + 12;

struct Okm;

impl KeyType for Okm {
    fn len(&self) -> usize {
        OKM_LEN
    }
}

pub fn info(machine: &str) -> Vec<u8> {
    [DOMAIN, b"\0", machine.as_bytes()].concat()
}

pub fn aad(machine: &str, kind: &str) -> Vec<u8> {
    [DOMAIN, b"\0", machine.as_bytes(), b"\0", kind.as_bytes()].concat()
}

pub fn derive(shared: &[u8], fixed: &[u8], ephemeral: &[u8], device: &[u8], desktop: &[u8], machine: &str) -> Option<Zeroizing<[u8; OKM_LEN]>> {
    let salt = [ephemeral, device, desktop].concat();
    let ikm = Zeroizing::new([shared, fixed].concat());
    let prk = Salt::new(HKDF_SHA256, &salt).extract(&ikm);
    let info = info(machine);
    let mut out = Zeroizing::new([0u8; OKM_LEN]);
    prk.expand(&[&info], Okm).ok()?.fill(&mut out[..]).ok()?;
    Some(out)
}

pub fn seal_fresh(device: &Device, now: u64) -> bool {
    device.push_seal && device.push_seal_ok && now.saturating_sub(device.push_seal_at) <= SEAL_OK_TTL
}

pub fn padded(plain: &[u8]) -> Vec<u8> {
    let mut out = plain.to_vec();
    out.resize(plain.len().div_ceil(PAD_STEP).max(1) * PAD_STEP, b' ');
    out
}

pub fn device_key(pubkey: &str) -> Option<[u8; KEY_LEN]> {
    unb64(pubkey).and_then(|raw| <[u8; KEY_LEN]>::try_from(raw).ok())
}

pub fn seal_with(ephemeral: EphemeralPrivateKey, desktop: &StaticKeypair, device: &[u8], machine: &str, kind: &str, plain: &[u8]) -> Option<Vec<u8>> {
    if device.len() != KEY_LEN {
        return None;
    }
    let fixed = desktop.agree(device).ok()?;
    let public = ephemeral.compute_public_key().ok()?.as_ref().to_vec();
    let okm = agree_ephemeral(ephemeral, &UnparsedPublicKey::new(&X25519, device), |shared| derive(shared, &fixed[..], &public, device, desktop.public(), machine)).ok()??;
    let key = LessSafeKey::new(UnboundKey::new(&CHACHA20_POLY1305, &okm[..AEAD_KEY_LEN]).ok()?);
    let nonce = Nonce::try_assume_unique_for_key(&okm[AEAD_KEY_LEN..]).ok()?;
    let mut body = padded(plain);
    key.seal_in_place_append_tag(nonce, Aad::from(aad(machine, kind)), &mut body).ok()?;
    Some([public, body].concat())
}

pub fn seal(desktop: &StaticKeypair, device: &[u8], machine: &str, kind: &str, plain: &[u8]) -> Option<String> {
    let ephemeral = EphemeralPrivateKey::generate(&X25519, &SystemRandom::new()).ok()?;
    seal_with(ephemeral, desktop, device, machine, kind, plain).map(|raw| b64(&raw))
}

pub fn sealed_plain(job: &Job, title: &str, body: &str) -> Vec<u8> {
    let mut out = json!({ "t": title, "b": body });
    if !job.pane.is_empty() {
        out["p"] = Value::from(job.pane.clone());
    }
    out.to_string().into_bytes()
}

pub fn data_size(msg: &Value) -> usize {
    msg["message"]["data"].as_object().map_or(0, |d| d.iter().map(|(k, v)| k.len() + v.as_str().map_or(0, str::len)).sum())
}

pub fn sealed_message(job: &Job, sealed: &str) -> Value {
    json!({
        "message": {
            "token": job.token,
            "data": { "v": SEAL_VERSION, "m": job.machine, "k": job.kind, "e": sealed },
            "android": { "priority": "high" },
        }
    })
}

pub fn probe_message(job: &Job, sealed: &str) -> Value {
    let mut out = message(job);
    out["message"]["data"]["v"] = Value::from(SEAL_VERSION);
    out["message"]["data"]["e"] = Value::from(sealed);
    out
}

pub fn sealed_fcm(job: &Job, desktop: &StaticKeypair, device: &[u8], title: &str, body: &str) -> Option<Value> {
    let sealed = seal(desktop, device, &job.machine, job.kind, &sealed_plain(job, title, body))?;
    let out = if job.seal_ok { sealed_message(job, &sealed) } else { probe_message(job, &sealed) };
    (data_size(&out) <= DATA_MAX).then_some(out)
}

impl Ctx {
    pub fn fcm_message(&self, job: &Job) -> Value {
        let Some(device) = job.seal_key.filter(|_| job.content) else { return message(job) };
        let (title, body) = self.notice(job);
        sealed_fcm(job, &self.keys.noise, &device, &title, &body).unwrap_or_else(|| message(job))
    }
}
