use super::branches::push_range;
use super::exec::{exec_git, ExecOpts};
use crate::claude::mask::named_secrets;
use core_transcript::mask::find_tokens;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

const MAX_DIFF: usize = 16 * 1024 * 1024;
const MAX_FILE_ADDED: usize = 1024 * 1024;
const MAX_LINE: usize = 4096;
const MAX_HITS: usize = 200;
const MAX_COMMITS: &str = "200";
const MAX_ALLOW: u64 = 64 * 1024;
const SCAN_BUDGET: Duration = Duration::from_secs(20);
pub const ALLOW_FILE: &str = ".orrery-secret-allow";
const INLINE_ALLOW: &str = "orrery-secret-allow";
const MASK: &str = "••••";
const PLACEHOLDERS: [&str; 14] = ["example", "xxx", "changeme", "change_me", "placeholder", "your_", "your-", "yourkey", "dummy", "redacted", "sample", "****", "...", "fake"];
const CONFIG_EXT: [&str; 10] = [".properties", ".ini", ".cfg", ".conf", ".toml", ".yaml", ".yml", ".json", ".xml", ".plist"];
const CONFIG_NAMES: [&str; 4] = [".npmrc", ".pypirc", ".netrc", ".git-credentials"];
const TEMPLATE_SUFFIX: [&str; 5] = [".example", ".sample", ".template", ".dist", ".defaults"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    pub masked: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: u32,
    pub kind: &'static str,
    pub masked: String,
    pub commit: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub hits: Vec<Hit>,
    pub skipped: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Default)]
pub struct Allow {
    texts: Vec<String>,
    paths: Vec<String>,
}

impl Allow {
    pub fn parse(text: &str) -> Self {
        let mut a = Allow::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match line.strip_prefix("path:") {
                Some(p) => a.paths.push(p.trim().replace('\\', "/")),
                None => a.texts.push(line.to_owned()),
            }
        }
        a
    }

    pub fn load(root: &str) -> Self {
        let file = Path::new(root).join(ALLOW_FILE);
        match std::fs::metadata(&file) {
            Ok(m) if m.is_file() && m.len() <= MAX_ALLOW => Allow::parse(&std::fs::read_to_string(&file).unwrap_or_default()),
            _ => Allow::default(),
        }
    }

    fn path_allowed(&self, path: &str) -> bool {
        self.paths.iter().any(|p| match p.strip_prefix('*') {
            Some(suffix) => path.ends_with(suffix),
            None if p.ends_with('/') => path.starts_with(p.as_str()),
            None => path == p,
        })
    }

    fn line_allowed(&self, line: &str) -> bool {
        line.contains(INLINE_ALLOW) || self.texts.iter().any(|t| line.contains(t.as_str()))
    }
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn is_template(path: &str) -> bool {
    let b = base_name(path).to_ascii_lowercase();
    TEMPLATE_SUFFIX.iter().any(|s| b.ends_with(s))
}

fn is_env(path: &str) -> bool {
    let b = base_name(path).to_ascii_lowercase();
    b == ".env" || b.starts_with(".env.") || b.ends_with(".env")
}

fn is_config(path: &str) -> bool {
    let b = base_name(path).to_ascii_lowercase();
    CONFIG_NAMES.contains(&b.as_str()) || CONFIG_EXT.iter().any(|e| b.ends_with(e))
}

fn is_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if PLACEHOLDERS.iter().any(|p| lower.contains(p)) || lower.starts_with('<') {
        return true;
    }
    let mut seen: Vec<char> = value.chars().collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len() < 5
}

fn unquote(v: &str) -> (&str, bool) {
    let quoted = v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')));
    if quoted {
        (&v[1..v.len() - 1], true)
    } else {
        (v, false)
    }
}

fn is_reference(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    v.starts_with(['$', '%', '{', '[', '/', '.', '@', '&', '*'])
        || v.contains('(')
        || lower.starts_with("http")
        || lower.contains("process.env")
        || lower.contains("os.environ")
        || lower.contains("getenv")
        || matches!(lower.as_str(), "true" | "false" | "null" | "none" | "nil" | "undefined")
}

fn strong(v: &str) -> bool {
    let digit = v.chars().any(|c| c.is_ascii_digit());
    let upper = v.chars().any(|c| c.is_ascii_uppercase());
    let lower = v.chars().any(|c| c.is_ascii_lowercase());
    digit && (upper || lower) && ((upper && lower) || v.chars().count() >= 20)
}

fn named_kind(path: &str, value: &str) -> Option<&'static str> {
    let (v, quoted) = unquote(value.trim());
    let n = v.chars().count();
    if v.is_empty() || v.chars().any(char::is_whitespace) || is_reference(v) || is_placeholder(v) {
        return None;
    }
    if is_env(path) {
        return (n >= 8 && !v.chars().all(|c| c.is_ascii_digit())).then_some("env");
    }
    if is_config(path) || quoted {
        return (n >= 12 && strong(v)).then_some("assignment");
    }
    None
}

fn cloudflare(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("cfut_") {
        let before_ok = rest[..at].chars().last().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let tail = &rest[at + 5..];
        let body: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if before_ok && body.len() >= 16 && !is_placeholder(&body) {
            out.push(Finding { kind: "cloudflare", masked: format!("cfut_{MASK}") });
        }
        rest = &tail[body.len()..];
    }
    out
}

