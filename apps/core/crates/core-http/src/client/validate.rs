use serde_json::{Map, Number, Value};

use crate::js::{len16, trim};

pub const TOAST_KINDS: [&str; 10] = ["waiting", "done", "job", "loop", "stuck", "context", "limit", "idle", "sensitive", "disk"];
pub const SENSITIVE_GROUPS: [&str; 6] = ["secrets", "outside", "git", "delete", "data", "custom"];
const STUCK_CHOICES: [u64; 5] = [0, 5, 10, 15, 30];
const IDLE_CHOICES: [u64; 6] = [0, 15, 30, 60, 120, 240];
const SECRET_FILE_GLOBS: [&str; 10] = [
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
const COMMAND_RULES: [&str; 8] = ["git-force-push", "git-reset-hard", "git-clean", "rm-rf", "del-s", "remove-item-recurse", "adb-uninstall", "drop-table"];
const MAX_SENSITIVE_GLOBS: usize = 50;
const MAX_IGNORE_PROJECTS: usize = 200;
const MAX_RULES_PER_PROJECT: usize = 64;

pub fn is_search_id(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn is_session_id(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5 && [8, 4, 4, 4, 12].iter().zip(&groups).all(|(n, g)| g.len() == *n && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn is_hm(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && ((b[0] <= b'1') || (b[0] == b'2' && b[1] <= b'3'))
        && b[3] <= b'5'
}

pub fn is_absolute_path(p: &str) -> bool {
    if cfg!(windows) {
        let b = p.as_bytes();
        matches!(b.first(), Some(b'/' | b'\\')) || (b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'/' | b'\\'))
    } else {
        p.starts_with('/')
    }
}

pub fn is_rule(s: &str) -> bool {
    len16(s) <= 220 && (s == "outside-project" || s.starts_with("custom:") || SECRET_FILE_GLOBS.contains(&s) || COMMAND_RULES.contains(&s))
}

fn whole_number(n: &Number) -> Option<u64> {
    if let Some(u) = n.as_u64() {
        return Some(u);
    }
    n.as_f64().filter(|f| f.fract() == 0.0 && *f >= 0.0 && *f < 9.0e15).map(|f| f as u64)
}

pub fn choice(v: &Value, allowed: &[u64]) -> Option<u64> {
    let Value::Number(n) = v else { return None };
    whole_number(n).filter(|n| allowed.contains(n))
}

pub fn sanitize_stuck(v: &Value) -> Option<u64> {
    choice(v, &STUCK_CHOICES)
}

pub fn sanitize_idle(v: &Value) -> Option<u64> {
    choice(v, &IDLE_CHOICES)
}

fn js_object(v: &Value) -> Option<&Map<String, Value>> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    match v {
        Value::Object(m) => Some(m),
        Value::Array(_) => Some(EMPTY.get_or_init(Map::new)),
        _ => None,
    }
}

pub fn sanitize_quiet(v: &Value) -> Option<Value> {
    let q = js_object(v)?;
    let enabled = q.get("enabled")?.as_bool()?;
    let from = q.get("from")?.as_str().filter(|s| is_hm(s))?;
    let to = q.get("to")?.as_str().filter(|s| is_hm(s))?;
    let mut out = Map::new();
    out.insert("enabled".into(), Value::Bool(enabled));
    out.insert("from".into(), Value::from(from));
    out.insert("to".into(), Value::from(to));
    Some(Value::Object(out))
}

fn bool_fields(v: &Value, names: &[&str]) -> Option<Map<String, Value>> {
    let src = js_object(v)?;
    let mut out = Map::new();
    for name in names {
        if let Some(Value::Bool(b)) = src.get(*name) {
            out.insert((*name).into(), Value::Bool(*b));
        }
    }
    Some(out)
}

pub fn sanitize_kinds(v: &Value) -> Option<Map<String, Value>> {
    bool_fields(v, &TOAST_KINDS)
}

pub fn sanitize_groups(v: &Value) -> Option<Map<String, Value>> {
    bool_fields(v, &SENSITIVE_GROUPS)
}

pub fn sanitize_globs(v: &Value) -> Option<Vec<String>> {
    let Value::Array(items) = v else { return None };
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let glob = trim(item.as_str()?);
        let ok = !glob.is_empty() && len16(glob) <= 200 && !glob.contains(['\0', '\r', '\n']);
        if ok && !out.iter().any(|g| g == glob) {
            out.push(glob.to_owned());
        }
    }
    out.truncate(MAX_SENSITIVE_GLOBS);
    Some(out)
}

pub fn sanitize_ignore(v: &Value) -> Option<Map<String, Value>> {
    let Value::Object(src) = v else { return None };
    let mut out = Map::new();
    for (dir, rules) in src.iter().take(MAX_IGNORE_PROJECTS) {
        let Value::Array(rules) = rules else { continue };
        if dir.is_empty() || len16(dir) > 1024 {
            continue;
        }
        let mut list: Vec<Value> = Vec::new();
        for rule in rules.iter().filter_map(Value::as_str).filter(|r| is_rule(r)) {
            if !list.iter().any(|r| r.as_str() == Some(rule)) {
                list.push(Value::from(rule));
            }
        }
        list.truncate(MAX_RULES_PER_PROJECT);
        if !list.is_empty() {
            out.insert(dir.clone(), Value::Array(list));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn ids_and_times() {
        assert!(is_search_id("a_B-9") && !is_search_id("") && !is_search_id("a b") && !is_search_id(&"x".repeat(65)));
        assert!(is_session_id("0123abcd-0123-4abc-8def-0123456789AB") && !is_session_id("0123abcd-0123-4abc-8def-0123456789A") && !is_session_id("x"));
        for ok in ["00:00", "09:59", "19:30", "23:59"] {
            assert!(is_hm(ok), "{ok}");
        }
        for bad in ["24:00", "9:00", "12:60", "12-30", "ab:cd", "1230"] {
            assert!(!is_hm(bad), "{bad}");
        }
    }

    #[test]
    fn quiet_kinds_choices() {
        assert_eq!(sanitize_quiet(&json!({"enabled": true, "from": "22:00", "to": "08:00", "x": 1})), Some(json!({"enabled": true, "from": "22:00", "to": "08:00"})));
        assert_eq!(sanitize_quiet(&json!({"enabled": 1, "from": "22:00", "to": "08:00"})), None);
        assert_eq!(sanitize_quiet(&json!({"enabled": true, "from": "25:00", "to": "08:00"})), None);
        assert_eq!(sanitize_quiet(&json!("x")), None);
        assert_eq!(sanitize_kinds(&json!({"done": true, "bogus": true, "waiting": "yes", "disk": false})).map(Value::Object), Some(json!({"done": true, "disk": false})));
        assert_eq!(sanitize_kinds(&json!(null)), None);
        assert_eq!(sanitize_stuck(&json!(10)), Some(10));
        assert_eq!(sanitize_stuck(&json!(10.0)), Some(10));
        assert_eq!(sanitize_stuck(&json!(11)), None);
        assert_eq!(sanitize_stuck(&json!("10")), None);
        assert_eq!(sanitize_idle(&json!(240)), Some(240));
        assert_eq!(sanitize_idle(&json!(-1)), None);
    }

    #[test]
    fn sensitive_sanitizers() {
        assert_eq!(sanitize_globs(&json!([" *.secret ", "", "a\nb", "*.secret", "x"])), Some(vec!["*.secret".to_owned(), "x".to_owned()]));
        assert_eq!(sanitize_globs(&json!(["a", 1])), None);
        assert_eq!(sanitize_globs(&json!({})), None);
        let many: Vec<String> = (0..80).map(|i| format!("g{i}")).collect();
        assert_eq!(sanitize_globs(&json!(many)).map(|g| g.len()), Some(50));
        let ignore = sanitize_ignore(&json!({"D:\\a": ["rm-rf", "rm-rf", "nope", "custom:x", 5], "": ["rm-rf"], "D:\\b": [], "D:\\c": "rm-rf"})).map(Value::Object);
        assert_eq!(ignore, Some(json!({"D:\\a": ["rm-rf", "custom:x"]})));
        assert_eq!(sanitize_ignore(&json!([])), None);
        assert_eq!(sanitize_groups(&json!({"git": false, "zzz": true})).map(Value::Object), Some(json!({"git": false})));
        assert!(is_rule("outside-project") && is_rule(".env*") && is_rule("custom:abc") && is_rule("rm-rf") && !is_rule("rm") && !is_rule(&format!("custom:{}", "x".repeat(220))));
    }

    #[test]
    fn absolute_paths() {
        assert!(is_absolute_path("/usr/x"));
        if cfg!(windows) {
            assert!(is_absolute_path("D:\\work") && is_absolute_path("C:/x") && is_absolute_path("\\\\srv\\share") && !is_absolute_path("D:work") && !is_absolute_path("work"));
        } else {
            assert!(!is_absolute_path("D:\\work") && !is_absolute_path("work"));
        }
    }
}
