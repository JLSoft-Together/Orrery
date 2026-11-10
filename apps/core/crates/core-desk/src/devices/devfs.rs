use super::toggles::{is_package, sh_quote};
use crate::apk::analyze::locale_cmp;
use crate::js;
use crate::run::android::jsre::{groups, Js};
use serde_json::{json, Value};

pub const SDCARD_ROOTS: [&str; 2] = ["/sdcard", "/data/local/tmp"];
pub const PART_SUFFIX: &str = ".orrery-part";

#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    Sdcard,
    App(String),
}

pub fn clean_scope(v: &Value) -> Option<Scope> {
    let o = v.as_object()?;
    match o.get("kind")?.as_str()? {
        "sdcard" => Some(Scope::Sdcard),
        "app" => o.get("pkg").and_then(Value::as_str).filter(|p| is_package(p)).map(|p| Scope::App(p.to_string())),
        _ => None,
    }
}

pub struct Listing {
    pub entries: Vec<Value>,
    pub skipped: u64,
    pub error: Option<&'static str>,
}

pub fn parse_ls_la(out: &str) -> Listing {
    static MODE: Js = Js::new(r"^([-dlcbps])[-rwxsStTlL]{9}[.+@]?\s+", "");
    static ISO: Js = Js::new(r"\s(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2})(?::\d{2}(?:\.\d+)?)?(?:\s+[+-]\d{4})?\s", "");
    static BB: Js = Js::new(r"\s([A-Z][a-z]{2})\s+(\d{1,2})\s+(\d{2}:\d{2}|\d{4})\s", "");
    static TOTAL: Js = Js::new(r"^total\s+\d+", "");
    static DENIED: Js = Js::new(r"Permission denied", "i");
    static GONE: Js = Js::new(r"No such file", "i");
    static NOTDIR: Js = Js::new(r"Not a directory", "i");
    let mut rows: Vec<(bool, String, Value)> = Vec::new();
    let (mut skipped, mut error) = (0u64, None::<&'static str>);
    for raw in out.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.trim().is_empty() || TOTAL.test(line) {
            continue;
        }
        let Some(mode) = MODE.re().find(line) else {
            if DENIED.test(line) {
                error = error.or(Some("permission"));
            } else if GONE.test(line) {
                error = error.or(Some("not-found"));
            } else if NOTDIR.test(line) {
                error = error.or(Some("not-dir"));
            } else {
                skipped += 1;
            }
            continue;
        };
        let mg = groups(&mode, line);
        let iso = ISO.re().find(line);
        let bb = if iso.is_none() { BB.re().find(line) } else { None };
        let Some(at) = iso.as_ref().or(bb.as_ref()) else {
            skipped += 1;
            continue;
        };
        let kind = match mg[1].as_deref().unwrap_or("") {
            "d" => "dir",
            "l" => "link",
            "-" => "file",
            _ => "other",
        };
        let between = line[mode.end()..at.start()].trim();
        let tokens: Vec<&str> = if between.is_empty() { vec![""] } else { between.split_whitespace().collect() };
        let last = tokens.last().copied().unwrap_or("");
        let prev = if tokens.len() >= 2 { tokens[tokens.len() - 2] } else { "" };
        let sized = kind == "file" || kind == "link";
        let size = if sized && !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit()) && !prev.ends_with(',') && tokens.len() > 2 { last.parse::<f64>().ok() } else { None };
        let mut name = line[at.end()..].to_string();
        let mut target: Option<String> = None;
        if kind == "link" {
            if let Some(arrow) = name.find(" -> ").filter(|a| *a > 0) {
                target = Some(name[arrow + 4..].to_string());
                name.truncate(arrow);
            }
        }
        if name == "." || name == ".." {
            continue;
        }
        if name.is_empty() || name.chars().any(|c| (c as u32) < 0x20) {
            skipped += 1;
            continue;
        }
        let ag = groups(at, line);
        let modified = if iso.is_some() { format!("{} {}", ag[1].as_deref().unwrap_or(""), ag[2].as_deref().unwrap_or("")) } else { format!("{} {} {}", ag[1].as_deref().unwrap_or(""), ag[2].as_deref().unwrap_or(""), ag[3].as_deref().unwrap_or("")) };
        let size_v = size.map(|s| if s.fract() == 0.0 && s < 9.0e15 { json!(s as i64) } else { json!(s) }).unwrap_or(Value::Null);
        let mut e = json!({"name": name, "kind": kind, "size": size_v, "modified": modified});
        if let Some(t) = target {
            e["target"] = json!(t);
        }
        rows.push((kind == "dir", name, e));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| locale_cmp(&a.1, &b.1)));
    let entries: Vec<Value> = rows.into_iter().map(|r| r.2).collect();
    let error = if entries.is_empty() { error } else { None };
    Listing { entries, skipped, error }
}

