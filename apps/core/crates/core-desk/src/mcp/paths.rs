use crate::util::{is_inside_or_same, path_str, resolve};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const PATH_MAX: usize = 4096;
pub const SEGMENTS_MAX: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum Inside {
    Ok { path: String, exists: bool },
    Err(&'static str),
}

fn unc(s: &str) -> bool {
    let b: Vec<char> = s.chars().take(2).collect();
    b.len() == 2 && b.iter().all(|c| *c == '\\' || *c == '/')
}

pub fn is_remote_link(to: &str) -> bool {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    let local = A.get_or_init(|| Regex::new(r"^[\\/]{2}[?.][\\/][A-Za-z]:[\\/]").unwrap());
    let share = B.get_or_init(|| Regex::new(r"(?i)^[\\/]{2}\?[\\/]UNC[\\/]").unwrap());
    (unc(to) && !local.is_match(to)) || share.is_match(to)
}

fn device_name(seg: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^(con|prn|aux|nul|com[0-9¹²³]|lpt[0-9¹²³]|conin\$|conout\$)\s*(\..*)?$").unwrap()).is_match(seg)
}

pub fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = path_str(&p);
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
            return PathBuf::from(rest);
        }
    }
    p
}

pub fn real_of(p: &str) -> Option<String> {
    std::fs::canonicalize(p).ok().map(strip_verbatim).map(|p| path_str(&p))
}

fn remote_link_below(root: &str, target: &str) -> bool {
    let root_p = Path::new(root);
    let Ok(rel) = Path::new(target).strip_prefix(root_p) else { return false };
    let mut at = root_p.to_path_buf();
    for part in rel.components() {
        at.push(part.as_os_str());
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.file_type().is_symlink() => {
                if let Ok(to) = std::fs::read_link(&at) {
                    if is_remote_link(&path_str(&to)) {
                        return true;
                    }
                }
            }
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

fn resolve_from(root: &str, raw: &str) -> String {
    let p = Path::new(raw);
    if p.is_absolute() {
        path_str(&resolve(raw))
    } else {
        path_str(&resolve(&path_str(&Path::new(root).join(raw))))
    }
}

fn parent_of(p: &str) -> String {
    Path::new(p).parent().map(path_str).unwrap_or_else(|| p.to_string())
}

fn base_of(p: &str) -> String {
    Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

pub fn resolve_inside(root: &str, raw: Option<&serde_json::Value>, allow_missing: bool) -> Inside {
    let Some(raw) = raw.and_then(|v| v.as_str()) else { return Inside::Err("path") };
    if raw.trim().is_empty() || raw.encode_utf16().count() > PATH_MAX || raw.contains(['\0', '\r', '\n']) {
        return Inside::Err("path");
    }
    if unc(raw) || unc(root) {
        return Inside::Err("path");
    }
    let segments: Vec<&str> = raw.split(['\\', '/']).filter(|s| !s.is_empty()).collect();
    if segments.len() > SEGMENTS_MAX {
        return Inside::Err("path");
    }
    if cfg!(windows) && segments.iter().any(|s| device_name(s)) {
        return Inside::Err("path");
    }
    let Some(real_root) = real_of(root) else { return Inside::Err("missing") };
    let asked = resolve_from(&real_root, raw);
    if !is_inside_or_same(&real_root, &asked) {
        return Inside::Err("outside");
    }
    if remote_link_below(&real_root, &asked) {
        return Inside::Err("path");
    }
    let (real, exists) = match real_of(&asked) {
        Some(r) => (r, true),
        None => {
            if !allow_missing {
                return Inside::Err("missing");
            }
            let mut dir = parent_of(&asked);
            let mut rest = vec![base_of(&asked)];
            loop {
                if let Some(r) = real_of(&dir) {
                    let mut p = PathBuf::from(r);
                    for part in &rest {
                        p.push(part);
                    }
                    break (path_str(&p), false);
                }
                let up = parent_of(&dir);
                if up == dir || !is_inside_or_same(&real_root, &up) {
                    return Inside::Err("missing");
                }
                rest.insert(0, base_of(&dir));
                dir = up;
            }
        }
    };
    if unc(&real) || !is_inside_or_same(&real_root, &real) {
        return Inside::Err("outside");
    }
    if exists {
        match std::fs::metadata(&real) {
            Ok(m) if m.is_dir() => return Inside::Err("folder"),
            Ok(m) if !m.is_file() => return Inside::Err("path"),
            Ok(_) => {}
            Err(_) => return Inside::Err("missing"),
        }
    }
    Inside::Ok { path: real, exists }
}

pub fn mtime_of(file: &str) -> Option<f64> {
    let m = std::fs::metadata(file).ok()?;
    if !m.is_file() {
        return None;
    }
    Some(crate::util::mtime_ms(&m).round())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mcp-paths-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.txt"), "x").unwrap();
        PathBuf::from(real_of(&path_str(&d)).unwrap())
    }

    #[test]
    fn inside_outside_missing_folder() {
        let d = tmp("basic");
        let root = path_str(&d);
        let v = |s: &str| serde_json::json!(s);
        assert_eq!(resolve_inside(&root, Some(&v("src/a.txt")), false), Inside::Ok { path: path_str(&d.join("src").join("a.txt")), exists: true });
        assert_eq!(resolve_inside(&root, Some(&v("../x")), false), Inside::Err("outside"));
        assert_eq!(resolve_inside(&root, Some(&v("src/none.txt")), false), Inside::Err("missing"));
        assert_eq!(resolve_inside(&root, Some(&v("src/new/b.txt")), true), Inside::Ok { path: path_str(&d.join("src").join("new").join("b.txt")), exists: false });
        assert_eq!(resolve_inside(&root, Some(&v("src")), false), Inside::Err("folder"));
        assert_eq!(resolve_inside(&root, Some(&v("//host/share")), false), Inside::Err("path"));
        assert_eq!(resolve_inside(&root, None, false), Inside::Err("path"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn links_out_of_the_root_are_outside() {
        let d = tmp("link");
        std::os::unix::fs::symlink("/etc", d.join("out")).unwrap();
        assert_eq!(resolve_inside(&path_str(&d), Some(&serde_json::json!("out/hosts")), false), Inside::Err("outside"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn remote_links() {
        assert!(is_remote_link(r"\\server\share"));
        assert!(!is_remote_link(r"\\?\C:\x"));
        assert!(is_remote_link(r"\\?\UNC\server\share"));
    }
}
