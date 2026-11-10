use super::i18n_parse::{parse_android_strings, parse_arb, parse_json_keys, Entries};
use super::paths::{is_within, relative, resolve_str};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::OnceLock;

const MAX_FILES: i64 = 400;
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_DIRS: usize = 4000;
const MAX_KEYS: usize = 500;
const MAX_DEPTH: usize = 10;
const SKIP_DIRS: [&str; 24] = [
    "node_modules", "build", ".git", ".gradle", ".idea", ".dart_tool", ".pub-cache", "dist", "out", "Pods", ".next", ".nuxt", ".svelte-kit", "coverage", "vendor", "target", ".venv", "venv", "__pycache__", ".cxx", "intermediates", "generated", ".vitest", "release",
];

fn rel(root: &str, p: &str) -> String {
    relative(root, p).split(std::path::MAIN_SEPARATOR).collect::<Vec<_>>().join("/")
}

enum Small {
    Missing,
    TooLarge,
    Text(String),
}

fn read_small(file: &Path, budget: &mut i64) -> Small {
    if *budget <= 0 {
        return Small::Missing;
    }
    let Some(m) = std::fs::symlink_metadata(file).ok().filter(|m| m.is_file()) else { return Small::Missing };
    if m.len() > MAX_BYTES {
        return Small::TooLarge;
    }
    *budget -= 1;
    match std::fs::read(file) {
        Ok(b) => Small::Text(String::from_utf8_lossy(&b).to_string()),
        Err(_) => Small::Missing,
    }
}

struct Dirent {
    name: String,
    file: bool,
    dir: bool,
}

fn read_dir(dir: &Path) -> Option<Vec<Dirent>> {
    let rd = std::fs::read_dir(dir).ok()?;
    Some(
        rd.filter_map(Result::ok)
            .map(|e| {
                let t = e.file_type().ok();
                Dirent { name: e.file_name().to_string_lossy().to_string(), file: t.map(|t| t.is_file()).unwrap_or(false), dir: t.map(|t| t.is_dir()).unwrap_or(false) }
            })
            .collect(),
    )
}

#[derive(Default)]
struct Found {
    android: Vec<String>,
    flutter: Vec<String>,
    json: Vec<String>,
    truncated: bool,
}

fn walk(root: &str) -> Found {
    let mut found = Found::default();
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(root.to_string(), 0)]);
    let mut visited = 0;
    while let Some((dir, depth)) = queue.pop_front() {
        visited += 1;
        if visited > MAX_DIRS {
            found.truncated = true;
            break;
        }
        let Some(ents) = read_dir(Path::new(&dir)) else { continue };
        let name = crate::util::basename(&dir);
        if ents.iter().any(|e| e.file && e.name == "pubspec.yaml") {
            found.flutter.push(dir.clone());
        }
        if name == "res" && ents.iter().any(|e| e.dir && e.name == "values") {
            found.android.push(dir.clone());
            continue;
        }
        if ["locales", "i18n", "lang"].contains(&name.to_lowercase().as_str()) && ents.iter().filter(|e| e.file && e.name.to_lowercase().ends_with(".json")).count() >= 2 {
            found.json.push(dir.clone());
        }
        if depth >= MAX_DEPTH {
            continue;
        }
        for e in &ents {
            if e.dir && !SKIP_DIRS.contains(&e.name.as_str()) && !(e.name.starts_with('.') && e.name != ".") {
                queue.push_back((Path::new(&dir).join(&e.name).to_string_lossy().to_string(), depth + 1));
            }
        }
    }
    found
}