pub fn is_device_path(p: &str) -> bool {
    if !p.starts_with('/') || js::len(p) > 1024 || p.contains(['\0', '\n', '\r']) {
        return false;
    }
    if p == "/" {
        return true;
    }
    if p.ends_with('/') {
        return false;
    }
    p[1..].split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

pub fn in_sdcard(p: &str) -> bool {
    SDCARD_ROOTS.iter().any(|r| p == *r || p.starts_with(&format!("{r}/")))
}

pub fn valid_path(scope: &Scope, p: &Value) -> Option<String> {
    let s = p.as_str()?;
    (is_device_path(s) && (matches!(scope, Scope::App(_)) || in_sdcard(s))).then(|| s.to_string())
}

pub fn deletable(scope: &Scope, p: &str) -> bool {
    match scope {
        Scope::App(_) => p != "/",
        Scope::Sdcard => SDCARD_ROOTS.iter().any(|r| p.starts_with(&format!("{r}/"))),
    }
}

pub fn is_entry_name(n: &str) -> bool {
    let len = js::len(n);
    (1..=255).contains(&len) && n != "." && n != ".." && !n.contains(['/', '\0', '\n', '\r'])
}

pub fn join_path(dir: &str, name: &str) -> String {
    if dir == "/" {
        format!("/{name}")
    } else {
        format!("{dir}/{name}")
    }
}

pub fn base_name(p: &str) -> String {
    p[p.rfind('/').map(|i| i + 1).unwrap_or(0)..].to_string()
}

fn device_arg(scope: &Scope, p: &str) -> String {
    match scope {
        Scope::Sdcard => p.to_string(),
        Scope::App(_) if p == "/" => ".".into(),
        Scope::App(_) => format!(".{p}"),
    }
}

fn prefix(scope: &Scope) -> Vec<String> {
    match scope {
        Scope::App(pkg) => vec!["run-as".into(), sh_quote(pkg)],
        Scope::Sdcard => Vec::new(),
    }
}

fn with(scope: &Scope, rest: Vec<String>) -> Vec<String> {
    let mut v = prefix(scope);
    v.extend(rest);
    v
}

pub fn ls_args(scope: &Scope, dir: &str) -> Vec<String> {
    let target = if dir == "/" && *scope == Scope::Sdcard { "/".to_string() } else { format!("{}/", device_arg(scope, dir)) };
    with(scope, vec!["ls".into(), "-la".into(), sh_quote(&target)])
}

pub fn stat_args(scope: &Scope, p: &str) -> Vec<String> {
    with(scope, vec!["ls".into(), "-la".into(), sh_quote(&device_arg(scope, p))])
}

pub fn exists_args(scope: &Scope, p: &str) -> Vec<String> {
    with(scope, vec!["ls".into(), "-d".into(), sh_quote(&device_arg(scope, p))])
}

pub fn cat_args(scope: &Scope, p: &str) -> Vec<String> {
    with(scope, vec!["cat".into(), sh_quote(&device_arg(scope, p))])
}

pub fn mkdir_args(scope: &Scope, p: &str) -> Vec<String> {
    with(scope, vec!["mkdir".into(), sh_quote(&device_arg(scope, p))])
}

pub fn rm_args(scope: &Scope, p: &str, dir: bool) -> Vec<String> {
    let mut rest = vec!["rm".to_string()];
    if dir {
        rest.push("-r".into());
    }
    rest.push("-f".into());
    rest.push(sh_quote(&device_arg(scope, p)));
    with(scope, rest)
}

pub fn part_rm_args(scope: &Scope, p: &str) -> Vec<String> {
    with(scope, vec!["rm".into(), "-f".into(), sh_quote(&(device_arg(scope, p) + PART_SUFFIX))])
}

pub fn write_args(scope: &Scope, p: &str) -> Vec<String> {
    let arg = device_arg(scope, p);
    let tmp = arg.clone() + PART_SUFFIX;
    with(scope, vec!["sh".into(), "-c".into(), sh_quote(&format!("cat > {} && mv -f {} {}", sh_quote(&tmp), sh_quote(&tmp), sh_quote(&arg)))])
}

pub fn local_safe_name(name: &str) -> String {
    static RESERVED: Js = Js::new(r"^(con|prn|aux|nul|com\d|lpt\d)(\..*)?$", "i");
    let replaced: String = name.chars().map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20 { '_' } else { c }).collect();
    let mut n = replaced.trim_end_matches(['.', ' ']).to_string();
    if RESERVED.test(&n) {
        n = format!("_{n}");
    }
    let cut = js::slice(&n, 200);
    if cut.is_empty() {
        "file".into()
    } else {
        cut
    }
}

