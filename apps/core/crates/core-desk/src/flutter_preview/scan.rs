use super::mask::{line_at, mask_dart, match_close, split_top, sub, top_ranges};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewHit {
    pub line: u32,
    pub symbol: String,
    pub label: String,
    pub call: String,
    pub kind: Option<&'static str>,
    pub skipped: Option<&'static str>,
}

pub fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

pub fn part_of(masked: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?m)^\s*part\s+of\b").is_match(masked)
}

fn preview_name(src: &str, masked: &str, from: usize, to: usize) -> String {
    static KEY: OnceLock<Regex> = OnceLock::new();
    static VALUE: OnceLock<Regex> = OnceLock::new();
    let key_re = re(&KEY, r"^\s*name\s*:");
    let inner = sub(masked, from, to);
    for (a, b) in top_ranges(inner) {
        let Some(key) = key_re.find(sub(masked, from + a, from + b)) else { continue };
        let tail = sub(src, from + a + key.end(), from + b);
        let value = re(&VALUE, r#"^\s*(?:'([^'\\$\n]*)'|"([^"\\$\n]*)")\s*$"#).captures(tail);
        return value.and_then(|v| v.get(1).or_else(|| v.get(2))).map(|m| m.as_str().to_string()).unwrap_or_default();
    }
    String::new()
}

pub fn has_required_params(params: &str) -> bool {
    static REQ: OnceLock<Regex> = OnceLock::new();
    let named = params.find(['{', '[']);
    let positional = named.map_or(params, |n| &params[..n]);
    if !split_top(positional).is_empty() {
        return true;
    }
    let Some(named) = named else { return false };
    if params.as_bytes()[named] == b'[' {
        return false;
    }
    let end = params.rfind('}').unwrap_or(params.len().saturating_sub(1));
    split_top(sub(params, named + 1, end)).iter().any(|p| re(&REQ, r"(?:^|[\s)])required\s").is_match(p))
}

pub struct Scope {
    pub head: usize,
    pub open: usize,
    pub close: usize,
    pub class_name: Option<String>,
}

pub fn top_scopes(masked: &str) -> Vec<Scope> {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    let head_re = re(&HEAD, r"\b(?:class|mixin|enum|extension(?:\s+type)?)\s+([A-Za-z0-9_]+)");
    let b = masked.as_bytes();
    let mut scopes = Vec::new();
    let (mut k, mut segment) = (0usize, 0usize);
    while k < b.len() {
        if b[k] == b';' {
            segment = k + 1;
        }
        if b[k] == b'{' {
            let end = match_close(masked, k).unwrap_or(masked.len());
            let class_name = head_re.captures(sub(masked, segment, k)).map(|m| m[1].to_string());
            scopes.push(Scope { head: segment, open: k, close: end, class_name });
            k = end;
            segment = end;
            continue;
        }
        k += 1;
    }
    scopes
}

#[derive(Debug, Clone)]
pub struct Decl {
    pub symbol: String,
    pub call: String,
    pub skipped: Option<&'static str>,
}

fn skip(symbol: &str, why: &'static str) -> Decl {
    Decl { symbol: symbol.to_string(), call: String::new(), skipped: Some(why) }
}

pub struct Header {
    pub words: Vec<String>,
    pub name: String,
    pub params: String,
}

pub fn header(masked: &str, from: usize) -> Option<Header> {
    static OTHER: OnceLock<Regex> = OnceLock::new();
    static NAME: OnceLock<Regex> = OnceLock::new();
    let b = masked.as_bytes();
    let mut k = from;
    loop {
        while k < b.len() && b[k].is_ascii_whitespace() {
            k += 1;
        }
        let Some(other) = re(&OTHER, r"^@[A-Za-z_][A-Za-z0-9_.]*\s*").find(sub(masked, k, k + 200)) else { break };
        k += other.end();
        if b.get(k) == Some(&b'(') {
            k = match_close(masked, k)?;
        }
    }
    let mut angle = 0i32;
    let mut j = k;
    while j < b.len() {
        let c = b[j];
        if c == b'<' {
            angle += 1;
        } else if c == b'>' {
            angle -= 1;
        } else if angle == 0 && matches!(c, b'{' | b';' | b'=') {
            return None;
        } else if angle == 0 && c == b'(' {
            let end = match_close(masked, j)?;
            let before = sub(masked, k, j).trim();
            let name = re(&NAME, r"([A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)?)$").captures(before)?;
            let name_m = name.get(1)?;
            if name_m.as_str() == "Function" {
                j = end;
                continue;
            }
            let words = before[..name_m.start()].split_whitespace().map(String::from).collect();
            return Some(Header { words, name: name_m.as_str().to_string(), params: sub(masked, j + 1, end - 1).to_string() });
        }
        j += 1;
    }
    None
}

pub fn is_private(name: &str) -> bool {
    name.split('.').any(|p| p.starts_with('_'))
}

const NOT_WIDGET: [&str; 13] = ["Function", "Future", "FutureOr", "Stream", "Iterable", "List", "Map", "Set", "Object", "String", "Null", "Never", "Record"];

pub fn widget_return(words: &[String]) -> bool {
    static TYPE: OnceLock<Regex> = OnceLock::new();
    static BUILDER: OnceLock<Regex> = OnceLock::new();
    let ty = words.iter().filter(|w| !matches!(w.as_str(), "static" | "external")).cloned().collect::<Vec<_>>().join(" ");
    if ty.is_empty() {
        return true;
    }
    let Some(m) = re(&TYPE, r"^(?:[A-Za-z_][A-Za-z0-9_]*\.)?([A-Z][A-Za-z0-9_]*)$").captures(&ty) else { return false };
    let name = &m[1];
    !NOT_WIDGET.contains(&name) && name != "Type" && !re(&BUILDER, r".Builder$").is_match(name)
}

fn top_decl(masked: &str, from: usize) -> Option<Decl> {
    let h = header(masked, from)?;
    if h.name.contains('.') || h.words.iter().any(|w| matches!(w.as_str(), "class" | "get" | "set" | "operator")) {
        return None;
    }
    let symbol = h.name.as_str();
    if is_private(symbol) {
        return Some(skip(symbol, "private"));
    }
    if !widget_return(&h.words) {
        return Some(skip(symbol, "type"));
    }
    if has_required_params(&h.params) {
        return Some(skip(symbol, "params"));
    }
    Some(Decl { symbol: symbol.to_string(), call: format!("{symbol}()"), skipped: None })
}

fn member_decl(masked: &str, from: usize, class_name: &str) -> Option<Decl> {
    let h = header(masked, from)?;
    if h.words.iter().any(|w| matches!(w.as_str(), "get" | "set" | "operator")) {
        return None;
    }
    let ctor = h.name == class_name || h.name.starts_with(&format!("{class_name}."));
    let symbol = if ctor { h.name.clone() } else { format!("{class_name}.{}", h.name) };
    if !ctor && (h.name.contains('.') || !h.words.iter().any(|w| w == "static")) {
        return Some(skip(&symbol, "unknown"));
    }
    if is_private(class_name) || is_private(&h.name) {
        return Some(skip(&symbol, "private"));
    }
    if !ctor && !widget_return(&h.words) {
        return Some(skip(&symbol, "type"));
    }
    if has_required_params(&h.params) {
        return Some(skip(&symbol, "params"));
    }
    Some(Decl { call: format!("{symbol}()"), symbol, skipped: None })
}

fn depth_in(masked: &str, open: usize, at: usize) -> i32 {
    masked.as_bytes()[open..at.min(masked.len())].iter().fold(0, |d, c| d + i32::from(*c == b'{') - i32::from(*c == b'}'))
}

pub fn scan_dart_source(src: &str) -> Vec<PreviewHit> {
    scan_dart_masked(src, &mask_dart(src))
}

pub fn scan_dart_masked(src: &str, masked: &str) -> Vec<PreviewHit> {
    static ANNOTATION: OnceLock<Regex> = OnceLock::new();
    let annotation = re(&ANNOTATION, r"@(?:[A-Za-z_][A-Za-z0-9_]*\.)?Preview\s*\(");
    let scopes = top_scopes(masked);
    let part = part_of(masked);
    let mut hits: Vec<PreviewHit> = Vec::new();
    let mut pos = 0usize;
    while let Some(m) = annotation.find_at(masked, pos) {
        pos = m.end();
        let at = m.start();
        let args_open = m.end() - 1;
        let Some(args_end) = match_close(masked, args_open) else { continue };
        pos = args_end;
        let line = line_at(src, at);
        let label = preview_name(src, masked, args_open + 1, args_end - 1);
        let scope = scopes.iter().find(|s| at > s.open && at < s.close);
        let hit = match scope {
            Some(s) => match &s.class_name {
                Some(c) if depth_in(masked, s.open, at) == 1 => member_decl(masked, args_end, c),
                _ => None,
            },
            None => top_decl(masked, args_end),
        };
        let found = hit.unwrap_or_else(|| skip(&format!("?{line}"), "unknown"));
        let decl = if part { skip(&found.symbol, "part") } else { found };
        if hits.iter().any(|h| h.symbol == decl.symbol) {
            continue;
        }
        let label = if label.trim().is_empty() { decl.symbol.clone() } else { label.trim().to_string() };
        hits.push(PreviewHit { line, symbol: decl.symbol, label, call: decl.call, kind: None, skipped: decl.skipped });
    }
    hits
}
