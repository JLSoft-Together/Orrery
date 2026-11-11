use core_remote_proto::relay::{
    decode, encode_control, encode_data, issue_token, verify_token, Control, ControlKind, Envelope, RelayKey,
};
use core_remote_proto::Error;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

const NOW: u64 = 1_800_000_000;

#[test]
fn valid_token_passes() {
    let key = RelayKey::generate().unwrap();
    let token = issue_token(&key, "machine-a", "dev-1", NOW + 60).unwrap();
    assert_eq!(verify_token(&key, "machine-a", &token, NOW).unwrap(), "dev-1");
}

#[test]
fn expired_token_fails() {
    let key = RelayKey::generate().unwrap();
    let token = issue_token(&key, "machine-a", "dev-1", NOW).unwrap();
    assert_eq!(verify_token(&key, "machine-a", &token, NOW).unwrap_err(), Error::Expired);
    assert_eq!(verify_token(&key, "machine-a", &token, NOW + 1).unwrap_err(), Error::Expired);
}

#[test]
fn tampered_token_fails() {
    let key = RelayKey::generate().unwrap();
    let token = issue_token(&key, "machine-a", "dev-1", NOW + 60).unwrap();
    let raw = URL_SAFE_NO_PAD.decode(&token).unwrap();
    for i in 0..raw.len() {
        let mut bad = raw.clone();
        bad[i] ^= 0x01;
        let text = URL_SAFE_NO_PAD.encode(&bad);
        assert!(verify_token(&key, "machine-a", &text, NOW).is_err(), "byte {i}");
    }
    let mut extended = raw.clone();
    let exp_at = raw.len() - 40;
    extended[exp_at..exp_at + 8].copy_from_slice(&u64::MAX.to_be_bytes());
    let text = URL_SAFE_NO_PAD.encode(&extended);
    assert_eq!(verify_token(&key, "machine-a", &text, NOW).unwrap_err(), Error::BadToken);
}

#[test]
fn wrong_machine_or_key_fails() {
    let key = RelayKey::generate().unwrap();
    let other = RelayKey::generate().unwrap();
    let token = issue_token(&key, "machine-a", "dev-1", NOW + 60).unwrap();
    assert_eq!(verify_token(&key, "machine-b", &token, NOW).unwrap_err(), Error::BadToken);
    assert_eq!(verify_token(&other, "machine-a", &token, NOW).unwrap_err(), Error::BadToken);
}

#[test]
fn junk_tokens_fail() {
    let key = RelayKey::from_bytes(&[3u8; 32]).unwrap();
    for junk in ["", "a", "!!!!", "AAAA", &"A".repeat(600), &"A".repeat(60)] {
        assert!(verify_token(&key, "machine-a", junk, NOW).is_err());
    }
    assert_eq!(issue_token(&key, "", "dev", NOW).unwrap_err(), Error::BadToken);
    assert_eq!(issue_token(&key, "m", "", NOW).unwrap_err(), Error::BadToken);
    assert!(RelayKey::from_bytes(&[0u8; 31]).is_err());
    assert_eq!(format!("{key:?}"), "RelayKey(..)");
}

#[test]
fn envelope_round_trip() {
    let data = encode_data(5, b"noise").unwrap();
    assert_eq!(&data[..2], &[0, 5]);
    assert_eq!(decode(&data).unwrap(), Envelope::Data { conn: 5, bytes: b"noise" });

    let open = Control {
        t: ControlKind::Open,
        conn: 9,
    };
    let bytes = encode_control(&open).unwrap();
    assert_eq!(&bytes[2..], br#"{"t":"open","conn":9}"#);
    assert_eq!(decode(&bytes).unwrap(), Envelope::Control(open));
    assert_eq!(
        decode(b"\0\0{\"t\":\"close\",\"conn\":2}").unwrap(),
        Envelope::Control(Control {
            t: ControlKind::Close,
            conn: 2
        })
    );
}

#[test]
fn bad_envelopes_fail() {
    assert_eq!(encode_data(0, b"x").unwrap_err(), Error::BadMessage);
    assert_eq!(decode(&[1]).unwrap_err(), Error::BadMessage);
    assert_eq!(decode(b"\0\0{\"t\":\"boom\",\"conn\":2}").unwrap_err(), Error::BadMessage);
    assert_eq!(decode(b"\0\0{\"t\":\"open\",\"conn\":0}").unwrap_err(), Error::BadMessage);
    assert_eq!(decode(b"\0\0not json").unwrap_err(), Error::BadMessage);
    assert_eq!(decode(&vec![0u8; 2 + 65_536]).unwrap_err(), Error::TooLarge);
    let control = Control {
        t: ControlKind::Open,
        conn: 0,
    };
    assert_eq!(encode_control(&control).unwrap_err(), Error::BadMessage);
}

#[test]
fn close_control_carries_the_code_and_still_parses_as_a_close() {
    let bytes = core_remote_proto::relay::encode_close(7, 4401).unwrap();
    assert_eq!(&bytes[..2], &[0, 0]);
    let value: serde_json::Value = serde_json::from_slice(&bytes[2..]).unwrap();
    assert_eq!(value, serde_json::json!({ "t": "close", "conn": 7, "code": 4401 }));
    assert_eq!(decode(&bytes).unwrap(), Envelope::Control(Control { t: ControlKind::Close, conn: 7 }));
    assert_eq!(core_remote_proto::relay::encode_close(0, 4401).unwrap_err(), Error::BadMessage);
}
