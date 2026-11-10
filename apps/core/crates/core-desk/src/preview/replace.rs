use super::files::{decode_utf8, write_text_file, MAX_WRITE_BYTES};
use super::paths::{is_absolute, real_or_null, root_of};
use super::search::{clean_query, Query, SEARCH_MAX_MATCHES};
use crate::js;
use serde_json::{json, Value};
use std::path::Path;

pub const REPLACE_MAX_FILES: usize = 500;
const MAX_REPLACEMENT: usize = 1000;
const SNIFF_BYTES: usize = 8 * 1024;

pub struct Line {
    pub line: usize,
    pub text: String,
}

pub struct Target {
    pub path: String,
    pub lines: Vec<Line>,
}

pub struct Request {
    pub query: Query,
    pub replacement: String,
    pub files: Vec<Target>,
}

pub enum Cleaned {
    Ok(Request),
    Error(&'static str),
    Denied,
}

fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub fn matcher(q: &Query) -> Option<regress::Regex> {
    let body = if q.regex { q.q.clone() } else { escape(&q.q) };
    let pattern = if q.word { format!("(?<![\\p{{L}}\\p{{N}}_])(?:{body})(?![\\p{{L}}\\p{{N}}_])") } else { body };
    regress::Regex::with_flags(&pattern, if q.case_sensitive { "u" } else { "iu" }).ok()
}

fn expand(template: &str, text: &str, m: &regress::Match, regex: bool) -> String {
    if !regex {
        return template.to_string();
    }
    let b = template.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        if b[i] == b'$' && i + 1 < b.len() && (matches!(b[i + 1], b'1'..=b'9') || b[i + 1] == b'&') {
            let k = b[i + 1];
            out.push_str(&template[last..i]);
            if k == b'&' {
                out.push_str(&text[m.range()]);
            } else {
                let n = (k - b'0') as usize;
                if n < m.captures.len() + 1 {
                    if let Some(r) = m.group(n) {
                        out.push_str(&text[r]);
                    }
                } else {
                    out.push_str(&template[i..i + 2]);
                }
            }
            i += 2;
            last = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&template[last..]);
    out
}

pub fn replace_in_line(line: &str, re: &regress::Regex, template: &str, regex: bool) -> (String, usize) {
    let mut out = String::new();
    let mut last = 0;
    let mut count = 0;
    for m in re.find_iter(line) {
        if m.range().is_empty() {
            continue;
        }
        let at = m.start();
        out.push_str(&line[last..at]);
        out.push_str(&expand(template, line, &m, regex));
        last = m.end();
        count += 1;
    }
    if count == 0 {
        return (line.to_string(), 0);
    }
    out.push_str(&line[last..]);
    (out, count)
}

fn snippet_of(shown: &str) -> &str {
    let s = shown.strip_prefix('…').unwrap_or(shown);
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

pub fn plan_file(text: &str, lines: &[Line], re: &regress::Regex, template: &str, regex: bool) -> Result<(String, usize), &'static str> {
    let mut rows: Vec<String> = text.split('\n').map(String::from).collect();
    let mut count = 0;
    for l in lines {
        let Some(raw) = rows.get(l.line - 1).cloned() else { return Err("changed") };
        let cr = raw.ends_with('\r');
        let body = if cr { &raw[..raw.len() - 1] } else { &raw[..] };
        if !body.contains(snippet_of(&l.text)) {
            return Err("changed");
        }
        let (replaced, n) = replace_in_line(body, re, template, regex);
        if n == 0 {
            return Err("no-match");
        }
        rows[l.line - 1] = if cr { format!("{replaced}\r") } else { replaced };
        count += n;
    }
    Ok((rows.join("\n"), count))
}

fn is_rel(p: &Value) -> Option<&str> {
    let s = p.as_str()?;
    let bad = s.is_empty() || js::len(s) > 1024 || s.contains(['\0', '\r', '\n']) || is_absolute(s) || (s.len() >= 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':') || s.split(['\\', '/']).any(|c| c == "..");
    (!bad).then_some(s)
}

pub fn clean_replace(input: &Value) -> Cleaned {
    let Some(query) = clean_query(input.get("query").unwrap_or(&Value::Null)) else { return Cleaned::Error("bad-query") };
    let Some(replacement) = input.get("replacement").and_then(Value::as_str).filter(|r| js::len(r) <= MAX_REPLACEMENT && !r.contains(['\0', '\r', '\n'])) else { return Cleaned::Denied };
    let Some(files) = input.get("files").and_then(Value::as_array).filter(|f| !f.is_empty()) else { return Cleaned::Denied };
    if files.len() > REPLACE_MAX_FILES {
        return Cleaned::Error("too-many");
    }
    let mut out: Vec<Target> = Vec::new();
    let mut total = 0;
    for f in files {
        let Some(rel) = is_rel(f.get("path").unwrap_or(&Value::Null)) else { return Cleaned::Denied };
        let Some(lines) = f.get("lines").and_then(Value::as_array).filter(|l| !l.is_empty()) else { return Cleaned::Denied };
        if out.iter().any(|t| t.path == rel) {
            return Cleaned::Denied;
        }
        let mut clean: Vec<Line> = Vec::new();
        for l in lines {
            let n = l.get("line").and_then(Value::as_f64).filter(|n| n.fract() == 0.0 && *n >= 1.0 && *n < 9e15);
            let text = l.get("text").and_then(Value::as_str).filter(|t| js::len(t) <= 400);
            let (Some(n), Some(text)) = (n, text) else { return Cleaned::Denied };
            clean.push(Line { line: n as usize, text: text.to_string() });
        }
        total += clean.len();
        if total > SEARCH_MAX_MATCHES {
            return Cleaned::Error("too-many");
        }
        clean.sort_by_key(|l| l.line);
        let mut dedup: Vec<Line> = Vec::new();
        for l in clean {
            if dedup.last().map(|p| p.line != l.line).unwrap_or(true) {
                dedup.push(l);
            }
        }
        out.push(Target { path: rel.to_string(), lines: dedup });
    }
    Cleaned::Ok(Request { query, replacement: replacement.to_string(), files: out })
}

fn skip(path: &str, error: &str) -> Value {
    json!({"path": path, "ok": false, "error": error})
}

fn replace_one(root: &str, t: &Target, re: &regress::Regex, req: &Request, roots: &[String], snapshot: &dyn Fn(&str, &str)) -> Value {
    let Some(real) = real_or_null(&Path::new(root).join(&t.path).to_string_lossy()) else { return skip(&t.path, "missing") };
    if root_of(&real, &[root.to_string()]).is_none() || root_of(&real, roots).is_none() {
        return skip(&t.path, "denied");
    }
    let Some(meta) = std::fs::metadata(&real).ok().filter(|m| m.is_file()) else { return skip(&t.path, "missing") };
    if meta.len() as usize > MAX_WRITE_BYTES {
        return skip(&t.path, "too-large");
    }
    let Ok(bytes) = std::fs::read(&real) else { return skip(&t.path, "missing") };
    if bytes[..bytes.len().min(SNIFF_BYTES)].contains(&0) {
        return skip(&t.path, "encoding");
    }
    let Some(text) = decode_utf8(&bytes) else { return skip(&t.path, "encoding") };
    let (next, count) = match plan_file(&text, &t.lines, re, &req.replacement, req.query.regex) {
        Ok(p) => p,
        Err(e) => return skip(&t.path, e),
    };
    if next == text {
        return json!({"path": t.path, "ok": true, "count": 0});
    }
    snapshot(&real, &text);
    let written = write_text_file(&json!({"path": real, "text": next, "expectedMtime": crate::util::mtime_ms(&meta)}), roots, &|_, _| {});
    if written["ok"] == true {
        return json!({"path": t.path, "ok": true, "count": count});
    }
    let err = written["error"].as_str().unwrap_or("failed");
    skip(&t.path, if matches!(err, "missing" | "denied" | "too-large" | "encoding" | "conflict" | "busy") { err } else { "failed" })
}

pub fn apply_replace(root: &str, req: &Request, roots: &[String], snapshot: &dyn Fn(&str, &str)) -> Value {
    let Some(re) = matcher(&req.query) else { return json!({"ok": false, "error": "unsupported"}) };
    let mut results: Vec<Value> = Vec::new();
    let (mut files, mut count) = (0, 0u64);
    for t in &req.files {
        let r = replace_one(root, t, &re, req, roots, snapshot);
        if r["ok"] == true {
            files += 1;
            count += r["count"].as_u64().unwrap_or(0);
        }
        results.push(r);
    }
    json!({"ok": true, "results": results, "files": files, "count": count})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str, regex: bool, case: bool, word: bool) -> Query {
        Query { q: text.into(), regex, case_sensitive: case, word }
    }

