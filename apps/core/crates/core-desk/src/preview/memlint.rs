use super::ctx::{imports_of, js_lines, JS_S};
use crate::claude;
use core_transcript::jsstr::{js_trim, units, utf16_cmp};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub const LARGE_CHARS: usize = 40_000;
pub const INDEX_LINES: usize = 200;
pub const INDEX_CHARS: usize = 25_000;
pub const FINDINGS_MAX: usize = 500;
const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

fn re(cell: &'static OnceLock<Regex>, pattern: impl FnOnce() -> String) -> &'static Regex {
    cell.get_or_init(|| Regex::new(&pattern()).expect("regex"))
}

fn top_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || format!(r"^([A-Za-z_][A-Za-z0-9_-]*)[{JS_S}]*:[{JS_S}]*({DOT}*)$"))
}

fn sub_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || format!(r"^[{JS_S}]+([A-Za-z_][A-Za-z0-9_-]*)[{JS_S}]*:[{JS_S}]*({DOT}*)$"))
}

fn fence_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || format!(r"^[{JS_S}]*(?:```|~~~)"))
}

fn split_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || format!(r"[{JS_S}`'\x22()<>\[\]{{}}|,;]+"))
}

fn tok_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || r"^[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+/?$".to_string())
}

fn ext_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || r"\.[A-Za-z0-9]{1,8}$".to_string())
}

fn wiki_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || r"\[\[([^\]|#\n]+)(?:[|#][^\]\n]*)?\]\]".to_string())
}

fn md_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || format!(r#"\[[^\]\n]*\]\(([^)\n{JS_S}]+)(?:[{JS_S}]+"[^"]*")?\)"#))
}

fn scheme_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, || r"(?i)^(?:[a-z][a-z0-9+.-]*:|#)".to_string())
}

pub struct Doc {
    pub path: String,
    pub role: &'static str,
    pub text: Option<String>,
    pub bytes: u64,
    pub in_project: bool,
}

pub struct Frontmatter {
    pub fields: HashMap<String, String>,
    pub end: usize,
}

fn unquote(v: &str) -> String {
    let v = js_trim(v);
    let v = v.strip_prefix(['"', '\'']).unwrap_or(v);
    v.strip_suffix(['"', '\'']).unwrap_or(v).to_string()
}

pub fn frontmatter(text: &str) -> Option<Frontmatter> {
    let lines = js_lines(text);
    if js_trim(lines.first()?) != "---" {
        return None;
    }
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut in_meta = false;
    for (i, raw) in lines.iter().enumerate().skip(1) {
        if js_trim(raw) == "---" {
            return Some(Frontmatter { fields, end: i + 1 });
        }
        if let Some(m) = top_re().captures(raw) {
            in_meta = &m[1] == "metadata";
            if !in_meta {
                fields.insert(m[1].to_string(), unquote(&m[2]));
            }
            continue;
        }
        if let Some(m) = sub_re().captures(raw) {
            if in_meta && !fields.contains_key(&m[1]) {
                fields.insert(m[1].to_string(), unquote(&m[2]));
            }
        }
    }
    None
}

fn prose_lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence = false;
    for (i, raw) in js_lines(text).into_iter().enumerate() {
        if fence_re().is_match(raw) {
            fence = !fence;
        } else if !fence {
            out.push((i + 1, raw));
        }
    }
    out
}

