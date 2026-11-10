use crate::claude::{self, Sys};
use crate::extensions::status;
use crate::js;
use core_transcript::collate::locale_cmp;
use core_transcript::jsstr::{units, utf16_cmp};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const READ_MAX: u64 = 256 * 1024;
const FILES_MAX: usize = 300;
const IMPORT_DEPTH: usize = 5;
const RULES_DEPTH: i32 = 4;
const PER_DIR: [&[&str]; 5] = [&["CLAUDE.md"], &["CLAUDE.local.md"], &["AGENTS.md"], &[".claude", "CLAUDE.md"], &[".claude", "AGENTS.md"]];

pub const JS_S: &str = r"\t\n\x0b\x0c\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

fn fence_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(&format!(r"^[{JS_S}]*(?:```|~~~)")).expect("regex"))
}

fn inline_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"`[^`]*`").expect("regex"))
}

fn import_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(&format!(r"(?:^|[{JS_S}])@([^{JS_S}`]+)")).expect("regex"))
}

fn trail_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[.,;:!?)\]]+$").expect("regex"))
}

fn ext_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\.[A-Za-z0-9]{1,8}$").expect("regex"))
}

pub fn js_lines(text: &str) -> Vec<&str> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

pub fn cwd_slug(cwd: &str) -> String {
    let mut out = String::new();
    for c in cwd.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                out.push('-');
            }
        }
    }
    out
}

pub fn managed_claude_md(win: bool) -> String {
    if win {
        r"C:\Program Files\ClaudeCode\CLAUDE.md".into()
    } else {
        "/Library/Application Support/ClaudeCode/CLAUDE.md".into()
    }
}

pub fn ancestors(cwd: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut d = crate::util::resolve(cwd);
    loop {
        out.insert(0, d.clone());
        match d.parent() {
            Some(p) if p != d => d = p.to_path_buf(),
            _ => break,
        }
    }
    out
}

pub fn imports_of(text: &str, dir: &str, home: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut fence = false;
    for raw in js_lines(text) {
        if fence_re().is_match(raw) {
            fence = !fence;
            continue;
        }
        if fence {
            continue;
        }
        let line = inline_re().replace_all(raw, " ");
        for m in import_re().captures_iter(&line) {
            let p = trail_re().replace(&m[1], "").to_string();
            if !ext_re().is_match(&p) && !p.contains('/') && !p.contains('\\') {
                continue;
            }
            let abs = if p.starts_with('~') {
                claude::join(home, &[p.chars().skip(2).collect::<String>().as_str()])
            } else if claude::is_absolute(&p) {
                p.clone()
            } else {
                claude::resolve(&Path::new(dir).join(&p).to_string_lossy())
            };
            if !out.contains(&abs) {
                out.push(abs);
            }
        }
    }
    out.truncate(50);
    out
}

pub fn project_dir(config: &str, cwd: &str, win: bool) -> String {
    let slug = cwd_slug(&crate::util::path_str(&crate::util::resolve(cwd)));
    let base = Path::new(config).join("projects");
    if !win {
        return base.join(slug).to_string_lossy().to_string();
    }
    let names: Vec<String> = std::fs::read_dir(&base).map(|r| r.filter_map(Result::ok).filter_map(|e| e.file_name().into_string().ok()).collect()).unwrap_or_default();
    let pick = names.iter().find(|n| **n == slug).or_else(|| names.iter().find(|n| n.to_lowercase() == slug.to_lowercase())).cloned().unwrap_or(slug);
    base.join(pick).to_string_lossy().to_string()
}

fn stat_file(file: &str) -> Option<u64> {
    std::fs::metadata(file).ok().filter(|m| m.is_file()).map(|m| m.len())
}

fn line_count(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.split('\n').count() - usize::from(text.ends_with('\n'))
    }
}

fn sorted_entries(dir: &str) -> Vec<(String, std::fs::FileType)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<(String, std::fs::FileType)> = rd.filter_map(Result::ok).filter_map(|e| Some((e.file_name().into_string().ok()?, e.file_type().ok()?))).collect();
    v.sort_by(|a, b| locale_cmp(&a.0, &b.0));
    v
}

pub fn md_files(dir: &str, depth: i32, out: &mut Vec<String>) {
    if depth < 0 || out.len() >= FILES_MAX {
        return;
    }
    for (name, ft) in sorted_entries(dir) {
        let p = Path::new(dir).join(&name).to_string_lossy().to_string();
        if ft.is_dir() && !name.starts_with('.') {
            md_files(&p, depth - 1, out);
        } else if ft.is_file() && name.to_lowercase().ends_with(".md") {
            out.push(p);
        }
    }
}

fn skill_files(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = sorted_entries(dir).into_iter().filter(|(n, ft)| ft.is_dir() && !n.starts_with('.')).map(|(n, _)| Path::new(dir).join(n).join("SKILL.md").to_string_lossy().to_string()).collect();
    out.truncate(200);
    out
}

fn enabled_plugins(files: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in files {
        let Ok(bytes) = std::fs::read(f) else { continue };
        let Some(Value::Object(o)) = js::parse(&String::from_utf8_lossy(&bytes)) else { continue };
        if let Some(Value::Object(map)) = o.get("enabledPlugins") {
            for (k, on) in js::entries(map) {
                if on == &Value::Bool(true) && units(k) <= 200 && !out.contains(k) {
                    out.push(k.clone());
                }
            }
        }
    }
    out.sort_by(|a, b| utf16_cmp(a, b));
    out
}

