use crate::jsonline::sanitize;
use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr,
    Obj(Vec<(String, J)>),
}

fn index_key(k: &str) -> Option<u32> {
    if k.is_empty() || k.len() > 10 || (k.len() > 1 && k.starts_with('0')) || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    k.parse::<u64>().ok().filter(|n| *n < u64::from(u32::MAX)).map(|n| n as u32)
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(v) => v.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn is_obj(&self) -> bool {
        matches!(self, J::Obj(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            J::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn values_js_order(&self) -> Vec<&J> {
        let J::Obj(v) = self else { return Vec::new() };
        let mut indexed: Vec<(u32, &J)> = v.iter().filter_map(|(k, x)| index_key(k).map(|i| (i, x))).collect();
        indexed.sort_by_key(|(i, _)| *i);
        let mut out: Vec<&J> = indexed.into_iter().map(|(_, x)| x).collect();
        out.extend(v.iter().filter(|(k, _)| index_key(k).is_none()).map(|(_, x)| x));
        out
    }
}

#[derive(Clone, Copy)]
enum Level {
    Record,
    Payload,
    Any(u8),
}

const PAYLOAD_SCALARS: [&str; 13] =
    ["id", "session_id", "timestamp", "cwd", "originator", "cli_version", "model", "effort", "type", "model_context_window", "call_id", "namespace", "name"];

fn child(level: Level, key: &str) -> Option<Level> {
    match (level, key) {
        (Level::Record, "type" | "timestamp") => Some(Level::Any(0)),
        (Level::Record, "payload") => Some(Level::Payload),
        (Level::Record, _) => None,
        (Level::Payload, "source") => Some(Level::Any(5)),
        (Level::Payload, "collaboration_mode" | "info" | "rate_limits") => Some(Level::Any(2)),
        (Level::Payload, k) if PAYLOAD_SCALARS.contains(&k) => Some(Level::Any(0)),
        (Level::Payload, _) => None,
        (Level::Any(n), _) => Some(Level::Any(n.saturating_sub(1))),
    }
}

struct Seed(Level);

impl<'de> DeserializeSeed<'de> for Seed {
    type Value = J;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<J, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed {
    type Value = J;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<J, E> {
        Ok(J::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<J, E> {
        Ok(J::Num(v as f64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<J, E> {
        Ok(J::Num(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<J, E> {
        Ok(J::Num(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<J, E> {
        Ok(J::Str(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<J, E> {
        Ok(J::Str(v))
    }

    fn visit_unit<E>(self) -> Result<J, E> {
        Ok(J::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<J, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(J::Arr)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<J, A::Error> {
        if matches!(self.0, Level::Any(0)) {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            return Ok(J::Obj(Vec::new()));
        }
        let mut out: Vec<(String, J)> = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            match child(self.0, &key) {
                Some(level) => {
                    let v = map.next_value_seed(Seed(level))?;
                    match out.iter_mut().find(|(k, _)| *k == key) {
                        Some(slot) => slot.1 = v,
                        None => out.push((key, v)),
                    }
                }
                None => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        Ok(J::Obj(out))
    }
}

fn attempt(line: &str) -> Option<J> {
    let mut de = serde_json::Deserializer::from_str(line);
    let v = Seed(Level::Record).deserialize(&mut de).ok()?;
    de.end().ok()?;
    Some(v)
}

pub fn parse_record(line: &str) -> Option<J> {
    attempt(line).or_else(|| sanitize(line).and_then(|fixed| attempt(&fixed)))
}

pub fn unescape(raw: &str) -> Option<String> {
    let quoted = format!("\"{raw}\"");
    serde_json::from_str::<String>(&quoted).ok().or_else(|| sanitize(&quoted).and_then(|f| serde_json::from_str::<String>(&f).ok()))
}
