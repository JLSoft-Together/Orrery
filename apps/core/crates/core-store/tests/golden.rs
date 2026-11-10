use std::collections::BTreeMap;
use std::path::PathBuf;

use core_store::{MonitorEvent, ServerMessage};
use serde_json::Value;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden")
}

fn same(a: &Value, b: &Value, at: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if x.as_f64() != y.as_f64() {
                out.push(format!("{at}: {x} != {y}"));
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{at}: length {} != {}", x.len(), y.len()));
                return;
            }
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{at}[{i}]"), out);
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) => same(v, w, &format!("{at}.{k}"), out),
                    None => out.push(format!("{at}.{k}: dropped")),
                }
            }
            for k in y.keys().filter(|k| !x.contains_key(*k)) {
                out.push(format!("{at}.{k}: added"));
            }
        }
        _ => {
            if a != b {
                out.push(format!("{at}: {a} != {b}"));
            }
        }
    }
}

fn event_type(v: &Value) -> String {
    v.get("type").and_then(Value::as_str).unwrap_or("?").to_string()
}

#[test]
fn golden_messages_round_trip() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .expect("tests/golden")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no golden samples");
    let mut per_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let doc: Value = serde_json::from_str(&text).unwrap();
        let messages = doc["messages"].as_array().unwrap_or_else(|| panic!("{}: messages", file.display()));
        for (i, raw) in messages.iter().enumerate() {
            let name = format!("{}#{i}", file.file_name().unwrap().to_string_lossy());
            let parsed: ServerMessage = match serde_json::from_value(raw.clone()) {
                Ok(m) => m,
                Err(e) => {
                    failures.push(format!("{name}: {e}"));
                    continue;
                }
            };
            let back = serde_json::to_value(&parsed).unwrap();
            let mut diffs = Vec::new();
            same(raw, &back, &name, &mut diffs);
            failures.extend(diffs);
            let text = serde_json::to_string(&parsed).unwrap();
            let again: ServerMessage = serde_json::from_str(&text).unwrap();
            assert_eq!(again, parsed, "{name}: second round trip");
            for e in raw["events"].as_array().unwrap() {
                *per_type.entry(event_type(e)).or_default() += 1;
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    for needed in ["snapshot", "terminal.created", "terminal.updated", "agent.created", "agent.updated", "activity", "tool.span", "toolSpans.data", "history.data", "diagnostics.data"] {
        assert!(per_type.contains_key(needed), "golden samples miss {needed}: {per_type:?}");
    }
}

#[test]
fn null_in_a_patch_survives_as_a_field_removal() {
    let raw = serde_json::json!({ "seq": 3, "events": [{ "type": "terminal.updated", "payload": { "id": "1:2", "waitingFor": null, "status": "idle" } }] });
    let msg: ServerMessage = serde_json::from_value(raw.clone()).unwrap();
    match &msg.events[0] {
        MonitorEvent::TerminalUpdated(p) => {
            assert_eq!(p.waiting_for, Some(None));
            assert_eq!(p.title, None);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(serde_json::to_value(&msg).unwrap(), raw);
}

#[test]
fn unknown_fields_are_refused() {
    let raw = serde_json::json!({ "seq": 1, "events": [{ "type": "agent.removed", "payload": { "id": "a", "extra": 1 } }] });
    assert!(serde_json::from_value::<ServerMessage>(raw).is_err());
}

#[test]
fn direct_replies_keep_the_flag() {
    let msg = ServerMessage::direct(MonitorEvent::Resources(serde_json::json!({ "at": "x" })));
    assert_eq!(serde_json::to_string(&msg).unwrap(), r#"{"seq":-1,"events":[{"type":"resources","payload":{"at":"x"}}],"direct":true}"#);
}
