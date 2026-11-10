use std::io::{BufRead, Write};

use core_monitor::script::Script;
use serde_json::{json, Value};

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut script: Option<Script> = None;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Err(e) => json!({ "ok": false, "error": e.to_string() }),
            Ok(v) if v.get("cmd").and_then(Value::as_str) == Some("init") => match Script::init(&v) {
                Ok(s) => {
                    script = Some(s);
                    json!({ "ok": true, "frames": [] })
                }
                Err(e) => json!({ "ok": false, "error": e }),
            },
            Ok(v) if v.get("cmd").and_then(Value::as_str) == Some("exit") => break,
            Ok(v) => match script.as_mut() {
                None => json!({ "ok": false, "error": "init first" }),
                Some(s) => s.run(&v).unwrap_or_else(|e| json!({ "ok": false, "error": e })),
            },
        };
        if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
    if let Some(mut s) = script {
        s.monitor.stop();
    }
}
