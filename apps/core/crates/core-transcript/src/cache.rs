use crate::index::{FileEntry, UsageIndex};
use crate::omap::{Num, OMap};
use crate::record::Totals;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

pub const CACHE_VERSION: f64 = 4.0;
const RENAME_RETRIES: u32 = 5;

#[derive(Debug, Clone)]
pub enum Node {
    Obj(Vec<(String, Node)>),
    Other(Value),
}

impl Node {
    fn obj(&self) -> Option<&[(String, Node)]> {
        match self {
            Node::Obj(v) => Some(v),
            Node::Other(_) => None,
        }
    }

    fn get(&self, key: &str) -> Option<&Node> {
        self.obj()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn value(&self) -> Option<&Value> {
        match self {
            Node::Other(v) => Some(v),
            Node::Obj(_) => None,
        }
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Node;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("json")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
                let mut out: Vec<(String, Node)> = Vec::new();
                let mut at: HashMap<String, usize> = HashMap::new();
                while let Some(k) = map.next_key::<String>()? {
                    let v = map.next_value::<Node>()?;
                    match at.get(&k) {
                        Some(&i) => out[i].1 = v,
                        None => {
                            at.insert(k.clone(), out.len());
                            out.push((k, v));
                        }
                    }
                }
                Ok(Node::Obj(out))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = seq.next_element::<Value>()? {
                    out.push(v);
                }
                Ok(Node::Other(Value::Array(out)))
            }
            fn visit_bool<E>(self, b: bool) -> Result<Node, E> {
                Ok(Node::Other(Value::Bool(b)))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Node, E> {
                Ok(Node::Other(Value::from(v)))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Node, E> {
                Ok(Node::Other(Value::from(v)))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Node, E> {
                Ok(Node::Other(Value::from(v)))
            }
            fn visit_str<E>(self, s: &str) -> Result<Node, E> {
                Ok(Node::Other(Value::String(s.to_owned())))
            }
            fn visit_string<E>(self, s: String) -> Result<Node, E> {
                Ok(Node::Other(Value::String(s)))
            }
            fn visit_unit<E>(self) -> Result<Node, E> {
                Ok(Node::Other(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}

fn num(n: Option<&Node>) -> Option<f64> {
    n.and_then(Node::value).and_then(Value::as_f64)
}

fn int(n: Option<&Node>) -> f64 {
    num(n).filter(|v| *v >= 0.0).unwrap_or(0.0)
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn day_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && digits(&s[..4]) && digits(&s[5..7]) && digits(&s[8..])
}

pub fn hour_key_ok(key: &str) -> bool {
    let Some((day, hour)) = key.split_once('|') else { return false };
    day_ok(day) && (1..=2).contains(&hour.len()) && digits(hour)
}

pub fn tool_key_ok(key: &str) -> bool {
    let mut parts = key.splitn(4, '|');
    let (Some(day), Some(kind), Some(name), Some(rest)) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return false };
    day_ok(day) && matches!(kind, "t" | "m" | "s" | "a" | "e" | "f") && !name.is_empty() && !rest.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
}

fn counts(node: Option<&Node>, ok: fn(&str) -> bool) -> Option<OMap<f64>> {
    let obj = node?.obj()?;
    let mut out = OMap::new();
    for (k, v) in obj {
        let n = int(Some(v));
        if n == 0.0 || !ok(k) {
            continue;
        }
        out.insert(k, n);
    }
    Some(out)
}

fn totals_of(t: &Node) -> Totals {
    Totals {
        messages: int(t.get("messages")),
        input: int(t.get("input")),
        output: int(t.get("output")),
        cache_write_5m: int(t.get("cacheWrite5m")),
        cache_write_1h: int(t.get("cacheWrite1h")),
        cache_read: int(t.get("cacheRead")),
    }
}

impl UsageIndex {
    pub fn load_cache(&mut self) {
        let Some(file) = self.cache_file.clone() else { return };
        let Ok(bytes) = std::fs::read(&file) else { return };
        let Ok(root) = serde_json::from_slice::<Node>(&bytes) else { return };
        if num(root.get("version")) != Some(CACHE_VERSION) {
            return;
        }
        let Some(files) = root.get("files").and_then(Node::obj) else { return };
        for (rel, raw) in files {
            let Some(buckets) = raw.get("buckets").and_then(Node::obj) else { continue };
            let ids: Vec<String> = match raw.get("ids").and_then(Node::value) {
                Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect(),
                _ => Vec::new(),
            };
            let mut entry = FileEntry {
                size: int(raw.get("size")) as u64,
                offset: int(raw.get("offset")) as u64,
                mtime_ms: num(raw.get("mtimeMs")),
                ids,
                gone: matches!(raw.get("gone").and_then(Node::value), Some(Value::Bool(true))),
                ..FileEntry::default()
            };
            let slot = self.files.slot_of(rel).unwrap_or_else(|| self.files.insert(rel, FileEntry::default()));
            for id in &entry.ids {
                self.owners.entry(id.clone()).or_insert(slot);
            }
            for (key, t) in buckets {
                if t.obj().is_none() {
                    continue;
                }
                let totals = totals_of(t);
                entry.buckets.insert(key, totals);
                self.add_total(key, &totals, 1.0);
            }
            entry.hours = counts(raw.get("hours"), hour_key_ok);
            for (k, n) in entry.hours.iter().flat_map(|h| h.iter()) {
                self.add_hour(k, *n);
            }
            entry.tools = counts(raw.get("tools"), tool_key_ok);
            for (k, n) in entry.tools.iter().flat_map(|h| h.iter()) {
                self.add_tool(k, *n);
            }
            *self.files.at_mut(slot).expect("file slot") = entry;
        }
        self.dirty.clear();
    }

    pub fn save(&mut self) {
        let Some(file) = self.cache_file.clone() else { return };
        if !self.save_needed {
            return;
        }
        self.save_needed = false;
        if let Err(e) = write_cache(&file, &CacheBody(&self.files)) {
            self.save_needed = true;
            eprintln!("[collector] cannot save usage cache: {e}");
        }
    }
}

struct CacheBody<'a>(&'a OMap<FileEntry>);

impl Serialize for CacheBody<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("version", &Num(CACHE_VERSION))?;
        m.serialize_entry("files", self.0)?;
        m.end()
    }
}

struct TotalsOut<'a>(&'a Totals);

impl Serialize for TotalsOut<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let t = self.0;
        let mut m = s.serialize_map(Some(6))?;
        m.serialize_entry("messages", &Num(t.messages))?;
        m.serialize_entry("input", &Num(t.input))?;
        m.serialize_entry("output", &Num(t.output))?;
        m.serialize_entry("cacheWrite5m", &Num(t.cache_write_5m))?;
        m.serialize_entry("cacheWrite1h", &Num(t.cache_write_1h))?;
        m.serialize_entry("cacheRead", &Num(t.cache_read))?;
        m.end()
    }
}

