use super::host::{Args, ToolHost};
use super::registry::{Bridge, Registry};
use super::rpc::{handle_mcp, mcp_gate, Gate};
use super::server::McpServer;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

struct Fake {
    pages: Mutex<Vec<Value>>,
}

impl ToolHost for Fake {
    fn root(&self) -> String {
        std::env::temp_dir().to_string_lossy().into_owned()
    }
    fn preset(&self) -> Option<String> {
        Some("claude".into())
    }
    fn take(&self, _key: &str, _max: usize, _window_ms: u64) -> bool {
        true
    }
    fn open_session(&self, _args: &Args) -> Value {
        json!({ "ok": false, "error": "limit" })
    }
    fn notify(&self, _text: &str, _level: &str) -> Value {
        json!({ "ok": true, "sent": true, "name": "p", "inbox": true })
    }
    fn checkout_roots(&self) -> Vec<String> {
        vec![]
    }
    fn page(&self, call: Value) -> Value {
        self.pages.lock().unwrap().push(call);
        json!({ "ok": true, "tool": "list_sessions", "sessions": [] })
    }
    fn desk(&self, _call: Value) -> Option<Value> {
        None
    }
}

fn fake() -> Fake {
    Fake { pages: Mutex::new(vec![]) }
}

#[test]
fn json_rpc_shapes_match_the_ts_handler() {
    let h = fake();
    let (s, b) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } }), &h, "1.2.3", false);
    let b = b.unwrap();
    assert_eq!(s, 200);
    assert_eq!(b["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(b["result"]["serverInfo"], json!({ "name": "orrery", "version": "1.2.3" }));
    assert!(!b["result"]["instructions"].as_str().unwrap().contains("hand_back"));
    let (_, list) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": "x", "method": "tools/list" }), &h, "1", false);
    let tools = list.unwrap()["result"]["tools"].as_array().unwrap().clone();
    assert_eq!(tools[0]["name"], "open_session");
    assert_eq!(tools.len(), 18);
    assert!(!tools.iter().any(|t| t["name"] == "hand_back"));
    assert_eq!(handle_mcp(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }), &h, "1", false), (202, None));
    let (_, e) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "hand_back" } }), &h, "1", false);
    assert_eq!(e.unwrap()["error"]["message"], "Unknown tool");
    let (_, e) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "list_sessions", "arguments": [1] } }), &h, "1", false);
    assert_eq!(e.unwrap()["error"]["code"], -32602);
    let (_, e) = handle_mcp(&json!([]), &h, "1", false);
    assert_eq!(e.unwrap()["error"]["code"], -32600);
    let many: Vec<Value> = (0..17).map(|i| json!({ "jsonrpc": "2.0", "id": i, "method": "ping" })).collect();
    let (_, e) = handle_mcp(&Value::Array(many), &h, "1", false);
    assert_eq!(e.unwrap()["error"]["message"], "Invalid Request: at most 16 messages per batch");
    let (_, r) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "list_sessions" } }), &h, "1", false);
    assert_eq!(r.unwrap()["result"]["content"][0]["text"], "No session of this project is open in the workspace.");
    assert_eq!(h.pages.lock().unwrap()[0], json!({ "tool": "list_sessions" }));
    assert_eq!(handle_mcp(&json!({ "jsonrpc": "2.0", "id": 4, "result": {} }), &h, "1", false), (202, None));
    let (_, e) = handle_mcp(&json!({ "jsonrpc": "2.0", "id": {}, "method": "ping" }), &h, "1", false);
    assert_eq!(e.unwrap()["error"]["code"], -32600);
}

#[test]
fn the_gate_wants_loopback_host_no_origin_post_and_a_bearer() {
    let tok = "a".repeat(64);
    let auth = format!("Bearer {tok}");
    let g = |m: &str, u: &str, h: Option<&str>, o: Option<&str>, a: Option<&str>| match mcp_gate(m, u, h, o, a, 5000) {
        Gate::Deny(s, _) => s,
        Gate::Token(_) => 0,
    };
    assert_eq!(g("POST", "/x", Some("127.0.0.1:5000"), None, Some(&auth)), 404);
    assert_eq!(g("POST", "/mcp", Some("localhost:5000"), None, Some(&auth)), 403);
    assert_eq!(g("POST", "/mcp", Some("127.0.0.1:5000"), Some("http://evil"), Some(&auth)), 403);
    assert_eq!(g("GET", "/mcp", Some("127.0.0.1:5000"), None, Some(&auth)), 405);
    assert_eq!(g("POST", "/mcp", Some("127.0.0.1:5000"), None, Some("Bearer short")), 401);
    assert_eq!(g("POST", "/mcp?x=1", Some("127.0.0.1:5000"), None, Some(&format!("bearer {tok}"))), 0);
}

