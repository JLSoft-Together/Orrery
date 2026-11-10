use super::paths::{is_within, real_or_null};
use crate::claude::diff::{format_settings, json_diff};
use crate::claude::write_atomic;
use crate::js;
use core_transcript::jsstr::{js_space, js_trim, units};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::OnceLock;

pub const PERM_ASK_DAYS: f64 = 7.0;
pub const PERM_RULES_MAX: usize = 20;
const ASK_FILE_MAX: u64 = 1024 * 1024;
const SETTINGS_MAX: u64 = 512 * 1024;
const RULES_SHOWN: usize = 200;
const ASK_FILES: [&str; 2] = ["permission-asks.1.jsonl", "permission-asks.jsonl"];
const RISKY_COMMANDS: [&str; 30] = [
    "rm", "rmdir", "del", "sudo", "su", "doas", "chmod", "chown", "curl", "wget", "ssh", "scp", "sh", "bash", "zsh", "pwsh", "powershell", "cmd", "eval", "exec", "xargs", "env", "node", "python",
    "python3", "npx", "dd", "mkfs", "format", "kill",
];
const RISKY_COMMANDS_MORE: [&str; 6] = ["pkill", "shutdown", "reboot", "Remove-Item", "Invoke-Expression", "iex"];

fn rule_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_-]{0,127})(?:\(([^()\r\n]+)\))?$").expect("regex"))
}

fn tool_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9_.:-]{1,80}$").expect("regex"))
}

fn rule_content_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9_ .:*@+,~-]{1,120}$").expect("regex"))
}

fn prefix_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(?:\./)?[A-Za-z0-9_.:@+-]{1,30}(?: (?:\./)?[A-Za-z0-9_.:@+-]{1,30})?$").expect("regex"))
}

fn risky_git_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^git[\s\x{feff}]+(?:push|reset|clean|rebase|filter-branch|checkout[\s\x{feff}]+--|restore|branch[\s\x{feff}]+-D)(?-u:\b)").expect("regex"))
}

pub struct PermRule {
    pub tool: String,
    pub content: Option<String>,
}

pub fn parse_perm_rule(rule: &Value) -> Option<PermRule> {
    let s = js_trim(rule.as_str()?);
    let m = rule_re().captures(s)?;
    let content = match m.get(2) {
        Some(c) => {
            let n = units(c.as_str());
            if !(1..=200).contains(&n) {
                return None;
            }
            Some(js_trim(c.as_str()).to_string())
        }
        None => None,
    };
    Some(PermRule { tool: m[1].to_string(), content })
}

fn rule_text(r: &PermRule) -> String {
    match &r.content {
        None => r.tool.clone(),
        Some(c) => format!("{}({})", r.tool, c),
    }
}

pub fn is_risky_perm_rule(rule: &str) -> bool {
    let Some(r) = parse_perm_rule(&json!(rule)) else { return true };
    let c = r.content.as_deref();
    if let Some(c) = c {
        if c.chars().all(|ch| ch == '*' || ch == ':' || js_space(ch)) {
            return true;
        }
    }
    if r.tool != "Bash" && r.tool != "PowerShell" {
        return matches!(c, Some("**" | "/**" | "//**"));
    }
    let Some(c) = c else { return true };
    if c.chars().any(|ch| matches!(ch, ';' | '&' | '|' | '`' | '$' | '<' | '>' | '\n' | '\\' | '"' | '\'')) {
        return true;
    }
    let first = c.split(|ch: char| js_space(ch) || ch == ':').next().unwrap_or("");
    let lower = first.to_lowercase();
    let hit = |w: &str| RISKY_COMMANDS.contains(&w) || RISKY_COMMANDS_MORE.contains(&w);
    hit(first) || hit(&lower) || risky_git_re().is_match(c)
}

pub fn safe_ask_rule(rule: &Value) -> Option<String> {
    let r = parse_perm_rule(rule)?;
    match &r.content {
        None => Some(r.tool),
        Some(c) => rule_content_re().is_match(c).then(|| format!("{}({})", r.tool, c)),
    }
}

