use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Procs {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::Receiver<Value>,
}

impl Procs {
    fn start() -> Procs {
        let mut child = Command::new(env!("CARGO_BIN_EXE_orrery-core")).arg("procs").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let v: Value = serde_json::from_str(&line).expect("every reply is one JSON line");
                if tx.send(v).is_err() {
                    break;
                }
            }
        });
        Procs { stdin: child.stdin.take(), child, rx }
    }

    fn raw(&mut self, bytes: &[u8]) {
        let s = self.stdin.as_mut().unwrap();
        s.write_all(bytes).unwrap();
        s.flush().unwrap();
    }

    fn ask(&mut self, v: Value) -> Value {
        let id = v["id"].clone();
        self.raw(format!("{v}\n").as_bytes());
        self.wait(&id)
    }

    fn wait(&mut self, id: &Value) -> Value {
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            let left = end.saturating_duration_since(Instant::now());
            let v = self.rx.recv_timeout(left).unwrap_or_else(|_| panic!("no reply for id {id}"));
            if &v["id"] == id {
                return v;
            }
        }
    }

    fn rows(&mut self, v: Value) -> Vec<Value> {
        let r = self.ask(v);
        assert_eq!(r["ok"], true, "{r}");
        r["rows"].as_array().unwrap().clone()
    }
}

