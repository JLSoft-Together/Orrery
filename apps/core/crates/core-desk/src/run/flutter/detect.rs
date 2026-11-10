use super::pth;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

const MAX_FILE: u64 = 256 * 1024;
const MAX_ENTRIES: usize = 400;
const MAX_DIRS: usize = 1500;
const MAX_PROJECTS: usize = 50;
const MAX_FLAVORS: usize = 50;
const MAX_DEPTH: usize = 2;
const SKIP_DIRS: [&str; 4] = ["build", "node_modules", "Pods", "DerivedData"];
const PROJECT_OWNED: [&str; 12] = ["lib", "test", "integration_test", "assets", "android", "ios", "web", "windows", "macos", "linux", "bin", "tool"];
const PLATFORMS: [&str; 6] = ["android", "ios", "web", "windows", "macos", "linux"];

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

const WIN: bool = cfg!(windows);

fn join(parts: &[&str]) -> String {
    pth::join(parts, WIN)
}

struct Entry {
    name: String,
    is_dir: bool,
}

fn readdir(dir: &str) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Entry> = rd.flatten().map(|e| Entry { name: e.file_name().to_string_lossy().to_string(), is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false) }).collect();
    out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    out
}

fn is_file(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn read_small(file: &str) -> Option<String> {
    let m = std::fs::metadata(file).ok()?;
    if !m.is_file() || m.len() > MAX_FILE {
        return None;
    }
    std::fs::read_to_string(file).ok()
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    let b = t.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        return t[1..t.len() - 1].to_string();
    }
    t.to_string()
}

pub fn yaml_keys(text: &str) -> BTreeMap<String, String> {
    static LINE: OnceLock<Regex> = OnceLock::new();
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    let line_re = re(&LINE, r#"^(\s*)(["']?[A-Za-z0-9_.-]+["']?)\s*:(?:\s+(.*))?$"#);
    let comment_re = re(&COMMENT, r"\s+#.*$");
    let block_re = re(&BLOCK, r"^[|>][-+0-9]*$");
    let mut out = BTreeMap::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut block_indent: Option<usize> = None;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for raw in normalized.split('\n') {
        let trimmed = raw.trim_start();
        if raw.trim().is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.chars().count() - trimmed.chars().count();
        if let Some(b) = block_indent {
            if indent > b {
                continue;
            }
            block_indent = None;
        }
        let cleaned = comment_re.replace(raw, "");
        let Some(m) = line_re.captures(&cleaned) else { continue };
        while stack.last().is_some_and(|(i, _)| *i >= indent) {
            stack.pop();
        }
        let key = unquote(&m[2]);
        let value = m.get(3).map(|v| v.as_str().trim().to_string()).unwrap_or_default();
        stack.push((indent, key));
        let full = stack.iter().map(|(_, k)| k.as_str()).collect::<Vec<_>>().join(".");
        out.entry(full).or_insert_with(|| unquote(&value));
        if block_re.is_match(&value) {
            block_indent = Some(indent);
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct PubspecInfo {
    pub name: String,
    pub flutter: bool,
    pub plugin: bool,
}

pub fn parse_pubspec(text: &str) -> PubspecInfo {
    static DEP: OnceLock<Regex> = OnceLock::new();
    let keys = yaml_keys(text);
    let dep = keys.get("dependencies.flutter").map(String::as_str).unwrap_or("");
    let flutter = keys.get("dependencies.flutter.sdk").map(String::as_str) == Some("flutter") || re(&DEP, r#"^\{\s*sdk\s*:\s*['"]?flutter['"]?\s*\}$"#).is_match(dep);
    PubspecInfo { name: keys.get("name").cloned().unwrap_or_default(), flutter, plugin: keys.contains_key("flutter.plugin") }
}

pub fn metadata_type(text: &str) -> Option<&'static str> {
    static R: OnceLock<Regex> = OnceLock::new();
    let m = re(&R, r#"(?m)^project_type:\s*["']?([A-Za-z0-9_-]+)"#).captures(text)?;
    Some(match &m[1] {
        "plugin" | "plugin_ffi" => "plugin",
        "app" | "skeleton" | "module" => "app",
        _ => "package",
    })
}

fn strip_comments(s: &str) -> String {
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static LINE: OnceLock<Regex> = OnceLock::new();
    let a = re(&BLOCK, r"(?s)/\*.*?\*/").replace_all(s, "");
    re(&LINE, r"(?m)(^|\s)//.*$").replace_all(&a, "$1").to_string()
}

pub fn gradle_flavors(text: &str) -> Vec<String> {
    static START: OnceLock<Regex> = OnceLock::new();
    static CREATE: OnceLock<Regex> = OnceLock::new();
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    let src = strip_comments(text);
    let Some(start) = re(&START, r"\bproductFlavors\s*\{").find(&src) else { return Vec::new() };
    let mut depth = 0usize;
    let mut flat = String::new();
    for c in src[start.end()..].chars() {
        match c {
            '{' => {
                if depth == 0 {
                    flat.push('{');
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 {
                    flat.push(';');
                }
            }
            _ if depth == 0 => flat.push(c),
            _ => {}
        }
    }
    let mut names = Vec::new();
    for m in re(&CREATE, r#"\b(?:create|register|maybeCreate)\s*\(\s*["']([A-Za-z0-9_.-]+)["']\s*\)"#).captures_iter(&flat) {
        names.push(m[1].to_string());
    }
    for m in re(&BLOCK, r"(?:^|[\s;])([A-Za-z_][A-Za-z0-9_]*)\s*\{").captures_iter(&flat) {
        if !["all", "configureEach", "forEach", "each"].contains(&&m[1]) {
            names.push(m[1].to_string());
        }
    }
    names
}

pub fn parse_fvm_config(text: &str) -> Option<String> {
    let json: Value = serde_json::from_str(text).ok()?;
    let v = json.get("flutter").filter(|v| !v.is_null()).or_else(|| json.get("flutterSdkVersion"))?;
    let s = v.as_str()?.trim();
    (!s.is_empty()).then(|| s.to_string())
}

pub fn read_fvm_version(dir: &str) -> Option<String> {
    for file in [join(&[dir, ".fvmrc"]), join(&[dir, ".fvm", "fvm_config.json"])] {
        if let Some(v) = read_small(&file).and_then(|t| parse_fvm_config(&t)) {
            return Some(v);
        }
    }
    None
}

fn entrypoints(dir: &str) -> Vec<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    let main = re(&R, r"^main[A-Za-z0-9_.-]*\.dart$");
    let mut files: Vec<String> = readdir(&join(&[dir, "lib"])).into_iter().filter(|e| !e.is_dir && main.is_match(&e.name)).map(|e| e.name).collect();
    files.sort_by(|a, b| match (a.as_str() == "main.dart", b.as_str() == "main.dart") {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)),
    });
    files.into_iter().take(MAX_ENTRIES).map(|f| format!("lib/{f}")).collect()
}

fn flavors(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let add = |f: String, out: &mut Vec<String>| {
        if !out.contains(&f) {
            out.push(f);
        }
    };
    for file in ["build.gradle", "build.gradle.kts"] {
        if let Some(text) = read_small(&join(&[dir, "android", "app", file])) {
            for f in gradle_flavors(&text) {
                add(f, &mut out);
            }
        }
    }
    for e in readdir(&join(&[dir, "ios", "Runner.xcodeproj", "xcshareddata", "xcschemes"])) {
        if let Some(name) = e.name.strip_suffix(".xcscheme") {
            if !e.is_dir && !name.is_empty() && name != "Runner" {
                add(name.to_string(), &mut out);
            }
        }
    }
    out.truncate(MAX_FLAVORS);
    out
}

fn mtime(p: &str) -> Option<(bool, std::time::SystemTime)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.is_file(), m.modified().unwrap_or(std::time::UNIX_EPOCH)))
}

