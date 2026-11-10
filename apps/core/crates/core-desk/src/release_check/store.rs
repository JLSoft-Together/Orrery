use crate::js;
use crate::preview::paths::root_key;
use crate::util::write_atomic;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

pub const RELEASE_DIR: &str = "release-check";
const MAX_BYTES: u64 = 64 * 1024;
const SAFE_INT: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Default)]
pub struct Memory {
    pub released: Option<Value>,
    pub cert: Option<Value>,
}

pub fn is_cert_sha256(v: &str) -> bool {
    let parts: Vec<&str> = v.split(':').collect();
    parts.len() == 32 && parts.iter().all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b)))
}

fn safe_integer(v: &Value) -> bool {
    match v.as_f64() {
        Some(f) => f.fract() == 0.0 && f.abs() <= SAFE_INT,
        None => false,
    }
}

fn clean(v: &Value) -> Memory {
    let released = v.get("released").filter(|r| r.is_object()).and_then(|r| {
        let at = r.get("at").filter(|a| a.is_number())?;
        let code = r.get("code")?;
        let name = r.get("name")?;
        if !(code.is_null() || safe_integer(code)) || !(name.is_null() || name.is_string()) {
            return None;
        }
        let name = name.as_str().map(|s| json!(js::slice(s, 64))).unwrap_or(Value::Null);
        Some(json!({"code": code, "name": name, "at": at}))
    });
    let cert = v.get("cert").filter(|c| c.is_object()).and_then(|c| {
        let sha = c.get("sha256").and_then(Value::as_str).filter(|s| is_cert_sha256(s))?;
        let at = c.get("at").filter(|a| a.is_number())?;
        Some(json!({"sha256": sha, "at": at}))
    });
    Memory { released, cert }
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(data_dir: &Path) -> Self {
        Store { dir: data_dir.join(RELEASE_DIR) }
    }

    pub fn file_of(&self, app_dir: &str) -> PathBuf {
        let hex: String = Sha1::digest(root_key(app_dir).as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        self.dir.join(format!("{hex}.json"))
    }

    pub fn get(&self, app_dir: &str) -> Memory {
        let file = self.file_of(app_dir);
        let Some(meta) = std::fs::metadata(&file).ok() else { return Memory::default() };
        if meta.len() > MAX_BYTES {
            return Memory::default();
        }
        std::fs::read_to_string(&file).ok().and_then(|t| js::parse(&t)).map(|v| clean(&v)).unwrap_or_default()
    }

    fn write(&self, app_dir: &str, next: &Memory) -> bool {
        let body = json!({"released": next.released, "cert": next.cert});
        std::fs::create_dir_all(&self.dir).is_ok() && write_atomic(&self.file_of(app_dir), &js::stringify(&body)).is_ok()
    }

    pub fn mark_released(&self, app_dir: &str, code: &Value, name: &Value, now: f64) -> bool {
        let mut cur = self.get(app_dir);
        cur.released = Some(json!({"code": code, "name": name, "at": now as i64}));
        self.write(app_dir, &cur)
    }

    pub fn remember_cert(&self, app_dir: &str, sha256: &str, now: f64) -> bool {
        if !is_cert_sha256(sha256) {
            return false;
        }
        let mut cur = self.get(app_dir);
        cur.cert = Some(json!({"sha256": sha256, "at": now as i64}));
        self.write(app_dir, &cur)
    }
}
