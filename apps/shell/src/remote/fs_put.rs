use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::{json, Value};

use super::super::ctx::Host;
use super::super::viewer_events::run_log::valid_id;
use crate::state::lock;

pub const FS_PUT: &str = "ccm:remote-fs-put";
pub const MAX_CHUNK_B64: usize = 512 * 1024;
pub const MAX_FILE: usize = 8 * 1024 * 1024;
pub const MAX_OPEN: usize = 4;
pub const MAX_SEQ: u64 = 64;
pub const TTL: Duration = Duration::from_secs(60);
const MAX_TOTAL_OPEN: usize = 32;
pub const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const NAME_TRIES: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fail {
    Invalid,
    Denied,
    Order,
    Expired,
    Size,
    Busy,
    Image,
    Io,
}

impl Fail {
    pub fn code(self) -> &'static str {
        match self {
            Fail::Invalid => "invalid",
            Fail::Denied => "denied",
            Fail::Order => "order",
            Fail::Expired => "expired",
            Fail::Size => "size",
            Fail::Busy => "busy",
            Fail::Image => "image",
            Fail::Io => "io",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub root: String,
    pub up: String,
    pub seq: u64,
    pub last: bool,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Next(u64),
    Done { root: String, bytes: Vec<u8> },
}

struct Buf {
    root: String,
    next: u64,
    bytes: Vec<u8>,
    touched: Instant,
}

#[derive(Default)]
pub struct Puts {
    open: Mutex<HashMap<String, Buf>>,
}

static GLOBAL: LazyLock<Puts> = LazyLock::new(Puts::default);

fn key(owner: &str, up: &str) -> String {
    format!("{owner}\n{up}")
}

impl Puts {
    pub fn take(&self, owner: &str, c: Chunk, now: Instant) -> Result<Step, Fail> {
        let mut map = lock(&self.open);
        map.retain(|_, b| now.duration_since(b.touched) < TTL);
        let k = key(owner, &c.up);
        if c.seq > MAX_SEQ {
            map.remove(&k);
            return Err(Fail::Order);
        }
        if c.seq == 0 {
            let prefix = format!("{owner}\n");
            let mine = map.keys().filter(|x| x.starts_with(&prefix)).count();
            if map.remove(&k).is_some() {
                return Err(Fail::Order);
            }
            if mine >= MAX_OPEN || map.len() >= MAX_TOTAL_OPEN {
                return Err(Fail::Busy);
            }
            map.insert(k.clone(), Buf { root: c.root.clone(), next: 0, bytes: Vec::new(), touched: now });
        }
        let held: usize = map.values().map(|b| b.bytes.len()).sum();
        if held + c.data.len() > MAX_TOTAL_BYTES {
            map.remove(&k);
            return Err(Fail::Busy);
        }
        let Some(buf) = map.get_mut(&k) else { return Err(Fail::Expired) };
        if buf.next != c.seq || buf.root != c.root {
            map.remove(&k);
            return Err(Fail::Order);
        }
        if buf.bytes.len() + c.data.len() > MAX_FILE {
            map.remove(&k);
            return Err(Fail::Size);
        }
        buf.bytes.extend_from_slice(&c.data);
        buf.next = c.seq + 1;
        buf.touched = now;
        if c.last {
            let done = map.remove(&k).map(|b| Step::Done { root: b.root, bytes: b.bytes });
            return done.ok_or(Fail::Expired);
        }
        Ok(Step::Next(c.seq + 1))
    }

    pub fn open_count(&self) -> usize {
        lock(&self.open).len()
    }
}

pub fn global() -> &'static Puts {
    &GLOBAL
}

pub fn parse(args: &Value) -> Option<Chunk> {
    let req = args.get(0)?;
    let root = req.get("root")?.as_str().filter(|r| !r.is_empty())?.to_owned();
    let up = req.get("up")?.as_str().filter(|u| valid_id(u))?.to_owned();
    let seq = req.get("seq")?.as_u64()?;
    let last = req.get("last")?.as_bool()?;
    let b64 = req.get("b64")?.as_str().filter(|b| !b.is_empty() && b.len() <= MAX_CHUNK_B64)?;
    let data = B64.decode(b64).ok().filter(|d| !d.is_empty())?;
    Some(Chunk { root, up, seq, last, data })
}

fn stamp(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn ext_of(bytes: &[u8]) -> Result<&'static str, Fail> {
    match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => Ok("jpg"),
        Ok(image::ImageFormat::Png) => Ok("png"),
        _ => Err(Fail::Image),
    }
}

fn plain_dir(path: &Path) -> Result<(), Fail> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() || !m.is_dir() => Err(Fail::Denied),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(Fail::Io),
    }
}

pub fn save(root: &Path, bytes: &[u8], now: SystemTime) -> Result<(PathBuf, String), Fail> {
    let ext = ext_of(bytes)?;
    if root.components().any(|c| matches!(c, Component::ParentDir)) || !root.is_dir() {
        return Err(Fail::Denied);
    }
    let base = root.join(".orrery");
    let dir = base.join("inbox");
    plain_dir(&base)?;
    plain_dir(&dir)?;
    std::fs::create_dir_all(&dir).map_err(|_| Fail::Io)?;
    plain_dir(&dir)?;
    let stamp = stamp(now);
    for n in 0..NAME_TRIES {
        let name = if n == 0 { format!("{stamp}.{ext}") } else { format!("{stamp}-{n}.{ext}") };
        let path = dir.join(&name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                if f.write_all(bytes).is_err() {
                    drop(f);
                    let _ = std::fs::remove_file(&path);
                    return Err(Fail::Io);
                }
                return Ok((path, name));
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(Fail::Io),
        }
    }
    Err(Fail::Io)
}

fn root_ok(host: &dyn Host, owner: &str, accept: &[String], root: &str) -> bool {
    if root.contains('\0') || root.starts_with("\\\\") || root.starts_with("//") {
        return false;
    }
    let env = host.viewer(owner, "ccm:list-dir", json!([{ "root": root, "dir": root }]), accept);
    env.get("raw").is_some_and(Value::is_array)
}

fn fail(f: Fail) -> Value {
    json!({ "ok": false, "error": f.code() })
}

pub fn run(host: &dyn Host, owner: &str, accept: &[String], args: &Value, puts: &Puts, now: Instant, wall: SystemTime) -> Value {
    let Some(chunk) = parse(args) else { return fail(Fail::Invalid) };
    if !host.control() {
        return fail(Fail::Denied);
    }
    if chunk.seq == 0 && !root_ok(host, owner, accept, &chunk.root) {
        return fail(Fail::Denied);
    }
    match puts.take(owner, chunk, now) {
        Err(f) => fail(f),
        Ok(Step::Next(next)) => json!({ "ok": true, "done": false, "next": next }),
        Ok(Step::Done { root, bytes }) => match save(Path::new(&root), &bytes, wall) {
            Ok((path, name)) => json!({ "ok": true, "done": true, "path": path.to_string_lossy(), "name": name }),
            Err(f) => fail(f),
        },
    }
}

pub fn reply(host: &dyn Host, owner: &str, accept: &[String], args: &Value) -> Value {
    json!({ "raw": run(host, owner, accept, args, global(), Instant::now(), SystemTime::now()) })
}

#[cfg(test)]
#[path = "fs_put_tests.rs"]
mod tests;
