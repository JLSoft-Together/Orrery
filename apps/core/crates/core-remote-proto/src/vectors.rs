use serde_json::{json, Value};

use crate::pair::{PairCode, PairInitiator, PairResponder, QrPayload};
use crate::relay::{issue_token, RelayKey};
use crate::session::{accept_with, SessionInitiator};
use crate::{StaticKeypair, KEY_LEN};

const MACHINE: &str = "machine-0001";
const DEVICE: &str = "device-0001";
const DEVICE_NAME: &str = "Pixel 9";
const EXP: u64 = 1_900_000_000;
const NOW: u64 = 1_800_000_000;

fn seq(start: u8) -> [u8; KEY_LEN] {
    std::array::from_fn(|i| start.wrapping_add(i as u8))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn build() -> Value {
    let desktop = StaticKeypair::from_private(&seq(0x01)).unwrap();
    let phone = StaticKeypair::from_private(&seq(0x21)).unwrap();
    let desktop_pair_e = seq(0x41);
    let phone_pair_e = seq(0x61);
    let desktop_session_e = seq(0x81);
    let phone_session_e = seq(0xa1);
    let code_raw: [u8; 16] = std::array::from_fn(|i| 0xc1u8.wrapping_add(i as u8));
    let code = PairCode::from_bytes(&code_raw).unwrap();
    let relay_raw = seq(0xe1);
    let relay = RelayKey::from_bytes(&relay_raw).unwrap();

    let qr = QrPayload {
        machine: MACHINE.into(),
        name: "Studio".into(),
        host: Some("192.168.1.20".into()),
        port: Some(4318),
        relay: None,
        mkey: *desktop.public(),
        code: code.clone(),
        exp: EXP,
    };
    let qr_text = qr.encode().unwrap();
    let psk = code.derive_psk(MACHINE);

    let (init, pair1) = PairInitiator::start_with(&phone, &qr, DEVICE, DEVICE_NAME, NOW, Some(&phone_pair_e)).unwrap();
    let (resp, pair2) = PairResponder::start_with(&desktop, &code, MACHINE, &pair1, Some(&desktop_pair_e)).unwrap();
    let (mut paired_desktop, pair3) = init.finish(&pair2).unwrap();
    let mut paired_device = resp.finish(&pair3).unwrap();
    let pair_transport = paired_desktop.transport.seal(b"hello").unwrap();
    assert_eq!(paired_device.transport.open(&pair_transport).unwrap(), b"hello");

    let (sinit, sess1) = SessionInitiator::start_with(&phone, desktop.public(), DEVICE, Some(&phone_session_e)).unwrap();
    let (mut accepted, sess2) = accept_with(&desktop, &sess1, |_, _| true, Some(&desktop_session_e)).unwrap();
    let mut phone_t = sinit.finish(&sess2).unwrap();
    let sess_transport = phone_t.seal(b"ping").unwrap();
    assert_eq!(accepted.transport.open(&sess_transport).unwrap(), b"ping");

    let token = issue_token(&relay, MACHINE, DEVICE, EXP).unwrap();

    json!({
        "note": "hex unless noted; msg1 of both handshakes = proto u16 BE prefix + noise message; prologue = 'orrery-remote/' + proto u16 BE",
        "proto": crate::PROTO,
        "prologue": hex(&crate::prologue(crate::PROTO)),
        "machineId": MACHINE,
        "deviceId": DEVICE,
        "deviceName": DEVICE_NAME,
        "now": NOW,
        "exp": EXP,
        "desktopStaticPrivate": hex(&seq(0x01)),
        "desktopStaticPublic": hex(desktop.public()),
        "phoneStaticPrivate": hex(&seq(0x21)),
        "phoneStaticPublic": hex(phone.public()),
        "pair": {
            "pattern": "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s",
            "code": hex(&code_raw),
            "psk": hex(psk.as_slice()),
            "phoneEphemeralPrivate": hex(&phone_pair_e),
            "desktopEphemeralPrivate": hex(&desktop_pair_e),
            "qr": qr_text,
            "msg1": hex(&pair1),
            "msg2": hex(&pair2),
            "msg3": hex(&pair3),
            "msg3Payload": format!("{{\"deviceId\":\"{DEVICE}\",\"name\":\"{DEVICE_NAME}\"}}"),
            "transportPhoneToDesktop": hex(&pair_transport),
            "transportPlaintext": "hello"
        },
        "session": {
            "pattern": "Noise_IK_25519_ChaChaPoly_BLAKE2s",
            "phoneEphemeralPrivate": hex(&phone_session_e),
            "desktopEphemeralPrivate": hex(&desktop_session_e),
            "msg1": hex(&sess1),
            "msg1Payload": format!("{{\"deviceId\":\"{DEVICE}\",\"proto\":{}}}", crate::PROTO),
            "msg2": hex(&sess2),
            "msg2Payload": format!("{{\"proto\":{}}}", crate::PROTO),
            "transportPhoneToDesktop": hex(&sess_transport),
            "transportPlaintext": "ping"
        },
        "relayToken": {
            "relayKey": hex(&relay_raw),
            "macInput": "'orrery-relay/1' || machineId || deviceId || exp u64 BE",
            "token": token
        }
    })
}

#[test]
fn vectors_match_file() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors.json");
    let fresh = build();
    assert_eq!(build(), fresh);
    if std::env::var_os("CRP_WRITE_VECTORS").is_some() {
        let text = serde_json::to_string_pretty(&fresh).unwrap();
        std::fs::write(&path, text + "\n").unwrap();
        return;
    }
    let text = std::fs::read_to_string(&path).expect("tests/vectors.json missing, run with CRP_WRITE_VECTORS=1");
    let stored: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(stored, fresh);
}
