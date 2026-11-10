use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

const MAX_FILE: u64 = 256 * 1024;
const MAX_ITEMS: usize = 200;
const MAX_DETAIL: usize = 200;

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex"))
}

fn safe_name(n: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"^[A-Za-z0-9_][A-Za-z0-9_:.@/+-]*$").is_match(n)
}

fn clip(s: &str) -> String {
    if crate::js::len(s) > MAX_DETAIL {
        format!("{}…", crate::js::slice(s, MAX_DETAIL - 1))
    } else {
        s.to_string()
    }
}

fn read_small(file: &Path) -> Option<String> {
    std::fs::metadata(file).ok().filter(|m| m.is_file() && m.len() <= MAX_FILE)?;
    std::fs::read(file).ok().map(|b| String::from_utf8_lossy(&b).to_string())
}

fn exists(file: &Path) -> bool {
    std::fs::metadata(file).map(|m| m.is_file()).unwrap_or(false)
}

fn lines(text: &str) -> Vec<&str> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

pub fn package_scripts(text: &str, runner: &str) -> Vec<Value> {
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    let Some(Value::Object(scripts)) = doc.get("scripts") else { return Vec::new() };
    let prefix = if runner == "yarn" { "yarn".to_string() } else { format!("{runner} run") };
    crate::js::entries(scripts)
        .into_iter()
        .filter(|(n, b)| safe_name(n) && b.is_string())
        .take(MAX_ITEMS)
        .map(|(n, b)| json!({"name": n, "detail": clip(b.as_str().unwrap_or("")), "run": format!("{prefix} {n}")}))
        .collect()
}

pub fn make_targets(text: &str) -> Vec<Value> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = re(&R, r"^([A-Za-z0-9_][A-Za-z0-9_.\-/]*)\s*:");
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in lines(text) {
        let Some(m) = r.captures(line) else { continue };
        let end = m.get(0).map(|x| x.end()).unwrap_or(0);
        if matches!(line[end..].chars().next(), Some(':') | Some('=')) {
            continue;
        }
        let name = m[1].to_string();
        if seen.contains(&name) || !safe_name(&name) {
            continue;
        }
        seen.push(name.clone());
        out.push(json!({"name": name, "run": format!("make {name}")}));
        if out.len() >= MAX_ITEMS {
            break;
        }
    }
    out
}

pub fn just_recipes(text: &str) -> Vec<Value> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = re(&R, r"^@?([A-Za-z_][A-Za-z0-9_-]*)\b([^:=]*):");
    let mut out: Vec<Value> = Vec::new();
    for line in lines(text) {
        let Some(m) = r.captures(line) else { continue };
        let end = m.get(0).map(|x| x.end()).unwrap_or(0);
        if line[end..].starts_with('=') {
            continue;
        }
        let name = m[1].to_string();
        if ["set", "alias", "export", "import", "mod"].contains(&name.as_str()) || out.iter().any(|r| r["name"] == json!(name)) {
            continue;
        }
        if !m[2].trim().is_empty() {
            continue;
        }
        out.push(json!({"name": name, "run": format!("just {name}")}));
        if out.len() >= MAX_ITEMS {
            break;
        }
    }
    out
}

fn toml_sections(text: &str) -> Vec<(String, Vec<String>)> {
    static H: OnceLock<Regex> = OnceLock::new();
    let head = re(&H, r"^\[([^\[\]]+)\]$");
    let mut sections: Vec<(String, Vec<String>)> = vec![(String::new(), Vec::new())];
    let mut current = String::new();
    for raw in lines(text) {
        let line = raw.trim();
        if let Some(c) = head.captures(line) {
            current = c[1].trim().to_string();
            if !sections.iter().any(|(k, _)| *k == current) {
                sections.push((current.clone(), Vec::new()));
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            if let Some((_, l)) = sections.iter_mut().find(|(k, _)| *k == current) {
                l.push(line.to_string());
            }
        }
    }
    sections
}

fn section<'a>(s: &'a [(String, Vec<String>)], k: &str) -> Option<&'a Vec<String>> {
    s.iter().find(|(x, _)| x == k).map(|(_, l)| l)
}

