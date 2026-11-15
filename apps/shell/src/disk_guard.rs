use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plat {
    Win,
    Mac,
    Linux,
}

impl Plat {
    pub fn host() -> Plat {
        if cfg!(windows) {
            Plat::Win
        } else if cfg!(target_os = "macos") {
            Plat::Mac
        } else {
            Plat::Linux
        }
    }

    fn sep(self) -> char {
        if self == Plat::Win {
            '\\'
        } else {
            '/'
        }
    }
}

pub const MAX_PATH_CHARS: usize = 32_000;

const WIN_SUBTREE: [&str; 6] = ["Windows", "$Recycle.Bin", "System Volume Information", "Recovery", "Boot", "$WinREAgent"];
const WIN_SELF: [&str; 5] = ["Program Files", "Program Files (x86)", "ProgramData", "Users", "PerfLogs"];
const MAC_SUBTREE: [&str; 9] = ["/System", "/usr", "/bin", "/sbin", "/etc", "/private", "/dev", "/cores", "/Library/Apple"];
const MAC_SELF: [&str; 7] = ["/Library", "/Applications", "/Users", "/Volumes", "/opt", "/var", "/tmp"];
const UNIX_SUBTREE: [&str; 14] = ["/usr", "/bin", "/sbin", "/etc", "/boot", "/proc", "/sys", "/dev", "/lib", "/lib32", "/lib64", "/libx32", "/run", "/snap"];
const UNIX_SELF: [&str; 8] = ["/home", "/root", "/opt", "/var", "/tmp", "/srv", "/mnt", "/media"];
const HOME_SELF_WIN: [&str; 5] = ["AppData", "AppData\\Roaming", "AppData\\Local", "AppData\\LocalLow", ".config"];
const HOME_SELF_MAC: [&str; 2] = ["Library", ".config"];
const HOME_SELF_UNIX: [&str; 3] = [".config", ".local", ".local/share"];
const HOME_TREE_WIN: [&str; 2] = [".ssh", ".gnupg"];
const HOME_TREE_MAC: [&str; 5] = [".ssh", ".gnupg", "Library/Mobile Documents", "Library/CloudStorage", "Library/Keychains"];
const HOME_TREE_UNIX: [&str; 2] = [".ssh", ".gnupg"];

#[derive(Clone, Debug, Default)]
pub struct GuardContext {
    pub home: String,
    pub keep: Vec<String>,
    pub roots: Vec<String>,
    pub volumes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canonical {
    pub path: String,
    pub link: bool,
    pub missing: bool,
}

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

fn drive_at(b: &[u8]) -> bool {
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

pub fn is_absolute(p: &str, plat: Plat) -> bool {
    if p.is_empty() || p.chars().count() > MAX_PATH_CHARS || p.contains('\0') {
        return false;
    }
    if plat != Plat::Win {
        return p.starts_with('/');
    }
    let b = p.as_bytes();
    if drive_at(b) && b.len() >= 3 && (b[2] == b'\\' || b[2] == b'/') {
        return true;
    }
    let chars: Vec<char> = p.chars().collect();
    if chars.len() < 2 || !is_sep(chars[0]) || !is_sep(chars[1]) {
        return false;
    }
    let rest = &chars[2..];
    let Some(&first) = rest.first() else { return false };
    if is_sep(first) || first == '?' || first == '.' {
        return false;
    }
    let Some(i) = rest.iter().position(|c| is_sep(*c)) else { return false };
    rest.get(i + 1).is_some_and(|c| !is_sep(*c))
}

fn win_root(p: &str) -> (String, &str) {
    let b = p.as_bytes();
    if drive_at(b) {
        let letter = (b[0] as char).to_ascii_uppercase();
        let rest = &p[2..];
        if rest.starts_with(is_sep) {
            return (format!("{letter}:\\"), &rest[1..]);
        }
        return (format!("{letter}:"), rest);
    }
    if p.len() >= 2 && p.starts_with(is_sep) && p[1..].starts_with(is_sep) {
        let body = &p[2..];
        let mut parts = body.splitn(3, is_sep);
        let a = parts.next().unwrap_or("");
        let c = parts.next();
        let rest = parts.next().unwrap_or("");
        if !a.is_empty() {
            if let Some(c) = c.filter(|c| !c.is_empty()) {
                return (format!("\\\\{a}\\{c}\\"), rest);
            }
        }
        return ("\\".into(), body);
    }
    if p.starts_with(is_sep) {
        return ("\\".into(), &p[1..]);
    }
    (String::new(), p)
}

fn split_root(p: &str, plat: Plat) -> (String, &str) {
    if plat == Plat::Win {
        return win_root(p);
    }
    if let Some(rest) = p.strip_prefix('/') {
        return ("/".into(), rest);
    }
    (String::new(), p)
}

pub fn root_of(p: &str, plat: Plat) -> String {
    split_root(p, plat).0
}

pub fn normalize(p: &str, plat: Plat) -> String {
    let (root, rest) = split_root(p, plat);
    let mut segs: Vec<&str> = Vec::new();
    let split: Box<dyn Iterator<Item = &str>> = if plat == Plat::Win { Box::new(rest.split(is_sep)) } else { Box::new(rest.split('/')) };
    for s in split {
        match s {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|l| *l != "..") {
                    segs.pop();
                } else if root.is_empty() {
                    segs.push("..");
                }
            }
            other => segs.push(other),
        }
    }
    let body = segs.join(&plat.sep().to_string());
    if root.is_empty() && body.is_empty() {
        return ".".into();
    }
    format!("{root}{body}")
}

