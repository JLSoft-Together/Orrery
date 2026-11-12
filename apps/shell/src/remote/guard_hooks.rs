use std::net::IpAddr;

use super::config::now_secs;
use super::ctx::Ctx;
use super::guard::Entry;
use super::guard_ip::parse_ip;
use super::link::{Link, Out, Peer};

pub fn link_ip(link: &Link) -> Option<IpAddr> {
    match link.peer {
        Peer::Lan(ip) => Some(ip),
        Peer::Relay(_) => link.origin.ip.as_deref().and_then(parse_ip),
    }
}

pub fn via(link: &Link) -> &'static str {
    match link.peer {
        Peer::Lan(_) => "lan",
        Peer::Relay(_) => "relay",
    }
}

fn clean_country(c: Option<&str>) -> Option<String> {
    c.filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())).map(str::to_owned)
}

impl Ctx {
    pub fn record(&self, link: &Link, result: &'static str, why: &'static str, device: Option<&str>) {
        let entry = Entry {
            at: now_secs(),
            ip: link_ip(link).map_or_else(|| "?".to_owned(), |ip| ip.to_string()),
            via: via(link),
            country: clean_country(link.origin.country.as_deref()),
            device: device.map(super::text::shown_name),
            result,
            why,
        };
        if self.guard.log(entry) {
            self.changed();
        }
    }

    pub fn record_lan(&self, ip: IpAddr, result: &'static str, why: &'static str) {
        let entry = Entry { at: now_secs(), ip: ip.to_string(), via: "lan", country: None, device: None, result, why };
        if self.guard.log(entry) {
            self.changed();
        }
    }

    pub fn strike_lan(&self, ip: IpAddr, why: &'static str) {
        if let Some(row) = self.guard.strike(ip, why, now_secs()) {
            self.record_lan(ip, "blocked", why);
            self.alert_blocked(&row.k, "lan");
            self.changed();
        }
    }

    pub async fn refuse(&self, link: &Link, code: u16, reason: &'static str, why: &'static str) {
        self.record(link, "fail", why, None);
        match link.peer {
            Peer::Lan(ip) => self.strike_lan(ip, why),
            Peer::Relay(_) => {
                let _ = link.out.send(Out::Strike(why)).await;
            }
        }
        link.out.close(code, reason).await;
    }

    pub fn session_opened(&self, link: &Link, device: &str) -> Option<IpAddr> {
        let ip = link_ip(link);
        if let Some(ip) = ip {
            self.guard.good_open(ip, now_secs());
        }
        let country = clean_country(link.origin.country.as_deref());
        let shown_ip = ip.map(|i| i.to_string());
        let relay = matches!(link.peer, Peer::Relay(_));
        self.note_place(device, shown_ip.as_deref(), country.as_deref(), relay);
        let name = crate::state::lock(&self.devices).find(device).map(|d| d.display_name().to_owned());
        self.record(link, "ok", "session", name.as_deref());
        ip
    }

    pub fn session_closed(&self, ip: Option<IpAddr>) {
        if let Some(ip) = ip {
            self.guard.good_close(ip, now_secs());
        }
    }

    pub fn paired(&self, link: &Link, device: &str, name: &str) {
        let ip = link_ip(link);
        let country = clean_country(link.origin.country.as_deref());
        self.note_place(device, None, country.as_deref(), false);
        self.record(link, "ok", "paired", Some(name));
        let shown_ip = ip.map(|i| i.to_string());
        self.alert_paired(device, name, shown_ip.as_deref(), country.as_deref(), via(link));
    }
}
