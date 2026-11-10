use super::common::{date, num, pct};
use crate::jsstr::num as json_num;
use serde_json::{json, Value};

fn window(v: Option<&Value>) -> Option<Value> {
    let o = v?.as_object()?;
    let used = num(o.get("usedPct"))?;
    let mut r = json!({"usedPct": json_num(used.clamp(0.0, 100.0))});
    if let Some(s) = date(o.get("resetsAt")) {
        r["resetsAt"] = Value::String(s);
    }
    Some(r)
}

pub fn status(v: &Value) -> Option<Value> {
    let o = v.as_object()?;
    let id = o.get("sessionId")?.as_str()?;
    let at = date(o.get("at"))?;
    let mut r = json!({"sessionId": id, "at": at});
    if let Some(n) = num(o.get("costUsd")) {
        r["costUsd"] = json_num(n);
    }
    if let Some(n) =
        num(o.get("contextWindow")).filter(|n| (1000.0..=100_000_000.0).contains(n))
    {
        r["contextWindow"] = json_num(n.round());
    }
    for key in ["contextPct", "bufferPct"] {
        if let Some(n) = pct(o.get(key)) {
            r[key] = json_num(n);
        }
    }
    for key in ["fiveHour", "sevenDay"] {
        if let Some(w) = window(o.get(key)) {
            r[key] = w;
        }
    }
    Some(r)
}
