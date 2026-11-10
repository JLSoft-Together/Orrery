use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const FILE: &str = "telegram-mtproto.json";
const SEALED_MAX: usize = 16_384;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved {
    pub bot: Option<(String, String)>,
    pub user: Option<(i64, String)>,
    pub noticed: bool,
}

pub fn path_for(telegram_json: Option<&Path>) -> Option<PathBuf> {
    Some(telegram_json?.parent()?.join(FILE))
}

pub fn fingerprint(api_id: i64, api_hash: &str, token: &str) -> String {
    let mut h = Sha256::new();
    h.update(format!("{api_id}:{api_hash}:{token}").as_bytes());
    h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn sealed(v: &Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty() && s.len() <= SEALED_MAX).map(str::to_string)
}

pub fn read(file: Option<&Path>) -> Saved {
    let Some(text) = file.and_then(|f| std::fs::read_to_string(f).ok()) else { return Saved::default() };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return Saved::default() };
    if v["v"].as_i64() != Some(1) {
        return Saved::default();
    }
    let bot = match (v["bot"]["key"].as_str(), sealed(&v["bot"]["session"])) {
        (Some(k), Some(s)) if k.len() == 16 => Some((k.to_string(), s)),
        _ => None,
    };
    let user = match (v["user"]["apiId"].as_i64().filter(|n| *n > 0), sealed(&v["user"]["session"])) {
        (Some(id), Some(s)) => Some((id, s)),
        _ => None,
    };
    Saved { bot, user, noticed: v["noticed"] == true }
}

pub fn to_value(s: &Saved) -> Value {
    let mut m = Map::new();
    m.insert("v".into(), json!(1));
    if let Some((key, session)) = &s.bot {
        m.insert("bot".into(), json!({"key": key, "session": session}));
    }
    if let Some((id, session)) = &s.user {
        m.insert("user".into(), json!({"apiId": id, "session": session}));
    }
    if s.noticed {
        m.insert("noticed".into(), json!(true));
    }
    Value::Object(m)
}

pub fn write(file: Option<&Path>, s: &Saved) -> bool {
    let Some(file) = file else { return true };
    let Some(dir) = file.parent() else { return false };
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    if *s == Saved::default() {
        return match std::fs::remove_file(file) {
            Ok(()) => true,
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        };
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    if super::super::shape_out::write_private(&tmp, &to_value(s).to_string()).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    std::fs::rename(&tmp, file).is_ok()
}
