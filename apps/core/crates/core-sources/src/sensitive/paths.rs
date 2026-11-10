use core_transcript::jsstr::units;

fn utf16_slice_from(s: &str, start: usize) -> String {
    let u: Vec<u16> = s.encode_utf16().collect();
    String::from_utf16_lossy(&u[start.min(u.len())..])
}

fn utf16_tail(s: &str, n: usize) -> String {
    let u: Vec<u16> = s.encode_utf16().collect();
    String::from_utf16_lossy(&u[u.len().saturating_sub(n)..])
}

pub fn slice_from(s: &str, start: usize) -> String {
    utf16_slice_from(s, start)
}

pub fn norm_path(p: &str, win: bool) -> String {
    let mut s: Vec<char> = p.chars().map(|c| if c == '\\' { '/' } else { c }).collect();
    if win {
        if s.len() >= 2 && s[0] == '/' && s[1].is_ascii_alphabetic() && (s.len() == 2 || s[2] == '/') {
            let consumed = if s.len() == 2 { 2 } else { 3 };
            let mut next = vec![s[1], ':', '/'];
            next.extend_from_slice(&s[consumed..]);
            s = next;
        }
        if s.len() == 2 && s[0].is_ascii_alphabetic() && s[1] == ':' {
            s.push('/');
        }
    }
    let drive = win && s.len() >= 3 && s[0].is_ascii_alphabetic() && s[1] == ':' && s[2] == '/';
    let absolute = drive || s.first() == Some(&'/');
    let skip = if drive {
        3
    } else if absolute {
        1
    } else {
        0
    };
    let rest: String = s[skip.min(s.len())..].iter().collect();
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if parts.last().is_some_and(|l| *l != "..") {
                parts.pop();
            } else if !absolute {
                parts.push("..");
            }
        } else {
            parts.push(part);
        }
    }
    let prefix = if drive {
        format!("{}:/", s[0].to_ascii_lowercase())
    } else if absolute {
        "/".to_owned()
    } else {
        String::new()
    };
    prefix + &parts.join("/")
}

fn is_abs(p: &str, win: bool) -> bool {
    let c: Vec<char> = p.chars().take(2).collect();
    p.starts_with('/') || (win && c.len() == 2 && c[0].is_ascii_alphabetic() && c[1] == ':') || (win && p.starts_with('\\'))
}

fn home_prefix_len(t: &str) -> Option<usize> {
    let forms = ["~", "$HOME", "${HOME}", "%USERPROFILE%", "$env:USERPROFILE", "$env:HOME"];
    let b = t.as_bytes();
    for f in forms {
        let n = f.len();
        if b.len() >= n && b[..n].eq_ignore_ascii_case(f.as_bytes()) && b.get(n).is_none_or(|c| *c == b'\\' || *c == b'/') {
            return Some(n);
        }
    }
    None
}

pub fn resolve_target(target: &str, base: Option<&str>, home: &str, win: bool) -> Option<String> {
    let mut t = match home_prefix_len(target) {
        Some(n) => format!("{home}{}", &target[n..]),
        None => target.to_owned(),
    };
    if t.contains(['$', '%', '`']) {
        return None;
    }
    if !is_abs(&t, win) {
        let base = base.filter(|b| !b.is_empty())?;
        t = format!("{base}/{t}");
    }
    Some(norm_path(&t, win))
}