    fn lines(v: &[(usize, &str)]) -> Vec<Line> {
        v.iter().map(|(l, t)| Line { line: *l, text: t.to_string() }).collect()
    }

    #[test]
    fn matcher_and_templates() {
        let re = matcher(&q("a.b", false, false, false)).unwrap();
        assert_eq!(replace_in_line("A.B axb a.b", &re, "X", false), ("X axb X".to_string(), 2));
        let re = matcher(&q("(\\w+)-(\\d)", true, true, false)).unwrap();
        assert_eq!(replace_in_line("ab-1 cd-2", &re, "$2:$1 $& $$ $3", true), ("1:ab ab-1 $$ $3 2:cd cd-2 $$ $3".to_string(), 2));
        let re = matcher(&q("(?<=x)y", true, true, false)).unwrap();
        assert_eq!(replace_in_line("xy zy", &re, "Q", true), ("xQ zy".to_string(), 1));
        let re = matcher(&q("cat", false, true, true)).unwrap();
        assert_eq!(replace_in_line("cat concat cat_ é cat", &re, "dog", false).1, 2);
        let re = matcher(&q("é", false, false, false)).unwrap();
        assert_eq!(replace_in_line("xÉx", &re, "e", false), ("xex".to_string(), 1));
        assert!(matcher(&q("(", true, true, false)).is_none());
        assert!(matcher(&q("^", true, true, false)).is_some());
        let re = matcher(&q("^", true, true, false)).unwrap();
        assert_eq!(replace_in_line("abc", &re, "X", true).1, 0);
    }

