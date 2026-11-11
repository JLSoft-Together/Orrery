use rust_lib_orrery_remote::api::remote::{decode_qr, static_public_key};
use serde_json::Value;

fn vectors() -> Value {
    let text = include_str!("../../../core/crates/core-remote-proto/tests/vectors.json");
    serde_json::from_str(text).unwrap()
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

#[test]
fn qr_vector_decodes_to_vector_machine_and_key() {
    let v = vectors();
    let info = decode_qr(v["pair"]["qr"].as_str().unwrap().to_owned()).unwrap();
    assert_eq!(info.machine, v["machineId"].as_str().unwrap());
    assert_eq!(info.mkey, hex(v["desktopStaticPublic"].as_str().unwrap()));
    assert_eq!(info.exp, v["exp"].as_u64().unwrap());
}

#[test]
fn static_key_vector_derives_vector_public_key() {
    let v = vectors();
    let public = static_public_key(hex(v["desktopStaticPrivate"].as_str().unwrap())).unwrap();
    assert_eq!(public, hex(v["desktopStaticPublic"].as_str().unwrap()));
}
