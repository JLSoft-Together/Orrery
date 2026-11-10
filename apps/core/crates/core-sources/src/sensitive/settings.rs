use core_store::extra::{Dict, SensitiveSettings};
use core_transcript::jsstr::{js_trim, units};
use serde_json::Value;

pub const SENSITIVE_GROUPS: [&str; 6] = ["secrets", "outside", "git", "delete", "data", "custom"];
pub const SECRET_FILE_GLOBS: [&str; 10] = [
    ".env*",
    "*.jks",
    "*.keystore",
    "*.p12",
    "*.pem",
    "key.properties",
    "local.properties",
    "google-services.json",
    "GoogleService-Info.plist",
    "*.mobileprovision",
];
pub const OUTSIDE_RULE: &str = "outside-project";
pub const CUSTOM_PREFIX: &str = "custom:";
pub const MAX_SENSITIVE_GLOBS: usize = 50;
pub const MAX_IGNORE_PROJECTS: usize = 200;
const MAX_RULES_PER_PROJECT: usize = 64;

pub fn command_group(rule: &str) -> Option<&'static str> {
    match rule {
        "git-force-push" | "git-reset-hard" | "git-clean" => Some("git"),
        "rm-rf" | "del-s" | "remove-item-recurse" => Some("delete"),
        "adb-uninstall" | "drop-table" => Some("data"),
        _ => None,
    }
}

pub fn rule_group(rule: &str) -> Option<&'static str> {
    if rule == OUTSIDE_RULE {
        return Some("outside");
    }
    if rule.starts_with(CUSTOM_PREFIX) {
        return Some("custom");
    }
    if SECRET_FILE_GLOBS.contains(&rule) {
        return Some("secrets");
    }
    command_group(rule)
}

pub fn is_sensitive_rule(rule: &str) -> bool {
    units(rule) <= 220 && rule_group(rule).is_some()
}

pub fn default_settings() -> SensitiveSettings {
    let mut groups = Dict::new();
    for g in SENSITIVE_GROUPS {
        groups.insert(g.to_owned(), true);
    }
    SensitiveSettings { groups, globs: Vec::new(), ignore: Dict::new() }
}

pub fn sanitize_globs(v: &Value) -> Option<Vec<String>> {
    let Value::Array(items) = v else { return None };
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let glob = js_trim(item.as_str()?);
        let ok = !glob.is_empty() && units(glob) <= 200 && !glob.contains(['\0', '\r', '\n']);
        if ok && !out.iter().any(|g| g == glob) {
            out.push(glob.to_owned());
        }
    }
    out.truncate(MAX_SENSITIVE_GLOBS);
    Some(out)
}

pub fn sanitize_ignore(v: &Value) -> Option<Dict<Vec<String>>> {
    let Value::Object(src) = v else { return None };
    let mut out = Dict::new();
    for (dir, rules) in src.iter().take(MAX_IGNORE_PROJECTS) {
        let Value::Array(rules) = rules else { continue };
        if dir.is_empty() || units(dir) > 1024 {
            continue;
        }
        let mut list: Vec<String> = Vec::new();
        for rule in rules.iter().filter_map(Value::as_str).filter(|r| is_sensitive_rule(r)) {
            if !list.iter().any(|r| r == rule) {
                list.push(rule.to_owned());
            }
        }
        list.truncate(MAX_RULES_PER_PROJECT);
        if !list.is_empty() {
            out.insert(dir.clone(), list);
        }
    }
    Some(out)
}

pub fn sanitize_groups(v: &Value) -> Option<Vec<(String, bool)>> {
    let src = match v {
        Value::Object(m) => Some(m),
        Value::Array(_) => None,
        _ => return None,
    };
    let mut out = Vec::new();
    if let Some(m) = src {
        for g in SENSITIVE_GROUPS {
            if let Some(Value::Bool(b)) = m.get(g) {
                out.push((g.to_owned(), *b));
            }
        }
    }
    Some(out)
}

pub fn sanitize_sensitive(v: Option<&Value>) -> SensitiveSettings {
    let mut base = default_settings();
    let Some(Value::Object(m)) = v else { return base };
    if let Some(groups) = m.get("groups").and_then(sanitize_groups) {
        for (g, b) in groups {
            base.groups.insert(g, b);
        }
    }
    base.globs = m.get("globs").and_then(sanitize_globs).unwrap_or_default();
    base.ignore = m.get("ignore").and_then(sanitize_ignore).unwrap_or_default();
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rule_groups_follow_the_shared_table() {
        assert_eq!(rule_group(".env*"), Some("secrets"));
        assert_eq!(rule_group("custom:*.x"), Some("custom"));
        assert_eq!(rule_group("outside-project"), Some("outside"));
        assert_eq!(rule_group("rm-rf"), Some("delete"));
        assert_eq!(rule_group("drop-table"), Some("data"));
        assert_eq!(rule_group("git-clean"), Some("git"));
        assert_eq!(rule_group("bogus"), None);
        assert!(!is_sensitive_rule(&format!("custom:{}", "x".repeat(220))));
    }

    #[test]
    fn sanitizers_clean_what_the_page_sends() {
        assert_eq!(sanitize_globs(&json!([" *.secret ", "", "a\nb", "*.secret", "x"])), Some(vec!["*.secret".to_owned(), "x".to_owned()]));
        assert_eq!(sanitize_globs(&json!(["a", 1])), None);
        let ignore = sanitize_ignore(&json!({"D:\\a": ["rm-rf", "rm-rf", "nope", "custom:x", 5], "": ["rm-rf"], "D:\\b": [], "D:\\c": "rm-rf"})).unwrap();
        assert_eq!(serde_json::to_value(&ignore).unwrap(), json!({"D:\\a": ["rm-rf", "custom:x"]}));
        assert_eq!(sanitize_groups(&json!({"git": false, "zzz": true})), Some(vec![("git".to_owned(), false)]));
        assert_eq!(sanitize_groups(&json!([])), Some(Vec::new()));
        assert_eq!(sanitize_groups(&json!(3)), None);
        let s = sanitize_sensitive(Some(&json!({"groups": {"data": false}, "globs": ["*.k"], "ignore": {"p": ["rm-rf"]}})));
        assert_eq!(serde_json::to_value(&s).unwrap(), json!({"groups": {"secrets": true, "outside": true, "git": true, "delete": true, "data": false, "custom": true}, "globs": ["*.k"], "ignore": {"p": ["rm-rf"]}}));
        assert_eq!(sanitize_sensitive(None), default_settings());
    }
}
