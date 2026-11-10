use super::ctx::{js_lines, JS_S};
use super::scripts::detect_scripts;
use super::spawn::{self, End, Opts, Spec};
use crate::claude::tail;
use crate::js;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const MAX_TAIL: usize = 64 * 1024;
const MAX_LOG: usize = 1024 * 1024;
const MAX_ERRORS: usize = 500;
const MAX_RUNNING: usize = 2;
const TIMEOUT_MS: u64 = 15 * 60_000;
const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

pub struct CheckRequest {
    pub source: String,
    pub name: String,
}

pub fn clean_check(input: &Value) -> Option<CheckRequest> {
    let o = input.as_object()?;
    let source = o.get("source")?.as_str()?;
    let name = o.get("name")?.as_str()?;
    if name.is_empty() || js::len(name) > 200 {
        return None;
    }
    Some(CheckRequest { source: source.to_string(), name: name.to_string() })
}

pub fn resolve_check(dir: &str, req: &CheckRequest) -> Option<String> {
    if req.source == "flutter" {
        if req.name != "analyze" {
            return None;
        }
        return std::fs::metadata(Path::new(dir).join("pubspec.yaml")).map(|m| m.is_file()).unwrap_or(false).then(|| "flutter analyze --no-pub".to_string());
    }
    let found = detect_scripts(dir);
    let group = found["groups"].as_array()?.iter().find(|g| g["source"] == json!(req.source))?;
    let item = group["items"].as_array()?.iter().find(|i| i["name"] == json!(req.name))?;
    item["run"].as_str().map(String::from)
}

fn rx(cell: &'static OnceLock<Regex>, pattern: String) -> &'static Regex {
    cell.get_or_init(|| Regex::new(&pattern).expect("regex"))
}

macro_rules! lazy {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            rx(&R, $pat)
        }
    };
}

lazy!(ansi_re, r"\x1b\[[0-9;?]*[ -/]*[@-~]".to_string());
lazy!(tsc_paren, format!(r"^({DOT}+?)\(([0-9]+),([0-9]+)\): (error|warning) (TS[0-9]+: {DOT}*)$"));
lazy!(tsc_pretty, format!(r"^({DOT}+?):([0-9]+):([0-9]+) - (error|warning) (TS[0-9]+: {DOT}*)$"));
lazy!(dart_re, format!(r"^[{JS_S}]*(error|warning|info)[{JS_S}]+[•-][{JS_S}]+({DOT}+?)[{JS_S}]+[•-][{JS_S}]+({DOT}+?):([0-9]+):([0-9]+)[{JS_S}]+[•-][{JS_S}]+([^{JS_S}]+)[{JS_S}]*$"));
lazy!(kotlin_re, format!(r"^([ew]): (file://[^{JS_S}]+?|[A-Za-z]:[\\/][^{JS_S}]+?|/[^{JS_S}]+?):([0-9]+):([0-9]+) ({DOT}*)$"));
lazy!(vitest_re, format!(r"^[{JS_S}]*❯[{JS_S}]+([^{JS_S}]+?):([0-9]+):([0-9]+)[{JS_S}]*$"));
lazy!(err_re, format!(r"^[{JS_S}]*(?:AssertionError|Error|TypeError|ReferenceError)(?:[{JS_S}]*\[[^\]]*\])?:[{JS_S}]*({DOT}+)$"));
lazy!(path_line_re, format!(r"^(?:[A-Za-z]:[\\/]|/)[^{JS_S}]{DOT}*\.[A-Za-z0-9_]+$"));
lazy!(eslint_re, format!(r"^[{JS_S}]+([0-9]+):([0-9]+)[{JS_S}]+(error|warning)[{JS_S}]+({DOT}+?)(?:[{JS_S}]{{2,}}[^{JS_S}]+)?$"));
lazy!(file_scheme_re, r"^file:///?".to_string());
lazy!(drive_re, r"^[A-Za-z]:[\\/]".to_string());

