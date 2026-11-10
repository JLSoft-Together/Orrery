use crate::util::resolve;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

fn fold(p: &str) -> String {
    if cfg!(windows) {
        p.to_lowercase()
    } else {
        p.to_string()
    }
}

pub fn resolve_str(p: &str) -> String {
    let s = resolve(p).to_string_lossy().to_string();
    if cfg!(windows) {
        let s = s.replace('/', "\\");
        if s.len() == 2 && s.ends_with(':') {
            return format!("{s}\\");
        }
        return s;
    }
    s
}

pub fn is_within(root: &str, target: &str) -> bool {
    let r = fold(&resolve_str(root));
    let t = fold(&resolve_str(target));
    if t == r {
        return true;
    }
    let sep = MAIN_SEPARATOR.to_string();
    let prefix = if r.ends_with(&sep) { r } else { format!("{r}{sep}") };
    t.starts_with(&prefix)
}

pub fn root_key(root: &str) -> String {
    fold(&resolve_str(root))
}

pub fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        return PathBuf::from(format!("\\\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix("\\\\?\\") {
        return PathBuf::from(rest);
    }
    p
}

pub fn real_or_null(p: &str) -> Option<String> {
    std::fs::canonicalize(p).ok().map(strip_verbatim).map(|p| p.to_string_lossy().to_string())
}

pub fn is_absolute(p: &str) -> bool {
    if cfg!(windows) {
        let b = p.as_bytes();
        (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')) || p.starts_with('\\') || p.starts_with('/')
    } else {
        p.starts_with('/')
    }
}

pub fn root_of(target: &str, roots: &[String]) -> Option<String> {
    let mut best: Option<&String> = None;
    for r in roots {
        if is_within(r, target) && best.map(|b| r.len() > b.len()).unwrap_or(true) {
            best = Some(r);
        }
    }
    best.cloned()
}

pub fn dirname(p: &str) -> String {
    Path::new(p).parent().map(|d| d.to_string_lossy().to_string()).filter(|d| !d.is_empty()).unwrap_or_else(|| p.to_string())
}

pub fn join(root: &str, rel: &str) -> String {
    resolve_str(&Path::new(root).join(rel).to_string_lossy())
}

pub fn relative(from: &str, to: &str) -> String {
    let a = resolve(from);
    let b = resolve(to);
    let ac: Vec<String> = a.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let bc: Vec<String> = b.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let mut i = 0;
    while i < ac.len() && i < bc.len() && fold(&ac[i]).replace('/', "\\") == fold(&bc[i]).replace('/', "\\") {
        i += 1;
    }
    if i == 0 && cfg!(windows) {
        return resolve_str(to);
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in i..ac.len() {
        parts.push("..".into());
    }
    for c in &bc[i..] {
        parts.push(c.clone());
    }
    parts.join(std::path::MAIN_SEPARATOR_STR)
}

pub struct SafePath {
    pub rel: String,
    pub abs: String,
}

pub fn safe_repo_path(root: &str, rel: &serde_json::Value) -> Option<SafePath> {
    let rel = rel.as_str()?;
    if rel.is_empty() || crate::js::len(rel) > 4096 || rel.contains('\0') {
        return None;
    }
    let b = rel.as_bytes();
    if is_absolute(rel) || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':') || rel.starts_with("\\\\") {
        return None;
    }
    if !cfg!(windows) && rel.starts_with('/') {
        return None;
    }
    let clean = rel.replace('\\', "/");
    if clean.split('/').any(|p| p == "..") {
        return None;
    }
    let abs = join(root, &clean);
    if abs == resolve_str(root) || !is_within(root, &abs) {
        return None;
    }
    Some(SafePath { rel: clean, abs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn safe_paths() {
        let root = if cfg!(windows) { "C:\\r\\repo" } else { "/r/repo" };
        assert!(safe_repo_path(root, &json!("a/b.txt")).is_some());
        assert_eq!(safe_repo_path(root, &json!("a\\b.txt")).unwrap().rel, "a/b.txt");
        assert!(safe_repo_path(root, &json!("../x")).is_none());
        assert!(safe_repo_path(root, &json!("C:/x")).is_none());
        assert!(safe_repo_path(root, &json!(".")).is_none());
        assert!(safe_repo_path(root, &json!("")).is_none());
        assert!(safe_repo_path(root, &json!(3)).is_none());
        let sub = if cfg!(windows) { "C:\\r\\repo\\a\\b" } else { "/r/repo/a/b" };
        assert_eq!(relative(root, sub), if cfg!(windows) { "a\\b" } else { "a/b" });
        assert_eq!(root_of(sub, &[root.to_string(), sub.to_string()]).as_deref(), Some(sub));
    }
}