struct Pages {
    asked: Mutex<Vec<(String, Value)>>,
    reg: Mutex<Option<Arc<Registry>>>,
}

impl Bridge for Pages {
    fn ask(&self, action: &str, args: Value, _pane: &str, _timeout_ms: u64) -> Value {
        self.asked.lock().unwrap().push((action.to_string(), args.clone()));
        if action == "session.open" {
            let reg = self.reg.lock().unwrap().clone().unwrap();
            let open = args["id"].as_str().unwrap().to_string();
            let preset = args["preset"].as_str().unwrap().to_string();
            std::thread::spawn(move || {
                let t = reg.take_ticket(&open, &preset);
                assert!(t.is_some());
                reg.add("new-pane", &preset, "/", None, Some(&open));
                reg.spawned("new-pane", 42);
            });
            return json!({ "ok": true, "paneKey": "k" });
        }
        json!({ "ok": true, "name": "proj", "inbox": true })
    }
    fn desk(&self, _channel: &str, _call: Value) -> Value {
        json!({ "ok": false, "error": "no-report" })
    }
    fn note(&self, _note: Value) {}
}

fn post(port: u16, token: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let req = format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let status = out[9..12].parse().unwrap();
    (status, out.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
}

#[test]
fn a_pane_token_reaches_its_tools_over_http_and_open_session_waits_for_the_spawn() {
    let bridge = Arc::new(Pages { asked: Mutex::new(vec![]), reg: Mutex::new(None) });
    let reg = Registry::new(bridge.clone(), false, std::env::temp_dir().to_string_lossy().into_owned(), Arc::new(crate::util::now_ms));
    *bridge.reg.lock().unwrap() = Some(reg.clone());
    let cwd = super::paths::real_of(&std::env::temp_dir().to_string_lossy()).unwrap();
    let token = reg.add("p1", "claude", &cwd, None, None);
    let r2 = reg.clone();
    let server = McpServer::start(Arc::new(move |t: &str| r2.caller(t)), "1.0.0", false).unwrap();
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    assert_eq!(post(server.port(), &token, ping).0, 401);
    reg.spawned("p1", 7);
    assert_eq!(post(server.port(), &"b".repeat(64), ping).0, 401);
    let (status, body) = post(server.port(), &token, ping);
    assert_eq!((status, body.as_str()), (200, r#"{"jsonrpc":"2.0","id":1,"result":{}}"#));
    assert_eq!(post(server.port(), &token, "{nope").0, 400);
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"open_session","arguments":{"agent":"codex","prompt":"go","submit":true}}}"#;
    let (_, body) = post(server.port(), &token, call);
    let v: Value = serde_json::from_str(&body).unwrap();
    let sc = &v["result"]["structuredContent"];
    assert_eq!(sc["paneId"], "new-pane");
    assert_eq!(sc["submitted"], true);
    assert_eq!(sc["preset"], "codex");
    let asked = bridge.asked.lock().unwrap()[0].1.clone();
    assert_eq!(asked["from"], "p1");
    assert_eq!(asked["submit"], true);
    let shell = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"open_session","arguments":{"agent":"shell","prompt":"rm -rf /","submit":true}}}"#;
    let (_, body) = post(server.port(), &token, shell);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["result"]["structuredContent"].get("submitted").is_none());
    assert_eq!(v["result"]["structuredContent"]["prompt"], false);
    let deep = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"open_session","arguments":{"agent":"deepseek"}}}"#;
    let (_, body) = post(server.port(), &token, deep);
    assert!(body.contains("\"error\":\"preset\""));
    let servers = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_servers"}}"#;
    let (_, body) = post(server.port(), &token, servers);
    assert!(body.contains("no-report"));
    reg.removed("p1");
    assert_eq!(post(server.port(), &token, ping).0, 401);
    server.stop();
}

#[test]
fn opens_stop_at_sixteen_panes() {
    let bridge = Arc::new(Pages { asked: Mutex::new(vec![]), reg: Mutex::new(None) });
    let reg = Registry::new(bridge, false, std::env::temp_dir().to_string_lossy().into_owned(), Arc::new(crate::util::now_ms));
    reg.add("p1", "claude", "/", None, None);
    reg.spawned("p1", 1);
    for i in 0..15 {
        reg.add(&format!("x{i}"), "shell", "/", None, None);
    }
    let out = reg.open_from("p1", &Args::new());
    assert_eq!(out, json!({ "ok": false, "error": "limit" }));
    assert_eq!(reg.live_count(), 16);
}