pub fn needs_pub_get(dir: &str) -> bool {
    if !is_file(&join(&[dir, ".dart_tool", "package_config.json"])) {
        return true;
    }
    let yaml = mtime(&join(&[dir, "pubspec.yaml"]));
    let Some((true, lock)) = mtime(&join(&[dir, "pubspec.lock"])) else { return true };
    yaml.is_some_and(|(_, y)| y > lock)
}

fn inspect(dir: &str) -> Option<(Value, Option<String>)> {
    let text = read_small(&join(&[dir, "pubspec.yaml"]))?;
    let pub_info = parse_pubspec(&text);
    let mains = entrypoints(dir);
    let name = if pub_info.name.is_empty() { pth::basename(dir, WIN) } else { pub_info.name.clone() };
    let mut project = json!({"kind": "flutter", "dir": dir, "name": name, "entrypoints": mains, "needsPubGet": needs_pub_get(dir), "fvm": read_fvm_version(dir)});
    let obj = project.as_object_mut()?;
    if !pub_info.flutter {
        obj.insert("type".into(), json!("dart"));
        obj.insert("platforms".into(), json!([]));
        obj.insert("flavors".into(), json!([]));
        return Some((project, None));
    }
    let platforms: Vec<&str> = PLATFORMS.iter().copied().filter(|p| is_dir(&join(&[dir, p]))).collect();
    let meta = read_small(&join(&[dir, ".metadata"])).and_then(|m| metadata_type(&m));
    let ty = meta.unwrap_or(if pub_info.plugin {
        "plugin"
    } else if !mains.is_empty() && !platforms.is_empty() {
        "app"
    } else {
        "package"
    });
    obj.insert("type".into(), json!(ty));
    obj.insert("platforms".into(), json!(platforms));
    obj.insert("flavors".into(), json!(flavors(dir)));
    let mut example = None;
    if ty == "plugin" && is_file(&join(&[dir, "example", "pubspec.yaml"])) {
        let ex = join(&[dir, "example"]);
        obj.insert("example".into(), json!(ex));
        example = Some(ex);
    }
    Some((project, example))
}

fn pubspec_dirs(root: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut level = vec![root.to_string()];
    let mut visited = 0;
    let mut depth = 0;
    while depth <= MAX_DEPTH && !level.is_empty() {
        let mut next = Vec::new();
        for dir in &level {
            visited += 1;
            if visited > MAX_DIRS {
                return found;
            }
            let mut entries = readdir(dir);
            entries.truncate(MAX_ENTRIES);
            let is_project = entries.iter().any(|e| !e.is_dir && e.name == "pubspec.yaml");
            if is_project {
                found.push(dir.clone());
            }
            if depth == MAX_DEPTH {
                continue;
            }
            for e in &entries {
                if !e.is_dir || e.name.starts_with('.') || SKIP_DIRS.contains(&e.name.as_str()) {
                    continue;
                }
                if is_project && PROJECT_OWNED.contains(&e.name.as_str()) {
                    continue;
                }
                next.push(join(&[dir, &e.name]));
            }
        }
        level = next;
        depth += 1;
    }
    found
}

pub fn detect_flutter(root: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let resolved = pth::resolve(root, WIN);
    for dir in pubspec_dirs(&resolved) {
        let mut cur = Some(dir);
        while let Some(d) = cur.take() {
            if seen.contains(&d) || out.len() >= MAX_PROJECTS {
                break;
            }
            seen.insert(d.clone());
            let Some((project, example)) = inspect(&d) else { break };
            out.push(project);
            cur = example;
        }
    }
    out
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