fn key_of(p: &str, plat: Plat) -> String {
    let n = normalize(p, plat);
    if plat == Plat::Linux {
        n
    } else {
        n.to_lowercase()
    }
}

pub fn join(base: &str, child: &str, plat: Plat) -> String {
    normalize(&format!("{base}{}{child}", plat.sep()), plat)
}

pub fn parent(n: &str, plat: Plat) -> String {
    let (root, rest) = split_root(n, plat);
    let trimmed = rest.trim_end_matches(|c| if plat == Plat::Win { is_sep(c) } else { c == '/' });
    match trimmed.rfind(|c| if plat == Plat::Win { is_sep(c) } else { c == '/' }) {
        Some(i) => format!("{root}{}", &trimmed[..i]),
        None if !root.is_empty() => root,
        None => ".".into(),
    }
}

pub fn base_name(n: &str, plat: Plat) -> String {
    let (_, rest) = split_root(n, plat);
    rest.split(|c| if plat == Plat::Win { is_sep(c) } else { c == '/' }).filter(|s| !s.is_empty()).last().unwrap_or("").to_owned()
}

pub fn is_under(child: &str, base: &str, plat: Plat) -> bool {
    let c = key_of(child, plat);
    let b = key_of(base, plat);
    if c == b {
        return true;
    }
    let sep = plat.sep();
    let prefix = if b.ends_with(sep) { b } else { format!("{b}{sep}") };
    c.starts_with(&prefix)
}

fn is_volume_root(n: &str, plat: Plat) -> bool {
    if plat != Plat::Win {
        return n == "/";
    }
    let b = n.as_bytes();
    if drive_at(b) && (b.len() == 2 || (b.len() == 3 && b[2] == b'\\')) {
        return true;
    }
    root_of(n, plat).trim_end_matches('\\').to_lowercase() == n.trim_end_matches('\\').to_lowercase()
}

fn is_drive_root(root: &str) -> bool {
    let b = root.as_bytes();
    b.len() == 3 && drive_at(b) && b[2] == b'\\'
}

fn system_reason(n: &str, plat: Plat) -> bool {
    if plat == Plat::Win {
        let root = root_of(n, plat);
        if !is_drive_root(&root) {
            return false;
        }
        if WIN_SUBTREE.iter().any(|d| is_under(n, &format!("{root}{d}"), plat)) {
            return true;
        }
        return WIN_SELF.iter().any(|d| key_of(n, plat) == key_of(&format!("{root}{d}"), plat));
    }
    let (sub, own): (&[&str], &[&str]) = if plat == Plat::Mac { (&MAC_SUBTREE, &MAC_SELF) } else { (&UNIX_SUBTREE, &UNIX_SELF) };
    if sub.iter().any(|d| is_under(n, d, plat)) {
        return true;
    }
    own.iter().any(|d| key_of(n, plat) == key_of(d, plat))
}

