use crate::run::android::jsre::{g, Js};
use serde_json::{json, Value};

#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    pub id: String,
    pub state: &'static str,
    pub model: Option<String>,
    pub product: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
    pub usb: Option<String>,
}

pub fn ok() -> Value {
    json!({"ok": true})
}

pub fn err(code: &str) -> Value {
    json!({"ok": false, "error": code})
}

pub fn err_detail(code: &str, detail: &str) -> Value {
    if detail.is_empty() {
        err(code)
    } else {
        json!({"ok": false, "error": code, "detail": detail})
    }
}

pub fn is_ok(v: &Value) -> bool {
    v.get("ok") == Some(&Value::Bool(true))
}

fn state_of(s: &str) -> &'static str {
    match s {
        "device" => "ready",
        "unauthorized" => "unauthorized",
        "offline" => "offline",
        "authorizing" => "authorizing",
        "recovery" => "recovery",
        "sideload" => "sideload",
        "bootloader" => "bootloader",
        "no permissions" => "no-permissions",
        _ => "unknown",
    }
}

pub fn parse_adb_device_rows(out: &str) -> Vec<Row> {
    static LINE: Js = Js::new(r"^(\S+)\s+(no permissions|\S+)(.*)$", "");
    static FIELD: Js = Js::new(r"\b(usb|product|model|device|transport_id):(\S+)", "g");
    let mut rows = Vec::new();
    for raw in out.split('\n') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
            continue;
        }
        let Some(m) = LINE.exec(line) else { continue };
        let id = g(&m, 1).unwrap_or("");
        if id.ends_with(':') {
            continue;
        }
        let mut fields: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for f in FIELD.all(g(&m, 3).unwrap_or("")) {
            fields.insert(g(&f, 1).unwrap_or("").to_string(), g(&f, 2).unwrap_or("").to_string());
        }
        rows.push(Row {
            id: id.to_string(),
            state: state_of(g(&m, 2).unwrap_or("")),
            model: fields.get("model").map(|s| s.replace('_', " ")),
            product: fields.get("product").cloned(),
            device: fields.get("device").cloned(),
            transport_id: fields.get("transport_id").cloned(),
            usb: fields.get("usb").cloned(),
        });
    }
    rows
}

pub fn is_wifi_id(id: &str) -> bool {
    id.contains(':') || id.contains("._adb-tls-connect._tcp")
}

pub fn transport_of(row: &Row) -> &'static str {
    if row.id.starts_with("emulator-") {
        "emulator"
    } else if is_wifi_id(&row.id) {
        "wifi"
    } else {
        "usb"
    }
}

pub struct Mdns {
    pub name: String,
    pub kind: &'static str,
    pub host: String,
    pub port: u16,
}

pub fn parse_mdns_services(out: &str) -> Vec<Mdns> {
    static ADDR: Js = Js::new(r"^(.+):(\d{1,5})$", "");
    let mut services = Vec::new();
    for line in out.split('\n') {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let kind = match parts[1].strip_suffix('.').unwrap_or(parts[1]) {
            "_adb-tls-pairing._tcp" => "pairing",
            "_adb-tls-connect._tcp" => "connect",
            "_adb._tcp" => "legacy",
            _ => continue,
        };
        let Some(addr) = ADDR.exec(parts[2]) else { continue };
        let port: u64 = g(&addr, 2).and_then(|p| p.parse().ok()).unwrap_or(0);
        if !(1..=65535).contains(&port) {
            continue;
        }
        services.push(Mdns { name: parts[0].to_string(), kind, host: g(&addr, 1).unwrap_or("").to_string(), port: port as u16 });
    }
    services
}

fn failure(code: &str, out: &str) -> Value {
    err_detail(code, out.trim())
}

pub fn parse_connect(out: &str) -> Value {
    static RE: Js = Js::new(r"^(already )?connected to ", "m");
    if RE.test(out) {
        ok()
    } else {
        failure("connect-failed", out)
    }
}

pub fn parse_pair(out: &str) -> Value {
    static RE: Js = Js::new(r"^Successfully paired", "m");
    if RE.test(out) {
        ok()
    } else {
        failure("pair-failed", out)
    }
}

pub fn parse_disconnect(out: &str) -> Value {
    static RE: Js = Js::new(r"^disconnected\b", "m");
    if RE.test(out) {
        ok()
    } else {
        failure("not-connected", out)
    }
}

pub fn parse_wlan_ip(out: &str) -> Option<String> {
    static RE: Js = Js::new(r"\binet (\d{1,3}(?:\.\d{1,3}){3})/", "");
    RE.exec(out).and_then(|m| g(&m, 1).map(String::from))
}

const DEFAULT_PORT: u64 = 5555;

pub struct Address {
    pub target: String,
    pub host: Option<String>,
    pub port: Option<u64>,
}

pub fn parse_address(input: &str) -> Option<Address> {
    static MDNS: Js = Js::new(r"^adb-[A-Za-z0-9_-]+\._adb-tls-connect\._tcp$", "");
    static HOST: Js = Js::new(r"^[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?$", "");
    static V6: Js = Js::new(r"^\[([0-9A-Fa-f:.]+)\](?::(\d+))?$", "");
    static PLAIN: Js = Js::new(r"^([^:]+)(?::(\d+))?$", "");
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    if MDNS.test(s) {
        return Some(Address { target: s.to_string(), host: None, port: None });
    }
    let (host, port_text) = if let Some(m) = V6.exec(s) {
        (format!("[{}]", g(&m, 1).unwrap_or("")), g(&m, 2).map(String::from))
    } else {
        let m = PLAIN.exec(s)?;
        let h = g(&m, 1).unwrap_or("");
        if !HOST.test(h) {
            return None;
        }
        (h.to_string(), g(&m, 2).map(String::from))
    };
    let port = match port_text {
        None => DEFAULT_PORT,
        Some(t) => t.parse::<u64>().ok()?,
    };
    if !(1..=65535).contains(&port) {
        return None;
    }
    Some(Address { target: format!("{host}:{port}"), host: Some(host), port: Some(port) })
}

pub fn address_key(host: &str, port: u64) -> String {
    format!("{}:{port}", host.trim_start_matches('[').trim_end_matches(']').to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_addresses() {
        let rows = parse_adb_device_rows("List of devices attached\nR58 device usb:1-1 model:Pixel_7 transport_id:2\n192.168.1.9:5555 no permissions\n* daemon started\nbad: device\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].model.as_deref(), Some("Pixel 7"));
        assert_eq!(rows[0].transport_id.as_deref(), Some("2"));
        assert_eq!(rows[1].state, "no-permissions");
        assert_eq!(transport_of(&rows[1]), "wifi");
        let a = parse_address(" Phone.local ").unwrap();
        assert_eq!((a.target.as_str(), a.port), ("Phone.local:5555", Some(5555)));
        assert_eq!(parse_address("[::1]:9000").unwrap().target, "[::1]:9000");
        assert!(parse_address("bad host").is_none() && parse_address("a:70000").is_none());
        assert_eq!(address_key("[::1]", 5), "::1:5");
        assert!(is_ok(&parse_connect("already connected to x:1")) && !is_ok(&parse_pair("Failed")));
        assert_eq!(parse_mdns_services("n _adb-tls-pairing._tcp. 10.0.0.1:4000\nx y z\n").len(), 1);
    }
}
