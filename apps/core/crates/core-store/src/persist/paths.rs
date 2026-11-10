#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Win,
    Posix,
}

impl Style {
    pub const HOST: Style = if cfg!(windows) { Style::Win } else { Style::Posix };
}

fn is_sep(c: char, style: Style) -> bool {
    c == '/' || (style == Style::Win && c == '\\')
}

fn drive_of(p: &str) -> Option<char> {
    let mut it = p.chars();
    match (it.next(), it.next()) {
        (Some(d), Some(':')) if d.is_ascii_alphabetic() => Some(d),
        _ => None,
    }
}

pub fn is_absolute(p: &str, style: Style) -> bool {
    match style {
        Style::Posix => p.starts_with('/'),
        Style::Win => {
            if p.starts_with(['/', '\\']) {
                return true;
            }
            drive_of(p).is_some() && p[2..].starts_with(['/', '\\'])
        }
    }
}

fn split_root(p: &str, style: Style) -> (String, &str) {
    if style == Style::Posix {
        return if p.starts_with('/') { ("/".into(), p.trim_start_matches('/')) } else { (String::new(), p) };
    }
    let b: Vec<char> = p.chars().take(2).collect();
    if b.len() == 2 && b.iter().all(|c| *c == '/' || *c == '\\') {
        let rest = &p[2..];
        let mut parts = rest.split(['/', '\\']).filter(|s| !s.is_empty());
        if let (Some(server), Some(share)) = (parts.next(), parts.next()) {
            let consumed = rest.find(share).map_or(0, |i| i + share.len());
            return (format!("\\\\{server}\\{share}\\"), rest[consumed..].trim_start_matches(['/', '\\']));
        }
    }
    if let Some(d) = drive_of(p) {
        let tail = &p[2..];
        return (format!("{d}:\\"), tail.trim_start_matches(['/', '\\']));
    }
    if p.starts_with(['/', '\\']) {
        return ("\\".into(), p.trim_start_matches(['/', '\\']));
    }
    (String::new(), p)
}

pub fn resolve(p: &str, style: Style, cwd: &str) -> String {
    let (mut root, mut rest) = split_root(p, style);
    let joined;
    if root.is_empty() || (style == Style::Win && root == "\\") {
        let (cwd_root, cwd_rest) = split_root(cwd, style);
        if root == "\\" {
            root = cwd_root;
            joined = rest.to_string();
        } else {
            root = cwd_root;
            joined = if rest.is_empty() { cwd_rest.to_string() } else { format!("{cwd_rest}/{rest}") };
        }
        rest = &joined;
    } else if style == Style::Win && drive_of(p).is_some() && !p[2..].starts_with(['/', '\\']) {
        let (cwd_root, cwd_rest) = split_root(cwd, style);
        if cwd_root.chars().next().map(|c| c.to_ascii_lowercase()) == drive_of(p).map(|c| c.to_ascii_lowercase()) {
            root = cwd_root;
            joined = if rest.is_empty() { cwd_rest.to_string() } else { format!("{cwd_rest}/{rest}") };
            rest = &joined;
        }
    }
    let mut segs: Vec<&str> = Vec::new();
    for s in rest.split(|c| is_sep(c, style)) {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            other => segs.push(other),
        }
    }
    let sep = if style == Style::Win { "\\" } else { "/" };
    format!("{root}{}", segs.join(sep))
}

pub fn basename(p: &str, style: Style) -> String {
    let start = if style == Style::Win && drive_of(p).is_some() { 2 } else { 0 };
    let t = p[start..].trim_end_matches(|c| is_sep(c, style));
    t.rsplit(|c| is_sep(c, style)).next().unwrap_or("").to_string()
}

pub fn cwd_string() -> String {
    std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| if cfg!(windows) { "C:\\".into() } else { "/".into() })
}

pub fn win_join(parts: &[&str]) -> String {
    let joined = parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join("\\");
    let unc = joined.starts_with(['\\', '/']) && joined.chars().nth(1).is_some_and(|c| c == '\\' || c == '/');
    let (root, rest) = match drive_of(&joined) {
        Some(d) => (format!("{d}:"), &joined[2..]),
        None => (String::new(), joined.as_str()),
    };
    let absolute = rest.starts_with(['\\', '/']);
    let mut segs: Vec<&str> = Vec::new();
    for s in rest.split(['\\', '/']) {
        match s {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|l| *l != "..") {
                    segs.pop();
                } else if !absolute {
                    segs.push("..");
                }
            }
            other => segs.push(other),
        }
    }
    let lead = if unc { "\\\\" } else if absolute { "\\" } else { "" };
    format!("{root}{lead}{}", segs.join("\\"))
}

pub fn posix_join(parts: &[&str]) -> String {
    let joined = parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join("/");
    let absolute = joined.starts_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for s in joined.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|l| *l != "..") {
                    segs.pop();
                } else if !absolute {
                    segs.push("..");
                }
            }
            other => segs.push(other),
        }
    }
    format!("{}{}", if absolute { "/" } else { "" }, segs.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_like_node_path() {
        assert_eq!(resolve("C:/Work/../App/", Style::Win, "D:\\x"), "C:\\App");
        assert_eq!(resolve("/Work/App", Style::Win, "D:\\x\\y"), "D:\\Work\\App");
        assert_eq!(resolve("a\\b", Style::Win, "D:\\x"), "D:\\x\\a\\b");
        assert_eq!(resolve("C:\\", Style::Win, "D:\\x"), "C:\\");
        assert_eq!(resolve("\\\\srv\\share\\a\\..\\b", Style::Win, "D:\\x"), "\\\\srv\\share\\b");
        assert_eq!(resolve("/a/./b//c/..", Style::Posix, "/cwd"), "/a/b");
        assert_eq!(resolve("rel", Style::Posix, "/cwd"), "/cwd/rel");
    }

    #[test]
    fn absolute_and_basename() {
        assert!(is_absolute("C:\\a", Style::Win) && is_absolute("C:/a", Style::Win) && is_absolute("/a", Style::Win));
        assert!(!is_absolute("C:a", Style::Win) && !is_absolute("a", Style::Win) && !is_absolute("C:\\a", Style::Posix));
        assert_eq!(basename("C:\\work\\app\\", Style::Win), "app");
        assert_eq!(basename("/work/app/", Style::Posix), "app");
        assert_eq!(basename("C:\\", Style::Win), "");
    }

    #[test]
    fn joins_like_node_path() {
        assert_eq!(win_join(&["C:\\Users\\a\\AppData\\Local", "ccm"]), "C:\\Users\\a\\AppData\\Local\\ccm");
        assert_eq!(win_join(&["C:/x//y/", "ccm"]), "C:\\x\\y\\ccm");
        assert_eq!(posix_join(&["/Users/a", "Library", "Application Support", "ccm"]), "/Users/a/Library/Application Support/ccm");
    }
}