pub fn path_tokens(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in split_re().split(line) {
        let mut t = raw.strip_prefix("./").unwrap_or(raw).to_string();
        if let Some(i) = t.find('#') {
            t.truncate(i);
        }
        while let Some(i) = t.rfind(':') {
            let digits = &t[i + 1..];
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                t.truncate(i);
            } else {
                break;
            }
        }
        let t = t.trim_end_matches(['.', ':', '!', '?']).to_string();
        if !tok_re().is_match(&t) || t.contains("..") || t.starts_with(".git/") {
            continue;
        }
        if !t.ends_with('/') && !ext_re().is_match(&t) {
            continue;
        }
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

fn wiki_links(line: &str) -> Vec<String> {
    wiki_re().captures_iter(line).map(|m| js_trim(&m[1]).to_string()).filter(|s| !s.is_empty()).collect()
}

fn md_links(line: &str) -> Vec<String> {
    md_re().captures_iter(line).map(|m| m[1].to_string()).filter(|t| !scheme_re().is_match(t)).collect()
}

fn stem(file: &str) -> String {
    let base = crate::util::basename(file);
    if base.len() >= 3 && base.get(base.len() - 3..).is_some_and(|s| s.eq_ignore_ascii_case(".md")) {
        base[..base.len() - 3].to_string()
    } else {
        base
    }
}

fn strip_md(w: &str) -> &str {
    if w.len() >= 3 && w.is_char_boundary(w.len() - 3) && w[w.len() - 3..].eq_ignore_ascii_case(".md") {
        &w[..w.len() - 3]
    } else {
        w
    }
}

pub fn decode_uri(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[derive(Clone)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub kind: &'static str,
    pub text: String,
    pub limit: Option<usize>,
}

pub struct LintCtx<'a> {
    pub cwd: &'a str,
    pub home: &'a str,
    pub memory_dir: &'a str,
    pub sub_roots: &'a [String],
    pub fold_case: bool,
}

struct Lint<'a> {
    ctx: &'a LintCtx<'a>,
    findings: Vec<Finding>,
    known: HashMap<String, bool>,
}

impl Lint<'_> {
    fn fold(&self, p: &str) -> String {
        claude::fold(&claude::resolve(p), self.ctx.fold_case)
    }

    fn push(&mut self, f: Finding) {
        if !self.findings.iter().any(|x| x.file == f.file && x.line == f.line && x.kind == f.kind && x.text == f.text) {
            self.findings.push(f);
        }
    }

    fn exists(&mut self, abs: &str) -> bool {
        let k = self.fold(abs);
        *self.known.entry(k).or_insert_with(|| std::fs::metadata(abs).is_ok())
    }
}

