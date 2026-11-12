use ring::agreement::{EphemeralPrivateKey, ECDH_P256};
use ring::signature::{UnparsedPublicKey, ECDSA_P256_SHA256_FIXED};

use super::test_util::MemSecrets;
use super::webpush::*;

const PLAIN: &str = "V2hlbiBJIGdyb3cgdXAsIEkgd2FudCB0byBiZSBhIHdhdGVybWVsb24";
const AS_PUBLIC: &str = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";
const AS_PRIVATE: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
const UA_PUBLIC: &str = "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
const SALT: &str = "DGv6ra1nlYgDCS1FRnbzlw";
const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";
const ECDH: &str = "kyrL1jIIOHEzg3sM2ZWRHDRB62YACZhhSlknJ672kSs";
const PRK_KEY: &str = "Snr3JMxaHVDXHWJn5wdC52WjpCtd2EIEGBykDcZW32k";
const KEY_INFO: &str = "V2ViUHVzaDogaW5mbwAEJXGyvs3942BVGq8e0PTNNmwRzr5VX4m8t7GGpTM5FzFo7OLr4BhZe9MEebhuPI-OztV3ylkYfpJGmQ22ggCLDgT-M_SrDepxkU21WCP3O1SUj0EwbZIHMtu5pZpTKGSCIA5Zent7wmC6HCJ5mFgJkuk5cwAvMBKiiujwa7t45ewP";
const IKM: &str = "S4lYMb_L0FxCeq0WhDx813KgSYqU26kOyzWUdsXYyrg";
const PRK: &str = "09_eUZGrsvxChDCGRCdkLiDXrReGOEVeSCdCcPBSJSc";
const CEK: &str = "oIhVW04MRdy2XN9CiKLxTg";
const NONCE: &str = "4h_95klXJ5E_qnoN";
const HEADER: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";
const CIPHER: &str = "8pfeW0KbunFT06SuDKoJH9Ql87S1QUrdirN6GcG7sFz1y1sqLgVi1VhjVkHsUoEsbI_0LpXMuGvnzQ";
const BODY: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN";

const JWT: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9.eyJhdWQiOiJodHRwczovL3B1c2guZXhhbXBsZS5uZXQiLCJleHAiOjE0NTM1MjM3NjgsInN1YiI6Im1haWx0bzpwdXNoQGV4YW1wbGUuY29tIn0.i3CYb7t4xfxCDquptFOepC9GAu_HLGkMlMuCGSK2rpiUfnK9ojFwDXb1JrErtmysazNjjvW2L9OkSSHzvoD1oA";
const JWT_KEY: &str = "BA1Hxzyi1RUM1b5wjxsn7nGxAszw2u61m164i3MrAIxHF6YK5h4SDYic-dRuU_RCPCfA5aq9ojSwk5Y2EmClBPs";

fn d(s: &str) -> Vec<u8> {
    unb64(s).unwrap()
}

#[allow(deprecated)]
fn fixed_sender() -> EphemeralPrivateKey {
    let raw = d(AS_PRIVATE);
    EphemeralPrivateKey::generate(&ECDH_P256, &ring::test::rand::FixedSliceRandom { bytes: &raw }).unwrap()
}

#[test]
fn rfc8291_intermediate_values_match_byte_for_byte() {
    let ua = d(UA_PUBLIC);
    let asp = d(AS_PUBLIC);
    assert_eq!(key_info(&ua, &asp), d(KEY_INFO));
    let got = derive(&d(ECDH), &d(AUTH), &ua, &asp, &d(SALT));
    assert_eq!(got.prk_key.as_slice(), d(PRK_KEY).as_slice());
    assert_eq!(got.ikm.as_slice(), d(IKM).as_slice());
    assert_eq!(got.prk.as_slice(), d(PRK).as_slice());
    assert_eq!(got.cek.as_slice(), d(CEK).as_slice());
    assert_eq!(got.nonce.as_slice(), d(NONCE).as_slice());
    assert_eq!(b64(&header(&d(SALT), 4096, &asp)), HEADER);
    assert_eq!(b64(&seal(&got.cek, &got.nonce, &d(PLAIN), 0).unwrap()), CIPHER);
}

#[test]
fn rfc8291_the_ecdh_secret_from_the_sender_key_matches() {
    let key = fixed_sender();
    assert_eq!(b64(key.compute_public_key().unwrap().as_ref()), AS_PUBLIC);
    let ua = d(UA_PUBLIC);
    let secret = ring::agreement::agree_ephemeral(key, &ring::agreement::UnparsedPublicKey::new(&ECDH_P256, &ua), |s| s.to_vec()).unwrap();
    assert_eq!(b64(&secret), ECDH);
}

#[test]
fn rfc8291_section5_full_message_matches_byte_for_byte() {
    let body = encrypt_with(fixed_sender(), &d(SALT), &d(UA_PUBLIC), &d(AUTH), &d(PLAIN), 0).unwrap();
    assert_eq!(b64(&body), BODY);
    assert_eq!(body.len(), 86 + 41 + 1 + 16);
}

