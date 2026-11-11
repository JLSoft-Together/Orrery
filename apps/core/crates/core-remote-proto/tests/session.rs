use core_remote_proto::session::{accept, SessionInitiator};
use core_remote_proto::{Error, StaticKeypair, MIN_PROTO, PROTO};

#[test]
fn paired_device_gets_in() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let phone_pub = *phone.public();

    let (init, msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
    let (mut accepted, msg2) = accept(&desktop, &msg1, |id, key| id == "dev-1" && *key == phone_pub).unwrap();
    assert_eq!(accepted.device_id, "dev-1");
    assert_eq!(accepted.device_static, phone_pub);
    assert_eq!(accepted.proto, PROTO);

    let mut on_phone = init.finish(&msg2).unwrap();
    assert_eq!(on_phone.remote_static(), Some(*desktop.public()));
    let sealed = on_phone.seal(b"hi").unwrap();
    assert_eq!(accepted.transport.open(&sealed).unwrap(), b"hi");
    let back = accepted.transport.seal(b"yo").unwrap();
    assert_eq!(on_phone.open(&back).unwrap(), b"yo");
}

#[test]
fn unknown_device_is_refused() {
    let desktop = StaticKeypair::generate().unwrap();
    let stranger = StaticKeypair::generate().unwrap();
    let paired = StaticKeypair::generate().unwrap();
    let paired_pub = *paired.public();

    let (_init, msg1) = SessionInitiator::start(&stranger, desktop.public(), "dev-1").unwrap();
    let err = accept(&desktop, &msg1, |_, key| *key == paired_pub).unwrap_err();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn wrong_desktop_key_fails() {
    let desktop = StaticKeypair::generate().unwrap();
    let other = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let (_init, msg1) = SessionInitiator::start(&phone, other.public(), "dev-1").unwrap();
    let mut called = false;
    let err = accept(&desktop, &msg1, |_, _| {
        called = true;
        true
    })
    .unwrap_err();
    assert_eq!(err, Error::Noise);
    assert!(!called);
}

#[test]
fn old_protocol_is_refused() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let (_init, mut msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
    let old = MIN_PROTO - 1;
    msg1[..2].copy_from_slice(&old.to_be_bytes());
    let err = accept(&desktop, &msg1, |_, _| true).unwrap_err();
    assert_eq!(err, Error::ProtoTooOld { min: MIN_PROTO });

    let newer = PROTO + 1;
    msg1[..2].copy_from_slice(&newer.to_be_bytes());
    let err = accept(&desktop, &msg1, |_, _| true).unwrap_err();
    assert_eq!(err, Error::ProtoTooNew { max: PROTO });
}

#[test]
fn tampered_msg1_and_msg2_fail() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();

    let (_init, mut msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
    let last = msg1.len() - 1;
    msg1[last] ^= 0x80;
    assert!(accept(&desktop, &msg1, |_, _| true).is_err());

    let (init, msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
    let (_acc, mut msg2) = accept(&desktop, &msg1, |_, _| true).unwrap();
    msg2[3] ^= 1;
    assert!(init.finish(&msg2).is_err());

    assert_eq!(accept(&desktop, &[], |_, _| true).unwrap_err(), Error::BadMessage);
    assert_eq!(accept(&desktop, &[0, 1], |_, _| true).unwrap_err(), Error::Noise);
}

#[test]
fn bad_device_id_is_refused_before_sending() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    assert_eq!(
        SessionInitiator::start(&phone, desktop.public(), "").unwrap_err(),
        Error::BadPayload
    );
    assert_eq!(
        SessionInitiator::start(&phone, desktop.public(), &"x".repeat(129)).unwrap_err(),
        Error::BadPayload
    );
}

#[test]
fn keypair_export_import_round_trip() {
    let a = StaticKeypair::generate().unwrap();
    let b = StaticKeypair::from_private(a.export_private().as_slice()).unwrap();
    assert_eq!(a.public(), b.public());
    assert_eq!(StaticKeypair::from_private(&[1u8; 31]).unwrap_err(), Error::BadKey);
    assert_eq!(format!("{a:?}"), "StaticKeypair(..)");
}
