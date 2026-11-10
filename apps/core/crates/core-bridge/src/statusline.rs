use crate::jsfmt::{self, num};
use serde_json::{Map, Value};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn window(w: Option<&Value>) -> Option<Value> {
    let used = num(w.and_then(|w| w.get("used_percentage")))?;
    let mut out = Map::new();
    out.insert("usedPct".into(), Value::from(used.clamp(0.0, 100.0)));
    if let Some(r) = num(w.and_then(|w| w.get("resets_at"))).filter(|r| *r != 0.0) {
        out.insert("resetsAt".into(), Value::String(jsfmt::iso_string(r * 1000.0)?));
    }
    Some(Value::Object(out))
}

fn buffer_pct(cw: Option<&Value>, auto_window: Option<&str>) -> Option<f64> {
    let acw = jsfmt::parse_int(auto_window.unwrap_or("0"))?;
    if acw <= 0 {
        return None;
    }
    let total = num(cw.and_then(|c| c.get("total_tokens"))).filter(|t| *t != 0.0).unwrap_or(1_000_000.0);
    Some((acw as f64 / total * 100.0).min(100.0))
}

fn context_pct(cw: Option<&Value>) -> Option<f64> {
    let remaining = num(cw.and_then(|c| c.get("remaining_percentage")));
    let used = match remaining {
        Some(r) => Some(100.0 - r),
        None => num(cw.and_then(|c| c.get("used_percentage"))),
    };
    used.map(|u| u.clamp(0.0, 100.0))
}

fn put(m: &mut Map<String, Value>, k: &str, v: Option<f64>) {
    if let Some(v) = v {
        m.insert(k.into(), Value::from(v));
    }
}

pub fn record(input: &Value, now_ms: f64, auto_window: Option<&str>) -> Option<Value> {
    let id = input.get("session_id").and_then(Value::as_str).filter(|s| crate::is_session_id(s))?;
    let cw = input.get("context_window");
    let limits = input.get("rate_limits");
    let mut m = Map::new();
    m.insert("v".into(), Value::from(1));
    m.insert("sessionId".into(), Value::String(id.to_string()));
    m.insert("at".into(), Value::String(jsfmt::iso_string(now_ms)?));
    put(&mut m, "costUsd", num(input.get("cost").and_then(|c| c.get("total_cost_usd"))));
    put(&mut m, "contextWindow", num(cw.and_then(|c| c.get("context_window_size"))));
    put(&mut m, "contextPct", context_pct(cw));
    put(&mut m, "bufferPct", buffer_pct(cw, auto_window));
    let five = limits.and_then(|l| l.get("five_hour"));
    let seven = limits.and_then(|l| l.get("seven_day"));
    if five.and_then(|w| num(w.get("used_percentage"))).is_some() {
        m.insert("fiveHour".into(), window(five)?);
    }
    if seven.and_then(|w| num(w.get("used_percentage"))).is_some() {
        m.insert("sevenDay".into(), window(seven)?);
    }
    Some(Value::Object(m))
}

pub fn save(input: &Value, data_dir: &Path) {
    let auto = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW").ok();
    let Some(rec) = record(input, jsfmt::now_ms(), auto.as_deref().filter(|s| !s.is_empty())) else { return };
    let Some(id) = input.get("session_id").and_then(Value::as_str) else { return };
    let dir = data_dir.join("statusline");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let file = dir.join(format!("{id}.json"));
    let tmp = dir.join(format!("{id}.json.tmp"));
    let _ = crate::write_replace(&file, &tmp, &jsfmt::stringify(&rec));
}

fn truthy_text(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Bool(true) => Some("true".into()),
        Value::Number(n) => n.as_f64().filter(|f| *f != 0.0).map(jsfmt::number),
        Value::Object(_) => Some("[object Object]".into()),
        Value::Array(a) => Some(a.iter().map(|x| match x {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => jsfmt::stringify(other),
        }).collect::<Vec<_>>().join(",")),
        _ => None,
    }
}

