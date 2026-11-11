use core_remote_proto::session::{accept, SessionInitiator};
use core_remote_proto::{Error, StaticKeypair, Transport, MAX_PLAINTEXT};

fn pair_up() -> (Transport, Transport) {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let (init, msg1) = SessionInitiator::start(&phone, desktop.public(), "dev-1").unwrap();
    let (accepted, msg2) = accept(&desktop, &msg1, |_, _| true).unwrap();
    (init.finish(&msg2).unwrap(), accepted.transport)
}

#[test]
fn replayed_frame_fails() {
    let (mut phone, mut desktop) = pair_up();
    let a = phone.seal(b"one").unwrap();
    assert_eq!(desktop.open(&a).unwrap(), b"one");
    assert_eq!(desktop.open(&a).unwrap_err(), Error::Noise);
    assert!(desktop.is_closed());
    let b = phone.seal(b"two").unwrap();
    assert_eq!(desktop.open(&b).unwrap_err(), Error::Closed);
}

#[test]
fn reordered_frames_fail() {
    let (mut phone, mut desktop) = pair_up();
    let a = phone.seal(b"one").unwrap();
    let b = phone.seal(b"two").unwrap();
    assert_eq!(desktop.open(&b).unwrap_err(), Error::Noise);
    assert_eq!(desktop.open(&a).unwrap_err(), Error::Closed);
}

#[test]
fn flipped_byte_fails_anywhere() {
    for i in 0..(4 + 16) {
        let (mut phone, mut desktop) = pair_up();
        let mut a = phone.seal(b"data").unwrap();
        a[i] ^= 0x01;
        assert_eq!(desktop.open(&a).unwrap_err(), Error::Noise);
    }
}

#[test]
fn size_limits() {
    let (mut phone, mut desktop) = pair_up();
    assert_eq!(phone.seal(&vec![0u8; MAX_PLAINTEXT + 1]).unwrap_err(), Error::TooLarge);
    let big = phone.seal(&vec![7u8; MAX_PLAINTEXT]).unwrap();
    assert_eq!(big.len(), 65_535);
    assert_eq!(desktop.open(&big).unwrap().len(), MAX_PLAINTEXT);
    assert_eq!(desktop.open(&[0u8; 3]).unwrap_err(), Error::BadMessage);
}

#[test]
fn reflected_frame_fails() {
    let (mut phone, _desktop) = pair_up();
    let a = phone.seal(b"echo").unwrap();
    assert_eq!(phone.open(&a).unwrap_err(), Error::Noise);
}
