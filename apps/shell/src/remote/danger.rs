use regex::{Regex, RegexBuilder};
use serde_json::{json, Value};

pub const MAX_RULES: usize = 32;
pub const MAX_RULE_LEN: usize = 200;
const SIZE_LIMIT: usize = 1 << 20;

const DEFAULTS: [&str; 6] = [
    r"\bgit\s+push\b.*\s(?:-f|--force)(?:\s|$)",
    r"\breset\s+--hard\b",
    r"\brm\s+-[a-z]*(?:rf|fr)[a-z]*(?:\s|$)",
    r"\bgit\s+clean\s+-[a-z]*f",
    r"\bdrop\s+(?:table|database)\b",
    r"\bgit\s+branch\s+-d\b",
];

pub fn default_rules() -> Vec<String> {
    DEFAULTS.iter().map(|r| (*r).to_owned()).collect()
}

fn compile(rule: &str) -> Option<Regex> {
    if rule.is_empty() || rule.chars().count() > MAX_RULE_LEN {
        return None;
    }
    RegexBuilder::new(rule).case_insensitive(true).size_limit(SIZE_LIMIT).build().ok()
}

pub fn valid_rules(rules: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for rule in rules {
        if out.len() >= MAX_RULES {
            break;
        }
        if !out.contains(rule) && compile(rule).is_some() {
            out.push(rule.clone());
        }
    }
    out
}

pub fn matching(rules: &[String], text: &str) -> Option<String> {
    let lines: Vec<&str> = std::iter::once(text.trim()).chain(text.lines().map(str::trim)).filter(|l| !l.is_empty()).collect();
    valid_rules(rules).into_iter().find(|rule| compile(rule).is_some_and(|re| lines.iter().any(|l| re.is_match(l))))
}

pub fn rules_json(rules: &[String]) -> Value {
    Value::from(valid_rules(rules))
}

pub fn needs_auth(rules: &[String], cmd: &Value, vi: bool) -> Option<Value> {
    if cmd.get("authed") == Some(&Value::Bool(true)) {
        return None;
    }
    let text = cmd.get("text").and_then(Value::as_str)?;
    let rule = matching(rules, text)?;
    let message = if vi { "Lệnh này cần xác thực trên điện thoại, hoặc cập nhật app để gửi" } else { "This command needs device authentication on the phone, or an app update to send" };
    Some(json!({ "ok": false, "error": "needsAuth", "rule": rule, "message": message, "detail": message }))
}

pub fn strip_authed(cmd: &mut Value) {
    if let Some(obj) = cmd.as_object_mut() {
        obj.remove("authed");
    }
}
