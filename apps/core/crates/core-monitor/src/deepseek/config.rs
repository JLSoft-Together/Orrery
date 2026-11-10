use std::path::{Path, PathBuf};

use core_transcript::jsstr::{js_space, js_trim};

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '"' | '-')
}

fn toml_string(raw: &str) -> Option<String> {
    let v = js_trim(raw);
    if let Some(rest) = v.strip_prefix('\'') {
        return rest.find('\'').map(|end| rest[..end].to_owned());
    }
    let rest = v.strip_prefix('"')?;
    let mut chars = rest.char_indices();
    let end = loop {
        match chars.next()? {
            (i, '"') => break i,
            (_, '\\') => {
                chars.next()?;
            }
            _ => {}
        }
    };
    serde_json::from_str::<String>(&format!("\"{}\"", &rest[..end])).ok()
}

fn header_table(line: &str) -> Option<String> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    let inner = &rest[..close];
    let after = js_trim(&rest[close + 1..]);
    if inner.is_empty() || !(after.is_empty() || after.starts_with('#')) {
        return None;
    }
    Some(inner.chars().filter(|c| !js_space(*c)).collect())
}

fn key_value(line: &str) -> Option<(String, &str)> {
    let end = line.find(|c: char| !is_key_char(c)).unwrap_or(line.len());
    if end == 0 {
        return None;
    }
    let rest = line[end..].trim_start_matches(js_space).strip_prefix('=')?;
    let value = rest.trim_start_matches(js_space);
    (!value.is_empty()).then(|| (line[..end].replace('"', ""), value))
}

fn normalize(path: &str, win: bool) -> String {
    let sep = if win { '\\' } else { '/' };
    let unified = if win { path.replace('/', "\\") } else { path.to_owned() };
    let (root, rest) = if win && unified.len() >= 3 && unified.as_bytes()[1] == b':' && unified.as_bytes()[2] == b'\\' {
        (unified[..3].to_owned(), &unified[3..])
    } else if unified.starts_with(sep) {
        (sep.to_string(), &unified[1..])
    } else {
        (String::new(), &unified[..])
    };
    let mut stack: Vec<&str> = Vec::new();
    for part in rest.split(sep) {
        match part {
            "" | "." => {}
            ".." => {
                if stack.last().is_some_and(|l| *l != "..") {
                    stack.pop();
                } else if root.is_empty() {
                    stack.push("..");
                }
            }
            p => stack.push(p),
        }
    }
    format!("{root}{}", stack.join(&sep.to_string()))
}

fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/') || p.starts_with('\\') || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'/' | b'\\'))
}

pub fn codewhale_home(env_home: Option<String>, exists: impl Fn(&Path) -> bool, home: &Path) -> PathBuf {
    if let Some(h) = env_home.filter(|h| !h.is_empty()) {
        return PathBuf::from(h);
    }
    let main = home.join(".codewhale");
    let legacy = home.join(".deepseek");
    if !exists(&main) && exists(&legacy) {
        legacy
    } else {
        main
    }
}

pub fn outbox_path_from(toml: Option<&str>, codewhale_dir: &Path, home: &Path) -> Option<PathBuf> {
    let toml = toml.filter(|t| !t.is_empty())?;
    let mut table = String::new();
    let mut found: Option<String> = None;
    for raw in toml.split('\n') {
        let line = js_trim(raw);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(t) = header_table(line) {
            table = t;
            continue;
        }
        let Some((key, value)) = key_value(line) else { continue };
        let full = if table.is_empty() { key } else { format!("{table}.{key}") };
        if full == "lifecycle_outbox.path" {
            found = toml_string(value);
        }
    }
    let mut found = found.filter(|f| !f.is_empty() && !f.chars().any(|c| (c as u32) < 0x20))?;
    if found == "~" || found.starts_with("~/") || found.starts_with("~\\") {
        found = normalize(&format!("{}{}{}", home.to_string_lossy(), if cfg!(windows) { "\\" } else { "/" }, &found[1..]), cfg!(windows));
    }
    let win = cfg!(windows);
    Some(PathBuf::from(if is_absolute(&found) { normalize(&found, win) } else { normalize(&format!("{}{}{}", codewhale_dir.to_string_lossy(), if win { '\\' } else { '/' }, found), win) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(toml: &str) -> Option<String> {
        let home = if cfg!(windows) { "C:\\Users\\me" } else { "/home/me" };
        let dir = if cfg!(windows) { "C:\\cw" } else { "/cw" };
        outbox_path_from(Some(toml), Path::new(dir), Path::new(home)).map(|x| x.to_string_lossy().into_owned())
    }

    #[test]
    fn reads_only_the_lifecycle_outbox_path() {
        let abs = if cfg!(windows) { "C:\\data\\out.jsonl" } else { "/data/out.jsonl" };
        let toml = format!("[other]\npath = \"x\"\n[lifecycle_outbox]\n# note\npath = '{abs}'\n");
        assert_eq!(p(&toml).as_deref(), Some(abs));
        assert_eq!(p("lifecycle_outbox.path = \"rel/out.jsonl\"").map(|s| s.replace('\\', "/")), Some(if cfg!(windows) { "C:/cw/rel/out.jsonl" } else { "/cw/rel/out.jsonl" }.to_owned()));
        assert_eq!(p("[lifecycle_outbox]\nenabled = true\n"), None);
        assert_eq!(p("[lifecycle_outbox]\npath = 5\n"), None);
        assert_eq!(p("[lifecycle_outbox]\npath = \"~/o.jsonl\"").map(|s| s.replace('\\', "/")), Some(if cfg!(windows) { "C:/Users/me/o.jsonl" } else { "/home/me/o.jsonl" }.to_owned()));
        assert_eq!(outbox_path_from(None, Path::new("/cw"), Path::new("/h")), None);
    }

    #[test]
    fn decodes_escapes_in_basic_strings() {
        let toml = "[lifecycle_outbox]\npath = \"C:\\\\a\\\\b.jsonl\"\n";
        if cfg!(windows) {
            assert_eq!(p(toml).as_deref(), Some("C:\\a\\b.jsonl"));
        }
    }
}
