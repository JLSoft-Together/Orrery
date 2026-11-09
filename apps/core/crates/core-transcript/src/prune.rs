use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Clone, Copy)]
pub enum Shape {
    Record,
    Message,
    Content,
    Block,
    Texts,
    TextBlock,
    ToolResult,
}

enum Keep {
    Full,
    Present,
    Nested(Shape),
    Skip,
}

const RECORD_KEYS: [&str; 16] = [
    "type",
    "timestamp",
    "gitBranch",
    "permissionMode",
    "effort",
    "perTurnEffort",
    "isApiErrorMessage",
    "apiError",
    "quotaLimits",
    "isMeta",
    "origin",
    "uuid",
    "attachment",
    "content",
    "subtype",
    "compactMetadata",
];

fn keep(shape: Shape, key: &str) -> Keep {
    match (shape, key) {
        (Shape::Record, "message") => Keep::Nested(Shape::Message),
        (Shape::Record, "toolUseResult") => Keep::Nested(Shape::ToolResult),
        (Shape::Record, k) if RECORD_KEYS.contains(&k) => Keep::Full,
        (Shape::Message, "content") => Keep::Nested(Shape::Content),
        (Shape::Message, "model" | "id" | "usage" | "stop_reason") => Keep::Full,
        (Shape::Block, "content") => Keep::Nested(Shape::Texts),
        (Shape::Block, "type" | "id" | "name" | "input" | "text" | "tool_use_id" | "is_error") => Keep::Full,
        (Shape::TextBlock, "type" | "text") => Keep::Full,
        (Shape::ToolResult, "status" | "success" | "msg_id") => Keep::Full,
        (Shape::ToolResult, "pin" | "resumedAgentId") => Keep::Present,
        _ => Keep::Skip,
    }
}

fn element(shape: Shape) -> Option<Shape> {
    match shape {
        Shape::Content => Some(Shape::Block),
        Shape::Texts => Some(Shape::TextBlock),
        _ => None,
    }
}

pub struct Pruned(pub Shape);

impl<'de> DeserializeSeed<'de> for Pruned {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Pruned {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any")
    }
    fn visit_bool<E>(self, _: bool) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E>(self, _: i64) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_u64<E>(self, _: u64) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_f64<E>(self, _: f64) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(match element(self.0) {
            Some(_) => Value::String(s.to_owned()),
            None => Value::Null,
        })
    }
    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(match element(self.0) {
            Some(_) => Value::String(s),
            None => Value::Null,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let Some(inner) = element(self.0) else {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            return Ok(Value::Null);
        };
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(Pruned(inner))? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        if element(self.0).is_some() {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            return Ok(Value::Null);
        }
        let mut out = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            match keep(self.0, &key) {
                Keep::Full => {
                    let v = map.next_value::<Value>()?;
                    out.insert(key, v);
                }
                Keep::Present => {
                    map.next_value::<IgnoredAny>()?;
                    out.insert(key, Value::Null);
                }
                Keep::Nested(shape) => {
                    let v = map.next_value_seed(Pruned(shape))?;
                    out.insert(key, v);
                }
                Keep::Skip => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        Ok(Value::Object(out))
    }
}

pub fn record(line: &str) -> serde_json::Result<Value> {
    let mut de = serde_json::Deserializer::from_str(line);
    let v = Pruned(Shape::Record).deserialize(&mut de)?;
    de.end()?;
    Ok(v)
}
