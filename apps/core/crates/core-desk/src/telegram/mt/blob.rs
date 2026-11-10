use grammers_session::types::DcOption;
use grammers_session::SessionData;
use serde_json::{json, Value};

pub const VERSION: i64 = 1;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

pub fn encode(home: i32, dcs: &[DcOption]) -> String {
    let list: Vec<Value> = dcs
        .iter()
        .filter_map(|d| d.auth_key.map(|k| json!({"id": d.id, "ip4": d.ipv4.to_string(), "ip6": d.ipv6.to_string(), "key": hex(&k)})))
        .collect();
    json!({"v": VERSION, "dc": home, "dcs": list}).to_string()
}

pub fn decode(text: &str) -> Option<SessionData> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v["v"].as_i64() != Some(VERSION) {
        return None;
    }
    let home = i32::try_from(v["dc"].as_i64()?).ok()?;
    let mut data = SessionData::default();
    for d in v["dcs"].as_array()? {
        let id = i32::try_from(d["id"].as_i64()?).ok()?;
        let key: [u8; 256] = unhex(d["key"].as_str()?)?.try_into().ok()?;
        let base = data.dc_options.get(&id).cloned();
        let ipv4 = d["ip4"].as_str().and_then(|s| s.parse().ok()).or(base.as_ref().map(|b| b.ipv4))?;
        let ipv6 = d["ip6"].as_str().and_then(|s| s.parse().ok()).or(base.as_ref().map(|b| b.ipv6))?;
        data.dc_options.insert(id, DcOption { id, ipv4, ipv6, auth_key: Some(key) });
    }
    if !data.dc_options.get(&home).is_some_and(|d| d.auth_key.is_some()) {
        return None;
    }
    data.home_dc = home;
    Some(data)
}