pub fn default_line(input: Option<&Value>) -> String {
    let get = |path: &[&str]| -> Option<&Value> {
        let mut cur = input?;
        for p in path {
            cur = cur.get(*p)?;
        }
        Some(cur)
    };
    let mut parts = vec![truthy_text(get(&["model", "display_name"])).unwrap_or_else(|| "Claude".into())];
    if let Some(c) = num(get(&["context_window", "used_percentage"])) {
        parts.push(format!("ctx {}%", jsfmt::number(jsfmt::round(c))));
    }
    if let Some(f) = num(get(&["rate_limits", "five_hour", "used_percentage"])) {
        parts.push(format!("5h {}%", jsfmt::number(jsfmt::round(f))));
    }
    if let Some(c) = num(get(&["cost", "total_cost_usd"])) {
        parts.push(format!("${}", jsfmt::to_fixed(c, 2)));
    }
    parts.join(" | ")
}

pub fn chain_command(args: &[String]) -> String {
    if let Some(b) = args.iter().position(|a| a == "--chain-b64") {
        let raw = args.get(b + 1).map(String::as_str).unwrap_or("");
        return String::from_utf8_lossy(&jsfmt::b64_decode(raw)).into_owned();
    }
    if let Some(i) = args.iter().position(|a| a == "--chain") {
        return args[i + 1..].join(" ");
    }
    String::new()
}

fn shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let comspec = std::env::var("ComSpec").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "cmd.exe".into());
        let mut cmd = Command::new(comspec);
        cmd.raw_arg(format!("/d /s /c \"{command}\""));
        cmd.creation_flags(0x0800_0000);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

pub fn chain(command: &str, raw: &[u8]) -> bool {
    let Ok(mut child) = shell(command).stdin(Stdio::piped()).stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn() else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(raw);
    }
    let _ = child.wait();
    true
}

pub fn run(args: &[String], raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let input: Option<Value> = serde_json::from_str(&text).ok();
    if let Some(v) = &input {
        save(v, &crate::data_dir());
    }
    let command = chain_command(args);
    if !command.is_empty() && chain(&command, raw) {
        return String::new();
    }
    default_line(input.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn record_keeps_numbers_only() {
        let input = json!({"session_id": "0123abcd-0000-4000-8000-abcdef012345", "cost": {"total_cost_usd": 1.5},
            "context_window": {"context_window_size": 200000, "remaining_percentage": 30, "total_tokens": 0},
            "rate_limits": {"five_hour": {"used_percentage": 120, "resets_at": 1700000000}, "seven_day": {"used_percentage": 5}},
            "model": {"display_name": "Opus"}});
        let rec = record(&input, 0.0, Some("200000")).unwrap();
        assert_eq!(jsfmt::stringify(&rec), r#"{"v":1,"sessionId":"0123abcd-0000-4000-8000-abcdef012345","at":"1970-01-01T00:00:00.000Z","costUsd":1.5,"contextWindow":200000,"contextPct":70,"bufferPct":20,"fiveHour":{"usedPct":100,"resetsAt":"2023-11-14T22:13:20.000Z"},"sevenDay":{"usedPct":5}}"#);
        assert!(record(&json!({"session_id": "x"}), 0.0, None).is_none());
        assert_eq!(default_line(Some(&json!({"model": {"display_name": "Opus"}, "context_window": {"used_percentage": 12.5}, "cost": {"total_cost_usd": 0.125}}))), "Opus | ctx 13% | $0.13");
        assert_eq!(default_line(None), "Claude");
    }

    #[test]
    fn chain_args() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(chain_command(&a(&["statusline-bridge", "--chain-b64", "ZWNobyBoaQ=="])), "echo hi");
        assert_eq!(chain_command(&a(&["--chain", "echo", "a"])), "echo a");
        assert_eq!(chain_command(&a(&["statusline-bridge"])), "");
    }
}
