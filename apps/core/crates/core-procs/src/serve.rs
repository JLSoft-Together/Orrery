use crate::lines::Lines;
use crate::tcp::TcpRow;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write;
use std::sync::mpsc;

pub const PIDS_MAX: usize = 4096;

#[derive(Debug, PartialEq)]
pub enum Op {
    Ping,
    Table,
    Usage(Vec<u32>),
    Cmdline(Vec<u32>),
    Ports,
    Bad(&'static str),
}

#[derive(Debug, PartialEq)]
pub struct Request {
    pub id: Value,
    pub op: Op,
}

fn pids(v: Option<&Value>) -> Option<Vec<u32>> {
    let Some(Value::Array(items)) = v else { return None };
    if items.len() > PIDS_MAX {
        return None;
    }
    let mut seen = HashSet::new();
    Some(
        items
            .iter()
            .filter_map(|p| {
                let f = p.as_f64()?;
                (f.fract() == 0.0 && f > 0.0 && f <= f64::from(u32::MAX)).then_some(f as u32)
            })
            .filter(|p| seen.insert(*p))
            .collect(),
    )
}

pub fn request(line: &[u8]) -> Option<Request> {
    let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(line) else { return None };
    let id = m.get("id").filter(|v| v.is_number() || v.is_string())?.clone();
    let with_pids = |make: fn(Vec<u32>) -> Op| pids(m.get("pids")).map_or(Op::Bad("pids"), make);
    let op = match m.get("op").and_then(Value::as_str) {
        Some("ping") => Op::Ping,
        Some("table") => Op::Table,
        Some("usage") => with_pids(Op::Usage),
        Some("cmdline") => with_pids(Op::Cmdline),
        Some("ports") => Op::Ports,
        _ => Op::Bad("op"),
    };
    Some(Request { id, op })
}

fn rows<T: Serialize>(id: &Value, rows: Option<Vec<T>>) -> Value {
    match rows.map(serde_json::to_value) {
        Some(Ok(rows)) => json!({ "id": id, "ok": true, "rows": rows }),
        _ => failed(id, "scan"),
    }
}

fn failed(id: &Value, kind: &str) -> Value {
    json!({ "id": id, "ok": false, "error": kind })
}

pub fn reply(req: &Request) -> Value {
    let id = &req.id;
    match &req.op {
        Op::Ping => json!({ "id": id, "ok": true, "version": crate::VERSION }),
        Op::Table => rows(id, crate::table()),
        Op::Usage(p) => rows(id, Some(crate::usage(p))),
        Op::Cmdline(p) => rows(id, Some(crate::cmdline(p))),
        Op::Ports => rows(id, crate::ports()),
        Op::Bad(kind) => failed(id, kind),
    }
}

fn write(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}").and_then(|_| out.flush());
}

pub fn next_batch(rx: &mpsc::Receiver<Value>) -> Option<Vec<Value>> {
    let first = rx.recv().ok()?;
    let mut ids = vec![first];
    ids.extend(rx.try_iter());
    Some(ids)
}

pub fn ports_replies(ids: &[Value], scan: impl FnOnce() -> Option<Vec<TcpRow>>) -> Vec<Value> {
    match scan().map(serde_json::to_value) {
        Some(Ok(found)) => ids.iter().map(|id| json!({ "id": id, "ok": true, "rows": found })).collect(),
        _ => ids.iter().map(|id| failed(id, "scan")).collect(),
    }
}

