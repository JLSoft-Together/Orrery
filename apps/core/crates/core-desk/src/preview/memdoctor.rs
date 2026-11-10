use super::ctx::{context_sources, project_dir};
use super::memlint::{lint_docs, Doc, LintCtx};
use crate::claude::{self, Sys};
use serde_json::{json, Value};
use std::path::Path;

const READ_MAX: u64 = 256 * 1024;
const SUB_ROOTS_MAX: usize = 200;
const SKIP_DIRS: [&str; 7] = ["node_modules", "build", "dist", "out", "target", "release", "coverage"];

fn sub_roots(cwd: &str) -> Vec<String> {
    let dirs = |dir: &str| -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
        rd.filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| !n.starts_with('.') && !SKIP_DIRS.contains(&n.as_str()))
            .map(|n| Path::new(dir).join(n).to_string_lossy().to_string())
            .collect()
    };
    let mut out: Vec<String> = Vec::new();
    for d in dirs(cwd) {
        if out.len() >= SUB_ROOTS_MAX {
            break;
        }
        out.push(d.clone());
        for s in dirs(&d) {
            if out.len() < SUB_ROOTS_MAX {
                out.push(s);
            }
        }
    }
    out
}

fn role_of(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "managed" | "user" | "project" | "local" | "rules" | "import" => "instructions",
        "memory-index" => "memory-index",
        "memory" => "memory",
        _ => return None,
    })
}

fn inside(file: &str, dir: &str, win: bool) -> bool {
    let norm = |p: &str| claude::fold(&claude::resolve(p), win);
    let (f, d) = (norm(file), norm(dir));
    match Path::new(&f).strip_prefix(&d) {
        Ok(rel) => {
            let s = rel.to_string_lossy();
            !s.is_empty() && !s.starts_with("..")
        }
        Err(_) => false,
    }
}

pub fn memory_doctor(cwd: &str, sys: &Sys) -> Value {
    let src = context_sources(cwd, sys);
    let config = crate::util::path_str(&sys.claude_root());
    let memory_dir = Path::new(&project_dir(&config, cwd, sys.win)).join("memory").to_string_lossy().to_string();
    let mut docs: Vec<Doc> = Vec::new();
    for f in src["files"].as_array().cloned().unwrap_or_default() {
        let (Some(path), Some(kind), Some(bytes)) = (f["path"].as_str(), f["kind"].as_str(), f["bytes"].as_u64()) else { continue };
        let Some(role) = role_of(kind) else { continue };
        if !path.to_lowercase().ends_with(".md") {
            continue;
        }
        let text = if bytes <= READ_MAX { std::fs::read(path).ok().map(|b| String::from_utf8_lossy(&b).to_string()) } else { None };
        docs.push(Doc { path: path.to_string(), role, text, bytes, in_project: inside(path, cwd, sys.win) });
    }
    let roots = sub_roots(cwd);
    let lint = LintCtx { cwd, home: &sys.home, memory_dir: &memory_dir, sub_roots: &roots, fold_case: sys.win };
    let (findings, truncated) = lint_docs(&docs, &lint);
    let out: Vec<Value> = findings
        .iter()
        .map(|f| {
            let mut o = json!({"file": f.file, "line": f.line, "kind": f.kind, "text": f.text});
            if let Some(limit) = f.limit {
                o["limit"] = json!(limit);
            }
            o["openable"] = json!(inside(&f.file, cwd, sys.win) || inside(&f.file, &config, sys.win));
            o
        })
        .collect();
    json!({"ok": true, "cwd": cwd, "home": sys.home, "memoryDir": memory_dir, "files": docs.len(), "findings": out, "truncated": truncated || src["truncated"] == json!(true)})
}