#[test]
fn random_messages_have_the_86_byte_header_padding_and_fit_one_record() {
    let ua = d(UA_PUBLIC);
    let body = encrypt(&ua, &d(AUTH), b"hello").unwrap();
    assert_eq!(&body[16..20], &4096u32.to_be_bytes());
    assert_eq!(body[20], 65);
    assert_eq!(body[21], 4);
    assert_eq!(body.len(), 86 + 64 + 16);
    let other = encrypt(&ua, &d(AUTH), b"hello").unwrap();
    assert_ne!(body[..16], other[..16]);
    assert_eq!(pad_len(0), 63);
    assert_eq!(pad_len(63), 0);
    assert_eq!(pad_len(64), 63);
    assert_eq!(pad_len(MAX_PLAIN), 0);
    assert_eq!(pad_len(MAX_PLAIN - 10), 10);
    assert!(encrypt(&ua, &d(AUTH), &vec![b'x'; MAX_PLAIN + 1]).is_none());
    assert!(encrypt(&ua, &d(AUTH)[..15], b"x").is_none());
    assert!(encrypt(&ua[..64], &d(AUTH), b"x").is_none());
}

#[test]
fn only_points_on_p256_are_accepted_as_subscription_keys() {
    assert!(valid_public(&d(UA_PUBLIC)));
    let mut off = d(UA_PUBLIC);
    off[64] ^= 1;
    assert!(!valid_public(&off));
    assert!(!valid_public(&d(UA_PUBLIC)[..64]));
    let mut compressed = d(UA_PUBLIC);
    compressed[0] = 2;
    assert!(!valid_public(&compressed));
    assert!(!valid_public(&[4u8; 65]));
}

#[test]
fn rfc8292_example_token_verifies_with_its_key() {
    let (input, sig) = JWT.rsplit_once('.').unwrap();
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, d(JWT_KEY)).verify(input.as_bytes(), &d(sig)).unwrap();
    let claims: serde_json::Value = serde_json::from_slice(&d(input.split('.').nth(1).unwrap())).unwrap();
    assert_eq!(claims, serde_json::json!({ "aud": "https://push.example.net", "exp": 1453523768, "sub": "mailto:push@example.com" }));
}

#[test]
fn our_vapid_token_is_es256_with_origin_audience_12h_expiry_and_verifies() {
    let (vapid, _) = Vapid::generate().unwrap();
    let auth = vapid.authorization("https://wns2-sg2p.notify.windows.com/w/?token=AbC%2Bd", "mailto:me@orrery.dev", 1_000).unwrap();
    let rest = auth.strip_prefix("vapid t=").unwrap();
    let (jwt, k) = rest.split_once(", k=").unwrap();
    assert_eq!(k, vapid.public_key());
    assert_eq!(d(k).len(), 65);
    let mut parts = jwt.split('.');
    let head: serde_json::Value = serde_json::from_slice(&d(parts.next().unwrap())).unwrap();
    let claims: serde_json::Value = serde_json::from_slice(&d(parts.next().unwrap())).unwrap();
    assert_eq!(head, serde_json::json!({ "typ": "JWT", "alg": "ES256" }));
    assert_eq!(claims, serde_json::json!({ "aud": "https://wns2-sg2p.notify.windows.com", "exp": 1_000 + 12 * 3600, "sub": "mailto:me@orrery.dev" }));
    let (input, sig) = jwt.rsplit_once('.').unwrap();
    assert_eq!(d(sig).len(), 64);
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, d(k)).verify(input.as_bytes(), &d(sig)).unwrap();
    const { assert!(JWT_TTL <= 24 * 3600) };
    assert_eq!(audience("https://web.push.apple.com/QGc1"), Some("https://web.push.apple.com".into()));
    assert_eq!(audience("http://web.push.apple.com/x"), None);
}

#[test]
fn the_vapid_key_is_made_once_and_kept_in_the_keychain() {
    let store = MemSecrets::default();
    let a = Vapid::load_or_create(&store).unwrap();
    let b = Vapid::load_or_create(&store).unwrap();
    assert_eq!(a.public_key(), b.public_key());
    assert!(crate::state::lock(&store.0).contains_key(VAPID_SECRET));
    store.1.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(Vapid::load_or_create(&store).is_none());
}

#[test]
fn subjects_are_mailto_or_https_only() {
    assert!(valid_subject("mailto:me@orrery.dev"));
    assert!(valid_subject("https://orrery-relay.me.workers.dev"));
    assert!(!valid_subject("mailto:"));
    assert!(!valid_subject("mailto:@x"));
    assert!(!valid_subject("http://x.dev"));
    assert!(!valid_subject("https://x y"));
    assert!(!valid_subject(&format!("https://{}", "a".repeat(200))));
}