struct Counts<'a>(&'a OMap<f64>);

impl Serialize for Counts<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0.iter() {
            m.serialize_entry(k, &Num(*v))?;
        }
        m.end()
    }
}

impl Serialize for FileEntry {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("size", &self.size)?;
        m.serialize_entry("offset", &self.offset)?;
        if let Some(t) = self.mtime_ms {
            m.serialize_entry("mtimeMs", &Num(t))?;
        }
        m.serialize_entry("ids", &self.ids)?;
        let buckets: Vec<(&str, TotalsOut)> = self.buckets.iter().map(|(k, t)| (k, TotalsOut(t))).collect();
        m.serialize_entry("buckets", &Pairs(&buckets))?;
        if let Some(h) = &self.hours {
            m.serialize_entry("hours", &Counts(h))?;
        }
        if let Some(t) = &self.tools {
            m.serialize_entry("tools", &Counts(t))?;
        }
        if self.gone {
            m.serialize_entry("gone", &true)?;
        }
        m.end()
    }
}

struct Pairs<'a, T>(&'a [(&'a str, T)]);

impl<T: Serialize> Serialize for Pairs<'_, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

fn retryable(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(32 | 33) if cfg!(windows)) || matches!(e.raw_os_error(), Some(16) if cfg!(unix))
}

pub fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 1;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if attempt < RENAME_RETRIES && retryable(&e) => {
                std::thread::sleep(std::time::Duration::from_millis(100 * u64::from(attempt)));
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

fn write_cache(file: &Path, body: &CacheBody) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(format!(".tmp-{}", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);
    let bytes = serde_json::to_vec(body).map_err(std::io::Error::other)?;
    let done = std::fs::write(&tmp, bytes).and_then(|_| rename_with_retry(&tmp, file));
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}
