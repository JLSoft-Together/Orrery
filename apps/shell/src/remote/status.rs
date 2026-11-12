use serde_json::{json, Value};

use super::config::Config;
use super::devices::Device;

pub struct Shape<'a> {
    pub running: bool,
    pub port: u16,
    pub addresses: Vec<String>,
    pub relay: &'static str,
    pub pairing: Option<u64>,
    pub name: &'a str,
}

pub fn relay_kind(state: &str) -> &'static str {
    match state {
        "off" => "off",
        "online" => "online",
        "registering" | "connecting" => "connecting",
        _ => "error",
    }
}

pub fn device_json(d: &Device, via: Option<&str>) -> Value {
    let mut v = json!({
        "deviceId": d.device_id,
        "name": d.display_name(),
        "originalName": d.name,
        "alias": d.alias,
        "platform": super::devices::known_platform(&d.platform),
        "pairedAt": d.paired_at,
        "lastSeen": d.last_seen,
        "state": if via.is_some() { "connected" } else { "idle" },
    });
    if let Some(via) = via {
        v["via"] = Value::from(via);
    }
    v
}

impl Shape<'_> {
    pub fn json(&self, cfg: &Config, devices: &[Device], online: &[(String, &'static str)]) -> Value {
        let devices: Vec<Value> = devices.iter().map(|d| device_json(d, online.iter().find(|(id, _)| *id == d.device_id).map(|(_, via)| *via))).collect();
        json!({
            "enabled": cfg.enabled,
            "listening": self.running && !self.addresses.is_empty(),
            "running": self.running,
            "addresses": self.addresses,
            "port": self.port,
            "relay": if self.running { relay_kind(self.relay) } else { "off" },
            "relayDetail": self.relay,
            "relayUrl": cfg.relay.url,
            "relayRegistered": cfg.relay.registered,
            "keepAwake": cfg.keep_awake,
            "machine": cfg.machine_id,
            "name": self.name,
            "pairing": self.pairing.map(|exp| json!({ "exp": exp })),
            "devices": devices,
        })
    }
}
