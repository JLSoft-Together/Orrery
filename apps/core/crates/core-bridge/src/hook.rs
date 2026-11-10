use crate::jsfmt;
use serde_json::{Map, Value};
use std::io::Write;
use std::path::Path;

pub const ASKS_FILE: &str = "permission-asks.jsonl";
const ASKS_ROTATE: u64 = 512 * 1024;

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn is_tool(s: &str) -> bool {
    (1..=80).contains(&s.len()) && s.chars().all(|c| word_char(c) || matches!(c, '.' | ':' | '-'))
}

fn is_rule_name(s: &str) -> bool {
    let mut it = s.chars();
    matches!(it.next(), Some(c) if c.is_ascii_alphabetic()) && s.len() <= 80 && it.all(|c| word_char(c) || c == '-')
}

fn is_rule_content(s: &str) -> bool {
    (1..=120).contains(&s.len()) && s.chars().all(|c| word_char(c) || " .:*@+,~-".contains(c))
}

fn is_word(w: &str) -> bool {
    let body = w.strip_prefix("./").unwrap_or(w);
    let ok = |s: &str| (1..=30).contains(&s.len()) && s.chars().all(|c| word_char(c) || ".:@+-".contains(c));
    ok(body) || ok(w)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(crate::is_js_space)
}

fn safe_content(c: &str) -> bool {
    is_rule_content(c) && !c.split(' ').filter(|w| !w.is_empty()).any(|w| w.len() > 40)
}

pub fn suggested_rules(input: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let list = input.get("permission_suggestions").and_then(Value::as_array).cloned().unwrap_or_default();
    for s in &list {
        let Some(s) = s.as_object() else { continue };
        if s.get("type").and_then(Value::as_str) != Some("addRules") || s.get("behavior").and_then(Value::as_str) != Some("allow") {
            continue;
        }
        let Some(rules) = s.get("rules").and_then(Value::as_array) else { continue };
        for r in rules {
            let Some(name) = r.get("toolName").and_then(Value::as_str).filter(|n| is_rule_name(n)) else { continue };
            match r.get("ruleContent") {
                None | Some(Value::Null) => out.push(name.to_string()),
                Some(Value::String(c)) if c.is_empty() => out.push(name.to_string()),
                Some(Value::String(c)) if safe_content(js_trim(c)) => out.push(format!("{name}({})", js_trim(c))),
                _ => {}
            }
        }
    }
    let mut seen = Vec::new();
    for r in out {
        if !seen.contains(&r) {
            seen.push(r);
        }
    }
    seen.truncate(3);
    seen
}

pub fn command_prefix(command: Option<&Value>) -> Option<String> {
    let command = command?.as_str()?;
    let mut words = Vec::new();
    let trimmed = js_trim(command);
    let parts: Vec<&str> = if trimmed.is_empty() { vec![""] } else { trimmed.split(crate::is_js_space).filter(|w| !w.is_empty()).collect() };
    for w in parts.into_iter().take(2) {
        if !is_word(w) || w.starts_with('-') {
            break;
        }
        words.push(w);
    }
    (!words.is_empty()).then(|| words.join(" "))
}

pub fn ask_record(input: &Value, at: &str) -> Option<Value> {
    let tool = input.get("tool_name").and_then(Value::as_str).filter(|t| is_tool(t))?;
    let cwd = input.get("cwd").and_then(Value::as_str).filter(|c| !c.is_empty() && crate::utf16_len(c) <= 1024)?;
    let mut rec = Map::new();
    rec.insert("at".into(), Value::String(at.into()));
    rec.insert("cwd".into(), Value::String(cwd.into()));
    rec.insert("tool".into(), Value::String(tool.into()));
    rec.insert("rules".into(), Value::from(suggested_rules(input)));
    if tool == "Bash" || tool == "PowerShell" {
        if let Some(p) = command_prefix(input.get("tool_input").and_then(|t| t.get("command"))) {
            rec.insert("prefix".into(), Value::String(p));
        }
    }
    Some(Value::Object(rec))
}