pub fn cargo_commands(text: &str) -> Vec<Value> {
    let s = toml_sections(text);
    if section(&s, "package").is_none() && section(&s, "workspace").is_none() {
        return Vec::new();
    }
    let mut items = vec![json!({"name": "build", "run": "cargo build"}), json!({"name": "test", "run": "cargo test"})];
    if section(&s, "package").is_some() {
        items.push(json!({"name": "run", "run": "cargo run"}));
    }
    items
}

pub fn python_scripts(text: &str, runner: Option<&str>) -> Vec<Value> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = re(&R, r#"^"?([A-Za-z0-9_][A-Za-z0-9_.-]*)"?\s*=\s*"([^"]+)"$"#);
    let s = toml_sections(text);
    let mut all: Vec<String> = section(&s, "project.scripts").cloned().unwrap_or_default();
    all.extend(section(&s, "tool.poetry.scripts").cloned().unwrap_or_default());
    let mut out: Vec<Value> = Vec::new();
    for line in &all {
        let Some(m) = r.captures(line) else { continue };
        let name = m[1].to_string();
        if !safe_name(&name) || out.iter().any(|x| x["name"] == json!(name)) {
            continue;
        }
        let run = match runner {
            Some(rn) => format!("{rn} run {name}"),
            None => name.clone(),
        };
        out.push(json!({"name": name, "detail": clip(&m[2]), "run": run}));
        if out.len() >= MAX_ITEMS {
            break;
        }
    }
    out
}

fn group(groups: &mut Vec<Value>, source: &str, file: &str, items: Vec<Value>) {
    if !items.is_empty() {
        groups.push(json!({"source": source, "file": file, "items": items}));
    }
}

pub fn detect_scripts(dir: &str) -> Value {
    let at = |n: &str| Path::new(dir).join(n);
    let mut groups = Vec::new();
    if let Some(pkg) = read_small(&at("package.json")).filter(|s| !s.is_empty()) {
        let runner = if exists(&at("pnpm-lock.yaml")) {
            "pnpm"
        } else if exists(&at("yarn.lock")) {
            "yarn"
        } else if exists(&at("bun.lockb")) || exists(&at("bun.lock")) {
            "bun"
        } else {
            "npm"
        };
        group(&mut groups, runner, "package.json", package_scripts(&pkg, runner));
    }
    if let Some((name, text)) = ["Makefile", "makefile", "GNUmakefile"].iter().find_map(|n| read_small(&at(n)).map(|t| (*n, t))) {
        group(&mut groups, "make", name, make_targets(&text));
    }
    if let Some((name, text)) = ["justfile", "Justfile", ".justfile"].iter().find_map(|n| read_small(&at(n)).map(|t| (*n, t))) {
        group(&mut groups, "just", name, just_recipes(&text));
    }
    if let Some(cargo) = read_small(&at("Cargo.toml")).filter(|s| !s.is_empty()) {
        group(&mut groups, "cargo", "Cargo.toml", cargo_commands(&cargo));
    }
    if let Some(py) = read_small(&at("pyproject.toml")).filter(|s| !s.is_empty()) {
        let runner = if exists(&at("uv.lock")) {
            Some("uv")
        } else if exists(&at("poetry.lock")) {
            Some("poetry")
        } else {
            None
        };
        group(&mut groups, "python", "pyproject.toml", python_scripts(&py, runner));
    }
    json!({"dir": dir, "groups": groups})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_targets() {
        let m = make_targets("all: x\nX := 1\n.PHONY: all\nbuild:\n\tcc\nall:\n%.o: %.c\n");
        assert_eq!(m.iter().map(|v| v["name"].as_str().unwrap()).collect::<Vec<_>>(), vec!["all", "build"]);
        let j = just_recipes("set shell := [\"bash\"]\nbuild:\n  cargo\nrun arg:\n  x\nx := y\n@test: build\n");
        assert_eq!(j.iter().map(|v| v["name"].as_str().unwrap()).collect::<Vec<_>>(), vec!["build", "test"]);
        let p = package_scripts(r#"{"scripts":{"b":"x","2":"y","bad name":"z","n":1}}"#, "npm");
        assert_eq!(p[0]["name"], "2");
        assert_eq!(p[1]["run"], "npm run b");
        assert_eq!(cargo_commands("[workspace]\n").len(), 2);
        assert_eq!(python_scripts("[project.scripts]\ncli = \"pkg:main\"\n", Some("uv"))[0]["run"], "uv run cli");
    }
}
