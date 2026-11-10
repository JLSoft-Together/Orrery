use crate::js;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::sync::OnceLock;

pub const BACKUP_FORMAT: &str = "orrery-backup";
pub const MAX_BACKUP_BYTES: usize = 24 * 1024 * 1024;
const MAX_FILE_CHARS: usize = 4 * 1024 * 1024;
const MAX_LOCAL_CHARS: usize = 2 * 1024 * 1024;
const MAX_MISSING: usize = 20;
const MAX_GITHUB: usize = 20;
pub const REMOVED: &str = "[removed]";

pub const SECTIONS: [&str; 9] = ["appearance", "settings", "workspace", "notes", "names", "devices", "run", "updates", "github"];

pub const FILE_SECTION: [(&str, &str); 8] = [
    ("aliases.json", "names"),
    ("office-names.json", "names"),
    ("favorites.json", "names"),
    ("adb-devices.json", "devices"),
    ("layouts.json", "workspace"),
    ("run-settings.json", "run"),
    ("run-templates.json", "run"),
    ("update.json", "updates"),
];

struct Res {
    note: Regex,
    skip: Regex,
    appearance: Regex,
    workspace: Regex,
    notes: Regex,
    secret_key: Regex,
    secret_value: Regex,
    abs: Regex,
    login: Regex,
}

fn res() -> &'static Res {
    static R: OnceLock<Res> = OnceLock::new();
    R.get_or_init(|| Res {
        note: Regex::new(r"^notes/[0-9a-f]{40}\.md$").unwrap(),
        skip: Regex::new(r"^ccm\.(whatsNew\.|breakSeen|resumeSchedule|inbox$|backup\.|http\.env)").unwrap(),
        appearance: Regex::new(r"^ccm\.(theme|locale|font\.|fileIcons|minimap|keyHints|dust|office\.(pack|style)$)").unwrap(),
        workspace: Regex::new(r"^ccm\.(workspace\.|layout\.|panels$|bar$|panelSize$|mapLayout|mapView$|view$|sessionTab|run\.selected$|fwp\.|timelineWindow$|launchTerminal$|drawable\.|md\.|http\.open$|imageCompare\.)").unwrap(),
        notes: Regex::new(r"^ccm\.(prompts$|promptQueue\.|bookmarks$|recentFiles$|recentEdits$|resumeText$|notes\.|planBoard\.)").unwrap(),
        secret_key: Regex::new(r"(?i)(token|secret|passw|pwd|api[-_]?key|apikey|private[-_]?key|auth|cookie|credential|bearer)").unwrap(),
        secret_value: Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----|(?-u:\b)(?:ghp_|gho_|ghu_|ghs_|ghr_)[A-Za-z0-9]{20,}|(?-u:\b)github_pat_[A-Za-z0-9_]{20,}|(?-u:\b)sk-ant-[A-Za-z0-9_-]{10,}|(?-u:\b)sk-[A-Za-z0-9]{20,}|(?-u:\b)xox[abprs]-[A-Za-z0-9-]{10,}|(?-u:\b)AKIA[0-9A-Z]{16}(?-u:\b)|(?-u:\b)AIza[0-9A-Za-z_-]{35}|(?-u:\b)eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}").unwrap(),
        abs: Regex::new(r"^[A-Za-z]:[\\/]|^\\\\[^\\\s]+\\|^/[^/\s]+/\S*$").unwrap(),
        login: Regex::new(r"^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$").unwrap(),
    })
}

pub fn is_note_file(rel: &str) -> bool {
    res().note.is_match(rel)
}

pub fn file_section(rel: &str) -> Option<&'static str> {
    if let Some((_, s)) = FILE_SECTION.iter().find(|(f, _)| *f == rel) {
        return Some(s);
    }
    is_note_file(rel).then_some("notes")
}

pub fn local_section(key: &str) -> Option<&'static str> {
    let r = res();
    if !key.starts_with("ccm.") || js::len(key) > 200 || r.skip.is_match(key) || r.secret_key.is_match(key) {
        return None;
    }
    Some(if r.appearance.is_match(key) {
        "appearance"
    } else if r.workspace.is_match(key) {
        "workspace"
    } else if r.notes.is_match(key) {
        "notes"
    } else {
        "settings"
    })
}

pub fn scrub_text(s: &str) -> String {
    res().secret_value.replace_all(s, REMOVED).into_owned()
}