enum Loaded {
    Entries(Entries, String),
    Error(&'static str, String),
}

impl Loaded {
    fn file(&self) -> &str {
        match self {
            Loaded::Entries(_, f) | Loaded::Error(_, f) => f,
        }
    }
}

pub fn is_wordy(value: &str) -> bool {
    static STRIP: OnceLock<Regex> = OnceLock::new();
    static LETTER: OnceLock<Regex> = OnceLock::new();
    let strip = STRIP.get_or_init(|| Regex::new(r#"(?i)\{[^}]*\}|%(?:[0-9]+\$)?[a-z]|<[^>]+>|&[A-Za-z0-9_]+;|\\[nt'"@]"#).expect("regex"));
    let bare = strip.replace_all(value, "");
    LETTER.get_or_init(|| Regex::new(r"\p{L}").expect("regex")).find_iter(&bare).count() >= 3
}

fn key_json(key: &str, file: &str, line: usize) -> Value {
    json!({"key": key, "file": file, "line": line})
}

fn compare_locale(base: &Entries, entries: &Entries, base_file: &str, file: &str) -> (Value, Vec<Value>, Vec<Value>, Vec<Value>) {
    let (mut cm, mut ce, mut cs) = (0, 0, 0);
    let (mut missing, mut extra, mut same) = (Vec::new(), Vec::new(), Vec::new());
    for (key, b) in base.iter() {
        if b.fixed {
            continue;
        }
        match entries.get(key) {
            None => {
                cm += 1;
                if missing.len() < MAX_KEYS {
                    missing.push(key_json(key, base_file, b.line));
                }
            }
            Some(e) if e.value == b.value && is_wordy(&b.value) => {
                cs += 1;
                if same.len() < MAX_KEYS {
                    same.push(key_json(key, file, e.line));
                }
            }
            _ => {}
        }
    }
    for (key, e) in entries.iter() {
        if base.get(key).is_none() {
            ce += 1;
            if extra.len() < MAX_KEYS {
                extra.push(key_json(key, file, e.line));
            }
        }
    }
    (json!({"missing": cm, "extra": ce, "same": cs}), missing, extra, same)
}

fn build_set(kind: &str, dir: &str, base_locale: &str, base: &Loaded, locales: Vec<(String, Loaded)>) -> Value {
    let base_entries = match base {
        Loaded::Entries(e, _) => Some(e),
        _ => None,
    };
    let mut b = json!({"locale": base_locale, "files": [base.file()], "keys": base_entries.map(|e| e.len()).unwrap_or(0)});
    if let Loaded::Error(e, _) = base {
        b["error"] = json!(e);
    }
    let zero = json!({"missing": 0, "extra": 0, "same": 0});
    let list: Vec<Value> = locales
        .into_iter()
        .map(|(locale, loaded)| match &loaded {
            Loaded::Error(e, f) => json!({"locale": locale, "files": [f], "keys": 0, "counts": zero, "missing": [], "extra": [], "same": [], "error": e}),
            Loaded::Entries(entries, f) => {
                let (counts, missing, extra, same) = match base_entries {
                    Some(be) => compare_locale(be, entries, base.file(), f),
                    None => (zero.clone(), Vec::new(), Vec::new(), Vec::new()),
                };
                json!({"locale": locale, "files": [f], "keys": entries.len(), "counts": counts, "missing": missing, "extra": extra, "same": same})
            }
        })
        .collect();
    json!({"kind": kind, "dir": dir, "base": b, "locales": list})
}

fn android_locale(name: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^values-([a-z]{2,3}(?:-r[A-Z]{2})?|b\+[A-Za-z0-9+]+)$").expect("regex")).is_match(name) && &name[7..] != "car"
}

fn scan_android(root: &str, res: &str, budget: &mut i64) -> Option<Value> {
    let load = |dir: &str, budget: &mut i64| -> Option<Loaded> {
        let file = Path::new(res).join(dir).join("strings.xml").to_string_lossy().to_string();
        match read_small(Path::new(&file), budget) {
            Small::Missing => None,
            Small::TooLarge => Some(Loaded::Error("too-large", rel(root, &file))),
            Small::Text(t) => Some(Loaded::Entries(parse_android_strings(&t), rel(root, &file))),
        }
    };
    let base = load("values", budget)?;
    let mut dirs: Vec<String> = read_dir(Path::new(res)).unwrap_or_default().into_iter().filter(|e| e.dir && android_locale(&e.name)).map(|e| e.name).collect();
    dirs.sort();
    let mut locales = Vec::new();
    for d in dirs {
        if let Some(l) = load(&d, budget) {
            locales.push((d[7..].to_string(), l));
        }
    }
    Some(build_set("android", &rel(root, res), "default", &base, locales))
}

pub fn yaml_value(text: &str, key: &str) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| Regex::new(r"^([A-Za-z0-9_-]+)\s*:\s*(.*)$").expect("regex"));
    let c = C.get_or_init(|| Regex::new(r"\s+#.*$").expect("regex"));
    for raw in text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)) {
        let Some(m) = r.captures(raw).filter(|m| &m[1] == key) else { continue };
        let v = c.replace(&m[2], "").trim().to_string();
        let b = v.as_bytes();
        let v = if b.len() >= 2 && (b[0] == b'\'' || b[0] == b'"') && b[b.len() - 1] == b[0] { v[1..v.len() - 1].to_string() } else { v };
        return Some(v).filter(|v| !v.is_empty());
    }
    None
}

