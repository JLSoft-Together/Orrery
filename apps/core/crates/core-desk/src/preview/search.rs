use super::exec::{exec_git, ExecOpts};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const SEARCH_MAX_MATCHES: usize = 2000;
const MAX_TEXT: usize = 200;
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub struct Query {
    pub q: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub word: bool,
}

pub fn clean_query(v: &Value) -> Option<Query> {
    let q = v.get("q").and_then(Value::as_str)?;
    if q.is_empty() || crate::js::len(q) > 500 || q.contains(['\0', '\r', '\n']) {
        return None;
    }
    let t = |k: &str| v.get(k) == Some(&json!(true));
    Some(Query { q: q.to_string(), regex: t("regex"), case_sensitive: t("caseSensitive"), word: t("word") })
}

pub fn grep_args(q: &Query) -> Vec<&str> {
    let mut a = vec!["grep", "-n", "--column", "-I", "-z", "--untracked", "--no-color", if q.regex { "-E" } else { "-F" }];
    if !q.case_sensitive {
        a.push("-i");
    }
    if q.word {
        a.push("-w");
    }
    a.push("-e");
    a.push(&q.q);
    a
}

pub fn utf16_sub(s: &str, start: usize, end: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().collect();
    let a = start.min(units.len());
    let b = end.min(units.len()).max(a);
    String::from_utf16_lossy(&units[a..b])
}

fn js_int(s: &str) -> Option<f64> {
    let t = s.trim();
    let n: f64 = if t.is_empty() { 0.0 } else { t.parse().ok()? };
    if n.fract() == 0.0 && n.is_finite() {
        Some(n)
    } else {
        None
    }
}

pub fn parse_grep(out: &str, max: usize) -> (Vec<Value>, bool) {
    let mut matches = Vec::new();
    let mut truncated = false;
    for rec in out.split('\n') {
        if rec.is_empty() {
            continue;
        }
        let mut it = rec.splitn(4, '\0');
        let (Some(p), Some(l), Some(c), Some(raw)) = (it.next(), it.next(), it.next(), it.next()) else { continue };
        let (Some(line), Some(col)) = (js_int(l), js_int(c)) else { continue };
        if matches.len() >= max {
            truncated = true;
            break;
        }
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let start = (col - 1.0 - 60.0).max(0.0) as usize;
        let text = format!("{}{}", if start > 0 { "…" } else { "" }, utf16_sub(raw, start, start + MAX_TEXT));
        let col_out = col - start as f64 + if start > 0 { 1.0 } else { 0.0 };
        matches.push(json!({"path": p.replace('\\', "/"), "line": line as i64, "col": col_out as i64, "text": text}));
    }
    (matches, truncated)
}

pub fn search(dir: &str, q: &Query, signal: Arc<AtomicBool>) -> Value {
    let mut o = ExecOpts::timeout(15_000).with_max(MAX_BYTES);
    o.signal = Some(signal.clone());
    let res = exec_git(Path::new(dir), &grep_args(q), &o);
    if signal.load(Ordering::SeqCst) {
        return json!({"ok": false, "error": "cancelled"});
    }
    if res.code == Some(1) && res.stderr.trim().is_empty() {
        return json!({"ok": true, "matches": [], "truncated": false});
    }
    if !res.ok() {
        let low = res.stderr.to_lowercase();
        if res.stderr.contains("git not found") {
            return json!({"ok": false, "error": "no-git"});
        }
        if low.contains("not a git repository") {
            return json!({"ok": false, "error": "not-repo"});
        }
        let d = crate::js::slice(res.stderr.trim(), 200);
        if res.code == Some(128) && ["regex", "regexp", "unmatched", "invalid"].iter().any(|w| low.contains(w)) {
            return json!({"ok": false, "error": "bad-query", "detail": d});
        }
        return json!({"ok": false, "error": "failed", "detail": d});
    }
    let (matches, truncated) = parse_grep(&res.stdout, SEARCH_MAX_MATCHES);
    json!({"ok": true, "matches": matches, "truncated": truncated || res.truncated})
}

pub fn user_commands_dir() -> PathBuf {
    let cfg = std::env::var("CLAUDE_CONFIG_DIR").ok().filter(|v| !v.is_empty());
    let base = cfg.map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
        PathBuf::from(home).join(".claude")
    });
    base.join("commands")
}

fn read_head(file: &Path) -> String {
    let Ok(mut f) = std::fs::File::open(file) else { return String::new() };
    let mut buf = vec![0u8; 4096];
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    String::from_utf8_lossy(&buf[..n]).to_string()
}

fn clip(s: &str) -> String {
    if crate::js::len(s) > 200 {
        format!("{}…", crate::js::slice(s, 199))
    } else {
        s.to_string()
    }
}

fn unquote(s: &str) -> String {
    let b = s.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn describe(head: &str) -> String {
    let head = head.strip_prefix('\u{feff}').unwrap_or(head);
    let lines: Vec<&str> = head.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut i = 0;
    if lines.first().map(|l| l.trim()) == Some("---") {
        i = 1;
        while i < lines.len() && lines[i].trim() != "---" {
            if let Some(rest) = lines[i].strip_prefix("description:") {
                return clip(&unquote(rest.trim()));
            }
            i += 1;
        }
        i += 1;
    }
    while i < lines.len() {
        let l = lines[i].trim_start_matches('#').trim();
        if !l.is_empty() {
            return clip(l);
        }
        i += 1;
    }
    String::new()
}

pub fn list_commands(dirs: &[(PathBuf, &str)]) -> Vec<Value> {
    let name_ok = |n: &str| !n.is_empty() && n.chars().count() <= 80 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');
    let mut out = Vec::new();
    for (dir, scope) in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.filter_map(Result::ok) {
            if out.len() >= 200 {
                return out;
            }
            let file = e.file_name().to_string_lossy().to_string();
            if !e.file_type().map(|t| t.is_file()).unwrap_or(false) || !file.to_lowercase().ends_with(".md") {
                continue;
            }
            let name = &file[..file.len() - 3];
            if !name_ok(name) {
                continue;
            }
            out.push(json!({"name": name, "description": describe(&read_head(&dir.join(&file))), "scope": scope}));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grep_and_describe() {
        let (m, t) = parse_grep("src\\a.ts\x0012\x003\x00  hit\r\nbad\n", 10);
        assert!(!t);
        assert_eq!(m[0]["path"], "src/a.ts");
        assert_eq!(m[0]["col"], 3);
        assert_eq!(describe("---\ntitle: x\ndescription: \"Do it\"\n---\n# H"), "Do it");
        assert_eq!(describe("\n## Heading here\nmore"), "Heading here");
    }
}
