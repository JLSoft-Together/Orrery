use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Core {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::Receiver<Value>,
    backlog: Vec<Value>,
}

impl Core {
    fn start() -> Core {
        let mut child = Command::new(env!("CARGO_BIN_EXE_orrery-core")).arg("transcript").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(serde_json::from_str::<Value>(&line).expect("one JSON line")).is_err() {
                    break;
                }
            }
        });
        Core { stdin: child.stdin.take(), child, rx, backlog: Vec::new() }
    }

    fn send(&mut self, v: Value) {
        let s = self.stdin.as_mut().unwrap();
        s.write_all(format!("{v}\n").as_bytes()).unwrap();
        s.flush().unwrap();
    }

    fn next(&mut self) -> Value {
        self.rx.recv_timeout(Duration::from_secs(20)).expect("line in time")
    }

    fn until(&mut self, what: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.backlog.iter().position(&what) {
            return self.backlog.remove(i);
        }
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            let v = self.rx.recv_timeout(end.saturating_duration_since(Instant::now())).expect("reply in time");
            if what(&v) {
                return v;
            }
            self.backlog.push(v);
        }
    }

    fn ask(&mut self, v: Value) -> Value {
        let id = v["id"].clone();
        self.send(v);
        self.until(|r| r["id"] == id)
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assistant(id: &str, out: u32) -> String {
    format!("{}\n", json!({ "type": "assistant", "timestamp": "2026-10-07T09:00:00.000Z", "cwd": "D:\\x", "message": { "id": id, "model": "claude-opus-5-5", "usage": { "output_tokens": out } } }))
}

fn root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-core-transcript-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("projects/p")).unwrap();
    dir
}

