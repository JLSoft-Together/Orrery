use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub const MIN_PREFIX_V4: u8 = 8;
pub const MIN_PREFIX_V6: u8 = 16;
pub const V6_KEY_BITS: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    net: IpAddr,
    bits: u8,
}

pub fn norm(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    }
}

pub fn parse_ip(text: &str) -> Option<IpAddr> {
    text.parse::<IpAddr>().ok().map(norm)
}

fn masked(ip: IpAddr, bits: u8) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => {
            let m = if bits == 0 { 0 } else { u32::MAX << (32 - u32::from(bits)) };
            IpAddr::V4(Ipv4Addr::from(u32::from(v4) & m))
        }
        IpAddr::V6(v6) => {
            let m = if bits == 0 { 0 } else { u128::MAX << (128 - u32::from(bits)) };
            IpAddr::V6(Ipv6Addr::from(u128::from(v6) & m))
        }
    }
}

fn width(ip: IpAddr) -> u8 {
    if ip.is_ipv4() {
        32
    } else {
        128
    }
}

impl Cidr {
    pub fn parse(text: &str) -> Option<Cidr> {
        let text = text.trim();
        let (addr, bits) = match text.split_once('/') {
            Some((a, b)) => (a, Some(b)),
            None => (text, None),
        };
        let ip = parse_ip(addr)?;
        let bits = match bits {
            Some(b) if !b.is_empty() && b.len() <= 3 && b.bytes().all(|c| c.is_ascii_digit()) => b.parse::<u8>().ok()?,
            Some(_) => return None,
            None => width(ip),
        };
        let min = if ip.is_ipv4() { MIN_PREFIX_V4 } else { MIN_PREFIX_V6 };
        if bits > width(ip) || bits < min {
            return None;
        }
        Some(Cidr { net: masked(ip, bits), bits })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = norm(ip);
        ip.is_ipv4() == self.net.is_ipv4() && masked(ip, self.bits) == self.net
    }

    pub fn overlaps(&self, other: &Cidr) -> bool {
        if self.net.is_ipv4() != other.net.is_ipv4() {
            return false;
        }
        let bits = self.bits.min(other.bits);
        masked(self.net, bits) == masked(other.net, bits)
    }

    pub fn text(&self) -> String {
        if self.bits == width(self.net) {
            self.net.to_string()
        } else {
            format!("{}/{}", self.net, self.bits)
        }
    }
}

pub fn key(ip: IpAddr) -> String {
    match norm(ip) {
        IpAddr::V4(v4) => v4.to_string(),
        v6 => format!("{}/{V6_KEY_BITS}", masked(v6, V6_KEY_BITS)),
    }
}
