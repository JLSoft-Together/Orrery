use std::net::{IpAddr, Ipv4Addr};

use serde_json::Value;

use super::devices::Devices;
use super::guard_ip::norm;

pub const MAX_EXTRA_LAN: usize = 2;
pub const ADB_HASH_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Via {
    Usb,
    #[default]
    Qr,
    Lan,
}

impl Via {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Usb => "usb",
            Self::Qr => "qr",
            Self::Lan => "lan",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PairMeta {
    pub via: Via,
    pub ip: Option<IpAddr>,
    pub adb: String,
    pub extra: Vec<Ipv4Addr>,
}

pub fn adb_hash(raw: &str) -> Option<String> {
    (raw.len() == ADB_HASH_LEN && raw.bytes().all(|b| b.is_ascii_hexdigit())).then(|| raw.to_ascii_lowercase())
}

fn extra_lan(arg: &Value) -> Vec<Ipv4Addr> {
    let mut out: Vec<Ipv4Addr> = Vec::new();
    let list = arg.get("extraLan").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    for ip in list.iter().take(8).filter_map(Value::as_str).filter_map(|s| s.parse::<Ipv4Addr>().ok()).filter(Ipv4Addr::is_loopback) {
        if out.len() < MAX_EXTRA_LAN && !out.contains(&ip) {
            out.push(ip);
        }
    }
    out
}

impl PairMeta {
    pub fn from_web(arg: &Value) -> PairMeta {
        if arg.get("via").and_then(Value::as_str) != Some("usb") {
            return PairMeta::default();
        }
        let adb = arg.get("adbSerialHash").and_then(Value::as_str).and_then(adb_hash).unwrap_or_default();
        PairMeta { via: Via::Usb, ip: None, adb, extra: extra_lan(arg) }
    }

    pub fn lan(ip: IpAddr) -> PairMeta {
        PairMeta { via: Via::Lan, ip: Some(norm(ip)), adb: String::new(), extra: Vec::new() }
    }

    pub fn admits(&self, peer: Option<IpAddr>) -> bool {
        self.ip.is_none_or(|want| peer.map(norm) == Some(want))
    }

    pub fn wants_relay(&self) -> bool {
        self.via != Via::Lan
    }
}

impl Devices {
    pub fn set_paired(&mut self, id: &str, meta: &PairMeta) {
        let changed = self.update(id, |d| {
            d.paired_via = meta.via.name().to_owned();
            d.adb_serial_hash = meta.adb.clone();
            true
        });
        if changed {
            self.save();
        }
    }
}