struct Walk<'a> {
    sys: &'a Sys,
    files: Vec<Map<String, Value>>,
    seen: Vec<String>,
    truncated: bool,
}

impl Walk<'_> {
    fn key(&self, p: &str) -> String {
        claude::fold(&claude::resolve(p), self.sys.win)
    }

    fn add(&mut self, file: &str, kind: &str, load: &str, from: Option<&str>, depth: usize) {
        let key = self.key(file);
        if self.seen.contains(&key) {
            return;
        }
        if self.files.len() >= FILES_MAX {
            self.truncated = true;
            return;
        }
        let Some(bytes) = stat_file(file) else { return };
        self.seen.push(key);
        let mut entry = Map::new();
        entry.insert("kind".into(), json!(kind));
        entry.insert("load".into(), json!(load));
        if let Some(f) = from {
            entry.insert("from".into(), json!(f));
        }
        entry.insert("path".into(), json!(file));
        entry.insert("bytes".into(), json!(bytes));
        let at = self.files.len();
        self.files.push(entry);
        let md = file.to_lowercase().ends_with(".md");
        if !md || bytes > READ_MAX || kind == "skill" || load == "on-demand" {
            return;
        }
        let text = std::fs::read(file).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        self.files[at].insert("lines".into(), json!(line_count(&text)));
        if depth >= IMPORT_DEPTH {
            return;
        }
        let dir = claude::dirname(file);
        for imp in imports_of(&text, &dir, &self.sys.home) {
            self.add(&imp, "import", load, Some(file), depth + 1);
        }
    }
}

pub fn context_sources(cwd: &str, sys: &Sys) -> Value {
    let config = crate::util::path_str(&sys.claude_root());
    let mut w = Walk { sys, files: Vec::new(), seen: Vec::new(), truncated: false };
    w.add(&managed_claude_md(sys.win), "managed", "start", None, 0);
    w.add(&Path::new(&config).join("CLAUDE.md").to_string_lossy(), "user", "start", None, 0);
    let mut user_rules = Vec::new();
    md_files(&Path::new(&config).join("rules").to_string_lossy(), RULES_DEPTH, &mut user_rules);
    for f in &user_rules {
        w.add(f, "rules", "start", None, 0);
    }
    let home_key = w.key(&sys.home);
    for dir in ancestors(cwd) {
        let dir_s = dir.to_string_lossy().to_string();
        for parts in PER_DIR {
            let file = parts.iter().fold(dir.clone(), |p, part| p.join(part));
            let name = parts.join(std::path::MAIN_SEPARATOR_STR);
            let kind = if name.contains("local") {
                "local"
            } else if w.key(&dir_s) == home_key && name.starts_with(".claude") {
                "user"
            } else {
                "project"
            };
            w.add(&file.to_string_lossy(), kind, "start", None, 0);
        }
    }
    let mut rules = Vec::new();
    md_files(&Path::new(cwd).join(".claude").join("rules").to_string_lossy(), RULES_DEPTH, &mut rules);
    for f in &rules {
        w.add(f, "rules", "start", None, 0);
    }
    let mem_dir = Path::new(&project_dir(&config, cwd, sys.win)).join("memory").to_string_lossy().to_string();
    w.add(&Path::new(&mem_dir).join("MEMORY.md").to_string_lossy(), "memory-index", "start", None, 0);
    let mut mem = Vec::new();
    md_files(&mem_dir, 1, &mut mem);
    for f in &mem {
        w.add(f, "memory", "on-demand", None, 0);
    }
    let mut skills = skill_files(&Path::new(&config).join("skills").to_string_lossy());
    skills.extend(skill_files(&Path::new(cwd).join(".claude").join("skills").to_string_lossy()));
    for f in &skills {
        w.add(f, "skill", "listed", None, 0);
    }
    w.add(&Path::new(cwd).join(".mcp.json").to_string_lossy(), "mcp-config", "start", None, 0);
    let mcp = status::configured_json(&status::read_configured(&sys.claude_json(), Some(cwd), sys.win));
    let plugins = enabled_plugins(&[
        Path::new(&config).join("settings.json").to_string_lossy().to_string(),
        Path::new(cwd).join(".claude").join("settings.json").to_string_lossy().to_string(),
        Path::new(cwd).join(".claude").join("settings.local.json").to_string_lossy().to_string(),
    ]);
    json!({"ok": true, "cwd": cwd, "home": sys.home, "files": w.files, "mcp": mcp, "plugins": plugins, "truncated": w.truncated})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_imports_and_lines() {
        assert_eq!(cwd_slug("/a/b c"), "-a-b-c");
        assert_eq!(cwd_slug("/x😀"), "-x--");
        let text = "see @docs/a.md and @~/n.md.\n```\n@skip.md\n```\n`@in.md` @plain @x/y\n";
        let got = imports_of(text, "/proj", "/home/u");
        assert_eq!(got, vec!["/proj/docs/a.md", "/home/u/n.md", "/proj/x/y"]);
        assert_eq!(line_count("a\nb\n"), 2);
        assert_eq!(line_count("a\nb"), 2);
        assert_eq!(line_count(""), 0);
    }
}