impl Drop for Procs {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn node_child(marker: &str) -> Child {
    Command::new("node").args(["-e", "setInterval(() => {}, 1000)", marker]).stdin(Stdio::null()).stdout(Stdio::null()).spawn().expect("node on PATH")
}

#[test]
fn ping_table_usage_cmdline_ports_and_errors() {
    let mut p = Procs::start();
    let pong = p.ask(json!({ "id": 1, "op": "ping" }));
    assert_eq!(pong["ok"], true);
    assert!(pong["version"].is_string());

    let mut node = node_child("ccm-procs-marker-1");
    let me = std::process::id();
    let core = p.child.id();
    let table = p.rows(json!({ "id": 2, "op": "table" }));
    let find = |pid: u32| table.iter().find(|r| r["pid"] == pid).cloned();
    let core_row = find(core).expect("core row");
    assert_eq!(core_row["ppid"], me);
    assert_eq!(find(node.id()).expect("node row")["ppid"], me);
    assert!(core_row["name"].as_str().unwrap().starts_with("orrery-core"));
    assert!(core_row["startMs"].is_i64() && core_row["startRaw"].is_string());
    #[cfg(windows)]
    assert!(core_row["startRaw"].as_str().unwrap().bytes().all(|b| b.is_ascii_digit()));

    let pids = json!([node.id(), node.id(), 0, u32::MAX - 3]);
    let usage = p.rows(json!({ "id": "u", "op": "usage", "pids": pids }));
    assert_eq!(usage.len(), 1, "{usage:?}");
    assert_eq!(usage[0]["pid"], node.id());
    assert!(usage[0]["workingSet"].as_u64().unwrap() > 1_000_000);
    assert!(usage[0]["cpuMs"].as_f64().unwrap() >= 0.0);
    let mut cmd = Vec::new();
    for _ in 0..50 {
        cmd = p.rows(json!({ "id": "c", "op": "cmdline", "pids": [node.id()] }));
        if cmd.first().and_then(|c| c["cmd"].as_str()).is_some_and(|c| c.contains("ccm-procs-marker-1")) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(cmd[0]["cmd"].as_str().unwrap().contains("setInterval"), "{cmd:?}");
    let _ = node.kill();
    let _ = node.wait();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let ports = p.rows(json!({ "id": 5, "op": "ports" }));
    assert!(ports.iter().any(|r| r["pid"] == me && r["port"] == port && r["state"] == 2 && r["address"] == "127.0.0.1"), "{port} missing");

    p.raw(b"not json\n{}\n[1]\n{\"op\":\"ping\"}\n\n\r\n");
    let mut big = vec![b'x'; 1024 * 1024 + 100];
    big.push(b'\n');
    p.raw(&big);
    p.raw(b"{\"id\":6,\"op\":\"ping\"}\n");
    let first = p.rx.recv_timeout(Duration::from_secs(10)).expect("ping reply");
    assert_eq!(first["id"], 6, "broken lines got an answer: {first}");
    assert_eq!(p.ask(json!({ "id": 7, "op": "nope" })), json!({ "id": 7, "ok": false, "error": "op" }));
    let many: Vec<u32> = (1..=4097).collect();
    assert_eq!(p.ask(json!({ "id": 8, "op": "usage", "pids": many })), json!({ "id": 8, "ok": false, "error": "pids" }));
    assert_eq!(p.ask(json!({ "id": 9, "op": "cmdline", "pids": "1" }))["error"], "pids");
}

#[test]
fn a_burst_of_ports_requests_is_answered_for_every_id() {
    let mut p = Procs::start();
    let burst: String = (0..20).map(|i| format!("{}\n", json!({ "id": format!("p{i}"), "op": "ports" }))).collect();
    p.raw(burst.as_bytes());
    let replies: Vec<Value> = (0..20).map(|i| p.wait(&json!(format!("p{i}")))).collect();
    assert!(replies.iter().all(|r| r["ok"] == true && r["rows"].is_array()));
}

#[test]
fn stdin_eof_exits_zero() {
    let mut p = Procs::start();
    assert_eq!(p.ask(json!({ "id": 1, "op": "ping" }))["ok"], true);
    p.raw(br#"{"id":2,"op":"ports"}"#);
    p.raw(b"\n");
    p.stdin.take();
    let end = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(s) = p.child.try_wait().unwrap() {
            break s;
        }
        assert!(Instant::now() < end, "procs did not exit on stdin EOF");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(0));
    assert_eq!(p.wait(&json!(2))["ok"], true, "pending ports answered before exit");
}

fn pct(mut v: Vec<f64>) -> (f64, f64) {
    v.sort_by(f64::total_cmp);
    (v[v.len() / 2], v[v.len() - 1])
}

#[test]
#[ignore]
fn measure_table_usage_ports_and_rss() {
    let mut p = Procs::start();
    p.ask(json!({ "id": 0, "op": "ping" }));
    let mut id = 1;
    let mut time = |p: &mut Procs, op: Value, n: usize| -> (Vec<f64>, usize) {
        let mut ms = Vec::new();
        let mut rows = 0;
        for _ in 0..n {
            let mut v = op.clone();
            v["id"] = json!(id);
            id += 1;
            let t = Instant::now();
            rows = p.rows(v).len();
            ms.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        (ms, rows)
    };
    let (t, n_table) = time(&mut p, json!({ "op": "table" }), 50);
    let table = p.rows(json!({ "id": "t", "op": "table" }));
    let all: Vec<u64> = table.iter().filter_map(|r| r["pid"].as_u64()).filter(|&pid| pid > 4).collect();
    let open = p.rows(json!({ "id": "open", "op": "usage", "pids": all }));
    let pids: Vec<u64> = open.iter().filter_map(|r| r["pid"].as_u64()).take(50).collect();
    println!("pids asked: {}", pids.len());
    let (u, n_usage) = time(&mut p, json!({ "op": "usage", "pids": pids }), 50);
    let (c, n_cmd) = time(&mut p, json!({ "op": "cmdline", "pids": pids }), 50);
    let (ports, n_ports) = time(&mut p, json!({ "op": "ports" }), 50);
    let rss = p.rows(json!({ "id": "rss", "op": "usage", "pids": [p.child.id()] }));
    let report = |name: &str, v: Vec<f64>, rows: usize| {
        let (p50, max) = pct(v);
        println!("{name}: p50 {p50:.2} ms, max {max:.2} ms, rows {rows}");
    };
    report("table", t, n_table);
    report("usage 50", u, n_usage);
    report("cmdline 50", c, n_cmd);
    report("ports", ports, n_ports);
    println!("procs RSS {:.1} MB, private {:.1} MB", rss[0]["workingSet"].as_f64().unwrap_or(0.0) / 1e6, rss[0]["privateBytes"].as_f64().unwrap_or(0.0) / 1e6);
}
