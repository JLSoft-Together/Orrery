use super::log::locale_compare;
use super::paths::{dirname, real_or_null, root_key};
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

const TEMPLATES: [&str; 3] = [".env.example", ".env.sample", ".env.template"];
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_FILES: usize = 40;

pub fn is_env_file_name(name: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    crate::js::len(name) <= 255 && R.get_or_init(|| Regex::new(r"^\.env(\.[^/\\]+)?$").expect("regex")).is_match(name) && !name.contains("..")
}

pub fn is_env_template(name: &str) -> bool {
    let low = name.to_lowercase();
    is_env_file_name(name) && (low.ends_with(".example") || low.ends_with(".sample") || low.ends_with(".template"))
}

#[derive(Clone)]
pub struct Entry {
    pub value: String,
    pub line: usize,
}

fn read_quoted(lines: &[&str], start: usize, rest: &str, quote: char) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut text: Vec<char> = rest.chars().collect();
    let mut i = start;
    while i < lines.len() {
        let mut j = 0;
        while j < text.len() {
            let c = text[j];
            if c == quote {
                return Some((out, i));
            }
            if c == '\\' && quote == '"' && j + 1 < text.len() {
                j += 1;
                let n = text[j];
                match n {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '"' | '\\' => out.push(n),
                    other => {
                        out.push('\\');
                        out.push(other);
                    }
                }
            } else {
                out.push(c);
            }
            j += 1;
        }
        out.push('\n');
        text = lines.get(i + 1).map(|l| l.chars().collect()).unwrap_or_default();
        i += 1;
    }
    None
}

pub fn parse_env(text: &str) -> Vec<(String, Entry)> {
    static LINE: OnceLock<Regex> = OnceLock::new();
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    let line_re = LINE.get_or_init(|| Regex::new(r"^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_.-]*)\s*=\s?(.*)$").expect("regex"));
    let comment = COMMENT.get_or_init(|| Regex::new(r"\s+#.*$").expect("regex"));
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let norm = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = norm.split('\n').collect();
    let mut out: Vec<(String, Entry)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(m) = line_re.captures(lines[i]) else {
            i += 1;
            continue;
        };
        let key = m[1].to_string();
        let rest = m[2].trim_start_matches(super::branches::js_space).to_string();
        let line = i + 1;
        let quote = rest.chars().next();
        let quoted = match quote {
            Some(q @ ('"' | '\'' | '`')) => read_quoted(&lines, i, &rest[1..], q),
            _ => None,
        };
        let value = match quoted {
            Some((v, end)) => {
                i = end;
                v
            }
            None if rest.starts_with('#') => String::new(),
            None => comment.replace(&rest, "").trim_matches(super::branches::js_space).to_string(),
        };
        let e = Entry { value, line };
        match out.iter_mut().find(|(k, _)| *k == key) {
            Some((_, x)) => *x = e,
            None => out.push((key, e)),
        }
        i += 1;
    }
    out
}

fn is_empty(e: Option<&Entry>) -> bool {
    e.map(|e| e.value.trim_matches(super::branches::js_space).is_empty()).unwrap_or(false)
}

fn get<'a>(m: &'a [(String, Entry)], k: &str) -> Option<&'a Entry> {
    m.iter().find(|(x, _)| x == k).map(|(_, e)| e)
}

pub fn compare_env(template: Option<&[(String, Entry)]>, file: &[(String, Entry)]) -> Vec<Value> {
    let mut rows = Vec::new();
    if let Some(t) = template {
        for (key, _) in t {
            let e = get(file, key);
            let status = if e.is_none() {
                "missing"
            } else if is_empty(e) {
                "empty"
            } else {
                "ok"
            };
            rows.push(json!({"key": key, "status": status, "empty": is_empty(e), "line": e.map(|e| e.line)}));
        }
    }
    for (key, e) in file {
        if template.map(|t| get(t, key).is_some()).unwrap_or(false) {
            continue;
        }
        let status = if template.is_some() {
            "extra"
        } else if is_empty(Some(e)) {
            "empty"
        } else {
            "ok"
        };
        rows.push(json!({"key": key, "status": status, "empty": is_empty(Some(e)), "line": e.line}));
    }
    rows
}