pub fn lint_docs(docs: &[Doc], ctx: &LintCtx) -> (Vec<Finding>, bool) {
    let mut l = Lint { ctx, findings: Vec::new(), known: HashMap::new() };
    let fnd = |file: &str, line: usize, kind: &'static str, text: String, limit: Option<usize>| Finding { file: file.to_string(), line, kind, text, limit };
    let mut bases: Vec<String> = vec![ctx.cwd.to_string()];
    bases.extend(ctx.sub_roots.iter().cloned());
    let memories: Vec<&Doc> = docs.iter().filter(|d| d.role == "memory" && crate::util::basename(&d.path).to_lowercase() != "memory.md").collect();
    let index = docs.iter().find(|d| d.role == "memory-index");
    let mut names: Vec<String> = Vec::new();
    let mut by_name: Vec<(String, Vec<&Doc>)> = Vec::new();
    for m in &memories {
        names.push(stem(&m.path).to_lowercase());
        let fm = m.text.as_deref().and_then(frontmatter);
        if let Some(name) = fm.as_ref().and_then(|f| f.fields.get("name")).filter(|n| !n.is_empty()) {
            let lower = name.to_lowercase();
            names.push(lower.clone());
            match by_name.iter_mut().find(|(k, _)| *k == lower) {
                Some((_, v)) => v.push(m),
                None => by_name.push((lower, vec![m])),
            }
        }
        if m.text.is_some() {
            let missing: Vec<&str> = ["name", "description", "type"].into_iter().filter(|k| fm.as_ref().and_then(|f| f.fields.get(*k)).map(|v| v.is_empty()).unwrap_or(true)).collect();
            if !missing.is_empty() {
                l.push(fnd(&m.path, 1, "frontmatter", missing.join(", "), None));
            }
        }
    }
    for (name, list) in &by_name {
        if list.len() > 1 {
            for m in list {
                l.push(fnd(&m.path, 1, "duplicate-name", name.clone(), None));
            }
        }
    }
    let mut indexed: Vec<String> = Vec::new();
    if let Some(idx) = index.filter(|d| d.text.as_deref().is_some_and(|t| !t.is_empty())) {
        let text = idx.text.as_deref().unwrap_or("");
        let trimmed = js_trim(text);
        let count = if trimmed.is_empty() { 0 } else { trimmed.split('\n').count() };
        if count > INDEX_LINES {
            l.push(fnd(&idx.path, INDEX_LINES + 1, "index-too-long", count.to_string(), Some(INDEX_LINES)));
        }
        if units(trimmed) > INDEX_CHARS {
            l.push(fnd(&idx.path, 1, "too-large", units(trimmed).to_string(), Some(INDEX_CHARS)));
        }
        for (line, t) in prose_lines(text) {
            for target in md_links(t) {
                let rel = decode_uri(&target).unwrap_or_else(|| target.clone());
                let abs = claude::resolve(&Path::new(ctx.memory_dir).join(&rel).to_string_lossy());
                indexed.push(l.fold(&abs));
                if !l.exists(&abs) {
                    l.push(fnd(&idx.path, line, "index-missing-file", target, None));
                }
            }
            for w in wiki_links(t) {
                let p = claude::join(ctx.memory_dir, &[&format!("{}.md", strip_md(&w))]);
                indexed.push(l.fold(&p));
            }
        }
    }
    for m in &memories {
        if !indexed.contains(&l.fold(&m.path)) {
            l.push(fnd(&m.path, 1, "not-indexed", crate::util::basename(&m.path), None));
        }
    }
    for d in docs {
        let size = d.text.as_deref().map(units).unwrap_or(d.bytes as usize);
        if d.role != "memory-index" && size > LARGE_CHARS {
            l.push(fnd(&d.path, 1, "too-large", size.to_string(), Some(LARGE_CHARS)));
        }
        let Some(text) = d.text.as_deref() else { continue };
        let body = frontmatter(text);
        for (line, t) in prose_lines(text) {
            if body.as_ref().is_some_and(|b| line <= b.end) {
                continue;
            }
            if d.role != "instructions" {
                for w in wiki_links(t) {
                    if !names.contains(&strip_md(&w).to_lowercase()) {
                        l.push(fnd(&d.path, line, "broken-link", w, None));
                    }
                }
            }
            if d.role == "instructions" {
                for abs in imports_of(t, &claude::dirname(&d.path), ctx.home) {
                    if !l.exists(&abs) {
                        l.push(fnd(&d.path, line, "missing-import", abs, None));
                    }
                }
            }
            if d.role == "memory-index" || (d.role == "instructions" && !d.in_project) {
                continue;
            }
            for tok in path_tokens(t) {
                let first = tok.split('/').next().unwrap_or("").to_string();
                let (mut anchored, mut found) = (false, false);
                for base in &bases {
                    if !l.exists(&Path::new(base).join(&first).to_string_lossy()) {
                        continue;
                    }
                    anchored = true;
                    found = l.exists(&Path::new(base).join(&tok).to_string_lossy());
                    if found {
                        break;
                    }
                }
                if anchored && !found {
                    l.push(fnd(&d.path, line, "missing-path", tok, None));
                }
            }
        }
    }
    let mut f = l.findings;
    f.sort_by(|a, b| if a.file == b.file { a.line.cmp(&b.line) } else { utf16_cmp(&a.file, &b.file) });
    let truncated = f.len() > FINDINGS_MAX;
    f.truncate(FINDINGS_MAX);
    (f, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_tokens_links() {
        let fm = frontmatter("---\nname: \"a\"\ntype: x\nmetadata:\n  type: y\n  note: z\n---\nbody").unwrap();
        assert_eq!(fm.fields.get("name").map(String::as_str), Some("a"));
        assert_eq!(fm.fields.get("note").map(String::as_str), Some("z"));
        assert_eq!(fm.end, 7);
        assert!(frontmatter("no").is_none());
        assert_eq!(path_tokens("see `apps/web/x.ts:12` and .planning/ or (src/a/b.md#h)."), vec!["apps/web/x.ts", "src/a/b.md"]);
        assert_eq!(wiki_links("[[a]] [[b|c]] [[ d.md#e]]"), vec!["a", "b", "d.md"]);
        assert_eq!(md_links("[x](a%20b.md) [y](https://z) [z](#h)"), vec!["a%20b.md"]);
        assert_eq!(decode_uri("a%20b").as_deref(), Some("a b"));
        assert!(decode_uri("%zz").is_none());
    }
}
