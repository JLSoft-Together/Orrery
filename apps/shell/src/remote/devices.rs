use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use super::config::{write_json, DEVICES_FILE};
use super::pair::is_hidden;

pub const EXPIRE_SECS: u64 = 30 * 86_400;
pub const MAX_DEVICES: usize = 32;
pub const MAX_ALIAS: usize = 40;
pub const MAX_COUNTRIES: usize = 8;
const DAY: u64 = 86_400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alias: String,
    #[serde(default)]
    pub platform: String,
    pub pubkey: String,
    pub paired_at: u64,
    pub last_seen: u64,
    #[serde(default)]
    pub token_exp: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub push_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub push_locale: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub push_mute: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub countries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_push: Option<WebSub>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub push_content: bool,
    #[serde(default, skip_serializing_if = "is_no")]
    pub push_off: bool,
    #[serde(default, skip_serializing_if = "is_no")]
    pub push_seal: bool,
    #[serde(default, skip_serializing_if = "is_no")]
    pub push_seal_ok: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub push_seal_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_build: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub paired_via: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub adb_serial_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSub {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

fn yes() -> bool {
    true
}

fn is_yes(v: &bool) -> bool {
    *v
}

fn is_no(v: &bool) -> bool {
    !*v
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

impl Device {
    pub fn new(device_id: &str, name: &str, key: &[u8; 32], now: u64) -> Device {
        Device {
            device_id: device_id.to_owned(),
            name: name.to_owned(),
            alias: String::new(),
            platform: String::new(),
            pubkey: URL_SAFE_NO_PAD.encode(key),
            paired_at: now,
            last_seen: now,
            token_exp: 0,
            push_token: String::new(),
            push_locale: String::new(),
            push_mute: Vec::new(),
            countries: Vec::new(),
            web_push: None,
            push_content: true,
            push_off: false,
            push_seal: false,
            push_seal_ok: false,
            push_seal_at: 0,
            app_version: None,
            app_build: None,
            model: None,
            os: None,
            paired_via: String::new(),
            adb_serial_hash: String::new(),
        }
    }

    pub fn push_channel(&self) -> &'static str {
        if self.web_push.is_some() {
            "web"
        } else if !self.push_token.is_empty() {
            "fcm"
        } else {
            "none"
        }
    }

    pub fn display_name(&self) -> &str {
        if self.alias.is_empty() {
            &self.name
        } else {
            &self.alias
        }
    }

    pub fn expired(&self, now: u64) -> bool {
        now.saturating_sub(self.last_seen) >= EXPIRE_SECS
    }

    pub fn wants(&self, kind: &str) -> bool {
        !self.push_mute.iter().any(|m| m == push_group(kind))
    }
}

pub const PUSH_GROUPS: [&str; 3] = ["waiting", "done", "error"];

pub fn push_group(kind: &str) -> &'static str {
    match kind {
        "done" => "done",
        "error" => "error",
        _ => "waiting",
    }
}

pub fn clean_mute<'a>(raw: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let raw: Vec<&str> = raw.into_iter().take(16).collect();
    PUSH_GROUPS.iter().filter(|g| raw.contains(g)).map(|g| (*g).to_owned()).collect()
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    devices: Vec<Device>,
}

#[derive(Debug)]
pub struct Devices {
    path: PathBuf,
    pub(super) list: Vec<Device>,
}

pub fn valid_platform(p: &str) -> bool {
    !p.is_empty() && p.len() <= 16 && p.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn clean_alias(raw: &str) -> String {
    let kept: String = raw.chars().filter(|c| !c.is_control() && !is_hidden(*c)).collect();
    kept.trim().chars().take(MAX_ALIAS).collect::<String>().trim().to_owned()
}

pub fn known_platform(p: &str) -> &'static str {
    match p {
        "android" => "android",
        "web" | "ios" => "web",
        _ => "unknown",
    }
}

impl Devices {
    pub fn load(dir: &Path) -> Devices {
        let path = dir.join(DEVICES_FILE);
        let list = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<File>(&t).ok()).map(|f| f.devices).unwrap_or_default();
        let mut list = list;
        for d in list.iter_mut().filter(|d| d.platform == "ios") {
            d.platform = "web".to_owned();
        }
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
        let name = d.display_name().to_owned();
        self.save();
        Some((name, first_today))
    }

    pub fn rename(&mut self, id: &str, raw: &str) -> Option<String> {
        let alias = clean_alias(raw);
        let d = self.list.iter_mut().find(|d| d.device_id == id)?;
        if d.alias != alias {
            d.alias = alias;
            self.save();
        }
        self.find(id).map(|d| d.display_name().to_owned())
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

    pub(super) fn update(&mut self, id: &str, change: impl FnOnce(&mut Device) -> bool) -> bool {
        self.list.iter_mut().find(|d| d.device_id == id).is_some_and(change)
    }

    pub fn set_token_exp(&mut self, id: &str, exp: u64) {
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) {
            d.token_exp = exp;
            self.save();
        }
    }

    pub fn note_place(&mut self, id: &str, country: Option<&str>) -> bool {
        let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) else { return false };
        let Some(c) = country.filter(|c| !d.countries.iter().any(|k| k == c)) else { return false };
        let fresh = !d.countries.is_empty();
        d.countries.push(c.to_owned());
        if d.countries.len() > MAX_COUNTRIES {
            d.countries.remove(0);
        }
        self.save();
        fresh
    }

    pub fn expired(&self, now: u64) -> Vec<String> {
        self.list.iter().filter(|d| d.expired(now)).map(|d| d.device_id.clone()).collect()
    }

    pub fn ids(&self) -> Vec<String> {
        self.list.iter().map(|d| d.device_id.clone()).collect()
    }
}
