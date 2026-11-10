use crate::codex::value::J;
use crate::jsonline::sanitize;
use crate::jsstr::{js_slice, js_space, js_trim, units, word};
use crate::mask::mask_secrets;
use serde_json::Value;
use std::sync::OnceLock;

pub const TOOL_SUMMARY_MAX: usize = 200;
const RAW_MAX: usize = 4000;
const FLAG_WORDS: [&str; 4] = ["password", "passwd", "secret", "token"];

pub fn host_home() -> &'static str {
    static HOME: OnceLock<String> = OnceLock::new();
    HOME.get_or_init(|| std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default())
}

pub fn host_platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut gap = false;
    for c in s.chars() {
        if js_space(c) {
            gap = true;
            continue;
        }
        if gap && !out.is_empty() {
            out.push(' ');
        }
        gap = false;
        out.push(c);
    }
    out
}

fn ci_at(t: &[char], i: usize, w: &str) -> bool {
    i + w.len() <= t.len() && t[i..i + w.len()].iter().zip(w.chars()).all(|(a, b)| a.is_ascii() && a.eq_ignore_ascii_case(&b))
}

fn mask_url_creds(text: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        if t[i] == ':' && t.get(i + 1) == Some(&'/') && t.get(i + 2) == Some(&'/') {
            let mut j = i + 3;
            while j < t.len() && !js_space(t[j]) && !matches!(t[j], '/' | '@' | '?' | '#') {
                j += 1;
            }
            if j > i + 3 && t.get(j) == Some(&'@') {
                out.push_str("://[masked]@");
                i = j + 1;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

fn value_after(t: &[char], from: usize, min_units: usize, no_dash: bool) -> Option<(usize, usize)> {
    let mut j = from;
    while j < t.len() && js_space(t[j]) {
        j += 1;
    }
    if j == from || (no_dash && t.get(j) == Some(&'-')) {
        return None;
    }
    let start = j;
    let mut n = 0;
    while j < t.len() && !js_space(t[j]) {
        n += t[j].len_utf16();
        j += 1;
    }
    (n >= min_units.max(1)).then_some((start, j))
}

fn mask_bearer(text: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        if (i == 0 || !word(t[i - 1])) && ci_at(&t, i, "bearer") {
            if let Some((start, end)) = value_after(&t, i + 6, 4, false) {
                out.extend(&t[i..start]);
                out.push_str("[masked]");
                i = end;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

fn flag_word_len(t: &[char], i: usize) -> Option<usize> {
    if let Some(w) = FLAG_WORDS.iter().find(|w| ci_at(t, i, w)) {
        return Some(w.len());
    }
    if !ci_at(t, i, "api") {
        return None;
    }
    let j = if t.get(i + 3).is_some_and(|c| *c == '_' || *c == '-') && ci_at(t, i + 4, "key") { i + 4 } else { i + 3 };
    ci_at(t, j, "key").then_some(j + 3 - i)
}

fn mask_flags(text: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        if t[i] == '-' {
            if let Some((start, end)) = flag_word_len(&t, i + 1).and_then(|k| value_after(&t, i + 1 + k, 1, true)) {
                out.extend(&t[i..start]);
                out.push_str("[masked]");
                i = end;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

pub fn sanitize_tool_summary(raw: &str, home: &str, platform: &str) -> Option<String> {
    let out = collapse(js_slice(raw, RAW_MAX));
    if out.is_empty() {
        return None;
    }
    let out = mask_flags(&mask_bearer(&mask_url_creds(&mask_secrets(&out, home, platform))));
    let out = js_trim(&out);
    if out.is_empty() {
        return None;
    }
    Some(if units(out) > TOOL_SUMMARY_MAX { format!("{}…", js_slice(out, TOOL_SUMMARY_MAX)) } else { out.to_owned() })
}

fn text(v: Option<&Value>) -> Option<&str> {
    v?.as_str().filter(|s| !s.is_empty())
}

pub fn raw_tool_summary(name: &str, input: &Value) -> Option<String> {
    let i = input.as_object()?;
    let got = match name {
        "Bash" | "PowerShell" => text(i.get("command"))?.to_owned(),
        "Read" | "Edit" | "Write" | "NotebookEdit" => text(i.get("file_path")).or_else(|| text(i.get("notebook_path")))?.to_owned(),
        "Grep" => {
            let pattern = text(i.get("pattern"))?;
            match text(i.get("path")) {
                Some(path) => format!("{pattern} {path}"),
                None => pattern.to_owned(),
            }
        }
        "Glob" => text(i.get("pattern"))?.to_owned(),
        "Task" | "Agent" => text(i.get("description"))?.to_owned(),
        "WebFetch" => text(i.get("url"))?.split(['?', '#']).next().filter(|s| !s.is_empty())?.to_owned(),
        "WebSearch" => text(i.get("query"))?.to_owned(),
        _ => return None,
    };
    Some(got)
}

pub fn tool_summary_of(name: &str, input: &Value, home: &str, platform: &str) -> Option<String> {
    sanitize_tool_summary(&raw_tool_summary(name, input)?, home, platform)
}

fn patch_files(patch: &str) -> Option<String> {
    let mut files: Vec<&str> = Vec::new();
    for line in patch.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some(rest) = line.strip_prefix("*** ") else { continue };
        let Some(file) = ["Add File: ", "Update File: ", "Delete File: "].iter().find_map(|p| rest.strip_prefix(p)) else { continue };
        if file.is_empty() || file.contains(['\r', '\u{2028}', '\u{2029}']) {
            continue;
        }
        let file = js_trim(file);
        if !file.is_empty() {
            files.push(file);
        }
    }
    (!files.is_empty()).then(|| files.join(", "))
}

fn command_of(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => (!s.is_empty()).then(|| s.clone()),
        Value::Array(a) if !a.is_empty() => {
            let parts: Vec<&str> = a.iter().map(Value::as_str).collect::<Option<_>>()?;
            if parts.len() == 3 && (parts[1] == "-lc" || parts[1] == "-c") {
                return (!parts[2].is_empty()).then(|| parts[2].to_owned());
            }
            let joined = parts.join(" ");
            (!joined.is_empty()).then_some(joined)
        }
        _ => None,
    }
}

fn parse_args(args: &str) -> Option<Value> {
    serde_json::from_str::<Value>(args).ok().or_else(|| sanitize(args).and_then(|s| serde_json::from_str(&s).ok())).filter(Value::is_object)
}

pub fn raw_codex_tool_summary(item_type: &str, name: &str, namespaced: bool, payload: &J) -> Option<String> {
    if namespaced {
        return None;
    }
    if item_type == "custom_tool_call" {
        return if name == "apply_patch" { payload.get("input").and_then(J::as_str).and_then(patch_files) } else { None };
    }
    if item_type != "function_call" {
        return None;
    }
    let args = parse_args(payload.get("arguments")?.as_str()?)?;
    if name == "apply_patch" {
        return args.get("input").and_then(Value::as_str).and_then(patch_files);
    }
    command_of(args.get("cmd")).or_else(|| command_of(args.get("command")))
}

pub fn codex_tool_summary_of(item_type: &str, name: &str, namespaced: bool, payload: &J, home: &str, platform: &str) -> Option<String> {
    sanitize_tool_summary(&raw_codex_tool_summary(item_type, name, namespaced, payload)?, home, platform)
}
