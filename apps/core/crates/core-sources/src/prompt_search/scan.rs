use std::fs::File;
use std::io::Read;
use std::path::Path;

use core_transcript::jsonline::sanitize;
use memchr::memmem;
use serde_json::Value;

use super::nfc::fold_text;
use super::text::{matches_all, prompt_text_of, snippet_of, PROMPT_SEARCH_SNIPPETS, PROMPT_SEARCH_SNIPPET_MAX};

pub const MAX_LINE_BYTES: usize = 2 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionScan {
    pub cwd: Option<String>,
    pub started_at: Option<String>,
    pub git_branch: Option<String>,
    pub custom_title: Option<String>,
    pub ai_title: Option<String>,
    pub matches: u64,
    pub snippets: Vec<String>,
}

pub struct ScanOptions<'a> {
    pub terms: &'a [String],
    pub mask: &'a dyn Fn(&str) -> String,
    pub cancelled: &'a dyn Fn() -> bool,
}

fn has(line: &[u8], needle: &[u8]) -> bool {
    memmem::find(line, needle).is_some()
}

fn parse_plain(text: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(text) {
        Ok(v) => Some(v),
        Err(_) => serde_json::from_str::<Value>(&sanitize(text)?).ok(),
    }
}

fn on_line(line: &[u8], opts: &ScanOptions, out: &mut SessionScan) {
    let is_title = has(line, b"\"type\":\"custom-title\"") || has(line, b"\"type\":\"ai-title\"");
    if !is_title && (!has(line, b"\"type\":\"user\"") || has(line, b"\"tool_result\"") || has(line, b"\"isMeta\":true")) {
        return;
    }
    let text = String::from_utf8_lossy(line);
    let Some(rec) = parse_plain(&text) else { return };
    let Some(obj) = rec.as_object() else { return };
    let ty = obj.get("type").and_then(Value::as_str);
    if ty == Some("custom-title") {
        if let Some(t) = obj.get("customTitle").and_then(Value::as_str) {
            out.custom_title = Some(t.to_owned());
        }
    }
    if ty == Some("ai-title") {
        if let Some(t) = obj.get("aiTitle").and_then(Value::as_str) {
            out.ai_title = Some(t.to_owned());
        }
    }
    if ty != Some("user") {
        return;
    }
    if out.started_at.is_none() {
        if let Some(t) = obj.get("timestamp").and_then(Value::as_str) {
            out.started_at = Some(t.to_owned());
        }
    }
    if out.cwd.is_none() {
        if let Some(c) = obj.get("cwd").and_then(Value::as_str).filter(|c| !c.is_empty()) {
            out.cwd = Some(c.to_owned());
        }
    }
    if let Some(b) = obj.get("gitBranch").and_then(Value::as_str).filter(|b| !b.is_empty() && *b != "HEAD") {
        out.git_branch = Some(b.to_owned());
    }
    let Some(prompt) = prompt_text_of(&rec) else { return };
    if !matches_all(&fold_text(&prompt), opts.terms) {
        return;
    }
    out.matches += 1;
    if out.snippets.len() < PROMPT_SEARCH_SNIPPETS {
        out.snippets.push(snippet_of(&(opts.mask)(&prompt), opts.terms, PROMPT_SEARCH_SNIPPET_MAX));
    }
}

pub fn scan_transcript(file: &Path, opts: &ScanOptions) -> std::io::Result<SessionScan> {
    let mut out = SessionScan::default();
    let mut f = File::open(file)?;
    let mut remainder: Vec<u8> = Vec::new();
    let mut skipping = false;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if (opts.cancelled)() {
            break;
        }
        let mut data = std::mem::take(&mut remainder);
        data.extend_from_slice(&buf[..n]);
        let mut start = 0;
        if skipping {
            match memchr::memchr(b'\n', &data) {
                None => continue,
                Some(nl) => {
                    skipping = false;
                    start = nl + 1;
                }
            }
        }
        while let Some(rel) = memchr::memchr(b'\n', &data[start..]) {
            let idx = start + rel;
            if idx - start <= MAX_LINE_BYTES {
                on_line(&data[start..idx], opts, &mut out);
            }
            start = idx + 1;
        }
        let tail = &data[start..];
        if tail.len() > MAX_LINE_BYTES {
            skipping = true;
        } else {
            remainder = tail.to_vec();
        }
    }
    if !skipping && !remainder.is_empty() && !(opts.cancelled)() {
        on_line(&remainder, opts, &mut out);
    }
    Ok(out)
}
