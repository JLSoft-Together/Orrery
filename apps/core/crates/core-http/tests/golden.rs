mod common;

use std::path::PathBuf;

use common::*;
use core_http::{start, Config, Hooks, StoreHandle};
use core_store::replay::replay_store;
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("core-store").join("tests").join("replay").join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

fn snapshot_ops(ops: &Value) -> Vec<usize> {
    ops["ops"].as_array().unwrap().iter().enumerate().filter(|(_, op)| op["op"] == "snapshot").map(|(i, _)| i).collect()
}

fn node_snapshot_frames(trace: &Value) -> Vec<Value> {
    trace["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["frames"].as_array().unwrap().iter())
        .filter(|f| f["events"].as_array().is_some_and(|e| e.iter().any(|e| e["type"] == "snapshot")))
        .cloned()
        .collect()
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

#[tokio::test]
async fn the_first_websocket_message_equals_the_snapshot_the_node_collector_sent() {
    let ops = fixture("monitor.ops.json");
    let node = node_snapshot_frames(&fixture("monitor.node.trace.json"));
    let at = snapshot_ops(&ops);
    assert_eq!(at.len(), node.len());
    assert!(at.len() >= 2);
    for (index, golden) in at.iter().zip(&node) {
        let mut upto = ops.clone();
        upto["ops"].as_array_mut().unwrap().truncate(index + 1);
        let (store, _) = replay_store(&upto).unwrap();
        let handle = StoreHandle::new(store);
        let server = start(Config::new(0, dist_with_index("ok")), handle, Hooks::default()).await.unwrap();
        let mut ws = ws_connect(server.port(), &[]).await.unwrap();
        let frame = next_json(&mut ws).await;
        let mut diffs = Vec::new();
        same(golden, &frame, "snapshot", &mut diffs);
        assert!(diffs.is_empty(), "op {index}: {}", diffs.join("\n"));
        assert!(frame["events"][0]["payload"]["collectorStartedAt"].is_string());
        server.shutdown().await;
    }
}