fn abs_path(f: &str, dir: &str, win: bool) -> String {
    let clean = file_scheme_re().replace(f, "").to_string();
    let clean = core_transcript::jsstr::js_trim(&clean).to_string();
    let local = if drive_re().is_match(&clean) || clean.starts_with('/') { clean } else { Path::new(dir).join(&clean).to_string_lossy().to_string() };
    let local = if win && local.starts_with('/') && local.as_bytes().get(2) == Some(&b':') { local[1..].to_string() } else { local };
    crate::util::path_str(&crate::util::resolve(&local))
}

pub fn parse_check_output(text: &str, dir: &str, win: bool) -> (Vec<Value>, bool) {
    let mut errors: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut eslint_file: Option<String> = None;
    let mut pending: Option<String> = None;
    let mut push = |file: &str, line: u64, col: u64, message: &str, severity: &str| {
        if truncated {
            return;
        }
        let key = format!("{file}:{line}:{col}:{message}");
        if seen.contains(&key) {
            return;
        }
        seen.push(key);
        if errors.len() >= MAX_ERRORS {
            truncated = true;
            return;
        }
        let msg = js::slice(core_transcript::jsstr::js_trim(message), 500);
        errors.push(json!({"file": abs_path(file, dir, win), "line": line, "col": col, "message": msg, "severity": severity}));
    };
    let num = |s: &str| s.parse::<u64>().unwrap_or(0);
    for raw in js_lines(text) {
        let line = ansi_re().replace_all(raw, "").to_string();
        if let Some(m) = tsc_paren().captures(&line).or_else(|| tsc_pretty().captures(&line)) {
            push(&m[1], num(&m[2]), num(&m[3]), &m[5], &m[4]);
            continue;
        }
        if let Some(m) = dart_re().captures(&line) {
            if &m[1] != "info" {
                push(&m[3], num(&m[4]), num(&m[5]), &format!("{} ({})", &m[2], &m[6]), &m[1]);
            }
            continue;
        }
        if let Some(m) = kotlin_re().captures(&line) {
            push(&m[2], num(&m[3]), num(&m[4]), &m[5], if &m[1] == "e" { "error" } else { "warning" });
            continue;
        }
        if let Some(m) = vitest_re().captures(&line) {
            push(&m[1], num(&m[2]), num(&m[3]), pending.as_deref().unwrap_or("Test failed"), "error");
            pending = None;
            continue;
        }
        if let Some(m) = err_re().captures(&line) {
            pending = Some(m[1].to_string());
            continue;
        }
        let trimmed = core_transcript::jsstr::js_trim(&line);
        if path_line_re().is_match(trimmed) && !line.starts_with(' ') {
            eslint_file = Some(trimmed.to_string());
            continue;
        }
        if let (Some(m), Some(f)) = (eslint_re().captures(&line), eslint_file.as_deref()) {
            push(f, num(&m[1]), num(&m[2]), &m[4], &m[3]);
            continue;
        }
        if trimmed.is_empty() {
            eslint_file = None;
        }
    }
    (errors, truncated)
}

