pub mod diff;
pub mod mask;
pub mod proc;

use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const BACKUP_SUFFIX: &str = ".ccm-backup";

pub type Obj = Map<String, Value>;

#[derive(Debug, Clone)]
pub struct Sys {
    pub home: String,
    pub config_dir: Option<String>,
    pub win: bool,
}

impl Sys {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let home = if cfg!(windows) { var("USERPROFILE").or_else(|| var("HOME")) } else { var("HOME") };
        Sys { home: home.unwrap_or_default(), config_dir: var("CLAUDE_CONFIG_DIR"), win: cfg!(windows) }
    }

    pub fn platform(&self) -> &'static str {
        if self.win {
            "win32"
        } else {
            "darwin"
        }
    }

    pub fn claude_root(&self) -> PathBuf {
        match &self.config_dir {
            Some(d) => PathBuf::from(d),
            None => Path::new(&self.home).join(".claude"),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.claude_root().join("settings.json")
    }

    pub fn claude_json(&self) -> PathBuf {
        match &self.config_dir {
            Some(d) => Path::new(d).join(".claude.json"),
            None => Path::new(&self.home).join(".claude.json"),
        }
    }
}

pub fn is_obj(v: &Value) -> bool {
    v.is_object()
}

pub fn strip_verbatim(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
}

pub fn real(p: &str) -> Option<String> {
    std::fs::canonicalize(p).ok().map(|r| strip_verbatim(&r))
}

pub fn fold(p: &str, win: bool) -> String {
    if win {
        p.to_lowercase()
    } else {
        p.to_string()
    }
}

pub fn resolve(p: &str) -> String {
    crate::util::path_str(&crate::util::resolve(p))
}

pub fn join(base: &str, parts: &[&str]) -> String {
    let mut p = PathBuf::from(base);
    for part in parts {
        p.push(part);
    }
    let s = crate::util::path_str(&p);
    if is_absolute(&s) {
        resolve(&s)
    } else if cfg!(windows) {
        s.replace('/', "\\")
    } else {
        s
    }
}

pub fn is_within(root: &str, target: &str, win: bool) -> bool {
    let r = fold(&resolve(root), win);
    let t = fold(&resolve(target), win);
    if t == r {
        return true;
    }
    let sep = std::path::MAIN_SEPARATOR;
    let prefix = if r.ends_with(sep) { r } else { format!("{r}{sep}") };
    t.starts_with(&prefix)
}

pub fn same_dir(a: &str, b: &str, win: bool) -> bool {
    let norm = |p: &str| fold(p.replace('\\', "/").trim_end_matches('/'), win);
    norm(a) == norm(b)
}

pub fn norm_file(file: &str, win: bool) -> String {
    fold(&file.replace('\\', "/"), win)
}

pub fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    if cfg!(windows) {
        matches!(b.first(), Some(b'/' | b'\\')) || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'/' | b'\\'))
    } else {
        b.first() == Some(&b'/')
    }
}

pub fn dirname(p: &str) -> String {
    let path = Path::new(p);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().to_string(),
        Some(_) => ".".into(),
        None => p.to_string(),
    }
}

pub fn is_root(p: &str) -> bool {
    Path::new(p).parent().is_none()
}

pub enum DirArg {
    None,
    Dir(String),
    Bad,
}

pub fn check_dir(v: &Value) -> DirArg {
    let s = match v {
        Value::Null => return DirArg::None,
        Value::String(s) if s.is_empty() => return DirArg::None,
        Value::String(s) => s,
        _ => return DirArg::Bad,
    };
    let b = s.as_bytes();
    if core_transcript::jsstr::units(s) > 1024 || !is_absolute(s) || (b.len() >= 2 && matches!(b[0], b'/' | b'\\') && matches!(b[1], b'/' | b'\\')) || is_root(s) {
        return DirArg::Bad;
    }
    match real(s) {
        Some(r) if !is_root(&r) && Path::new(&r).is_dir() => DirArg::Dir(r),
        _ => DirArg::Bad,
    }
}

pub fn js_trim(s: &str) -> &str {
    core_transcript::jsstr::js_trim(s)
}

pub fn tail(s: &str, max: usize) -> &str {
    let total = core_transcript::jsstr::units(s);
    if total <= max {
        return s;
    }
    let mut skip = total - max;
    for (i, c) in s.char_indices() {
        if skip == 0 {
            return &s[i..];
        }
        skip = skip.saturating_sub(c.len_utf16());
    }
    ""
}

pub fn parse_text(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

pub fn write_atomic(file: &Path, text: &str, backup: bool) -> bool {
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".ccm-tmp");
    let tmp = PathBuf::from(tmp);
    let res = (|| -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if backup {
            let mut b = file.as_os_str().to_owned();
            b.push(BACKUP_SUFFIX);
            std::fs::copy(file, PathBuf::from(b))?;
        }
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, file)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res.is_ok()
}

pub fn write_with_backup(file: &Path, text: &str) -> bool {
    write_atomic(file, text, file.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_and_tails() {
        assert!(same_dir("C:\\a\\B\\", "c:/a/b", true));
        assert!(!same_dir("/a/B", "/a/b", false));
        assert_eq!(tail("abcdef", 3), "def");
        assert_eq!(tail("ab", 3), "ab");
        assert_eq!(norm_file("C:\\X\\s.json", true), "c:/x/s.json");
    }
}
