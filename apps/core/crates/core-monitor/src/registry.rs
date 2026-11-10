use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

const MAX_RECORD_BYTES: u64 = 262_144;
const TORN_RETRY_MS: u64 = 60;
const MAX_SOCKET_PATH: usize = 300;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RegistryRecord {
    pub pid: u32,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<f64>,
    pub proc_start: Option<String>,
    pub version: Option<String>,
    pub kind: Option<String>,
    pub entrypoint: Option<String>,
    pub name: Option<String>,
    pub name_source: Option<String>,
    pub status: Option<String>,
    pub waiting_for: Option<String>,
    pub updated_at: Option<f64>,
    pub status_updated_at: Option<f64>,
    pub sock_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegistryEntry {
    Record(Box<RegistryRecord>),
    Unreadable,
}

pub fn socket_key(path: Option<&Value>) -> Option<String> {
    let s = path?.as_str()?;
    let len = s.encode_utf16().count();
    if len == 0 || len > MAX_SOCKET_PATH {
        return None;
    }
    let digest = Sha256::digest(s.to_lowercase().as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Some(hex[..16].to_owned())
}

fn text(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k).and_then(Value::as_str).map(str::to_owned)
}

fn num(o: &Map<String, Value>, k: &str) -> Option<f64> {
    o.get(k).and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0)
}

pub fn pid_from_file_name(name: &str) -> Option<u32> {
    let base = name.strip_suffix(".json")?;
    if base.is_empty() || base.len() > 10 || !base.bytes().all(|b| b.is_ascii_digit()) || base.starts_with('0') {
        return None;
    }
    base.parse::<u32>().ok().filter(|p| *p > 0)
}

pub fn parse_registry_record(pid: u32, text_in: &str) -> Option<RegistryRecord> {
    let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text_in) else {
        return match serde_json::from_str::<Value>(text_in) {
            Ok(Value::Array(_)) => Some(RegistryRecord { pid, ..Default::default() }),
            _ => None,
        };
    };
    Some(RegistryRecord {
        pid,
        session_id: text(&o, "sessionId"),
        cwd: text(&o, "cwd"),
        started_at: num(&o, "startedAt"),
        proc_start: text(&o, "procStartFt").or_else(|| text(&o, "procStart")),
        version: text(&o, "version"),
        kind: text(&o, "kind"),
        entrypoint: text(&o, "entrypoint"),
        name: text(&o, "name"),
        name_source: text(&o, "nameSource"),
        status: text(&o, "status"),
        waiting_for: text(&o, "waitingFor"),
        updated_at: num(&o, "updatedAt"),
        status_updated_at: num(&o, "statusUpdatedAt"),
        sock_key: socket_key(o.get("messagingSocketPath")),
    })
}

fn read_record(file: &Path, pid: u32) -> Option<RegistryEntry> {
    for attempt in 0..2 {
        match std::fs::metadata(file) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Ok(m) if m.len() > MAX_RECORD_BYTES => return Some(RegistryEntry::Unreadable),
            _ => {}
        }
        match std::fs::read(file) {
            Ok(bytes) => {
                if bytes.len() as u64 > MAX_RECORD_BYTES {
                    return Some(RegistryEntry::Unreadable);
                }
                if let Some(rec) = parse_registry_record(pid, &String::from_utf8_lossy(&bytes)) {
                    return Some(RegistryEntry::Record(Box::new(rec)));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(_) => {}
        }
        if attempt == 0 {
            std::thread::sleep(Duration::from_millis(TORN_RETRY_MS));
        }
    }
    Some(RegistryEntry::Unreadable)
}

pub fn read_registry(sessions_dir: &Path) -> Vec<(u32, RegistryEntry)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(sessions_dir) else { return out };
    for e in entries.flatten() {
        let Ok(name) = e.file_name().into_string() else { continue };
        let Some(pid) = pid_from_file_name(&name) else { continue };
        if let Some(entry) = read_record(&sessions_dir.join(&name), pid) {
            out.push((pid, entry));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_and_records_like_node() {
        assert_eq!(pid_from_file_name("4242.json"), Some(4242));
        assert_eq!(pid_from_file_name("0042.json"), None);
        assert_eq!(pid_from_file_name("0.json"), None);
        assert_eq!(pid_from_file_name("12a.json"), None);
        assert_eq!(pid_from_file_name("12.txt"), None);
        let r = parse_registry_record(7, r#"{"sessionId":"s","procStart":"1","procStartFt":"2","startedAt":-1,"updatedAt":5,"messagingSocketPath":"\\\\.\\pipe\\X"}"#).unwrap();
        assert_eq!(r.proc_start.as_deref(), Some("2"));
        assert_eq!(r.started_at, None);
        assert_eq!(r.updated_at, Some(5.0));
        assert_eq!(r.sock_key.as_deref().map(str::len), Some(16));
        assert_eq!(socket_key(Some(&Value::from("\\\\.\\pipe\\x"))), r.sock_key);
        assert_eq!(socket_key(Some(&Value::from(""))), None);
        assert!(parse_registry_record(7, "nope").is_none());
        assert!(parse_registry_record(7, "null").is_none());
    }

    #[test]
    fn reads_a_sessions_dir() {
        let dir = std::env::temp_dir().join(format!("ccm-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("11.json"), r#"{"cwd":"D:\\w"}"#).unwrap();
        std::fs::write(dir.join("12.json"), "{torn").unwrap();
        std::fs::write(dir.join("x.json"), "{}").unwrap();
        let reg = read_registry(&dir);
        assert_eq!(reg.len(), 2);
        assert!(matches!(&reg[0], (11, RegistryEntry::Record(r)) if r.cwd.as_deref() == Some("D:\\w")));
        assert_eq!(reg[1], (12, RegistryEntry::Unreadable));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
