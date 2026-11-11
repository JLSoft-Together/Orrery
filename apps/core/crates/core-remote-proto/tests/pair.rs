use core_remote_proto::pair::{PairCode, PairInitiator, PairResponder, QrPayload, MAX_QR_LEN};
use core_remote_proto::{Error, StaticKeypair};

const NOW: u64 = 1_800_000_000;

fn qr_for(desktop: &StaticKeypair, code: &PairCode) -> QrPayload {
    QrPayload {
        machine: "machine-a".into(),
        name: "Studio".into(),
        host: Some("192.168.1.20".into()),
        port: Some(4318),
        relay: Some("https://relay.example.dev".into()),
        mkey: *desktop.public(),
        code: code.clone(),
        exp: NOW + 300,
    }
}

#[test]
fn full_pairing_gives_both_sides_keys_and_device_id() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = QrPayload::decode(&qr_for(&desktop, &code).encode().unwrap()).unwrap();

    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (resp, msg2) = PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap();
    let (mut on_phone, msg3) = init.finish(&msg2).unwrap();
    let mut on_desktop = resp.finish(&msg3).unwrap();

    assert_eq!(on_phone.desktop_static, *desktop.public());
    assert_eq!(on_phone.machine_id, "machine-a");
    assert_eq!(on_desktop.device_static, *phone.public());
    assert_eq!(on_desktop.device_id, "dev-1");
    assert_eq!(on_desktop.name, "Pixel");

    let sealed = on_desktop.transport.seal(b"welcome").unwrap();
    assert_eq!(on_phone.transport.open(&sealed).unwrap(), b"welcome");
    let back = on_phone.transport.seal(b"thanks").unwrap();
    assert_eq!(on_desktop.transport.open(&back).unwrap(), b"thanks");
}

#[test]
fn fake_desktop_stops_phone_at_msg2() {
    let real = StaticKeypair::generate().unwrap();
    let fake = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&real, &code);

    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (_resp, msg2) = PairResponder::start(&fake, &code, "machine-a", &msg1).unwrap();
    assert_eq!(init.finish(&msg2).unwrap_err(), Error::KeyMismatch);
}

#[test]
fn wrong_code_fails_the_handshake() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let other = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &other);

    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (resp, msg2) = PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap();
    let (_phone_side, msg3) = init.finish(&msg2).unwrap();
    assert_eq!(resp.finish(&msg3).unwrap_err(), Error::Noise);
}

#[test]
fn wrong_machine_id_fails_the_handshake() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &code);

    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (resp, msg2) = PairResponder::start(&desktop, &code, "machine-b", &msg1).unwrap();
    let (_phone_side, msg3) = init.finish(&msg2).unwrap();
    assert!(resp.finish(&msg3).is_err());
}

#[test]
fn expired_qr_is_refused() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &code);
    let err = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", qr.exp).unwrap_err();
    assert_eq!(err, Error::Expired);
}

#[test]
fn tampered_messages_fail() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &code);

    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (resp, mut msg2) = PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap();
    let last = msg2.len() - 1;
    msg2[last] ^= 1;
    assert!(init.finish(&msg2).is_err());
    drop(resp);

    let (_init, mut msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    msg1[0] = 0;
    msg1[1] = 0;
    assert_eq!(
        PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap_err(),
        Error::ProtoTooOld { min: 1 }
    );
}

#[test]
fn qr_round_trip_and_limits() {
    let desktop = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &code);
    let text = qr.encode().unwrap();
    assert!(text.len() <= MAX_QR_LEN);
    let back = QrPayload::decode(&text).unwrap();
    assert_eq!(back.machine, qr.machine);
    assert_eq!(back.host, qr.host);
    assert_eq!(back.port, qr.port);
    assert_eq!(back.relay, qr.relay);
    assert_eq!(back.mkey, qr.mkey);
    assert_eq!(back.code.as_bytes(), qr.code.as_bytes());
    assert_eq!(back.exp, qr.exp);
    assert!(!format!("{back:?}").contains("code"));

    assert_eq!(QrPayload::decode("").unwrap_err(), Error::BadQr);
    assert_eq!(QrPayload::decode("!!!").unwrap_err(), Error::BadQr);
    assert_eq!(QrPayload::decode(&"A".repeat(MAX_QR_LEN + 1)).unwrap_err(), Error::BadQr);
    let mut bad = qr.clone();
    bad.machine = String::new();
    assert_eq!(bad.encode().unwrap_err(), Error::BadQr);
}

#[test]
fn psk_depends_on_code_and_machine() {
    let code = PairCode::from_bytes(&[7u8; 16]).unwrap();
    let a = code.derive_psk("machine-a");
    let b = code.derive_psk("machine-b");
    assert_ne!(a.as_slice(), b.as_slice());
    assert_eq!(a.as_slice(), code.derive_psk("machine-a").as_slice());
    assert!(PairCode::from_bytes(&[0u8; 15]).is_err());
}