pub fn parse_asks(text: &str, root: &str, now: f64) -> Vec<Value> {
    let since = now - PERM_ASK_DAYS * 86_400_000.0;
    let mut out: Vec<(String, Value)> = Vec::new();
    for line in text.split('\n') {
        if js_trim(line).is_empty() {
            continue;
        }
        let Some(Value::Object(o)) = js::parse(line) else { continue };
        let (Some(at_s), Some(cwd), Some(tool)) = (o.get("at").and_then(Value::as_str), o.get("cwd").and_then(Value::as_str), o.get("tool").and_then(Value::as_str)) else { continue };
        if !tool_re().is_match(tool) {
            continue;
        }
        let Some(at) = core_transcript::time::parse_iso(at_s).map(|v| v as f64) else { continue };
        if at < since || at > now + 60_000.0 {
            continue;
        }
        if !super::paths::is_absolute(cwd) || !is_within(root, cwd) {
            continue;
        }
        let rules: Vec<Value> = match o.get("rules") {
            Some(Value::Array(a)) => a.iter().filter_map(safe_ask_rule).take(3).map(Value::String).collect(),
            _ => Vec::new(),
        };
        let iso = crate::util::iso_string(at);
        let mut ask = Map::new();
        ask.insert("at".into(), json!(iso));
        ask.insert("tool".into(), json!(tool));
        ask.insert("rules".into(), Value::Array(rules));
        if let Some(p) = o.get("prefix").and_then(Value::as_str).filter(|p| prefix_re().is_match(p)) {
            ask.insert("prefix".into(), json!(p));
        }
        out.push((iso, Value::Object(ask)));
    }
    out.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(&a.0, &b.0));
    out.into_iter().map(|(_, v)| v).collect()
}

fn read_asks(data_dir: &str, root: &str, now: f64) -> Vec<Value> {
    let mut out: Vec<(String, Value)> = Vec::new();
    for name in ASK_FILES {
        let file = Path::new(data_dir).join("hooks").join(name);
        if !std::fs::metadata(&file).map(|m| m.is_file() && m.len() <= ASK_FILE_MAX).unwrap_or(false) {
            continue;
        }
        let text = std::fs::read(&file).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        for v in parse_asks(&text, root, now) {
            out.push((v["at"].as_str().unwrap_or("").to_string(), v));
        }
    }
    out.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(&a.0, &b.0));
    out.into_iter().map(|(_, v)| v).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Ok,
    None,
    Unreadable,
}

impl FileState {
    pub fn name(&self) -> &'static str {
        match self {
            FileState::Ok => "ok",
            FileState::None => "none",
            FileState::Unreadable => "unreadable",
        }
    }
}

pub struct SettingsRead {
    pub state: FileState,
    pub text: String,
    pub settings: Option<Map<String, Value>>,
}

pub fn read_settings(file: &Path) -> SettingsRead {
    let unreadable = |text: String| SettingsRead { state: FileState::Unreadable, text, settings: None };
    let text = match std::fs::metadata(file) {
        Ok(m) if m.is_file() && m.len() <= SETTINGS_MAX => match std::fs::read(file) {
            Ok(b) => String::from_utf8_lossy(&b).to_string(),
            Err(_) => return unreadable(String::new()),
        },
        Ok(_) => return unreadable(String::new()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SettingsRead { state: FileState::None, text: String::new(), settings: Some(Map::new()) },
        Err(_) => return unreadable(String::new()),
    };
    let body = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let v = if js_trim(body).is_empty() { Some(json!({})) } else { js::parse(body) };
    let Some(Value::Object(obj)) = v else { return unreadable(text) };
    if let Some(p) = obj.get("permissions") {
        let Value::Object(p) = p else { return unreadable(text) };
        if p.get("allow").is_some_and(|a| !a.is_array()) || p.get("deny").is_some_and(|a| !a.is_array()) {
            return unreadable(text);
        }
    }
    SettingsRead { state: FileState::Ok, text, settings: Some(obj) }
}

fn rule_set(r: &SettingsRead) -> Value {
    let perms = r.settings.as_ref().and_then(|s| s.get("permissions")).and_then(Value::as_object);
    let list = |k: &str| -> Value {
        let items: Vec<Value> = match perms.and_then(|p| p.get(k)) {
            Some(Value::Array(a)) => a.iter().filter(|x| x.as_str().is_some_and(|s| units(s) <= 300)).take(RULES_SHOWN).cloned().collect(),
            _ => Vec::new(),
        };
        Value::Array(items)
    };
    json!({"state": r.state.name(), "allow": list("allow"), "deny": list("deny")})
}

pub fn read_permissions(root: &str, data_dir: Option<&str>, now: f64) -> Value {
    let base = Path::new(root).join(".claude");
    let asks = data_dir.map(|d| read_asks(d, root, now)).unwrap_or_default();
    let shared = read_settings(&base.join("settings.json"));
    let local = read_settings(&base.join("settings.local.json"));
    json!({"ok": true, "root": root, "asks": asks, "shared": rule_set(&shared), "local": rule_set(&local)})
}

pub fn clean_rules(rules: &Value) -> Result<Vec<String>, &'static str> {
    let Some(arr) = rules.as_array().filter(|a| !a.is_empty() && a.len() <= PERM_RULES_MAX) else { return Err("invalid") };
    let mut out: Vec<String> = Vec::new();
    for r in arr {
        let p = parse_perm_rule(r).ok_or("invalid")?;
        let text = rule_text(&p);
        if is_risky_perm_rule(&text) {
            return Err("risky");
        }
        if !out.contains(&text) {
            out.push(text);
        }
    }
    Ok(out)
}

