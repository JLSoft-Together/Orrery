use std::io::{BufRead, Write};

use core_toast::script::Script;
use serde_json::{json, Value};

fn main() {
    let mut script = Script::new();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(v) => script.run(&v).unwrap_or_else(|e| json!({ "ok": false, "error": e, "out": [] })),
            Err(e) => json!({ "ok": false, "error": e.to_string(), "out": [] }),
        };
        if writeln!(out, "{reply}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}
