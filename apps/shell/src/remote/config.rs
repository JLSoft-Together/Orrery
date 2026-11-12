use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 4320;
pub const CONFIG_FILE: &str = "config.json";
pub const DEVICES_FILE: &str = "devices.json";
const MACHINE_ID_LEN: usize = 16;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayConfig {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub registered: bool,
    #[serde(default)]
    pub admin_ts: u64,
    #[serde(default)]
    pub machine_ts: u64,
    #[serde(default)]
    pub pending_revoke: Vec<String>,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub deployed_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirebaseConfig {
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub sender_id: String,
    #[serde(default)]
    pub client_email: String,
}

impl FirebaseConfig {
    pub fn is_set(&self) -> bool {
        !self.project_id.is_empty() && !self.api_key.is_empty() && !self.app_id.is_empty() && !self.sender_id.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub machine_id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "yes")]
    pub keep_awake: bool,
    #[serde(default)]
    pub relay: RelayConfig,
    #[serde(default)]
    pub firebase: FirebaseConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config { machine_id: String::new(), enabled: false, port: DEFAULT_PORT, keep_awake: true, relay: RelayConfig::default(), firebase: FirebaseConfig::default() }
    }
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn yes() -> bool {
    true
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn new_machine_id() -> Option<String> {
    let mut raw = [0u8; MACHINE_ID_LEN];
    getrandom::fill(&mut raw).ok()?;
    Some(URL_SAFE_NO_PAD.encode(raw))
}

pub fn valid_machine_id(id: &str) -> bool {
    (16..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> bool {
    let Ok(text) = serde_json::to_string_pretty(value) else { return false };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, path).is_ok()
}

impl Config {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    pub fn load(dir: &Path) -> Config {
        let mut cfg: Config = std::fs::read_to_string(Self::path(dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if cfg.port == 0 {
            cfg.port = DEFAULT_PORT;
        }
        if !valid_machine_id(&cfg.machine_id) {
            if let Some(id) = new_machine_id() {
                cfg.machine_id = id;
                cfg.relay.registered = false;
                cfg.save(dir);
            }
        }
        cfg
    }

    pub fn save(&self, dir: &Path) -> bool {
        write_json(&Self::path(dir), self)
    }

    pub fn next_admin_ts(&mut self, now: u64) -> u64 {
        self.relay.admin_ts = now.max(self.relay.admin_ts + 1);
        self.relay.admin_ts
    }

    pub fn next_machine_ts(&mut self, now: u64) -> u64 {
        self.relay.machine_ts = now.max(self.relay.machine_ts + 1);
        self.relay.machine_ts
    }
}

pub fn port_override() -> Option<u16> {
    std::env::var("CCM_REMOTE_PORT").ok().and_then(|v| v.trim().parse::<u16>().ok()).filter(|p| *p > 0)
}

fn clean_name(raw: &str) -> Option<String> {
    let name: String = raw.trim().chars().filter(|c| !c.is_control()).take(64).collect();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "macos")]
fn os_name() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/scutil").args(["--get", "ComputerName"]).output().ok()?;
    clean_name(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(not(target_os = "macos"))]
fn os_name() -> Option<String> {
    std::env::var("COMPUTERNAME").ok().or_else(|| std::env::var("HOSTNAME").ok()).and_then(|n| clean_name(&n))
}

pub fn machine_name() -> String {
    os_name().unwrap_or_else(|| "Orrery".to_owned())
}