pub fn plan_rules(settings: &Map<String, Value>, key: &str, rules: &[String], dedupe: bool) -> (Value, Vec<String>) {
    let perms = settings.get("permissions").and_then(Value::as_object).cloned().unwrap_or_default();
    let list = perms.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let mut added: Vec<String> = Vec::new();
    for r in rules {
        if list.iter().any(|x| x.as_str() == Some(r)) || (dedupe && added.contains(r)) {
            continue;
        }
        added.push(r.clone());
    }
    let mut next_perms = perms;
    let mut next_list = list;
    next_list.extend(added.iter().map(|a| json!(a)));
    next_perms.insert(key.to_string(), Value::Array(next_list));
    let mut next = settings.clone();
    next.insert("permissions".into(), Value::Object(next_perms));
    (Value::Object(next), added)
}

pub fn write_allow(root: &str, rules: &Value, confirm: &dyn Fn(Value) -> bool) -> Value {
    let fail = |e: &str| json!({"ok": false, "error": e});
    let clean = match clean_rules(rules) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let real_root = real_or_null(root).unwrap_or_else(|| root.to_string());
    let dir = Path::new(root).join(".claude");
    if let Some(rd) = real_or_null(&dir.to_string_lossy()) {
        if !is_within(&real_root, &rd) {
            return fail("denied");
        }
    }
    let file = dir.join("settings.local.json");
    if let Some(rf) = real_or_null(&file.to_string_lossy()) {
        if !is_within(&real_root, &rf) {
            return fail("denied");
        }
    }
    let read = read_settings(&file);
    let Some(settings) = read.settings.as_ref().filter(|_| read.state != FileState::Unreadable) else { return fail("unreadable") };
    let (next, added) = plan_rules(settings, "allow", &clean, false);
    if added.is_empty() {
        return json!({"ok": true, "added": []});
    }
    let text = format_settings(&next, &read.text);
    let file_s = file.to_string_lossy().to_string();
    if !confirm(json!({"file": file_s, "added": added, "diff": json_diff(&read.text, &text)})) {
        return fail("cancelled");
    }
    let again = read_settings(&file);
    if again.text != read.text || again.state != read.state {
        return fail("changed");
    }
    if write_atomic(&file, &text, read.state == FileState::Ok) {
        json!({"ok": true, "added": added})
    } else {
        fail("failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_parse_and_risk() {
        let p = parse_perm_rule(&json!("  Bash(npm test:*) ")).unwrap();
        assert_eq!((p.tool.as_str(), p.content.as_deref()), ("Bash", Some("npm test:*")));
        assert!(parse_perm_rule(&json!("Bash()")).is_none());
        assert!(is_risky_perm_rule("Bash"));
        assert!(is_risky_perm_rule("Bash(rm -rf x)"));
        assert!(is_risky_perm_rule("Bash(git push origin)"));
        assert!(is_risky_perm_rule("Bash(a && b)"));
        assert!(!is_risky_perm_rule("Bash(npm test:*)"));
        assert!(is_risky_perm_rule("Read(**)"));
        assert!(!is_risky_perm_rule("Read(src/**)"));
        assert!(is_risky_perm_rule("Bash(::)"));
    }

    #[test]
    fn asks_filter_and_order() {
        let now = 1_700_000_000_000.0;
        let iso = |ms: f64| crate::util::iso_string(ms);
        let text = format!(
            "{}\n{}\n{}\nnot json\n",
            json!({"at": iso(now - 1000.0), "cwd": "/p/a", "tool": "Bash", "rules": ["Bash(npm test:*)", "Bash(a;b)"], "prefix": "npm test"}),
            json!({"at": iso(now - 5000.0), "cwd": "/p", "tool": "Edit", "rules": []}),
            json!({"at": iso(now - 9.0 * 86_400_000.0), "cwd": "/p", "tool": "Edit"}),
        );
        let asks = parse_asks(&text, "/p", now);
        assert_eq!(asks.len(), 2);
        assert_eq!(asks[0]["tool"], "Edit");
        assert_eq!(asks[1]["rules"], json!(["Bash(npm test:*)"]));
        assert_eq!(asks[1]["prefix"], "npm test");
    }
}
