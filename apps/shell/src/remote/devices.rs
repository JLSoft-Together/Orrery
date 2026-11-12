use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use super::config::{write_json, DEVICES_FILE};

pub const EXPIRE_SECS: u64 = 30 * 86_400;
pub const MAX_DEVICES: usize = 32;
const DAY: u64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    pub name: String,
    #[serde(default)]
    pub platform: String,
    pub pubkey: String,
    pub paired_at: u64,
    pub last_seen: u64,
    #[serde(default)]
    pub token_exp: u64,
}

impl Device {
    pub fn new(device_id: &str, name: &str, key: &[u8; 32], now: u64) -> Device {
        Device {
            device_id: device_id.to_owned(),
            name: name.to_owned(),
            platform: String::new(),
            pubkey: URL_SAFE_NO_PAD.encode(key),
            paired_at: now,
            last_seen: now,
            token_exp: 0,
        }
    }

    pub fn expired(&self, now: u64) -> bool {
        now.saturating_sub(self.last_seen) >= EXPIRE_SECS
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    devices: Vec<Device>,
}

#[derive(Debug)]
pub struct Devices {
    path: PathBuf,
    list: Vec<Device>,
}

pub fn valid_platform(p: &str) -> bool {
    !p.is_empty() && p.len() <= 16 && p.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn known_platform(p: &str) -> &'static str {
    match p {
        "android" => "android",
        "ios" => "ios",
        _ => "unknown",
    }
}

impl Devices {
    pub fn load(dir: &Path) -> Devices {
        let path = dir.join(DEVICES_FILE);
        let list = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<File>(&t).ok()).map(|f| f.devices).unwrap_or_default();
        Devices { path, list }
    }

    pub fn save(&self) -> bool {
        write_json(&self.path, &File { devices: self.list.clone() })
    }

    pub fn list(&self) -> &[Device] {
        &self.list
    }

    pub fn find(&self, id: &str) -> Option<&Device> {
        self.list.iter().find(|d| d.device_id == id)
    }

    pub fn authorize(&self, id: &str, key: &[u8; 32], now: u64) -> bool {
        let want = URL_SAFE_NO_PAD.encode(key);
        self.find(id).is_some_and(|d| d.pubkey == want && !d.expired(now))
    }

    pub fn is_full(&self, id: &str) -> bool {
        self.find(id).is_none() && self.list.len() >= MAX_DEVICES
    }

    pub fn add(&mut self, device: Device) -> bool {
        if self.is_full(&device.device_id) {
            return false;
        }
        self.list.retain(|d| d.device_id != device.device_id);
        self.list.push(device);
        self.save()
    }

    pub fn remove(&mut self, ids: &[String]) -> Vec<String> {
        let gone: Vec<String> = self.list.iter().filter(|d| ids.contains(&d.device_id)).map(|d| d.device_id.clone()).collect();
        if !gone.is_empty() {
            self.list.retain(|d| !gone.contains(&d.device_id));
            self.save();
        }
        gone
    }

    pub fn touch(&mut self, id: &str, now: u64) -> Option<(String, bool)> {
        let d = self.list.iter_mut().find(|d| d.device_id == id)?;
        let first_today = d.last_seen / DAY != now / DAY || d.last_seen == d.paired_at;
        d.last_seen = now;
        let name = d.name.clone();
        self.save();
        Some((name, first_today))
    }

    pub fn set_platform(&mut self, id: &str, platform: &str) {
        if !valid_platform(platform) {
            return;
        }
        let platform = known_platform(platform);
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id && d.platform != platform) {
            d.platform = platform.to_owned();
            self.save();
        }
    }

    pub fn set_token_exp(&mut self, id: &str, exp: u64) {
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) {
            d.token_exp = exp;
            self.save();
        }
    }

    pub fn expired(&self, now: u64) -> Vec<String> {
        self.list.iter().filter(|d| d.expired(now)).map(|d| d.device_id.clone()).collect()
    }

    pub fn ids(&self) -> Vec<String> {
        self.list.iter().map(|d| d.device_id.clone()).collect()
    }
}
