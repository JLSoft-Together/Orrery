fn is_sep(c: char, win: bool) -> bool {
    c == '/' || (win && c == '\\')
}

fn split_prefix(p: &str, win: bool) -> (String, bool, &str) {
    if !win {
        return if let Some(rest) = p.strip_prefix('/') { (String::new(), true, rest) } else { (String::new(), false, p) };
    }
    let b = p.as_bytes();
    if b.len() >= 2 && is_sep(b[0] as char, true) && is_sep(b[1] as char, true) {
        let rest = &p[2..];
        let mut it = rest.splitn(3, ['/', '\\']);
        if let (Some(server), Some(share)) = (it.next(), it.next()) {
            if !server.is_empty() && !share.is_empty() {
                let tail = it.next().unwrap_or("");
                return (format!("\\\\{server}\\{share}"), true, tail);
            }
        }
    }
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        let rest = &p[2..];
        let abs = rest.starts_with(['/', '\\']);
        return (p[..2].to_string(), abs, rest);
    }
    if p.starts_with(['/', '\\']) {
        return (String::new(), true, &p[1..]);
    }
    (String::new(), false, p)
}

pub fn normalize(p: &str, win: bool) -> String {
    if p.is_empty() {
        return ".".into();
    }
    let (prefix, abs, rest) = split_prefix(p, win);
    let trailing = rest.ends_with(|c| is_sep(c, win)) && !rest.is_empty();
    let mut segs: Vec<&str> = Vec::new();
    for s in rest.split(|c| is_sep(c, win)) {
        match s {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|l| *l != "..") {
                    segs.pop();
                } else if !abs {
                    segs.push("..");
                }
            }
            other => segs.push(other),
        }
    }
    let sep = if win { "\\" } else { "/" };
    let mut body = segs.join(sep);
    if trailing && !body.is_empty() {
        body.push_str(sep);
    }
    let root = if abs { sep } else { "" };
    let out = format!("{prefix}{root}{body}");
    if out.is_empty() {
        ".".into()
    } else {
        out
    }
}

pub fn join(win: bool, parts: &[&str]) -> String {
    let sep = if win { "\\" } else { "/" };
    let joined = parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(sep);
    normalize(&joined, win)
}

pub fn is_absolute(p: &str, win: bool) -> bool {
    if !win {
        return p.starts_with('/');
    }
    let b = p.as_bytes();
    p.starts_with(['/', '\\']) || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && is_sep(b[2] as char, true))
}

pub fn resolve(base: &str, rel: &str, win: bool) -> String {
    let full = if is_absolute(rel, win) { normalize(rel, win) } else { join(win, &[base, rel]) };
    let (prefix, abs, _) = split_prefix(&full, win);
    let keep = prefix.len() + usize::from(abs);
    let mut out = full;
    while out.len() > keep && out.ends_with(|c| is_sep(c, win)) {
        out.pop();
    }
    out
}

pub fn basename(p: &str, win: bool) -> String {
    let (_, _, rest) = split_prefix(p, win);
    let t = rest.trim_end_matches(|c| is_sep(c, win));
    t.rsplit(|c| is_sep(c, win)).next().unwrap_or("").to_string()
}

pub fn dirname(p: &str, win: bool) -> String {
    let (prefix, abs, rest) = split_prefix(p, win);
    let t = rest.trim_end_matches(|c| is_sep(c, win));
    let sep = if win { "\\" } else { "/" };
    let root = format!("{prefix}{}", if abs { sep } else { "" });
    match t.rfind(|c| is_sep(c, win)) {
        Some(i) => {
            let head = t[..i].trim_end_matches(|c| is_sep(c, win));
            if head.is_empty() {
                root
            } else {
                format!("{root}{head}")
            }
        }
        None if root.is_empty() => ".".into(),
        None => root,
    }
}

pub fn is_within(root: &str, target: &str, win: bool) -> bool {
    let fold = |s: String| if win { s.to_lowercase() } else { s };
    let r = fold(resolve("/", root, win));
    let t = fold(resolve("/", target, win));
    if r == t {
        return true;
    }
    let sep = if win { "\\" } else { "/" };
    let prefix = if r.ends_with(sep) { r } else { format!("{r}{sep}") };
    t.starts_with(&prefix)
}

pub fn host_win() -> bool {
    cfg!(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_and_win() {
        assert_eq!(join(false, &["/a", "b/../c", "d"]), "/a/c/d");
        assert_eq!(join(true, &["C:\\p", "gradle", "x.properties"]), "C:\\p\\gradle\\x.properties");
        assert_eq!(join(true, &["C:\\Program Files/Java", "jdk"]), "C:\\Program Files\\Java\\jdk");
        assert_eq!(resolve("/work/app", "../sdk", false), "/work/sdk");
        assert_eq!(resolve("/work/app", "/opt/jdk/", false), "/opt/jdk");
        assert_eq!(resolve("C:\\proj", "D:\\Android\\Sdk", true), "D:\\Android\\Sdk");
        assert_eq!(dirname("/usr/lib/jvm/bin/java", false), "/usr/lib/jvm/bin");
        assert_eq!(dirname("/a", false), "/");
        assert_eq!(dirname("C:\\a", true), "C:\\");
        assert_eq!(basename("/x/zulu-18.jdk/", false), "zulu-18.jdk");
        assert_eq!(basename("C:\\x\\jbr", true), "jbr");
        assert!(is_within("/work/app", "/work/app/tools/java", false));
        assert!(!is_within("/work/app", "/work/apple", false));
        assert!(is_within("C:\\Work", "c:\\work\\x", true));
        assert!(is_absolute("C:/x", true) && !is_absolute("x", true) && !is_absolute("C:\\x", false));
    }
}