pub fn run() -> i32 {
    let (tx, rx) = mpsc::channel::<Value>();
    let worker = std::thread::Builder::new().name("procs-ports".into()).spawn(move || {
        while let Some(ids) = next_batch(&rx) {
            for v in ports_replies(&ids, crate::ports) {
                write(&v);
            }
        }
    });
    let Ok(worker) = worker else { return 1 };
    let mut lines = Lines::new(std::io::stdin().lock());
    while let Some(line) = lines.next_line() {
        let Some(req) = request(&line) else { continue };
        if req.op == Op::Ports {
            if let Err(mpsc::SendError(id)) = tx.send(req.id) {
                write(&failed(&id, "scan"));
            }
        } else {
            write(&reply(&req));
        }
    }
    drop(tx);
    let _ = worker.join();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_ports_requests_share_one_scan() {
        let (tx, rx) = mpsc::channel();
        for id in [json!(1), json!("b"), json!(3)] {
            tx.send(id).unwrap();
        }
        let ids = next_batch(&rx).unwrap();
        assert_eq!(ids, vec![json!(1), json!("b"), json!(3)]);
        let mut scans = 0;
        let row = TcpRow { pid: 7, state: 2, address: "127.0.0.1".into(), port: 80 };
        let out = ports_replies(&ids, || {
            scans += 1;
            Some(vec![row.clone()])
        });
        assert_eq!(scans, 1);
        assert_eq!(out.len(), 3);
        assert_eq!(out[1], json!({ "id": "b", "ok": true, "rows": [{ "pid": 7, "state": 2, "address": "127.0.0.1", "port": 80 }] }));
        assert_eq!(ports_replies(&ids[..1], || None), vec![json!({ "id": 1, "ok": false, "error": "scan" })]);
        drop(tx);
        assert!(next_batch(&rx).is_none());
    }

    #[test]
    fn parses_requests_and_rejects_broken_lines() {
        assert_eq!(request(br#"{"id":1,"op":"ping"}"#), Some(Request { id: json!(1), op: Op::Ping }));
        assert_eq!(request(br#"{"id":"a","op":"usage","pids":[3,3,0,-1,2.5,"7",4294967295,4294967296,9]}"#).unwrap().op, Op::Usage(vec![3, u32::MAX, 9]));
        assert_eq!(request(br#"{"id":2,"op":"cmdline","pids":"1"}"#).unwrap().op, Op::Bad("pids"));
        assert_eq!(request(br#"{"id":2,"op":"usage"}"#).unwrap().op, Op::Bad("pids"));
        assert_eq!(request(br#"{"id":2,"op":"nope"}"#).unwrap().op, Op::Bad("op"));
        assert_eq!(request(br#"{"id":2}"#).unwrap().op, Op::Bad("op"));
        assert!(request(b"not json").is_none());
        assert!(request(br#"[1,2]"#).is_none());
        assert!(request(br#"{"op":"ping"}"#).is_none());
        assert!(request(br#"{"id":null,"op":"ping"}"#).is_none());
        assert!(request(br#"{"id":{"x":1},"op":"ping"}"#).is_none());
        let many: Vec<u32> = (1..=PIDS_MAX as u32 + 1).collect();
        let line = json!({ "id": 3, "op": "usage", "pids": many }).to_string();
        assert_eq!(request(line.as_bytes()).unwrap().op, Op::Bad("pids"));
        let max: Vec<u32> = (1..=PIDS_MAX as u32).collect();
        let line = json!({ "id": 3, "op": "usage", "pids": max }).to_string();
        assert!(matches!(request(line.as_bytes()).unwrap().op, Op::Usage(p) if p.len() == PIDS_MAX));
    }

    #[test]
    fn replies_carry_the_id_and_shape() {
        assert_eq!(reply(&Request { id: json!("p"), op: Op::Ping }), json!({ "id": "p", "ok": true, "version": crate::VERSION }));
        assert_eq!(reply(&Request { id: json!(5), op: Op::Bad("op") }), json!({ "id": 5, "ok": false, "error": "op" }));
        let t = reply(&Request { id: json!(6), op: Op::Table });
        let me = std::process::id();
        let row = t["rows"].as_array().unwrap().iter().find(|r| r["pid"] == me).expect("own row").clone();
        assert!(row["ppid"].is_u64() && row["name"].is_string() && row["startMs"].is_i64() && row["startRaw"].is_string());
        let u = reply(&Request { id: json!(7), op: Op::Usage(vec![me]) });
        let mut keys: Vec<&String> = u["rows"][0].as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["cpuMs", "pid", "privateBytes", "workingSet"]);
        let c = reply(&Request { id: json!(8), op: Op::Cmdline(vec![]) });
        assert_eq!(c, json!({ "id": 8, "ok": true, "rows": [] }));
    }
}
