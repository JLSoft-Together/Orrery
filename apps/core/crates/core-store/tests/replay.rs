use std::path::PathBuf;

use core_store::replay::replay;
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("replay").join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

fn same(a: &Value, b: &Value, at: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) if x.as_f64() == y.as_f64() => {}
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{at}[{i}]"), out);
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => same(p, q, &format!("{at}.{k}"), out),
                    (p, q) => out.push(format!("{at}.{k}: {p:?} != {q:?}")),
                }
            }
        }
        _ if a == b => {}
        _ => out.push(format!("{at}: {a} != {b}")),
    }
}

#[test]
fn monitor_scenario_replays_to_the_node_frames() {
    let ops = fixture("monitor.ops.json");
    let node = fixture("monitor.node.trace.json");
    let (rust, stats) = replay(&ops).unwrap();
    assert_eq!(stats.dropped, 0);
    assert_eq!(stats.mutated_refs, 0);
    let names = |t: &Value| t["steps"].as_array().unwrap().iter().map(|s| s["name"].clone()).collect::<Vec<_>>();
    assert_eq!(names(&rust), names(&node));
    let mut diffs = Vec::new();
    for (i, (r, n)) in rust["steps"].as_array().unwrap().iter().zip(node["steps"].as_array().unwrap()).enumerate() {
        same(&n["frames"], &r["frames"], &format!("step {i} ({})", n["name"]), &mut diffs);
    }
    assert!(diffs.is_empty(), "{}", diffs.join("\n"));
    let count = |t: &Value| t["steps"].as_array().unwrap().iter().map(|s| s["frames"].as_array().unwrap().len()).sum::<usize>();
    assert_eq!(stats.frames, count(&node));
    assert!(stats.frames >= 20, "{stats:?}");
}

fn doc(ops: Value) -> Value {
    json!({ "format": "ccm-store-ops/1", "header": { "scenario": "t", "wall": { "start": 0, "end": 0 } }, "ops": ops })
}

fn init() -> Value {
    json!({ "op": "init", "startedMs": 1_800_000_000_000_i64, "activityLimit": 10, "batchMs": 50 })
}

#[test]
fn replay_drops_flushes_nobody_received_and_keeps_object_identity() {
    let diff = json!({ "files": 1, "insertions": 2, "deletions": 0, "untracked": 0, "at": "2027-01-15T08:00:00.000Z" });
    let term = |title: &str| json!({ "id": "t", "title": title, "status": "idle", "diff": diff });
    let ops = json!([
        init(),
        { "op": "call", "name": "upsertTerminal", "args": [term("A")], "refs": { "diff": 1 } },
        { "op": "flush", "dropped": true },
        { "op": "step", "name": "s", "sent": [] },
        { "op": "call", "name": "upsertTerminal", "args": [term("A")], "refs": { "diff": 1 } },
        { "op": "call", "name": "upsertTerminal", "args": [term("B")], "refs": { "diff": 1 } },
        { "op": "call", "name": "upsertTerminal", "args": [term("B")], "refs": { "diff": 2 } },
        { "op": "call", "name": "pushActivity", "args": [{ "kind": "tool.started" }], "now": 1_800_000_000_100_i64, "at": "2027-01-15T08:00:00.101Z" },
        { "op": "flush", "dropped": false },
        { "op": "direct", "message": { "seq": -1, "direct": true, "events": [{ "type": "sessions.loading", "payload": { "loading": false } }] } },
        { "op": "snapshot" }
    ]);
    let (trace, stats) = replay(&doc(ops)).unwrap();
    assert_eq!((stats.dropped, stats.reused_refs, stats.clock_skews), (1, 2, 1));
    let frames = trace["steps"][0]["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0]["seq"], 2);
    let events = frames[0]["events"].as_array().unwrap();
    assert_eq!(events[0], json!({ "type": "terminal.updated", "payload": { "id": "t", "title": "B", "diff": diff } }));
    assert_eq!(events[1]["payload"]["at"], "2027-01-15T08:00:00.101Z");
    assert_eq!(events[1]["payload"]["id"], "mywpiwys-0");
    assert_eq!(frames[1]["direct"], true);
    assert_eq!(frames[2]["events"][0]["payload"]["collectorStartedAt"], "2027-01-15T08:00:00.000Z");
    assert_eq!(trace["source"], "rust");
}

#[test]
fn replay_reports_unknown_calls_and_unruled_object_fields() {
    let bad_call = json!([init(), { "op": "call", "name": "launchRockets", "args": [] }]);
    assert!(replay(&doc(bad_call)).unwrap_err().contains("unknown store call launchRockets"));
    let bad_ref = json!([init(), { "op": "call", "name": "upsertAgent", "args": [{ "id": "a", "terminalId": "t", "role": "main", "status": "idle" }], "refs": { "x": 1 } }]);
    assert!(replay(&doc(bad_ref)).unwrap_err().contains("no identity rule"));
    assert!(replay(&json!({ "format": "x" })).is_err());
}