pub fn scan_line(path: &str, line: &str) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
        out.push(Finding { kind: "private-key", masked: format!("-----BEGIN PRIVATE KEY----- {MASK}") });
    }
    for t in find_tokens(line) {
        let body = &t.value[t.prefix.len()..];
        if !is_placeholder(body) {
            out.push(Finding { kind: t.kind, masked: format!("{}{MASK}", t.prefix) });
        }
    }
    out.extend(cloudflare(line));
    if out.is_empty() {
        for (name, value) in named_secrets(line) {
            if let Some(kind) = named_kind(path, &value) {
                out.push(Finding { kind, masked: format!("{name}={MASK}") });
            }
        }
    }
    out
}

fn diff_path(header: &str) -> Option<String> {
    let p = header.trim_end_matches('\r');
    if p == "/dev/null" {
        return None;
    }
    let p = p.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(p);
    Some(p.strip_prefix("b/").unwrap_or(p).to_owned())
}

fn hunk_start(line: &str) -> u32 {
    line.split(' ').find_map(|part| part.strip_prefix('+')).and_then(|s| s.split(',').next()).and_then(|s| s.parse().ok()).unwrap_or(1)
}

pub fn scan_diff(diff: &str, allow: &Allow, deadline: Instant) -> Result<Scan, &'static str> {
    let mut scan = Scan::default();
    let mut commit: Option<String> = None;
    let mut path: Option<String> = None;
    let mut skip = true;
    let mut in_hunk = false;
    let mut added = 0usize;
    let mut line_no = 0u32;
    for (n, raw) in diff.split('\n').enumerate() {
        if n % 512 == 0 && Instant::now() > deadline {
            return Err("timeout");
        }
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(c) = line.strip_prefix('\u{1}') {
            commit = Some(c.trim().to_owned());
            in_hunk = false;
            skip = true;
            continue;
        }
        if line.starts_with("diff --git ") {
            in_hunk = false;
            skip = true;
            path = None;
            added = 0;
            continue;
        }
        if in_hunk {
            if let Some(text) = line.strip_prefix('+') {
                let at = line_no;
                line_no += 1;
                if skip {
                    continue;
                }
                added += text.len();
                let p = path.clone().unwrap_or_default();
                if added > MAX_FILE_ADDED {
                    skip = true;
                    scan.skipped.push(p);
                    continue;
                }
                if text.len() > MAX_LINE || allow.line_allowed(text) {
                    continue;
                }
                for f in scan_line(&p, text) {
                    let dup = scan.hits.iter().any(|h| h.path == p && h.line == at && h.kind == f.kind && h.commit == commit);
                    if dup {
                        continue;
                    }
                    if scan.hits.len() >= MAX_HITS {
                        scan.truncated = true;
                        return Ok(scan);
                    }
                    scan.hits.push(Hit { path: p.clone(), line: at, kind: f.kind, masked: f.masked, commit: commit.clone() });
                }
                continue;
            }
            if line.starts_with('-') || line.starts_with('\\') || line.starts_with(' ') {
                continue;
            }
            if !line.starts_with("@@") {
                in_hunk = false;
            }
        }
        if line.starts_with("@@") {
            in_hunk = path.is_some();
            line_no = hunk_start(line);
            continue;
        }
        if let Some(h) = line.strip_prefix("+++ ") {
            path = diff_path(h);
            skip = match &path {
                Some(p) => is_template(p) || allow.path_allowed(p) || base_name(p) == ALLOW_FILE,
                None => true,
            };
        }
    }
    Ok(scan)
}

fn run_scan(root: &str, args: &[&str]) -> Value {
    let started = Instant::now();
    let res = exec_git(Path::new(root), args, &ExecOpts::timeout(SCAN_BUDGET.as_millis() as u64).with_max(MAX_DIFF));
    if res.timed_out {
        return json!({"ok": false, "error": "timeout"});
    }
    if !res.ok() && !res.truncated {
        return json!({"ok": false, "error": "failed"});
    }
    match scan_diff(&res.stdout, &Allow::load(root), started + SCAN_BUDGET) {
        Ok(s) => report(&s, res.truncated),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

const DIFF_FLAGS: [&str; 6] = ["-U0", "--no-color", "--no-ext-diff", "--no-textconv", "--src-prefix=a/", "--dst-prefix=b/"];

pub fn scan_staged(root: &str) -> Value {
    let mut args = vec!["diff", "--cached", "--diff-filter=ACMR"];
    args.extend(DIFF_FLAGS);
    run_scan(root, &args)
}

pub fn scan_push(root: &str, r: &Value, remote: &Value, branch: &Value) -> Value {
    let Some((from, not)) = push_range(root, r, remote, branch) else { return json!({"ok": false, "error": "failed"}) };
    let mut args = vec!["log", "-p", "--no-merges", "--format=%x01%h", "-n", MAX_COMMITS];
    args.extend(DIFF_FLAGS);
    args.extend([from.as_str(), "--not", not.as_str(), "--"]);
    run_scan(root, &args)
}

pub fn report(s: &Scan, truncated: bool) -> Value {
    let hits: Vec<Value> = s
        .hits
        .iter()
        .map(|h| {
            let mut v = json!({"path": h.path, "line": h.line, "kind": h.kind, "masked": h.masked});
            if let Some(c) = &h.commit {
                v["commit"] = json!(c);
            }
            v
        })
        .collect();
    json!({"ok": true, "hits": hits, "skipped": s.skipped, "truncated": truncated || s.truncated})
}

pub fn blocked(scan: &Value) -> Option<Value> {
    if scan["ok"] != json!(true) {
        return Some(json!({"ok": false, "error": "secrets", "hits": [], "scanError": scan["error"].clone()}));
    }
    let hits = scan["hits"].as_array().filter(|h| !h.is_empty())?;
    Some(json!({"ok": false, "error": "secrets", "hits": hits, "skipped": scan["skipped"].clone(), "truncated": scan["truncated"].clone()}))
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod tests;