    #[test]
    fn plans_files() {
        let re = matcher(&q("foo", false, true, false)).unwrap();
        let ok = plan_file("a foo\r\nb\nfoo foo\n", &lines(&[(1, "a foo"), (3, "foo foo")]), &re, "bar", false).unwrap();
        assert_eq!(ok, ("a bar\r\nb\nbar bar\n".to_string(), 3));
        assert_eq!(plan_file("a foo", &lines(&[(1, "zzz")]), &re, "x", false), Err("changed"));
        assert_eq!(plan_file("a foo", &lines(&[(2, "foo")]), &re, "x", false), Err("changed"));
        assert_eq!(plan_file("a Foo", &lines(&[(1, "Foo")]), &re, "x", false), Err("no-match"));
        assert_eq!(plan_file("a foo", &lines(&[(1, "…a foo")]), &re, "x", false).unwrap().1, 1);
    }

    #[test]
    fn cleans_requests() {
        let good = json!({"query": {"q": "a"}, "replacement": "b", "files": [{"path": "x/y.ts", "lines": [{"line": 3, "text": "t"}, {"line": 1, "text": "u"}, {"line": 3, "text": "v"}]}]});
        let Cleaned::Ok(r) = clean_replace(&good) else { panic!("clean") };
        assert_eq!(r.files[0].lines.iter().map(|l| (l.line, l.text.as_str())).collect::<Vec<_>>(), vec![(1, "u"), (3, "t")]);
        assert!(matches!(clean_replace(&json!({"query": {}, "replacement": "b", "files": []})), Cleaned::Error("bad-query")));
        assert!(matches!(clean_replace(&json!({"query": {"q": "a"}, "replacement": "b\n", "files": []})), Cleaned::Denied));
        let abs = json!({"query": {"q": "a"}, "replacement": "b", "files": [{"path": "../x", "lines": [{"line": 1, "text": "t"}]}]});
        assert!(matches!(clean_replace(&abs), Cleaned::Denied));
    }
}
