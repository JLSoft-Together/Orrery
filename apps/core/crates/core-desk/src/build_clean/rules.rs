use crate::js;
use serde_json::Value;
use std::collections::HashSet;

pub const BUILD_DIR_NAMES: [&str; 14] = [
    "node_modules",
    "build",
    ".gradle",
    ".dart_tool",
    "dist",
    "out",
    "target",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
    "__pycache__",
    ".pytest_cache",
    ".venv",
];

pub const BUILD_SCAN_DEPTH: usize = 4;
pub const BUILD_PROJECTS_MAX: usize = 50;
pub const BUILD_ITEMS_MAX: usize = 500;

const GRADLE: [&str; 5] = ["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts", "gradlew"];

fn has(siblings: &HashSet<String>, names: &[&str]) -> bool {
    names.iter().any(|n| siblings.contains(*n))
}

pub fn build_dir_kind(name: &str, siblings: &HashSet<String>, git: bool) -> Option<&'static str> {
    let other = if git { Some("other") } else { None };
    match name {
        "node_modules" => Some("node"),
        ".dart_tool" => Some("flutter"),
        ".next" | ".nuxt" | ".turbo" | ".parcel-cache" => Some("web"),
        "__pycache__" | ".pytest_cache" | ".venv" => Some("python"),
        ".gradle" => (git || has(siblings, &GRADLE)).then_some("gradle"),
        "build" => {
            if has(siblings, &GRADLE) {
                Some("gradle")
            } else if has(siblings, &["pubspec.yaml"]) {
                Some("flutter")
            } else if has(siblings, &["package.json"]) {
                Some("web")
            } else {
                other
            }
        }
        "dist" | "out" => {
            if has(siblings, &["package.json"]) {
                Some("web")
            } else {
                other
            }
        }
        "target" => {
            if has(siblings, &["cargo.toml"]) {
                Some("rust")
            } else if has(siblings, &["pom.xml"]) {
                Some("maven")
            } else {
                other
            }
        }
        _ => None,
    }
}

pub fn needs_reinstall(name: &str) -> bool {
    name == "node_modules" || name == ".venv"
}

fn has_control(s: &str) -> bool {
    s.chars().any(|c| (c as u32) < 0x20)
}

pub fn text_ok(v: &Value, max: usize) -> Option<&str> {
    let s = v.as_str()?;
    let n = js::len(s);
    (n > 0 && n <= max && !has_control(s)).then_some(s)
}

pub fn parse_build_roots(raw: &Value) -> Option<Vec<String>> {
    let list = raw.as_array()?;
    if list.len() > BUILD_PROJECTS_MAX {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    for r in list {
        let s = text_ok(r, 1024)?;
        if !out.iter().any(|o| o == s) {
            out.push(s.to_string());
        }
    }
    Some(out)
}

pub fn is_build_rel(rel: &str) -> bool {
    let n = js::len(rel);
    if n == 0 || n > 1024 || has_control(rel) || rel.starts_with('/') || rel.contains('\\') {
        return false;
    }
    let b = rel.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return false;
    }
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() > BUILD_SCAN_DEPTH || parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..") {
        return false;
    }
    parts.last().is_some_and(|l| BUILD_DIR_NAMES.contains(l))
}

#[derive(Debug, Clone, PartialEq)]
pub struct CleanItem {
    pub root: String,
    pub rel: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Busy {
    pub name: String,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CleanRequest {
    pub items: Vec<CleanItem>,
    pub trash: bool,
    pub busy: Vec<Busy>,
    pub ignore_daemons: bool,
}

pub fn parse_build_clean(raw: &Value) -> Option<CleanRequest> {
    let r = raw.as_object()?;
    let items_raw = r.get("items")?.as_array()?;
    if items_raw.is_empty() || items_raw.len() > BUILD_ITEMS_MAX {
        return None;
    }
    let mut items = Vec::new();
    for it in items_raw {
        let i = it.as_object()?;
        let root = text_ok(i.get("root").unwrap_or(&Value::Null), 1024)?;
        let rel = i.get("rel")?.as_str()?;
        if !is_build_rel(rel) {
            return None;
        }
        items.push(CleanItem { root: root.to_string(), rel: rel.to_string() });
    }
    let mut busy = Vec::new();
    if let Some(b) = r.get("busy") {
        let list = b.as_array()?;
        if list.len() > 200 {
            return None;
        }
        for x in list {
            let x = x.as_object()?;
            let name = x.get("name")?.as_str()?;
            let cwd = text_ok(x.get("cwd").unwrap_or(&Value::Null), 1024)?;
            let clean: String = name.chars().map(|c| if (c as u32) < 0x20 { ' ' } else { c }).collect();
            busy.push(Busy { name: js::slice(&clean, 120), cwd: cwd.to_string() });
        }
    }
    Some(CleanRequest {
        items,
        trash: r.get("trash") == Some(&Value::Bool(true)),
        busy,
        ignore_daemons: r.get("ignoreDaemons") == Some(&Value::Bool(true)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kinds_and_rels() {
        let s = |n: &[&str]| n.iter().map(|x| x.to_string()).collect::<HashSet<_>>();
        assert_eq!(build_dir_kind("build", &s(&["build.gradle.kts"]), false), Some("gradle"));
        assert_eq!(build_dir_kind("target", &s(&["cargo.toml"]), false), Some("rust"));
        assert_eq!(build_dir_kind("out", &s(&[]), false), None);
        assert_eq!(build_dir_kind("out", &s(&[]), true), Some("other"));
        assert!(is_build_rel("app/build"));
        assert!(!is_build_rel("../node_modules"));
        assert!(!is_build_rel("C:/x/build"));
        assert!(!is_build_rel("a/b/c/d/build"));
        assert!(parse_build_clean(&json!({"items": [{"root": "C:\\p", "rel": "src"}]})).is_none());
        let r = parse_build_clean(&json!({"items": [{"root": "C:\\p", "rel": "build"}], "trash": true, "busy": [{"name": "a\nb", "cwd": "C:\\p"}]})).unwrap();
        assert_eq!(r.busy[0].name, "a b");
        assert!(r.trash && !r.ignore_daemons);
        assert_eq!(parse_build_roots(&json!(["a", "a", "b"])), Some(vec!["a".into(), "b".into()]));
    }
}
