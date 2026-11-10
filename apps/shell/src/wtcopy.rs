use std::fs;
use std::path::Path;

use serde_json::Value;

const MAX_FILE: u64 = 1024 * 1024;
const MAX_TOTAL: u64 = 5 * 1024 * 1024;
const MAX_DEPTH: usize = 4;
const MAX_PATTERNS: usize = 20;
const MAX_FOUND: usize = 200;
const SKIP_DIRS: [&str; 10] = [".git", "node_modules", "build", ".gradle", ".dart_tool", "dist", "out", ".idea", "Pods", ".venv"];

pub struct Pattern {
    deep: bool,
    glob: String,
}

pub fn parse_patterns(input: &Value) -> Vec<Pattern> {
    let Some(list) = input.as_array() else { return Vec::new() };
    let mut out = Vec::new();
    for raw in list.iter().take(MAX_PATTERNS) {
        let Some(raw) = raw.as_str() else { continue };
        let deep = raw.starts_with("**/");
        let name = if deep { &raw[3..] } else { raw };
        let bad = name.is_empty() || name.chars().count() > 100 || name.contains(['\\', '/', ':']) || name == "." || name == ".." || name.chars().all(|c| c == '*');
        if !bad {
            out.push(Pattern { deep, glob: name.to_lowercase() });
        }
    }
    out
}

pub fn glob_match(glob: &str, name: &str) -> bool {
    let (g, n): (Vec<char>, Vec<char>) = (glob.chars().collect(), name.to_lowercase().chars().collect());
    let (mut gi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if gi < g.len() && g[gi] == '*' {
            star = Some(gi);
            mark = ni;
            gi += 1;
        } else if gi < g.len() && g[gi] == n[ni] {
            gi += 1;
            ni += 1;
        } else if let Some(s) = star {
            gi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    g[gi..].iter().all(|c| *c == '*')
}

fn walk(dir: &Path, rel: &str, depth: usize, patterns: &[Pattern], found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_file() && patterns.iter().any(|p| glob_match(&p.glob, &name) && (p.deep || depth == 0)) {
            found.push(r);
        } else if kind.is_dir() && depth < MAX_DEPTH && !SKIP_DIRS.contains(&name.as_str()) && patterns.iter().any(|p| p.deep) {
            walk(&e.path(), &r, depth + 1, patterns, found);
        }
        if found.len() > MAX_FOUND {
            return;
        }
    }
}

pub fn copy_setup_files(main: &Path, dest: &Path, input: &Value) -> Vec<String> {
    let patterns = parse_patterns(input);
    if patterns.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    walk(main, "", 0, &patterns, &mut found);
    let (mut copied, mut total) = (Vec::new(), 0u64);
    for rel in found {
        let (from, to) = (main.join(&rel), dest.join(&rel));
        let Ok(meta) = fs::symlink_metadata(&from) else { continue };
        if !meta.is_file() || meta.len() > MAX_FILE || total + meta.len() > MAX_TOTAL || fs::symlink_metadata(&to).is_ok() {
            continue;
        }
        if let Some(dir) = to.parent() {
            if fs::create_dir_all(dir).is_err() {
                continue;
            }
        }
        if fs::OpenOptions::new().write(true).create_new(true).open(&to).and_then(|mut f| std::io::copy(&mut fs::File::open(&from)?, &mut f)).is_ok() {
            copied.push(rel);
            total += meta.len();
        }
    }
    copied
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn globs_match_names_case_insensitively_with_star() {
        assert!(glob_match(".env*", ".env.local"));
        assert!(glob_match(".env*", ".ENV"));
        assert!(glob_match("local.properties", "Local.Properties"));
        assert!(!glob_match(".env*", "my.env"));
        assert!(glob_match("*.json", "a.json"));
        assert!(!glob_match("a?c", "abc"));
    }

    #[test]
    fn bad_patterns_are_dropped() {
        let p = parse_patterns(&json!([".env*", "**/google-services.json", "a/b", "*", "..", 3, "x:y"]));
        assert_eq!(p.len(), 2);
        assert!(p[1].deep);
        assert!(parse_patterns(&json!("x")).is_empty());
    }

    #[test]
    fn only_missing_small_matching_files_are_copied() {
        let base = std::env::temp_dir().join(format!("orrery-wtcopy-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
        let (main, dest) = (base.join("main"), base.join("dest"));
        fs::create_dir_all(main.join("app")).unwrap();
        fs::create_dir_all(main.join("node_modules")).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(main.join(".env"), "A=1").unwrap();
        fs::write(main.join("app/google-services.json"), "{}").unwrap();
        fs::write(main.join("node_modules/google-services.json"), "{}").unwrap();
        fs::write(main.join("big.env"), "x").unwrap();
        fs::write(dest.join(".env"), "keep").unwrap();
        let mut got = copy_setup_files(&main, &dest, &json!([".env*", "**/google-services.json"]));
        got.sort();
        assert_eq!(got, ["app/google-services.json"]);
        assert_eq!(fs::read_to_string(dest.join(".env")).unwrap(), "keep");
        assert!(!dest.join("node_modules").exists());
        let _ = fs::remove_dir_all(&base);
    }
}
