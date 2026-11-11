use std::net::{IpAddr, Ipv4Addr};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use mdns_sd::{IfKind, ServiceDaemon, ServiceInfo};
use ring::digest::{digest, SHA256};

pub const SERVICE: &str = "_orrery._tcp.local.";

pub struct Mdns {
    daemon: ServiceDaemon,
    fullname: String,
}

pub fn tag(key: &[u8; 32]) -> String {
    let hash = digest(&SHA256, key);
    URL_SAFE_NO_PAD.encode(&hash.as_ref()[..8])
}

pub fn instance(tag: &str) -> String {
    format!("orrery-{}", tag.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase())
}

pub fn txt<'a>(tag: &'a str, name: &'a str) -> [(&'static str, &'a str); 3] {
    [("v", "1"), ("k", tag), ("n", name)]
}

pub fn start(name: &str, addrs: &[Ipv4Addr], port: u16, key: &[u8; 32]) -> Option<Mdns> {
    if addrs.is_empty() {
        return None;
    }
    let daemon = ServiceDaemon::new().ok()?;
    let _ = daemon.disable_interface(vec![IfKind::All, IfKind::LoopbackV4, IfKind::LoopbackV6]);
    for ip in addrs {
        let _ = daemon.enable_interface(IfKind::Addr(IpAddr::V4(*ip)));
    }
    let short: String = name.chars().take(60).collect();
    let tag = tag(key);
    let instance = instance(&tag);
    let host = format!("{instance}.local.");
    let ips: Vec<IpAddr> = addrs.iter().map(|ip| IpAddr::V4(*ip)).collect();
    let info = ServiceInfo::new(SERVICE, &instance, &host, ips.as_slice(), port, &txt(&tag, &short)[..]).ok()?;
    let fullname = info.get_fullname().to_owned();
    if daemon.register(info).is_err() {
        let _ = daemon.shutdown();
        return None;
    }
    Some(Mdns { daemon, fullname })
}

impl Drop for Mdns {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}
