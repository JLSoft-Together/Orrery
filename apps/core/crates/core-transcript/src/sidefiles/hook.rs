use super::common::{date, uuid};
use crate::jsstr::num as json_num;
use serde_json::{json, Value};

fn tool(s: &str) -> bool {
    (1..=80).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
}

pub fn hook(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let id = o.get("sessionId")?.as_str()?;
    if !uuid(id.as_bytes()) {
        return None;
    }
    let at = date(o.get("at"))?;
    let since = date(o.get("compactingSince"));
    let count = o
        .get("deniedCount")
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite() && n.fract() == 0.0 && *n > 0.0)
        .map_or(0.0, |n| n.min(1_000_000.0));
    let mut tools: Vec<&str> = o
        .get("deniedTools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|s| tool(s))
                .collect()
        })
        .unwrap_or_default();
    if tools.len() > 5 {
        tools.drain(..tools.len() - 5);
    }
    let mut r = json!({"sessionId": id, "at": at, "deniedCount": json_num(count), "deniedTools": tools});
    if let Some(s) = since {
        r["compactingSince"] = Value::String(s);
        let manual = o.get("compactTrigger").and_then(Value::as_str) == Some("manual");
        r["compactTrigger"] = json!(if manual { "manual" } else { "auto" });
    }
    Some(r)
}