fn record_ask(input: &Value, data_dir: &Path, at: &str) {
    let Some(rec) = ask_record(input, at) else { return };
    let dir = data_dir.join("hooks");
    let file = dir.join(ASKS_FILE);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    if std::fs::metadata(&file).is_ok_and(|m| m.len() > ASKS_ROTATE) {
        let _ = std::fs::rename(&file, dir.join("permission-asks.1.jsonl"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = f.write_all(format!("{}\n", jsfmt::stringify(&rec)).as_bytes());
    }
}

fn is_integer(v: &Value) -> bool {
    v.as_f64().is_some_and(|f| f.is_finite() && f.trunc() == f)
}

pub fn next_state(old: &Value, input: &Value, at: &str) -> Option<Value> {
    let id = input.get("session_id").and_then(Value::as_str)?;
    let event = input.get("hook_event_name").and_then(Value::as_str).unwrap_or("");
    let text = |k: &str| old.get(k).and_then(Value::as_str).map(str::to_string);
    let mut since = text("compactingSince");
    let mut trigger = text("compactTrigger");
    let mut count = old.get("deniedCount").filter(|v| is_integer(v)).and_then(Value::as_f64).unwrap_or(0.0);
    let mut tools: Vec<String> = old
        .get("deniedTools")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).filter(|t| is_tool(t)).map(str::to_string).collect())
        .unwrap_or_default();
    let keep_last = |v: &mut Vec<String>| {
        let n = v.len();
        if n > 5 {
            v.drain(..n - 5);
        }
    };
    keep_last(&mut tools);
    match event {
        "PreCompact" => {
            since = Some(at.to_string());
            trigger = Some(if input.get("trigger").and_then(Value::as_str) == Some("manual") { "manual" } else { "auto" }.into());
        }
        "PostCompact" => {
            since = None;
            trigger = None;
        }
        "PermissionDenied" => {
            count = (count + 1.0).min(1_000_000.0);
            if let Some(tool) = input.get("tool_name").and_then(Value::as_str).filter(|t| is_tool(t)) {
                tools.retain(|t| t != tool);
                tools.push(tool.to_string());
                keep_last(&mut tools);
            }
        }
        _ => return None,
    }
    let mut rec = Map::new();
    rec.insert("v".into(), Value::from(1));
    rec.insert("sessionId".into(), Value::String(id.into()));
    rec.insert("at".into(), Value::String(at.into()));
    if let Some(s) = since {
        rec.insert("compactingSince".into(), Value::String(s));
    }
    if let Some(t) = trigger {
        rec.insert("compactTrigger".into(), Value::String(t));
    }
    rec.insert("deniedCount".into(), Value::from(count));
    rec.insert("deniedTools".into(), Value::from(tools));
    Some(Value::Object(rec))
}

pub fn save(input: &Value, data_dir: &Path) {
    let Some(id) = input.get("session_id").and_then(Value::as_str).filter(|s| crate::is_session_id(s)) else { return };
    let Some(at) = jsfmt::iso_string(jsfmt::now_ms()) else { return };
    if input.get("hook_event_name").and_then(Value::as_str) == Some("PermissionRequest") {
        record_ask(input, data_dir, &at);
        return;
    }
    let dir = data_dir.join("hooks");
    let file = dir.join(format!("{id}.json"));
    let old = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or(Value::Null);
    let Some(rec) = next_state(&old, input, &at) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let tmp = dir.join(format!("{id}.json.{}.tmp", std::process::id()));
    let _ = crate::write_replace(&file, &tmp, &jsfmt::stringify(&rec));
}

pub fn run(raw: &[u8]) {
    let text = String::from_utf8_lossy(raw);
    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        save(&v, &crate::data_dir());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn asks_keep_safe_parts() {
        let input = json!({"tool_name": "Bash", "cwd": "/p", "tool_input": {"command": "  git push --force"},
            "permission_suggestions": [{"type": "addRules", "behavior": "allow", "rules": [{"toolName": "Bash", "ruleContent": " git push:* "}, {"toolName": "Bash", "ruleContent": "rm \"/x\""}, {"toolName": "Read"}, {"toolName": "Read", "ruleContent": ""}, {"toolName": "Edit"}, {"toolName": "Write"}]}]});
        let rec = ask_record(&input, "T").unwrap();
        assert_eq!(jsfmt::stringify(&rec), r#"{"at":"T","cwd":"/p","tool":"Bash","rules":["Bash(git push:*)","Read","Edit"],"prefix":"git push"}"#);
        assert_eq!(command_prefix(Some(&json!("./gradlew -x"))), Some("./gradlew".into()));
        assert_eq!(command_prefix(Some(&json!("C:\\x\\y.exe run"))), None);
        assert_eq!(command_prefix(Some(&json!("   "))), None);
    }

    #[test]
    fn compact_and_denied_state() {
        let pre = next_state(&Value::Null, &json!({"session_id": "s", "hook_event_name": "PreCompact", "trigger": "manual"}), "T1").unwrap();
        assert_eq!(jsfmt::stringify(&pre), r#"{"v":1,"sessionId":"s","at":"T1","compactingSince":"T1","compactTrigger":"manual","deniedCount":0,"deniedTools":[]}"#);
        let old = json!({"deniedCount": 2.0, "deniedTools": ["a", "b", "c", "d", "e", "f", "bad tool", 3], "compactingSince": "T1"});
        let d = next_state(&old, &json!({"session_id": "s", "hook_event_name": "PermissionDenied", "tool_name": "c"}), "T2").unwrap();
        assert_eq!(jsfmt::stringify(&d), r#"{"v":1,"sessionId":"s","at":"T2","compactingSince":"T1","deniedCount":3,"deniedTools":["b","d","e","f","c"]}"#);
        let post = next_state(&d, &json!({"session_id": "s", "hook_event_name": "PostCompact"}), "T3").unwrap();
        assert!(!jsfmt::stringify(&post).contains("compacting"));
        assert!(next_state(&Value::Null, &json!({"session_id": "s", "hook_event_name": "Stop"}), "T").is_none());
    }
}