fn segments(n: &str, plat: Plat) -> Vec<String> {
    let (_, rest) = split_root(n, plat);
    rest.split(|c| if plat == Plat::Win { is_sep(c) } else { c == '/' }).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

fn bad_segment(s: &str) -> bool {
    s.ends_with('.') || s.ends_with(' ') || s.chars().any(|c| matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

fn win_lexical_bad(n: &str) -> bool {
    segments(n, Plat::Win).iter().any(|s| bad_segment(s))
}

pub fn short_name(s: &str) -> bool {
    let Some(tilde) = s.find('~') else { return false };
    let head = &s[..tilde];
    let n = head.chars().count();
    if !(1..=6).contains(&n) {
        return false;
    }
    let tail = &s[tilde + 1..];
    let digits = tail.chars().take_while(char::is_ascii_digit).count();
    if !(1..=6).contains(&digits) {
        return false;
    }
    let ext = &tail[digits..];
    if ext.is_empty() {
        return true;
    }
    let Some(e) = ext.strip_prefix('.') else { return false };
    !e.contains('.') && e.chars().count() <= 3
}

fn aliases(n: &str, plat: Plat) -> Vec<String> {
    if plat != Plat::Win {
        return vec![n.to_owned()];
    }
    let b = n.as_bytes();
    if b.len() >= 2 && b[0] == b'\\' && b[1] == b'\\' {
        let body = &n[2..];
        let mut parts = body.splitn(3, '\\');
        let host = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        let sb = share.as_bytes();
        if !host.is_empty() && sb.len() == 2 && sb[0].is_ascii_alphabetic() && sb[1] == b'$' {
            let letter = (sb[0] as char).to_ascii_uppercase();
            return vec![n.to_owned(), normalize(&format!("{letter}:\\{rest}"), plat)];
        }
    }
    vec![n.to_owned()]
}

fn home_lists(plat: Plat) -> (&'static [&'static str], &'static [&'static str]) {
    match plat {
        Plat::Win => (&HOME_SELF_WIN, &HOME_TREE_WIN),
        Plat::Mac => (&HOME_SELF_MAC, &HOME_TREE_MAC),
        Plat::Linux => (&HOME_SELF_UNIX, &HOME_TREE_UNIX),
    }
}

fn user_parents(n: &str, home: &str, plat: Plat) -> Vec<String> {
    let mut out = Vec::new();
    if !home.is_empty() {
        out.push(parent(home, plat));
    }
    match plat {
        Plat::Win => {
            let root = root_of(n, plat);
            if is_drive_root(&root) {
                out.push(format!("{root}Users"));
            }
        }
        Plat::Mac => out.extend(["/Users".to_owned(), "/Volumes".to_owned()]),
        Plat::Linux => out.extend(["/home".to_owned(), "/mnt".to_owned(), "/media".to_owned()]),
    }
    out
}

fn is_protected(n: &str, ctx: &GuardContext, plat: Plat) -> bool {
    let home = if ctx.home.is_empty() { String::new() } else { normalize(&ctx.home, plat) };
    let (own, tree) = home_lists(plat);
    let mut keep: Vec<String> = vec![home.clone()];
    keep.extend(ctx.keep.iter().cloned());
    keep.extend(ctx.volumes.iter().cloned());
    if !home.is_empty() {
        keep.extend(own.iter().map(|s| join(&home, s, plat)));
    }
    if keep.iter().any(|k| !k.is_empty() && is_under(k, n, plat)) {
        return true;
    }
    let trees: Vec<String> = if home.is_empty() { Vec::new() } else { tree.iter().map(|s| join(&home, s, plat)).collect() };
    if trees.iter().any(|k| !k.is_empty() && (is_under(n, k, plat) || is_under(k, n, plat))) {
        return true;
    }
    let up = parent(n, plat);
    if up != n && user_parents(n, &home, plat).iter().any(|u| key_of(u, plat) == key_of(&up, plat)) {
        return true;
    }
    if plat == Plat::Linux && key_of(&parent(&up, plat), plat) == "/media" {
        return true;
    }
    !home.is_empty() && key_of(&up, plat) == key_of(&home, plat) && base_name(n, plat).to_lowercase().starts_with("onedrive")
}

fn verdict(n: &str, ctx: &GuardContext, plat: Plat, need_root: bool) -> Option<&'static str> {
    for a in aliases(n, plat) {
        if is_volume_root(&a, plat) {
            return Some("root");
        }
        if system_reason(&a, plat) {
            return Some("system");
        }
        if is_protected(&a, ctx, plat) {
            return Some("protected");
        }
    }
    if need_root && !ctx.roots.iter().any(|r| is_under(n, r, plat)) {
        return Some("outside");
    }
    None
}

pub fn check_deletable(p: &str, ctx: &GuardContext, plat: Plat) -> Option<&'static str> {
    if p.is_empty() || p.contains('\0') || p.chars().count() > MAX_PATH_CHARS {
        return Some("invalid");
    }
    if !is_absolute(p, plat) {
        return Some("relative");
    }
    let n = normalize(p, plat);
    if plat == Plat::Win && win_lexical_bad(&n) {
        return Some("invalid");
    }
    verdict(&n, ctx, plat, true)
}

