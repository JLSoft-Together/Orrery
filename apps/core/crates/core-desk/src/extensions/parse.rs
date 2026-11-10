use super::commands::{is_scope, plugin_name};
use crate::js;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn starts_non_word_then(head: &str, word_ci: &str) -> bool {
    let rest = head.trim_start_matches(|c: char| !word(c));
    rest.len() >= word_ci.len() && rest.is_char_boundary(word_ci.len()) && rest[..word_ci.len()].eq_ignore_ascii_case(word_ci)
}

fn ci_contains(hay: &str, needle: &str) -> bool {
    hay.to_ascii_lowercase().contains(needle)
}

pub fn parse_mcp_line(line: &str) -> Option<Value> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| Regex::new(r"^(.+?): .*? - ([✔✓✗✘⚠⏸].*|(?:Connected|Failed|Needs|Pending).*)$").unwrap());
    let c = r.captures(core_transcript::jsstr::js_trim(line))?;
    let name = core_transcript::jsstr::js_trim(&c[1]).to_string();
    let head = js::slice(&c[2], 40);
    let needs = Regex::new(r"(?i)needs? auth").map(|re| re.is_match(&head)).unwrap_or(false);
    let state = if starts_non_word_then(&head, "connected") {
        "connected"
    } else if starts_non_word_then(&head, "fail") || head.contains(['✗', '✘']) {
        "failed"
    } else if needs {
        "needsAuth"
    } else if ci_contains(&head, "pending") || ci_contains(&head, "approval") {
        "pending"
    } else if ci_contains(&head, "fail") || ci_contains(&head, "error") {
        "failed"
    } else {
        "unknown"
    };
    let mut out = Map::new();
    out.insert("name".into(), Value::String(name.clone()));
    out.insert("state".into(), Value::String(state.into()));
    if let Some(rest) = name.strip_prefix("plugin:") {
        if let Some((p, _)) = rest.split_once(':').filter(|(p, _)| !p.is_empty()) {
            out.insert("plugin".into(), Value::String(p.into()));
        }
    }
    if name.starts_with("claude.ai ") {
        out.insert("connector".into(), Value::Bool(true));
    }
    Some(Value::Object(out))
}

pub fn parse_mcp_list(text: &str) -> Vec<Value> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).filter_map(parse_mcp_line).collect()
}

fn parse(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

pub fn parse_plugin_list(text: &str, same_dir: &dyn Fn(&str) -> bool) -> Vec<Value> {
    let Some(raw) = parse(text) else { return Vec::new() };
    let list = match &raw {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o.get("installed").and_then(Value::as_array).cloned().unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for p in &list {
        let (Some(id), Some(scope)) = (p.get("id").and_then(Value::as_str), p.get("scope").filter(|s| is_scope(s))) else { continue };
        if scope != "user" {
            if let Some(pp) = p.get("projectPath").and_then(Value::as_str) {
                if !same_dir(pp) {
                    continue;
                }
            }
        }
        out.push(json!({"id": id, "scope": scope, "enabled": p.get("enabled") != Some(&Value::Bool(false))}));
    }
    out
}

pub fn parse_marketplaces(text: &str) -> Vec<String> {
    match parse(text) {
        Some(Value::Array(a)) => a.iter().filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

pub fn parse_available(text: &str, marketplace: &str, cap: usize) -> Vec<Value> {
    let Some(raw) = parse(text) else { return Vec::new() };
    let list = raw.get("available").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for p in &list {
        if p.get("marketplaceName").and_then(Value::as_str) != Some(marketplace) {
            continue;
        }
        let (Some(id), Some(name)) = (p.get("pluginId").and_then(Value::as_str), p.get("name").and_then(Value::as_str)) else { continue };
        let desc = p.get("description").and_then(Value::as_str).map(|d| js::slice(d, 300)).unwrap_or_default();
        out.push(json!({"id": id, "name": name, "marketplace": marketplace, "description": desc}));
        if out.len() >= cap {
            break;
        }
    }
    out
}

pub fn marketplace_from_output(output: &str, repo: &str) -> String {
    let trimmed = core_transcript::jsstr::js_trim(output);
    let last = trimmed.split('\n').next_back().unwrap_or("");
    let last = last.strip_suffix('\r').unwrap_or(last);
    if let Some(j) = parse(last) {
        let name = j.get("name").and_then(Value::as_str).or_else(|| j.get("marketplace").and_then(Value::as_str));
        if let Some(n) = name.filter(|n| plugin_name(n)) {
            return n.to_string();
        }
    }
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| Regex::new(r#"(?i)marketplace[:\s\x{feff}]+["']?([A-Za-z0-9._-]+)"#).unwrap());
    if let Some(c) = r.captures(output) {
        return c[1].to_string();
    }
    repo.split('/').nth(1).unwrap_or("undefined").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_lines() {
        let rows = parse_mcp_list("Checking MCP server health...\n\nfigma: https://x - ✔ Connected\r\nplugin:gh:gh: npx a - b - ✘ Failed to connect — Needs auth\nclaude.ai Gmail: https://g - Needs authentication\n");
        assert_eq!(js::stringify(&Value::Array(rows)), r#"[{"name":"figma","state":"connected"},{"name":"plugin:gh:gh","state":"failed","plugin":"gh"},{"name":"claude.ai Gmail","state":"needsAuth","connector":true}]"#);
        assert_eq!(marketplace_from_output("ok\n{\"name\":\"pm-skills\"}", "a/b"), "pm-skills");
        assert_eq!(marketplace_from_output("Added marketplace: \"cool\"", "a/b"), "cool");
        assert_eq!(marketplace_from_output("done", "a/b"), "b");
    }
}
