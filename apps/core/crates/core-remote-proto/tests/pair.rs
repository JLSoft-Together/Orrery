use core_remote_proto::pair::{LanAddr, PairCode, PairInitiator, PairResponder, PairTicket, QrPayload, MAX_QR_LEN};
use core_remote_proto::{Error, StaticKeypair};

const NOW: u64 = 1_800_000_000;

fn qr_for(desktop: &StaticKeypair, code: &PairCode) -> QrPayload {
    QrPayload {
        machine: "machine-a".into(),
        name: "Studio".into(),
        lan: vec![LanAddr::new("192.168.1.20", 4318)],
        relay: Some("https://relay.example.dev".into()),
        ticket: None,
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
    assert_eq!(back.lan, qr.lan);
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

fn b64(json: &serde_json::Value) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(json).unwrap())
}

fn wire(text: &str) -> serde_json::Value {
    use base64::Engine;
    serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(text).unwrap()).unwrap()
}

#[test]
fn v2_carries_every_lan_route_relay_and_ticket_and_stays_small() {
    let desktop = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let ticket = PairTicket::generate().unwrap();
    assert_eq!(ticket.as_str().len(), 22);
    let mut qr = qr_for(&desktop, &code);
    qr.machine = "AbCdEfGhIjKlMnOpQrStUv".into();
    qr.name = "DESKTOP-STUDIO01".into();
    qr.lan = vec![LanAddr::new("192.168.100.200", 4320), LanAddr::new("10.10.10.10", 4320)];
    qr.relay = Some("https://orrery-relay.orrery-abcdef.workers.dev".into());
    qr.ticket = Some(ticket.clone());
    let text = qr.encode().unwrap();
    println!("qr v2 2 lan + relay + pt: {} chars", text.len());
    assert!(text.len() <= 460, "{}", text.len());
    let json = wire(&text);
    assert_eq!(json["v"], 2);
    assert_eq!(json["lan"], serde_json::json!(["192.168.100.200:4320", "10.10.10.10:4320"]));
    assert_eq!(json["pt"], ticket.as_str());
    assert!(json.get("host").is_none() && json.get("port").is_none());
    let back = QrPayload::decode(&text).unwrap();
    assert_eq!(back.lan, qr.lan);
    assert_eq!(back.relay, qr.relay);
    assert_eq!(back.ticket.as_ref().map(PairTicket::as_str), Some(ticket.as_str()));
    assert_eq!(back.ticket.unwrap().hash(), ticket.hash());
    let shown = format!("{qr:?}");
    assert!(!shown.contains(ticket.as_str()));

    qr.lan = vec![LanAddr::new("192.168.1.2", 4320)];
    qr.relay = None;
    qr.ticket = None;
    let lan_only = qr.encode().unwrap();
    println!("qr v2 1 lan only: {} chars", lan_only.len());
    assert!(lan_only.len() <= 300, "{}", lan_only.len());
}

#[test]
fn v1_still_decodes_with_its_host_as_the_only_lan_route() {
    let desktop = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let mut json = wire(&qr_for(&desktop, &code).encode().unwrap());
    let obj = json.as_object_mut().unwrap();
    obj.insert("v".into(), 1.into());
    obj.remove("lan");
    obj.insert("host".into(), "192.168.1.9".into());
    obj.insert("port".into(), 4318.into());
    let v1 = QrPayload::decode(&b64(&json)).unwrap();
    assert_eq!(v1.lan, vec![LanAddr::new("192.168.1.9", 4318)]);
    assert!(v1.ticket.is_none());
    assert_eq!(v1.code.as_bytes(), code.as_bytes());

    json.as_object_mut().unwrap().remove("port");
    assert_eq!(QrPayload::decode(&b64(&json)).unwrap().lan, vec![LanAddr::new("192.168.1.9", 4320)]);
    let obj = json.as_object_mut().unwrap();
    obj.remove("host");
    obj.remove("relay");
    assert!(QrPayload::decode(&b64(&json)).unwrap().lan.is_empty());
}

#[test]
fn malformed_v2_and_mixed_versions_are_refused() {
    let desktop = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let base = wire(&qr_for(&desktop, &code).encode().unwrap());
    let with = |key: &str, value: serde_json::Value| {
        let mut j = base.clone();
        j.as_object_mut().unwrap().insert(key.into(), value);
        QrPayload::decode(&b64(&j))
    };
    assert_eq!(with("v", 3.into()).unwrap_err(), core_remote_proto::Error::BadQr);
    assert!(with("host", "1.2.3.4".into()).is_err());
    assert!(with("lan", serde_json::json!(["1.2.3.4"])).is_err());
    assert!(with("lan", serde_json::json!(["1.2.3.4:0"])).is_err());
    assert!(with("lan", serde_json::json!(["1.2.3.4:70000"])).is_err());
    assert!(with("lan", serde_json::json!([":4320"])).is_err());
    assert!(with("lan", serde_json::json!(["::1:4320"])).is_err());
    assert!(with("lan", serde_json::json!(["a:1", "b:1", "c:1", "d:1", "e:1"])).is_err());
    assert!(with("lan", serde_json::json!(["a:1", "b:1", "c:1", "d:1"])).is_ok());
    assert!(with("pt", "short".into()).is_err());
    assert!(with("pt", "!!!!!!!!!!!!!!!!!!!!!!".into()).is_err());
    assert!(with("pt", "AAAAAAAAAAAAAAAAAAAAAA".into()).is_ok());
    let mut v1 = base.clone();
    v1.as_object_mut().unwrap().insert("v".into(), 1.into());
    assert!(QrPayload::decode(&b64(&v1)).is_err());
    let mut no_relay = qr_for(&desktop, &code);
    no_relay.relay = None;
    no_relay.ticket = Some(PairTicket::generate().unwrap());
    assert_eq!(no_relay.encode().unwrap_err(), core_remote_proto::Error::BadQr);
}

#[test]
fn the_platform_in_msg3_reaches_the_desktop_and_bad_ones_are_refused() {
    let desktop = StaticKeypair::generate().unwrap();
    let phone = StaticKeypair::generate().unwrap();
    let code = PairCode::generate().unwrap();
    let qr = qr_for(&desktop, &code);
    let (init, msg1) = PairInitiator::start_on(&phone, &qr, "dev-1", "Pixel", "android", NOW).unwrap();
    let (resp, msg2) = PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap();
    let (_p, msg3) = init.finish(&msg2).unwrap();
    assert_eq!(resp.finish(&msg3).unwrap().platform.as_deref(), Some("android"));
    let (init, msg1) = PairInitiator::start(&phone, &qr, "dev-1", "Pixel", NOW).unwrap();
    let (resp, msg2) = PairResponder::start(&desktop, &code, "machine-a", &msg1).unwrap();
    let (_p, msg3) = init.finish(&msg2).unwrap();
    assert!(resp.finish(&msg3).unwrap().platform.is_none());
    assert!(PairInitiator::start_on(&phone, &qr, "dev-1", "Pixel", "i os", NOW).is_err());
    assert!(PairInitiator::start_on(&phone, &qr, "dev-1", "Pixel", "", NOW).is_err());
}
