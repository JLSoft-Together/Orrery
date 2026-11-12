use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256};
use serde_json::json;
use zeroize::Zeroizing;

use super::fcm_files::{pem_der, ServiceAccount};

pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";
pub const JWT_LIFETIME: u64 = 3600;

pub fn assertion(account: &ServiceAccount, now: u64) -> Option<String> {
    let header = URL_SAFE_NO_PAD.encode(json!({ "alg": "RS256", "typ": "JWT" }).to_string());
    let claims = URL_SAFE_NO_PAD.encode(
        json!({ "iss": account.client_email, "scope": SCOPE, "aud": TOKEN_URL, "iat": now, "exp": now + JWT_LIFETIME }).to_string(),
    );
    let signing_input = format!("{header}.{claims}");
    let der = Zeroizing::new(pem_der(&account.private_key)?);
    let key = RsaKeyPair::from_pkcs8(&der).ok()?;
    let mut sig = vec![0u8; key.public().modulus_len()];
    key.sign(&RSA_PKCS1_SHA256, &SystemRandom::new(), signing_input.as_bytes(), &mut sig).ok()?;
    Some(format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(sig)))
}