fn count_rows(rows: &[Value]) -> Value {
    let n = |s: &str| rows.iter().filter(|r| r["status"] == json!(s)).count();
    json!({"ok": n("ok"), "empty": n("empty"), "missing": n("missing"), "extra": n("extra")})
}

fn read_env(file: &Path) -> Result<Vec<(String, Entry)>, &'static str> {
    let meta = std::fs::symlink_metadata(file).ok().filter(|m| m.is_file()).ok_or("unreadable")?;
    if meta.len() > MAX_BYTES {
        return Err("too-large");
    }
    let b = std::fs::read(file).map_err(|_| "unreadable")?;
    Ok(parse_env(&String::from_utf8_lossy(&b)))
}

pub fn check_env(root: &str) -> Value {
    let Ok(rd) = std::fs::read_dir(root) else { return json!({"ok": false, "error": "failed"}) };
    let names: Vec<String> = rd
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| is_env_file_name(n))
        .collect();
    let template_name = TEMPLATES.iter().find(|t| names.iter().any(|n| n == *t)).map(|s| s.to_string());
    let template = template_name.as_ref().and_then(|t| read_env(&Path::new(root).join(t)).ok());
    let mut targets: Vec<String> = names.iter().filter(|n| !is_env_template(n)).cloned().collect();
    targets.sort_by(|a, b| {
        if a == b {
            std::cmp::Ordering::Equal
        } else if a == ".env" {
            std::cmp::Ordering::Less
        } else if b == ".env" {
            std::cmp::Ordering::Greater
        } else {
            locale_compare(a, b)
        }
    });
    let mut files = Vec::new();
    for name in targets.iter().take(MAX_FILES) {
        match read_env(&Path::new(root).join(name)) {
            Ok(parsed) => {
                let rows = compare_env(template.as_deref(), &parsed);
                let counts = count_rows(&rows);
                files.push(json!({"name": name, "rows": rows, "counts": counts}));
            }
            Err(e) => files.push(json!({"name": name, "rows": [], "counts": count_rows(&[]), "error": e})),
        }
    }
    let tpl = match (&template_name, &template) {
        (Some(n), Some(t)) => json!({"name": n, "keys": t.iter().map(|(k, e)| json!({"key": k, "empty": is_empty(Some(e)), "line": e.line})).collect::<Vec<_>>()}),
        _ => Value::Null,
    };
    json!({"ok": true, "root": root, "template": tpl, "files": files, "truncated": targets.len() > MAX_FILES})
}

pub fn reveal_env(root: &str, name: &Value, key: &Value) -> Value {
    let denied = json!({"ok": false, "error": "denied"});
    let (Some(name), Some(key)) = (name.as_str().filter(|n| is_env_file_name(n)), key.as_str().filter(|k| !k.is_empty() && crate::js::len(k) <= 256)) else { return denied };
    let Some(real) = real_or_null(&Path::new(root).join(name).to_string_lossy()) else { return denied };
    if root_key(&dirname(&real)) != root_key(root) || crate::util::basename(&real) != name {
        return denied;
    }
    match read_env(Path::new(&real)) {
        Err("too-large") => json!({"ok": false, "error": "too-large"}),
        Err(_) => json!({"ok": false, "error": "failed"}),
        Ok(parsed) => match get(&parsed, key) {
            Some(e) => json!({"ok": true, "value": e.value}),
            None => json!({"ok": false, "error": "missing"}),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv() {
        let p = parse_env("\u{feff}A=1 # c\nexport B = \"x\\ny\nz\"\nC=#c\nD='q'\nA=2\n");
        assert_eq!(p[0].1.value, "2");
        assert_eq!(p[1].1.value, "x\ny\nz");
        assert_eq!(p[2].1.value, "");
        assert_eq!(p[3].1.value, "q");
        assert!(is_env_template(".env.Example"));
        assert!(!is_env_file_name(".envrc"));
    }
}
