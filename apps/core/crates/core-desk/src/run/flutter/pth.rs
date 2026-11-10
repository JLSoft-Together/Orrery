fn is_sep(c: char, win: bool) -> bool {
    c == '/' || (win && c == '\\')
}

fn split_root(p: &str, win: bool) -> (String, &str) {
    if !win {
        return if p.starts_with('/') { ("/".into(), p.trim_start_matches('/')) } else { (String::new(), p) };
    }
    let b = p.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        let rest = &p[2..];
        if rest.starts_with(['\\', '/']) {
            return (format!("{}\\", &p[..2]), rest.trim_start_matches(['\\', '/']));
        }
        return (p[..2].to_string(), rest);
    }
    if p.starts_with("\\\\") || p.starts_with("//") {
        let rest = &p[2..];
        let parts: Vec<&str> = rest.splitn(3, ['\\', '/']).collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            let root = format!("\\\\{}\\{}\\", parts[0], parts[1]);
            return (root, parts.get(2).copied().unwrap_or(""));
        }
    }
    if p.starts_with(['\\', '/']) {
        return ("\\".into(), p.trim_start_matches(['\\', '/']));
    }
    (String::new(), p)
}

pub fn is_absolute(p: &str, win: bool) -> bool {
    if !win {
        return p.starts_with('/');
    }
    let (root, _) = split_root(p, true);
    root.ends_with('\\')
}

pub fn normalize(p: &str, win: bool) -> String {
    if p.is_empty() {
        return ".".into();
    }
    let (root, rest) = split_root(p, win);
    let absolute = root.ends_with('/') || root.ends_with('\\');
    let trailing = rest.ends_with(|c| is_sep(c, win)) && !rest.is_empty();
    let mut out: Vec<&str> = Vec::new();
    for seg in rest.split(|c| is_sep(c, win)) {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|l| *l != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let sep = if win { "\\" } else { "/" };
    let mut body = out.join(sep);
    if body.is_empty() && !absolute {
        body = ".".into();
    }
    if trailing && !body.is_empty() {
        body.push_str(sep);
    }
    format!("{root}{body}")
}

pub fn join(parts: &[&str], win: bool) -> String {
    let joined: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    if joined.is_empty() {
        return ".".into();
    }
    normalize(&joined.join(if win { "\\" } else { "/" }), win)
}

fn strip_trailing(p: String, win: bool) -> String {
    let (root, _) = split_root(&p, win);
    let mut s = p;
    while s.len() > root.len() && s.ends_with(|c| is_sep(c, win)) {
        s.pop();
    }
    s
}

pub fn resolve(p: &str, win: bool) -> String {
    if is_absolute(p, win) {
        return strip_trailing(normalize(p, win), win);
    }
    let cwd = std::env::current_dir().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|_| if win { "C:\\".into() } else { "/".into() });
    let base = if is_absolute(&cwd, win) { cwd } else if win { "C:\\".into() } else { "/".into() };
    strip_trailing(join(&[&base, p], win), win)
}

pub fn dirname(p: &str, win: bool) -> String {
    let (root, rest) = split_root(p, win);
    let rest = rest.trim_end_matches(|c| is_sep(c, win));
    match rest.rfind(|c| is_sep(c, win)) {
        Some(i) => format!("{root}{}", &rest[..i]),
        None if !root.is_empty() => root,
        None => ".".into(),
    }
}

pub fn basename(p: &str, win: bool) -> String {
    let (_, rest) = split_root(p, win);
    let rest = rest.trim_end_matches(|c| is_sep(c, win));
    rest.rsplit(|c| is_sep(c, win)).next().unwrap_or("").to_string()
}

pub fn is_within(root: &str, target: &str, win: bool) -> bool {
    let fold = |s: String| if win { s.to_lowercase() } else { s };
    let r = fold(resolve(root, win));
    let t = fold(resolve(target, win));
    if r == t {
        return true;
    }
    let sep = if win { '\\' } else { '/' };
    let prefix = if r.ends_with(sep) { r } else { format!("{r}{sep}") };
    t.starts_with(&prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_and_win32() {
        assert_eq!(join(&["/a/", "b", "../c"], false), "/a/c");
        assert_eq!(normalize("/sdk/a/bin", false), "/sdk/a/bin");
        assert_eq!(dirname("/sdk/a/bin", false), "/sdk/a");
        assert_eq!(dirname("/sdk", false), "/");
        assert_eq!(basename("C:\\src\\flutter\\bin\\flutter.bat", true), "flutter.bat");
        assert_eq!(join(&["C:\\src\\flutter", "bin", "flutter.bat"], true), "C:\\src\\flutter\\bin\\flutter.bat");
        assert_eq!(normalize("C:/a//b/./c/..", true), "C:\\a\\b");
        assert_eq!(dirname("C:\\a", true), "C:\\");
        assert!(is_absolute("C:\\x", true) && !is_absolute("relative", true) && !is_absolute("C:x", true));
        assert_eq!(resolve("/p/", false), "/p");
        assert_eq!(resolve("C:\\P\\x\\", true), "C:\\P\\x");
        assert!(is_within("/repo", "/repo/tools/flutter", false));
        assert!(!is_within("/repo", "/repository", false));
        assert!(is_within("C:\\Repo", "c:\\repo\\x", true));
    }
}
