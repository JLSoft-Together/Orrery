use super::spawn::{self, End, Opts, Spec};
use crate::claude::tail;
use crate::js;
use core_transcript::jsstr::js_trim;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

pub const FORMAT_TIMEOUT_MS: u64 = 20_000;
pub const FORMAT_MAX_OUT: usize = 8 * 1024 * 1024;
const TAIL: usize = 4 * 1024;
const MAX_RUNNING: usize = 2;
const PRETTIER_EXT: [&str; 21] = [
    "js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx", "vue", "json", "jsonc", "json5", "css", "scss", "less", "md", "markdown", "html", "htm", "yaml", "yml",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Prettier,
    Dart,
    Ktlint,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Prettier => "prettier",
            Kind::Dart => "dart",
            Kind::Ktlint => "ktlint",
        }
    }

    fn local(self) -> &'static [&'static str] {
        match self {
            Kind::Prettier => &["node_modules/.bin/prettier"],
            Kind::Dart => &[".fvm/flutter_sdk/bin/dart"],
            Kind::Ktlint => &["ktlint"],
        }
    }
}

pub fn formatter_for(file: &str) -> Option<Kind> {
    let ext = file.rsplit_once('.').filter(|(stem, _)| !stem.is_empty() && !stem.ends_with(['/', '\\'])).map(|(_, e)| e.to_lowercase())?;
    if PRETTIER_EXT.contains(&ext.as_str()) {
        Some(Kind::Prettier)
    } else if ext == "dart" {
        Some(Kind::Dart)
    } else if ext == "kt" || ext == "kts" {
        Some(Kind::Ktlint)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub file: String,
    pub args: Vec<String>,
    pub verbatim: bool,
}

fn meta_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"([()\]\[%!^"`<>&|;, *?])"#).expect("regex"))
}

fn caret(s: &str) -> String {
    meta_re().replace_all(s, "^$1").to_string()
}

fn checked(s: &str) -> Option<&str> {
    (!s.contains(['\r', '\n', '\0'])).then_some(s)
}

pub fn quote_cmd_arg(arg: &str) -> Option<String> {
    let a = checked(arg)?;
    let mut out = String::new();
    let mut slashes = 0usize;
    for c in a.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2));
                out.push_str("\\\"");
                slashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    Some(caret(&caret(&format!("\"{out}\""))))
}

pub fn cmd_shim(file: &str, args: &[String], env: &BTreeMap<String, String>) -> Option<Resolved> {
    let root = env.get("SystemRoot").or_else(|| env.get("SYSTEMROOT")).cloned().unwrap_or_else(|| "C:\\Windows".into());
    let cmd = format!("{}\\System32\\cmd.exe", root.trim_end_matches(['\\', '/']));
    let mut parts = vec![caret(checked(file)?)];
    for a in args {
        parts.push(quote_cmd_arg(a)?);
    }
    Some(Resolved { file: cmd, args: vec!["/d".into(), "/s".into(), "/c".into(), format!("\"{}\"", parts.join(" "))], verbatim: true })
}

pub fn with_args(cmd: &Resolved, args: &[String]) -> Option<Resolved> {
    if !cmd.verbatim {
        let mut out = cmd.clone();
        out.args.extend(args.iter().cloned());
        return Some(out);
    }
    let line = cmd.args.get(3).map(String::as_str).unwrap_or("\"\"");
    let mut quoted = Vec::new();
    for a in args {
        quoted.push(quote_cmd_arg(a)?);
    }
    let head = &line[..line.len().saturating_sub(1)];
    let mut out = cmd.clone();
    out.args = vec![cmd.args[0].clone(), cmd.args[1].clone(), cmd.args[2].clone(), format!("{head} {}\"", quoted.join(" "))];
    Some(out)
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

pub fn resolve_command(name: &str, cwd: &str, env: &BTreeMap<String, String>, win: bool) -> Option<Resolved> {
    let pathext: Vec<String> = if win { env.get("PATHEXT").map(String::as_str).unwrap_or(".COM;.EXE;.BAT;.CMD").split(';').filter(|e| !e.is_empty()).map(str::to_lowercase).collect() } else { vec![String::new()] };
    let with_ext = |base: &Path| -> Option<PathBuf> {
        if win && base.extension().is_some() && is_file(base) {
            return Some(base.to_path_buf());
        }
        for e in &pathext {
            let mut s = base.as_os_str().to_owned();
            s.push(e);
            let p = PathBuf::from(s);
            if is_file(&p) {
                return Some(p);
            }
        }
        None
    };
    let found = if name.contains('/') || name.contains('\\') {
        with_ext(&crate::util::resolve(&Path::new(cwd).join(name).to_string_lossy()))
    } else {
        let path = env.iter().find(|(k, _)| if win { k.eq_ignore_ascii_case("PATH") } else { k.as_str() == "PATH" }).map(|(_, v)| v.clone()).unwrap_or_default();
        path.split(if win { ';' } else { ':' }).filter(|d| !d.is_empty() && super::paths::is_absolute(d)).find_map(|d| with_ext(&Path::new(d).join(name)))
    }?;
    let s = found.to_string_lossy().to_string();
    if win && (s.to_lowercase().ends_with(".bat") || s.to_lowercase().ends_with(".cmd")) {
        return cmd_shim(&s, &[], env);
    }
    Some(Resolved { file: s, args: Vec::new(), verbatim: false })
}

pub fn dirs_up(file: &str, stop: &str, win: bool) -> Vec<String> {
    let fold = |s: &str| if win { s.to_lowercase() } else { s.to_string() };
    let end = fold(&crate::util::path_str(&crate::util::resolve(stop)));
    let resolved = crate::util::path_str(&crate::util::resolve(file));
    let parent = |p: &str| Path::new(p).parent().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string());
    let first = parent(&resolved);
    let mut out = Vec::new();
    let mut dir = first.clone();
    for _ in 0..64 {
        out.push(dir.clone());
        if fold(&dir) == end {
            return out;
        }
        let up = parent(&dir);
        if up == dir {
            break;
        }
        dir = up;
    }
    vec![first]
}

pub fn resolve_formatter(kind: Kind, file: &str, stop: &str, env: &BTreeMap<String, String>, win: bool) -> Option<(Resolved, bool)> {
    for dir in dirs_up(file, stop, win) {
        for rel in kind.local() {
            if let Some(cmd) = resolve_command(&Path::new(&dir).join(rel).to_string_lossy(), &dir, env, win) {
                return Some((cmd, true));
            }
        }
    }
    let dir = Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    resolve_command(kind.name(), &dir, env, win).map(|c| (c, false))
}

pub fn format_args(kind: Kind, file: &str) -> Vec<String> {
    match kind {
        Kind::Prettier => vec!["--stdin-filepath".into(), file.into()],
        Kind::Dart => vec!["format".into(), "--stdin-name".into(), file.into()],
        Kind::Ktlint => vec!["--format".into(), "--stdin".into()],
    }
}

fn tail_text(s: &[u8]) -> String {
    let text = String::from_utf8_lossy(s);
    js_trim(tail(&text, TAIL)).to_string()
}

fn error(e: &str, kind: Kind, detail: Option<String>) -> Value {
    let mut v = json!({"ok": false, "error": e, "tool": kind.name()});
    if let Some(d) = detail {
        v["detail"] = json!(d);
    }
    v
}

#[derive(Default)]
pub struct Formatter {
    running: AtomicUsize,
}

impl Formatter {
    pub fn format(&self, file: &str, stop: &str, input: &str) -> Value {
        let Some(kind) = formatter_for(file) else { return json!({"ok": false, "error": "unsupported"}) };
        let win = cfg!(windows);
        let env = spawn::run_env_raw();
        let Some((cmd, local)) = resolve_formatter(kind, file, stop, &env, win) else { return error("not-found", kind, None) };
        let Some(cmd) = with_args(&cmd, &format_args(kind, file)) else { return error("failed", kind, None) };
        if self.running.fetch_add(1, Ordering::SeqCst) >= MAX_RUNNING {
            self.running.fetch_sub(1, Ordering::SeqCst);
            return error("busy", kind, None);
        }
        let res = run_one(kind, &cmd, input, file);
        self.running.fetch_sub(1, Ordering::SeqCst);
        match res {
            Err(v) => v,
            Ok(text) => {
                let text = if input.contains('\r') { text } else { text.replace("\r\n", "\n").replace('\r', "\n") };
                json!({"ok": true, "text": text, "tool": kind.name(), "local": local})
            }
        }
    }
}

fn run_one(kind: Kind, cmd: &Resolved, input: &str, file: &str) -> Result<String, Value> {
    let cwd = Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let spec = Spec { file: cmd.file.clone(), args: cmd.args.clone(), cwd, verbatim: cmd.verbatim };
    let opts = Opts { input: Some(input.as_bytes().to_vec()), timeout_ms: FORMAT_TIMEOUT_MS, max_stdout: FORMAT_MAX_OUT, merged_cap: 1024, cancel: None };
    let out = match spawn::run(&spec, &opts) {
        Ok(o) => o,
        Err(e) => return Err(error(if e.kind() == std::io::ErrorKind::NotFound { "not-found" } else { "failed" }, kind, None)),
    };
    match out.end {
        End::Timeout => Err(error("timeout", kind, None)),
        End::TooLarge => Err(error("too-large", kind, None)),
        End::Cancelled => Err(error("failed", kind, None)),
        End::Exit(code) => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if code == Some(0) || (kind == Kind::Ktlint && code == Some(1) && !js_trim(&text).is_empty()) {
                Ok(text)
            } else {
                let err = tail_text(&out.stderr);
                let detail = if err.is_empty() { tail_text(&out.stdout) } else { err };
                Err(error("failed", kind, Some(detail)))
            }
        }
    }
}

pub fn too_large(text: &str, max: usize) -> bool {
    js::len(text) > max || text.len() > max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_dirs() {
        assert_eq!(formatter_for("/a/b.TS"), Some(Kind::Prettier));
        assert_eq!(formatter_for("/a/b.dart"), Some(Kind::Dart));
        assert_eq!(formatter_for("/a/b.kts"), Some(Kind::Ktlint));
        assert_eq!(formatter_for("/a/.json"), None);
        assert_eq!(formatter_for("/a/Makefile"), None);
        let r = |p: &str| crate::util::path_str(&crate::util::resolve(p));
        assert_eq!(dirs_up("/p/a/b/c.ts", "/p", false), vec![r("/p/a/b"), r("/p/a"), r("/p")]);
        assert_eq!(dirs_up("/q/c.ts", "/p", false), vec![r("/q")]);
    }

    #[test]
    fn cmd_quoting() {
        assert_eq!(quote_cmd_arg("a b").as_deref(), Some("^^^\"a^^^ b^^^\""));
        assert!(quote_cmd_arg("a\nb").is_none());
        let env = BTreeMap::from([("SystemRoot".to_string(), "C:\\W".to_string())]);
        let shim = cmd_shim("C:\\x\\p.cmd", &[], &env).unwrap();
        assert_eq!(shim.args[3], "\"C:\\x\\p.cmd\"");
        let more = with_args(&shim, &["--stdin-filepath".into(), "C:\\a b.ts".into()]).unwrap();
        assert!(more.args[3].starts_with("\"C:\\x\\p.cmd ^^^\"--stdin-filepath") && more.args[3].ends_with("b.ts^^^\"\""), "{}", more.args[3]);
    }

    #[cfg(unix)]
    #[test]
    fn formats_with_a_fake_tool() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("fmt-{}", crate::util::uuid()));
        std::fs::create_dir_all(dir.join("node_modules/.bin")).unwrap();
        let tool = dir.join("node_modules/.bin/prettier");
        std::fs::write(&tool, "#!/bin/sh\ntr a-z A-Z\n").unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let file = dir.join("x.ts");
        let f = Formatter::default();
        let out = f.format(&file.to_string_lossy(), &dir.to_string_lossy(), "abc\r\n");
        assert_eq!(out, json!({"ok": true, "text": "ABC\r\n", "tool": "prettier", "local": true}));
        let out = f.format(&file.to_string_lossy(), &dir.to_string_lossy(), "abc\n");
        assert_eq!(out["text"], "ABC\n");
        assert_eq!(f.format("/x/y.txt", "/x", "a"), json!({"ok": false, "error": "unsupported"}));
        std::fs::write(&tool, "#!/bin/sh\necho bad 1>&2\nexit 2\n").unwrap();
        let out = f.format(&file.to_string_lossy(), &dir.to_string_lossy(), "a");
        assert_eq!(out, json!({"ok": false, "error": "failed", "tool": "prettier", "detail": "bad"}));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