pub fn same(a: &str, b: &str, win: bool) -> bool {
    if win {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

pub fn is_inside(child: &str, parent: &str, win: bool) -> bool {
    let (c, p) = if win { (child.to_lowercase(), parent.to_lowercase()) } else { (child.to_owned(), parent.to_owned()) };
    let p = p.strip_suffix('/').unwrap_or(&p);
    c == p || c.strip_prefix(p).is_some_and(|rest| rest.starts_with('/'))
}

pub fn is_root(p: &str) -> bool {
    let b = p.as_bytes();
    p == "/" || (b.len() >= 2 && b.len() <= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b.len() == 2 || b[2] == b'/'))
}

pub fn short_path(file: &str, project: Option<&str>, home: &str, win: bool) -> String {
    let f = norm_path(file, win);
    let proj = project.filter(|p| !p.is_empty()).map(|p| norm_path(p, win));
    let out = if let Some(p) = proj.as_ref().filter(|p| is_inside(&f, p, win)) {
        let rest = slice_from(&f, units(p));
        let rest = rest.strip_prefix('/').unwrap_or(&rest).to_owned();
        if rest.is_empty() {
            ".".to_owned()
        } else {
            rest
        }
    } else if !home.is_empty() && is_inside(&f, &norm_path(home, win), win) {
        format!("~{}", slice_from(&f, units(&norm_path(home, win))))
    } else {
        let parts: Vec<&str> = f.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() > 3 {
            format!("…/{}", parts[parts.len() - 3..].join("/"))
        } else {
            f.clone()
        }
    };
    if units(&out) > 160 {
        format!("…{}", utf16_tail(&out, 159))
    } else {
        out
    }
}

#[derive(Clone, Copy)]
enum Tok {
    Lit(char),
    AnyStar,
    Star,
    One,
}

fn fold(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

pub fn glob_matches(glob: &str, text: &str, ci: bool) -> bool {
    let g: Vec<char> = glob.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < g.len() {
        match g[i] {
            '*' if g.get(i + 1) == Some(&'*') => {
                toks.push(Tok::AnyStar);
                i += 1;
                if g.get(i + 1) == Some(&'/') {
                    i += 1;
                }
            }
            '*' => toks.push(Tok::Star),
            '?' => toks.push(Tok::One),
            c => toks.push(Tok::Lit(c)),
        }
        i += 1;
    }
    let t: Vec<char> = text.chars().collect();
    let line_end = |c: char| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}');
    let mut dp = vec![vec![false; t.len() + 1]; toks.len() + 1];
    dp[toks.len()][t.len()] = true;
    for ti in (0..toks.len()).rev() {
        for xi in (0..=t.len()).rev() {
            dp[ti][xi] = match toks[ti] {
                Tok::Lit(c) => xi < t.len() && (t[xi] == c || (ci && fold(t[xi]) == fold(c))) && dp[ti + 1][xi + 1],
                Tok::One => xi < t.len() && t[xi] != '/' && dp[ti + 1][xi + 1],
                Tok::Star => dp[ti + 1][xi] || (xi < t.len() && t[xi] != '/' && dp[ti][xi + 1]),
                Tok::AnyStar => dp[ti + 1][xi] || (xi < t.len() && !line_end(t[xi]) && dp[ti][xi + 1]),
            };
        }
    }
    dp[0][0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_and_git_bash_paths() {
        assert_eq!(norm_path("C:\\a\\..\\b\\.\\c\\", true), "c:/b/c");
        assert_eq!(norm_path("/d/x/y", true), "d:/x/y");
        assert_eq!(norm_path("/x", true), "x:/");
        assert_eq!(norm_path("C:", true), "c:/");
        assert_eq!(norm_path("../a/../../b", false), "../../b");
        assert_eq!(norm_path("/a/../..", false), "/");
        assert_eq!(resolve_target("..\\x", Some("F:/work/app"), "C:/Users/me", true).as_deref(), Some("f:/work/x"));
        assert_eq!(resolve_target("$TMP", Some("F:/work/app"), "C:/Users/me", true), None);
        assert_eq!(resolve_target("~/x", None, "/home/me", false).as_deref(), Some("/home/me/x"));
        assert_eq!(resolve_target("$HOME", None, "/home/me", false).as_deref(), Some("/home/me"));
        assert_eq!(resolve_target("$HOMEX", Some("/p"), "/home/me", false), None);
        assert_eq!(resolve_target("rel", None, "/h", false), None);
        assert_eq!(short_path("/var/lib/a/b/c/d", None, "/home/me", false), "…/b/c/d");
        assert_eq!(short_path("/p/x", Some("/p"), "/h", false), "x");
        assert_eq!(short_path("/p", Some("/p"), "/h", false), ".");
        assert_eq!(short_path("/h/.ssh/config", Some("/p"), "/h", false), "~/.ssh/config");
    }

    #[test]
    fn globs_follow_the_shared_regexp_rules() {
        assert!(glob_matches(".env*", ".ENV.local", true));
        assert!(!glob_matches(".env*", ".ENV.local", false));
        assert!(glob_matches("config/prod/**", "config/prod/db.yml", false));
        assert!(glob_matches("config/prod/**", "config/prod/a/b.yml", false));
        assert!(!glob_matches("config/*.yml", "config/a/b.yml", false));
        assert!(glob_matches("**/x.txt", "a/b/x.txt", false));
        assert!(glob_matches("**/x.txt", "x.txt", false));
        assert!(glob_matches("a?c", "abc", false));
        assert!(!glob_matches("a?c", "a/c", false));
        assert!(glob_matches("a.b+c", "a.b+c", false));
        assert!(!glob_matches("a.b", "axb", false));
        assert!(!glob_matches("a**", "a\nb", false));
        assert!(glob_matches("a*", "a\nb", false));
    }

    #[test]
    fn inside_root_and_same() {
        assert!(is_inside("f:/work/app/x", "F:/WORK/App", true));
        assert!(!is_inside("f:/work/app2", "f:/work/app", true));
        assert!(is_inside("/", "/", false));
        assert!(is_root("c:/") && is_root("c:") && is_root("/") && !is_root("c:/x"));
        assert!(same("A", "a", true) && !same("A", "a", false));
    }
}