pub fn plain_canonical(raw: &str, plat: Plat) -> Option<String> {
    if plat != Plat::Win {
        return Some(raw.to_owned());
    }
    let b = raw.as_bytes();
    let prefixed = b.len() >= 4 && is_sep(b[0] as char) && is_sep(b[1] as char) && (b[2] == b'?' || b[2] == b'.') && is_sep(b[3] as char);
    if !prefixed {
        return Some(raw.to_owned());
    }
    let rest = &raw[4..];
    let rb = rest.as_bytes();
    if drive_at(rb) && rb.len() >= 3 && is_sep(rb[2] as char) {
        return Some(rest.to_owned());
    }
    if rest.len() >= 4 && rest[..3].eq_ignore_ascii_case("UNC") && is_sep(rb[3] as char) {
        return Some(format!("\\\\{}", &rest[4..]));
    }
    None
}

pub trait CanonicalIo {
    fn is_link(&self, p: &str) -> std::io::Result<bool>;
    fn realpath(&self, p: &str) -> std::io::Result<String>;
}

pub struct RealIo;

impl CanonicalIo for RealIo {
    fn is_link(&self, p: &str) -> std::io::Result<bool> {
        std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink())
    }

    fn realpath(&self, p: &str) -> std::io::Result<String> {
        std::fs::canonicalize(Path::new(p)).map(|p| p.to_string_lossy().into_owned())
    }
}

fn not_found(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::NotFound
}

pub fn canonical_of(p: &str, plat: Plat, io: &dyn CanonicalIo) -> Option<Canonical> {
    let n = normalize(p, plat);
    let link = match io.is_link(&n) {
        Ok(l) => l,
        Err(e) if not_found(&e) => {
            return match io.realpath(&parent(&n, plat)) {
                Ok(up) => plain_canonical(&up, plat).map(|base| Canonical { path: normalize(&join(&base, &base_name(&n, plat), plat), plat), link: false, missing: true }),
                Err(e) if not_found(&e) => Some(Canonical { path: n, link: false, missing: true }),
                Err(_) => None,
            };
        }
        Err(_) => return None,
    };
    let raw = if link { join(&io.realpath(&parent(&n, plat)).ok()?, &base_name(&n, plat), plat) } else { io.realpath(&n).ok()? };
    let plain = plain_canonical(&raw, plat)?;
    Some(Canonical { path: normalize(&plain, plat), link, missing: false })
}

fn canonical_verdict(c: &Canonical, ctx: &GuardContext, plat: Plat) -> Option<&'static str> {
    if !is_absolute(&c.path, plat) {
        return Some("invalid");
    }
    if plat == Plat::Win {
        if win_lexical_bad(&c.path) {
            return Some("invalid");
        }
        let segs = segments(&c.path, plat);
        let checked = if c.link || c.missing { &segs[..segs.len().saturating_sub(1)] } else { &segs[..] };
        if checked.iter().any(|s| short_name(s)) {
            return Some("invalid");
        }
    }
    verdict(&c.path, ctx, plat, false)
}

pub fn resolve_deletable(p: &str, ctx: &GuardContext, plat: Plat, io: &dyn CanonicalIo) -> Result<Canonical, &'static str> {
    if let Some(reason) = check_deletable(p, ctx, plat) {
        return Err(reason);
    }
    let c = canonical_of(p, plat, io).ok_or("invalid")?;
    match canonical_verdict(&c, ctx, plat) {
        Some(reason) => Err(reason),
        None => Ok(c),
    }
}

pub fn same_canonical(a: &Canonical, b: &Canonical, plat: Plat) -> bool {
    a.link == b.link && a.missing == b.missing && key_of(&a.path, plat) == key_of(&b.path, plat)
}

pub fn top_level(paths: &[String], plat: Plat) -> Vec<String> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for p in paths {
        let k = key_of(p, plat);
        if !seen.iter().any(|(sk, _)| *sk == k) {
            seen.push((k, normalize(p, plat)));
        }
    }
    let list: Vec<String> = seen.into_iter().map(|(_, p)| p).collect();
    list.iter().filter(|p| !list.iter().any(|q| q != *p && key_of(q, plat) != key_of(p, plat) && is_under(p, q, plat))).cloned().collect()
}

pub fn path_key(p: &str, plat: Plat) -> String {
    let t = p.trim_end_matches(['\\', '/']);
    if plat == Plat::Linux {
        t.to_owned()
    } else {
        t.to_lowercase()
    }
}

#[cfg(test)]
#[path = "disk_guard_tests.rs"]
mod tests;
