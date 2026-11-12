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

    let open = Control::new(ControlKind::Open, 9);
    let bytes = encode_control(&open).unwrap();
    assert_eq!(&bytes[2..], br#"{"t":"open","conn":9}"#);
    assert_eq!(decode(&bytes).unwrap(), Envelope::Control(open));
    assert_eq!(
        decode(b"\0\0{\"t\":\"close\",\"conn\":2}").unwrap(),
        Envelope::Control(Control::new(ControlKind::Close, 2))
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
    let control = Control::new(ControlKind::Open, 0);
    assert_eq!(encode_control(&control).unwrap_err(), Error::BadMessage);
}

#[test]
fn close_control_carries_the_code_and_still_parses_as_a_close() {
    let bytes = core_remote_proto::relay::encode_close(7, 4401).unwrap();
    assert_eq!(&bytes[..2], &[0, 0]);
    let value: serde_json::Value = serde_json::from_slice(&bytes[2..]).unwrap();
    assert_eq!(value, serde_json::json!({ "t": "close", "conn": 7, "code": 4401 }));
    assert_eq!(decode(&bytes).unwrap(), Envelope::Control(Control::new(ControlKind::Close, 7)));
    assert_eq!(core_remote_proto::relay::encode_close(0, 4401).unwrap_err(), Error::BadMessage);
}

#[test]
fn open_control_carries_ip_country_and_pairing_and_old_shape_still_parses() {
    let bytes = b"\0\0{\"t\":\"open\",\"conn\":3,\"ip\":\"203.0.113.9\",\"country\":\"VN\",\"pairing\":true}";
    let Envelope::Control(c) = decode(bytes).unwrap() else { panic!("control") };
    assert_eq!(c.t, ControlKind::Open);
    assert_eq!(c.conn, 3);
    assert_eq!(c.ip.as_deref(), Some("203.0.113.9"));
    assert_eq!(c.country.as_deref(), Some("VN"));
    assert!(c.pairing);
    let Envelope::Control(plain) = decode(b"\0\0{\"t\":\"open\",\"conn\":4,\"extra\":1}").unwrap() else { panic!("control") };
    assert_eq!(plain, Control::new(ControlKind::Open, 4));
}

#[test]
fn pair_open_and_close_controls_go_on_conn_zero() {
    let ticket = core_remote_proto::pair::PairTicket::parse("AAAAAAAAAAAAAAAAAAAAAA").unwrap();
    let open = core_remote_proto::relay::encode_pair_open(&ticket.hash(), 120).unwrap();
    assert_eq!(&open[..2], &[0, 0]);
    let value: serde_json::Value = serde_json::from_slice(&open[2..]).unwrap();
    assert_eq!(value, serde_json::json!({ "t": "pair.open", "h": ticket.hash_text(), "ttl": 120 }));
    assert!(core_remote_proto::relay::encode_pair_open(&ticket.hash(), 0).is_err());
    assert!(core_remote_proto::relay::encode_pair_open(&ticket.hash(), 301).is_err());
    assert_eq!(ticket.hash_text().len(), 43);
    let close = core_remote_proto::relay::encode_pair_close().unwrap();
    assert_eq!(&close[2..], br#"{"t":"pair.close"}"#);
    assert!(decode(&open).is_err());
}

#[test]
fn guard_controls_go_on_conn_zero_and_check_their_fields() {
    use core_remote_proto::relay::{encode_block_list, encode_block_remove, encode_block_set, encode_strike, encode_trust_set, BlockRule, MAX_GUARD_RULES};
    let json = |b: &[u8]| -> serde_json::Value { serde_json::from_slice(&b[2..]).unwrap() };
    let strike = encode_strike(7, "handshake").unwrap();
    assert_eq!(&strike[..2], &[0, 0]);
    assert_eq!(json(&strike), serde_json::json!({ "t": "strike", "conn": 7, "why": "handshake" }));
    assert!(encode_strike(0, "handshake").is_err());
    assert_eq!(json(&core_remote_proto::relay::encode_good(9).unwrap()), serde_json::json!({ "t": "good", "conn": 9 }));
    assert!(core_remote_proto::relay::encode_good(0).is_err());
    assert!(encode_strike(7, "203.0.113.9").is_err());
    assert_eq!(json(&encode_block_list().unwrap()), serde_json::json!({ "t": "block.list" }));
    assert_eq!(json(&encode_block_remove(Some("203.0.113.9")).unwrap()), serde_json::json!({ "t": "block.remove", "c": "203.0.113.9" }));
    assert_eq!(json(&encode_block_remove(None).unwrap()), serde_json::json!({ "t": "block.remove", "all": true }));
    assert!(encode_block_remove(Some("x\"}")).is_err());
    let rules = vec![BlockRule { cidr: "198.51.100.0/24".into(), ttl: Some(3600) }, BlockRule { cidr: "2001:db8::/32".into(), ttl: None }];
    assert_eq!(
        json(&encode_block_set(&rules).unwrap()),
        serde_json::json!({ "t": "block.set", "list": [{ "c": "198.51.100.0/24", "ttl": 3600 }, { "c": "2001:db8::/32" }] })
    );
    assert!(encode_block_set(&[BlockRule { cidr: "1.2.3.4".into(), ttl: Some(0) }]).is_err());
    let full: Vec<String> = (0..MAX_GUARD_RULES).map(|i| format!("ffff:ffff:ffff:ffff:ffff:ffff:ffff:{i:04x}/128")).collect();
    let trust = encode_trust_set(&full).unwrap();
    assert!(trust.len() > 256 && trust.len() <= 2 + 4096);
    let mut over = full.clone();
    over.push("10.0.0.1".into());
    assert!(encode_trust_set(&over).is_err());
    assert!(encode_trust_set(&["10.0.0.0/8 ".into()]).is_err());
    assert!(decode(&strike).is_err());
}

#[test]
fn relay_notices_decode_rows_and_drop_bad_ones() {
    use core_remote_proto::relay::{decode_notice, BlockRow, Notice};
    let row = BlockRow { k: "203.0.113.9".into(), until: 1_900_000_000, level: 2, n: 20, at: 1_899_990_000, why: "code".into() };
    let blocked = [&[0u8, 0][..], br#"{"t":"blocked","row":{"k":"203.0.113.9","until":1900000000,"level":2,"n":20,"at":1899990000,"why":"code"}}"#].concat();
    assert_eq!(decode_notice(&blocked), Some(Notice::Blocked(row.clone())));
    let list = [
        &[0u8, 0][..],
        br#"{"t":"block.list","total":3,"items":[{"k":"203.0.113.9","until":1900000000,"level":2,"n":20,"at":1899990000,"why":"code"},{"k":"bad<ip>","until":1,"level":1,"n":1,"at":1,"why":"code"},{"k":"10.0.0.1","until":1,"level":9,"n":1,"at":1,"why":"code"}]}"#,
    ]
    .concat();
    assert_eq!(decode_notice(&list), Some(Notice::BlockList { items: vec![row], total: 3 }));
    assert_eq!(decode_notice(br#"\0\0{"t":"open","conn":1}"#), None);
    assert_eq!(decode_notice(&[&[0u8, 1][..], br#"{"t":"blocked"}"#].concat()), None);
    assert_eq!(decode_notice(&[&[0u8, 0][..], br#"{"t":"blocked","row":{"k":"1.2.3.4"}}"#].concat()), None);
    assert!(decode(&blocked).is_err());
}