pub fn unique_name(name: &str, taken: impl Fn(&str) -> bool) -> Option<String> {
    if !taken(name) {
        return Some(name.to_string());
    }
    let (stem, ext) = match name.rfind('.').filter(|d| *d > 0) {
        Some(d) => (&name[..d], &name[d..]),
        None => (name, ""),
    };
    (1..1000).map(|i| format!("{stem} ({i}){ext}")).find(|n| !taken(n))
}

const RUNNABLE: [&str; 33] = [
    "exe", "bat", "cmd", "com", "msi", "msp", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "jar", "lnk", "scr", "pif", "cpl", "reg", "sh", "command", "app", "url", "appref-ms", "application", "gadget", "msc", "inf", "scf", "dll", "sys",
];

pub fn safe_to_open(name: &str) -> bool {
    match name.rfind('.') {
        Some(d) if d > 0 => !RUNNABLE.contains(&name[d + 1..].to_lowercase().as_str()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_toybox_and_busybox_lines() {
        let out = "total 12\ndrwxrwx--x 3 root sdcard_rw 4096 2024-01-02 10:11 DCIM\n-rw-rw---- 1 root sdcard_rw 1234 2024-01-02 10:12 my file.txt\nlrwxrwxrwx 1 root root 10 2024-01-02 10:12 link -> /x/y\n-rw-r--r-- 1 u g 5 Jan  2 10:12 old.bin\nls: bad\n";
        let l = parse_ls_la(out);
        let names: Vec<&str> = l.entries.iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["DCIM", "link", "my file.txt", "old.bin"]);
        assert_eq!(l.entries[2]["size"], json!(1234));
        assert_eq!(l.entries[1]["target"], json!("/x/y"));
        assert_eq!(l.entries[3]["modified"], json!("Jan 2 10:12"));
        assert_eq!(l.skipped, 1);
        assert_eq!(parse_ls_la("ls: x: Permission denied\n").error, Some("permission"));
    }

    #[test]
    fn paths_and_names() {
        assert!(is_device_path("/sdcard/a b"));
        assert!(!is_device_path("/sdcard/../x"));
        assert!(valid_path(&Scope::Sdcard, &json!("/sdcard/x")).is_some());
        assert!(valid_path(&Scope::Sdcard, &json!("/etc")).is_none());
        assert_eq!(local_safe_name("a:b."), "a_b");
        assert_eq!(local_safe_name("con.txt"), "_con.txt");
        assert_eq!(unique_name("a.txt", |n| n == "a.txt").as_deref(), Some("a (1).txt"));
        assert!(!safe_to_open("run.sh") && safe_to_open("pic.png") && !safe_to_open("noext"));
    }
}