fn scan_flutter(root: &str, project: &str, budget: &mut i64) -> Option<Value> {
    let conf = match read_small(&Path::new(project).join("l10n.yaml"), budget) {
        Small::Text(t) => t,
        _ => String::new(),
    };
    let arb_rel = yaml_value(&conf, "arb-dir").unwrap_or_else(|| Path::new("lib").join("l10n").to_string_lossy().to_string());
    let arb_dir = resolve_str(&Path::new(project).join(arb_rel).to_string_lossy());
    if !is_within(project, &arb_dir) {
        return None;
    }
    let template = crate::util::basename(&yaml_value(&conf, "template-arb-file").unwrap_or_else(|| "app_en.arb".into()));
    let mut names: Vec<String> = read_dir(Path::new(&arb_dir)).unwrap_or_default().into_iter().filter(|e| e.file && e.name.ends_with(".arb")).map(|e| e.name).collect();
    names.sort();
    if !names.contains(&template) {
        return None;
    }
    let load = |name: &str, budget: &mut i64| -> (Loaded, Option<String>) {
        let file = Path::new(&arb_dir).join(name).to_string_lossy().to_string();
        match read_small(Path::new(&file), budget) {
            Small::TooLarge => (Loaded::Error("too-large", rel(root, &file)), None),
            Small::Text(t) => match parse_arb(&t) {
                Some((entries, locale)) => (Loaded::Entries(entries, rel(root, &file)), locale),
                None => (Loaded::Error("invalid", rel(root, &file)), None),
            },
            Small::Missing => (Loaded::Error("invalid", rel(root, &file)), None),
        }
    };
    static PRE: OnceLock<Regex> = OnceLock::new();
    let prefix = PRE.get_or_init(|| Regex::new(r"^(.*_)[^_]+\.arb$").expect("regex")).captures(&template).map(|c| c[1].to_string()).unwrap_or_default();
    let locale_of = |name: &str, from: Option<String>| from.unwrap_or_else(|| if !prefix.is_empty() && name.starts_with(&prefix) { name[prefix.len()..name.len() - 4].to_string() } else { name[..name.len() - 4].to_string() });
    let (base, base_locale) = load(&template, budget);
    let mut locales = Vec::new();
    for n in &names {
        if *n == template {
            continue;
        }
        let (l, loc) = load(n, budget);
        locales.push((locale_of(n, loc), l));
    }
    Some(build_set("flutter", &rel(root, &arb_dir), &locale_of(&template, base_locale), &base, locales))
}

fn scan_json(root: &str, dir: &str, budget: &mut i64) -> Option<Value> {
    let mut names: Vec<String> = read_dir(Path::new(dir)).unwrap_or_default().into_iter().filter(|e| e.file && e.name.to_lowercase().ends_with(".json")).map(|e| e.name).collect();
    names.sort();
    let base_name = ["en.json", "en-us.json", "en_us.json", "en-gb.json"].iter().find_map(|n| names.iter().find(|x| x.to_lowercase() == *n).cloned()).or_else(|| names.first().cloned())?;
    let load = |name: &str, budget: &mut i64| -> Loaded {
        let file = Path::new(dir).join(name).to_string_lossy().to_string();
        match read_small(Path::new(&file), budget) {
            Small::TooLarge => Loaded::Error("too-large", rel(root, &file)),
            Small::Text(t) => match parse_json_keys(&t) {
                Some(e) => Loaded::Entries(e, rel(root, &file)),
                None => Loaded::Error("invalid", rel(root, &file)),
            },
            Small::Missing => Loaded::Error("invalid", rel(root, &file)),
        }
    };
    let base = load(&base_name, budget);
    if matches!(base, Loaded::Error("invalid", _)) {
        return None;
    }
    let mut locales = Vec::new();
    for n in &names {
        if *n != base_name {
            locales.push((n[..n.len() - 5].to_string(), load(n, budget)));
        }
    }
    Some(build_set("json", &rel(root, dir), &base_name[..base_name.len() - 5], &base, locales))
}

pub fn scan_i18n(root: &str) -> Value {
    if !std::fs::metadata(root).map(|m| m.is_dir()).unwrap_or(false) {
        return json!({"ok": false, "error": "failed"});
    }
    let found = walk(root);
    let mut budget = MAX_FILES;
    let mut sets: Vec<Value> = Vec::new();
    for res in &found.android {
        if let Some(s) = scan_android(root, res, &mut budget) {
            sets.push(s);
        }
    }
    for p in &found.flutter {
        if let Some(s) = scan_flutter(root, p, &mut budget) {
            if !sets.iter().any(|x| x["kind"] == json!("flutter") && x["dir"] == s["dir"]) {
                sets.push(s);
            }
        }
    }
    for d in &found.json {
        if let Some(s) = scan_json(root, d, &mut budget) {
            sets.push(s);
        }
    }
    json!({"ok": true, "root": root, "sets": sets, "truncated": found.truncated || budget <= 0})
}