#[test]
fn scan_notify_query_and_save_on_stdin_eof() {
    let dir = root("main");
    let file = dir.join("projects/p/s.jsonl");
    std::fs::write(&file, assistant("m1", 3)).unwrap();
    let cache = dir.join("data/usage-index.json");
    let mut c = Core::start();
    assert_eq!(c.ask(json!({ "id": 1, "op": "ping" }))["ok"], true);
    assert_eq!(c.ask(json!({ "id": 2, "op": "usage.query", "q": "scanInfo" })), json!({ "id": 2, "ok": false, "error": "closed" }));
    assert_eq!(c.ask(json!({ "id": 3, "op": "usage.scan" }))["error"], "projectsDir");
    let open = c.ask(json!({ "id": 4, "op": "usage.scan", "projectsDir": dir.join("projects"), "cacheFile": cache, "sweepMs": 0, "fileDebounceMs": 1, "saveDebounceMs": 1 }));
    assert_eq!(open["snapshot"]["scan"]["state"], "idle");
    let reset = c.until(|v| v["event"] == "usage.reset");
    assert_eq!(reset["snapshot"]["buckets"][0]["output"], 3);
    assert_eq!(c.ask(json!({ "id": 5, "op": "usage.scan", "projectsDir": "x" }))["error"], "open");
    std::fs::OpenOptions::new().append(true).open(&file).unwrap().write_all(assistant("m2", 4).as_bytes()).unwrap();
    c.send(json!({ "op": "usage.notify", "files": ["p/s.jsonl"] }));
    let update = c.until(|v| v["event"] == "usage.update");
    assert_eq!(update["buckets"][0]["messages"], 2);
    assert_eq!(c.ask(json!({ "id": 6, "op": "usage.idle" })), json!({ "id": 6, "ok": true }));
    let heat = c.ask(json!({ "id": 7, "op": "usage.query", "q": "heatmap", "days": 1, "today": "2026-10-07" }));
    assert_eq!(heat["heatmap"]["days"][0]["hours"].as_array().unwrap().iter().filter_map(Value::as_u64).sum::<u64>(), 2);
    assert_eq!(c.ask(json!({ "id": 8, "op": "usage.query", "q": "heatmap", "days": 1.5 }))["error"], "days");
    assert_eq!(c.ask(json!({ "id": 9, "op": "usage.query", "q": "toolStats", "days": 7, "project": 3 }))["error"], "project");
    assert_eq!(c.ask(json!({ "id": 10, "op": "nope" }))["error"], "op");
    drop(c.stdin.take());
    let status = c.child.wait().unwrap();
    assert!(status.success());
    let saved: Value = serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
    assert_eq!(saved["version"], 4);
    assert_eq!(saved["files"]["p/s.jsonl"]["ids"], json!(["m1", "m2"]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_reply_always_comes_before_scan_events() {
    let base = root("order");
    let dir = base.join("projects");
    for i in 0..5 {
        std::fs::write(dir.join(format!("p/s{i}.jsonl")), assistant(&format!("m{i}"), 1)).unwrap();
    }
    for round in 0..20 {
        let mut c = Core::start();
        c.send(json!({ "id": round, "op": "usage.scan", "projectsDir": dir, "cacheFile": null, "sweepMs": 0 }));
        let first = c.next();
        assert_eq!(first["id"], round, "round {round}: {first}");
        assert_eq!(first["ok"], true);
        let reset = c.until(|v| v["event"] == "usage.reset");
        assert_eq!(reset["snapshot"]["buckets"][0]["messages"], 5);
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn signals_open_read_close_next_to_usage() {
    let dir = root("signals");
    let file = dir.join("projects/p/s.jsonl");
    std::fs::write(&file, format!("{}{{\"type\":\"user\"", assistant("m1", 3))).unwrap();
    let mut c = Core::start();
    assert_eq!(c.ask(json!({ "id": 1, "op": "signals.open" }))["error"], "file");
    assert_eq!(c.ask(json!({ "id": 2, "op": "signals.open", "file": file, "lite": "yes" }))["error"], "lite");
    let open = c.ask(json!({ "id": 3, "op": "signals.open", "file": file, "spans": { "terminalId": "t", "agentId": "main" } }));
    let handle = open["handle"].clone();
    let first = c.ask(json!({ "id": 4, "op": "signals.read", "handle": handle }));
    assert_eq!(first["signals"], json!([{ "k": "seen", "at": "2026-10-07T09:00:00.000Z" }, { "k": "model", "model": "claude-opus-5-5" }, { "k": "usage", "messageId": "m1", "input": 0, "output": 3, "cacheRead": 0, "at": "2026-10-07T09:00:00.000Z" }]));
    assert_eq!(first["state"]["tokens"]["output"], 3);
    assert_eq!(first["spans"], json!([]));
    std::fs::OpenOptions::new().append(true).open(&file).unwrap().write_all(b",\"message\":{\"content\":\"hi\"}}\n").unwrap();
    let second = c.ask(json!({ "id": 5, "op": "signals.read", "handle": handle }));
    assert_eq!(second["signals"], json!([{ "k": "prompt" }]));
    assert_eq!(c.ask(json!({ "id": 6, "op": "usage.query", "q": "scanInfo" }))["error"], "closed");
    assert_eq!(c.ask(json!({ "id": 7, "op": "signals.close", "handle": handle })), json!({ "id": 7, "ok": true }));
    assert_eq!(c.ask(json!({ "id": 8, "op": "signals.read", "handle": handle }))["error"], "handle");
    drop(c.stdin.take());
    assert!(c.child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn signals_need_an_id_cap_ids_and_page_by_bytes() {
    let dir = root("signals-page");
    let file = dir.join("projects/p/s.jsonl");
    let body: String = (0..300).map(|i| assistant(&format!("m{i}"), 1)).collect();
    std::fs::write(&file, &body).unwrap();
    let mut c = Core::start();
    c.send(json!({ "op": "signals.open", "file": file }));
    let ids: Vec<String> = (0..20_001).map(|i| format!("t{i}")).collect();
    assert_eq!(c.ask(json!({ "id": 1, "op": "signals.open", "file": file, "lite": true, "toolUseIds": ids }))["error"], "toolUseIds");
    assert_eq!(c.ask(json!({ "id": 2, "op": "signals.nope" }))["error"], "op");
    let open = c.ask(json!({ "id": 3, "op": "signals.open", "file": file }));
    assert_eq!(open["handle"], 1);
    let (mut usage, mut pages) = (0, 0);
    loop {
        let r = c.ask(json!({ "id": 10 + pages, "op": "signals.read", "handle": 1, "maxBytes": 4096 }));
        usage += r["signals"].as_array().unwrap().iter().filter(|s| s["k"] == "usage").count();
        pages += 1;
        assert_eq!(c.ask(json!({ "id": "p", "op": "ping" }))["ok"], true);
        if r.get("more").is_none() {
            break;
        }
        assert_eq!(r["more"], true);
    }
    assert_eq!(usage, 300);
    assert!(pages > 5);
    drop(c.stdin.take());
    assert!(c.child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}