pub fn scrub_json(v: &Value, depth: usize) -> Value {
    match v {
        Value::String(s) => Value::String(scrub_text(s)),
        _ if depth > 40 => v.clone(),
        Value::Array(a) => Value::Array(a.iter().map(|x| scrub_json(x, depth + 1)).collect()),
        Value::Object(m) => {
            let mut out = Map::new();
            for (k, val) in js::entries(m) {
                if res().secret_key.is_match(k) {
                    continue;
                }
                let clean = if k == "env" && val.is_object() { json!({}) } else { scrub_json(val, depth + 1) };
                out.insert(k.clone(), clean);
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}

fn scrub_stored(value: &str, as_json: bool) -> Option<String> {
    let parsed: Option<Value> = serde_json::from_str(value).ok();
    if as_json {
        return parsed.map(|p| js::stringify(&scrub_json(&p, 0)));
    }
    match parsed {
        Some(p) if p.is_object() || p.is_array() => Some(js::stringify(&scrub_json(&p, 0))),
        _ => Some(scrub_text(value)),
    }
}

pub type Pairs = Vec<(String, String)>;

pub fn filter_local(raw: &Value) -> Pairs {
    let mut out = Pairs::new();
    let Some(m) = raw.as_object() else { return out };
    for (k, v) in js::entries(m) {
        let Some(s) = v.as_str() else { continue };
        if js::len(s) > MAX_LOCAL_CHARS || local_section(k).is_none() {
            continue;
        }
        if let Some(clean) = scrub_stored(s, false) {
            out.push((k.clone(), clean));
        }
    }
    out
}

pub fn filter_files(raw: &Value) -> Pairs {
    let mut out = Pairs::new();
    let Some(m) = raw.as_object() else { return out };
    for (rel, v) in js::entries(m) {
        let Some(s) = v.as_str() else { continue };
        if js::len(s) > MAX_FILE_CHARS || file_section(rel).is_none() {
            continue;
        }
        let clean = if rel.ends_with(".json") { scrub_stored(s, true) } else { Some(scrub_text(s)) };
        if let Some(c) = clean {
            out.push((rel.clone(), c));
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct GitHub {
    pub login: String,
    pub name: Option<String>,
}

pub fn filter_github(raw: &Value) -> Vec<GitHub> {
    let mut out: Vec<GitHub> = Vec::new();
    let Some(list) = raw.as_array() else { return out };
    for a in list {
        let Some(o) = a.as_object() else { continue };
        let Some(login) = o.get("login").and_then(Value::as_str) else { continue };
        if !res().login.is_match(login) || out.iter().any(|x| x.login == login) {
            continue;
        }
        let name = o.get("name").and_then(Value::as_str).map(|n| js::slice(n, 100));
        out.push(GitHub { login: login.to_string(), name });
        if out.len() >= MAX_GITHUB {
            break;
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupFile {
    pub app: String,
    pub platform: String,
    pub created_at: String,
    pub local: Pairs,
    pub files: Pairs,
    pub github: Vec<GitHub>,
}

fn pairs_value(p: &Pairs) -> Value {
    Value::Object(p.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect())
}

impl BackupFile {
    pub fn to_value(&self) -> Value {
        json!({
            "format": BACKUP_FORMAT,
            "version": 1,
            "app": self.app,
            "platform": self.platform,
            "createdAt": self.created_at,
            "local": pairs_value(&self.local),
            "files": pairs_value(&self.files),
            "github": self.github.iter().map(|g| json!({"login": g.login, "name": g.name})).collect::<Vec<_>>(),
        })
    }
}

pub fn build_backup(app: &str, platform: &str, now_ms: f64, local: &Value, files: &Value, github: &Value) -> BackupFile {
    BackupFile {
        app: app.to_string(),
        platform: platform.to_string(),
        created_at: crate::util::iso_string(now_ms),
        local: filter_local(local),
        files: filter_files(files),
        github: filter_github(github),
    }
}

pub fn parse_backup(text: &str) -> Option<BackupFile> {
    let raw: Value = serde_json::from_str(text).ok()?;
    let b = raw.as_object()?;
    if b.get("format").and_then(Value::as_str) != Some(BACKUP_FORMAT) || b.get("version").and_then(Value::as_f64) != Some(1.0) {
        return None;
    }
    let s = |k: &str, max: usize| b.get(k).and_then(Value::as_str).map(|v| js::slice(v, max)).unwrap_or_default();
    let get = |k: &str| b.get(k).cloned().unwrap_or(Value::Null);
    Some(BackupFile {
        app: s("app", 40),
        platform: s("platform", 20),
        created_at: s("createdAt", 40),
        local: filter_local(&get("local")),
        files: filter_files(&get("files")),
        github: filter_github(&get("github")),
    })
}

fn looks_absolute(s: &str) -> bool {
    let n = js::len(s);
    (4..=1024).contains(&n) && res().abs.is_match(s)
}

#[derive(Default)]
pub struct PathSet {
    pub list: Vec<String>,
    seen: HashSet<String>,
}

impl PathSet {
    fn add(&mut self, s: &str) {
        if self.seen.insert(s.to_string()) {
            self.list.push(s.to_string());
        }
    }
}

pub fn collect_paths(v: &Value, out: &mut PathSet, depth: usize) {
    match v {
        Value::String(s) => {
            if looks_absolute(s) {
                out.add(s);
            } else if (s.starts_with('{') || s.starts_with('[')) && depth < 4 && js::len(s) <= 1_000_000 {
                if let Ok(p) = serde_json::from_str::<Value>(s) {
                    collect_paths(&p, out, depth + 10);
                }
            }
        }
        _ if depth > 40 => {}
        Value::Array(a) => a.iter().for_each(|x| collect_paths(x, out, depth + 1)),
        Value::Object(m) => {
            for (k, val) in js::entries(m) {
                if looks_absolute(k) {
                    out.add(k);
                }
                collect_paths(val, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn content_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

pub fn sections_of(b: &BackupFile, exists: Option<&dyn Fn(&str) -> bool>) -> Value {
    let mut acc: Vec<(&str, usize, PathSet)> = Vec::new();
    let mut add = |id: &'static str, value: Value| {
        let i = match acc.iter().position(|(x, _, _)| *x == id) {
            Some(i) => i,
            None => {
                acc.push((id, 0, PathSet::default()));
                acc.len() - 1
            }
        };
        acc[i].1 += 1;
        if exists.is_some() {
            collect_paths(&value, &mut acc[i].2, 0);
        }
    };
    for (k, v) in &b.local {
        add(local_section(k).unwrap_or("settings"), content_of(v));
    }
    for (rel, v) in &b.files {
        add(file_section(rel).unwrap_or("names"), if rel.ends_with(".json") { content_of(v) } else { Value::Null });
    }
    for g in &b.github {
        add("github", Value::String(g.login.clone()));
    }
    let rows: Vec<Value> = SECTIONS
        .iter()
        .filter_map(|id| acc.iter().find(|(x, _, _)| x == id))
        .map(|(id, count, paths)| {
            let missing: Vec<&String> = match exists {
                Some(f) => paths.list.iter().filter(|p| !f(p)).take(MAX_MISSING).collect(),
                None => Vec::new(),
            };
            json!({"id": id, "count": count, "missingPaths": missing})
        })
        .collect();
    Value::Array(rows)
}

pub struct Picked {
    pub local: Pairs,
    pub files: Pairs,
    pub github: Vec<String>,
}

pub fn pick_sections(b: &BackupFile, ids: &[String]) -> Picked {
    let has = |s: Option<&str>| s.is_some_and(|s| ids.iter().any(|i| i == s));
    Picked {
        local: b.local.iter().filter(|(k, _)| has(local_section(k))).cloned().collect(),
        files: b.files.iter().filter(|(r, _)| has(file_section(r))).cloned().collect(),
        github: if has(Some("github")) { b.github.iter().map(|g| g.login.clone()).collect() } else { Vec::new() },
    }
}

pub fn pairs_to_value(p: &Pairs) -> Value {
    pairs_value(p)
}

pub fn clean_section_ids(v: &Value) -> Option<Vec<String>> {
    let a = v.as_array()?;
    if a.len() > SECTIONS.len() || !a.iter().all(|x| x.as_str().is_some_and(|s| SECTIONS.contains(&s))) {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    for x in a.iter().filter_map(Value::as_str) {
        if !out.iter().any(|o| o == x) {
            out.push(x.to_string());
        }
    }
    Some(out)
}

pub fn export_name((y, m, d): (i64, u32, u32)) -> String {
    format!("orrery-backup-{y}-{m:02}-{d:02}.json")
}

pub fn prune_list(entries: &[(String, f64)], prefix: &str, keep: usize) -> Vec<String> {
    let mut list: Vec<&(String, f64)> = entries.iter().filter(|(n, _)| n.starts_with(prefix) && n.ends_with(".json")).collect();
    list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| b.0.cmp(&a.0)));
    list.into_iter().skip(keep).map(|(n, _)| n.clone()).collect()
}