#[derive(Default)]
pub struct CheckRunner {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

fn fail(error: &str) -> Value {
    json!({"ok": false, "error": error})
}

impl CheckRunner {
    pub fn cancel(&self, key: &str) -> bool {
        match self.running.lock().unwrap().get(key) {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn run(&self, key: &str, dir: &str, line: &str) -> Value {
        let flag = Arc::new(AtomicBool::new(false));
        {
            let mut m = self.running.lock().unwrap();
            if m.contains_key(key) || m.len() >= MAX_RUNNING {
                return fail("busy");
            }
            m.insert(key.to_string(), flag.clone());
        }
        let win = cfg!(windows);
        let spec = if win {
            let comspec = std::env::var("ComSpec").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "cmd.exe".into());
            Spec { file: comspec, args: vec!["/d".into(), "/s".into(), "/c".into(), format!("\"{line}\"")], cwd: dir.to_string(), verbatim: true }
        } else {
            Spec { file: "/bin/sh".into(), args: vec!["-c".into(), line.to_string()], cwd: dir.to_string(), verbatim: false }
        };
        let started = Instant::now();
        let opts = Opts { input: None, timeout_ms: TIMEOUT_MS, max_stdout: usize::MAX, merged_cap: MAX_LOG, cancel: Some(flag.clone()) };
        let res = spawn::run(&spec, &opts);
        self.running.lock().unwrap().remove(key);
        let out = match res {
            Ok(o) => o,
            Err(e) => return json!({"ok": false, "error": "failed", "detail": js::slice(&format!("Error: {e}"), 300)}),
        };
        if matches!(out.end, End::Cancelled | End::Timeout) {
            return fail("cancelled");
        }
        let code = match out.end {
            End::Exit(c) => c,
            _ => None,
        };
        let log = String::from_utf8_lossy(&out.merged).to_string();
        let log = tail(&log, MAX_LOG).to_string();
        let (errors, truncated) = parse_check_output(&log, dir, win);
        let shown = ansi_re().replace_all(tail(&log, MAX_TAIL), "").to_string();
        let mut o = Map::new();
        o.insert("ok".into(), json!(true));
        o.insert("code".into(), json!(code));
        o.insert("ms".into(), json!(started.elapsed().as_millis() as u64));
        o.insert("errors".into(), Value::Array(errors));
        o.insert("errorsTruncated".into(), json!(truncated));
        o.insert("tail".into(), json!(shown));
        Value::Object(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tool_output() {
        let text = "src/a.ts(3,4): error TS2322: bad\nsrc/b.ts:5:6 - error TS1: pretty\n  error • Unused • lib/x.dart:7:8 • unused_x\ne: file:///k/y.kt:9:1 boom\n/abs/e.js\n  12:3  warning  no-var  rule\n\n ❯ t/z.test.ts:20:5\nsrc/a.ts(3,4): error TS2322: bad\n";
        let (errors, truncated) = parse_check_output(text, "/proj", false);
        assert!(!truncated);
        let got: Vec<(String, u64, String, String)> = errors.iter().map(|e| (e["file"].as_str().unwrap().into(), e["line"].as_u64().unwrap(), e["message"].as_str().unwrap().into(), e["severity"].as_str().unwrap().into())).collect();
        assert_eq!(got[0], ("/proj/src/a.ts".into(), 3, "TS2322: bad".into(), "error".into()));
        assert_eq!(got[2].2, "Unused (unused_x)");
        assert_eq!(got[3].0, "/proj/k/y.kt");
        assert_eq!(got[4], ("/abs/e.js".into(), 12, "no-var".into(), "warning".into()));
        assert_eq!(got[5].2, "Test failed");
        assert_eq!(got.len(), 6);
    }

    #[cfg(unix)]
    #[test]
    fn runs_and_cancels() {
        let dir = std::env::temp_dir().to_string_lossy().to_string();
        let r = CheckRunner::default();
        let out = r.run("k", &dir, "echo 'a.ts(1,2): error TS9: x'; exit 2");
        assert_eq!((out["ok"].clone(), out["code"].clone(), out["errors"].as_array().map(Vec::len)), (json!(true), json!(2), Some(1)));
        assert_eq!(out["tail"], "a.ts(1,2): error TS9: x\n");
        let r = Arc::new(CheckRunner::default());
        let r2 = r.clone();
        let h = std::thread::spawn(move || r2.run("k", &std::env::temp_dir().to_string_lossy(), "sleep 5"));
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert_eq!(r.run("k", &dir, "true"), fail("busy"));
        assert!(r.cancel("k") && !r.cancel("other"));
        assert_eq!(h.join().unwrap(), fail("cancelled"));
    }
}
