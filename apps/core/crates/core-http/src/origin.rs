use subtle::ConstantTimeEq;

const LOCAL_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "[::1]"];

fn port_ok(port: &str) -> bool {
    port.is_empty() || (port.len() <= 5 && port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u32>().is_ok_and(|p| p <= 65_535))
}

pub fn host_of(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let rest = value.split_once("://").map_or(value, |(_, r)| r);
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let authority = authority.rsplit_once('@').map_or(authority, |(_, a)| a);
    let (host, port) = if authority.starts_with('[') {
        let close = authority.find(']')?;
        let after = &authority[close + 1..];
        let port = if after.is_empty() { "" } else { after.strip_prefix(':')? };
        (&authority[..=close], port)
    } else {
        authority.split_once(':').unwrap_or((authority, ""))
    };
    if host.is_empty() || !port_ok(port) {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

pub fn is_allowed_origin(origin: &str, ports: &[u16]) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else { return false };
    let (host, port) = if rest.starts_with('[') {
        let Some(close) = rest.find(']') else { return false };
        let Some(port) = rest[close + 1..].strip_prefix(':') else { return false };
        (&rest[..=close], port)
    } else {
        let Some(pair) = rest.rsplit_once(':') else { return false };
        pair
    };
    if !LOCAL_HOSTS.contains(&host) || port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match port.parse::<u16>() {
        Ok(p) => p.to_string() == port && ports.contains(&p),
        Err(_) => false,
    }
}

pub fn has_app_key(header: Option<&str>, key: Option<&str>) -> bool {
    let Some(key) = key else { return true };
    let Some(got) = header else { return false };
    got.len() == key.len() && bool::from(got.as_bytes().ct_eq(key.as_bytes()))
}

pub fn is_local_request(host: Option<&str>, origin: Option<&str>, ports: &[u16]) -> bool {
    let Some(host) = host.and_then(host_of) else { return false };
    if !LOCAL_HOSTS.contains(&host.as_str()) {
        return false;
    }
    origin.is_none_or(|o| is_allowed_origin(o, ports))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORTS: [u16; 2] = [4317, 5173];

    #[test]
    fn accepts_the_dashboard_and_dev_origins_on_local_hosts() {
        for o in ["http://127.0.0.1:4317", "http://localhost:4317", "http://[::1]:4317", "http://127.0.0.1:5173"] {
            assert!(is_allowed_origin(o, &PORTS), "{o}");
        }
    }

    #[test]
    fn rejects_other_local_apps_other_schemes_and_remote_hosts() {
        for o in [
            "http://127.0.0.1:8888",
            "http://localhost",
            "https://127.0.0.1:4317",
            "http://evil.example:4317",
            "http://127.0.0.1:4317/path",
            "null",
            "not a url",
            "http://127.0.0.1:04317",
            "HTTP://127.0.0.1:4317",
            "http://LOCALHOST:4317",
            "http://127.1:4317",
            "http://127.0.0.1:4317 ",
            "http://127.0.0.1:99999",
        ] {
            assert!(!is_allowed_origin(o, &PORTS), "{o}");
        }
    }

    #[test]
    fn host_header_must_be_a_local_name() {
        for h in ["127.0.0.1:4317", "localhost:4317", "LOCALHOST", "[::1]:4317", "127.0.0.1"] {
            assert!(is_local_request(Some(h), None, &PORTS), "{h}");
        }
        for h in ["evil.example", "evil.example:4317", "127.0.0.1.evil.test", "", "127.0.0.1:99999", "127.0.0.1:abc"] {
            assert!(!is_local_request(Some(h), None, &PORTS), "{h}");
        }
        assert!(!is_local_request(None, None, &PORTS));
        assert!(!is_local_request(Some("127.0.0.1:4317"), Some("http://localhost:9999"), &PORTS));
        assert!(is_local_request(Some("127.0.0.1:4317"), Some("http://localhost:4317"), &PORTS));
    }

    #[test]
    fn app_key_compare() {
        assert!(has_app_key(None, None));
        assert!(has_app_key(Some("k"), None));
        assert!(has_app_key(Some("secret"), Some("secret")));
        assert!(!has_app_key(Some("secreT"), Some("secret")));
        assert!(!has_app_key(Some("secre"), Some("secret")));
        assert!(!has_app_key(Some("secrets"), Some("secret")));
        assert!(!has_app_key(None, Some("secret")));
    }
}
