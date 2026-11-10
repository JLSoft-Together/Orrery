pub use super::catalog::{norm_key, parse_catalog_libraries, parse_catalog_plugins, Catalog, CatalogLibrary};
use super::jsre::{g, Js};
use super::pathx;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MAX_FILE: u64 = 256 * 1024;
const MAX_DIRS: usize = 400;
const SKIP_DIRS: [&str; 4] = ["node_modules", "build", "Pods", "DerivedData"];
const APP_ID: &str = "com.android.application";
const LIBRARY_IDS: [&str; 4] = ["com.android.library", "com.android.dynamic-feature", "com.android.test", "com.android.kotlin.multiplatform.library"];
const KMP_ID: &str = "org.jetbrains.kotlin.multiplatform";

pub fn read_text(file: &str) -> Option<String> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    std::fs::read_to_string(file).ok()
}

pub fn list_dirs(dir: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).map(|e| e.file_name().to_string_lossy().to_string()).collect()
}

fn win() -> bool {
    pathx::host_win()
}

fn join(parts: &[&str]) -> String {
    pathx::join(win(), parts)
}

pub fn strip_comments(text: &str) -> String {
    static BLOCK: Js = Js::new(r"\/\*[\s\S]*?\*\/", "g");
    static LINE: Js = Js::new(r#"(^|[^:"'])\/\/.*$"#, "gm");
    LINE.replace_all(&BLOCK.replace_all(text, ""), "$1")
}

static STRING_RE: Js = Js::new(r#"["']([^"'\r\n]+)["']"#, "g");

fn push_unique(out: &mut Vec<String>, v: String) {
    if !out.contains(&v) {
        out.push(v);
    }
}

pub fn parse_includes(settings: &str) -> Vec<String> {
    static INC: Js = Js::new(r"\binclude\b\s*(\()?", "g");
    static NAME: Js = Js::new(r"^[\w.\-:]+$", "");
    static NL: Js = Js::new(r"\r?\n", "");
    let text = strip_comments(settings);
    let mut out = Vec::new();
    for m in INC.re().find_iter(&text) {
        let start = m.end();
        let body = if m.group(1).is_some() {
            match text[start..].find(')') {
                Some(i) => text[start..start + i].to_string(),
                None => text[start..].to_string(),
            }
        } else {
            let rest = &text[start..];
            let lines: Vec<&str> = split_re(&NL, rest);
            let mut body = lines.first().map(|s| s.to_string()).unwrap_or_default();
            let mut i = 1;
            while body.trim_end().ends_with(',') && i < lines.len() {
                body.push('\n');
                body.push_str(lines[i]);
                i += 1;
            }
            body
        };
        for s in STRING_RE.all(&body) {
            let name = g(&s, 1).unwrap_or("").trim().trim_start_matches(':').to_string();
            if !name.is_empty() && NAME.test(&name) {
                push_unique(&mut out, format!(":{name}"));
            }
        }
    }
    out
}

pub fn split_re<'a>(re: &Js, text: &'a str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut last = 0;
    for m in re.re().find_iter(text) {
        out.push(&text[last..m.start()]);
        last = m.end();
    }
    out.push(&text[last..]);
    out
}

pub fn split_lines(text: &str) -> Vec<&str> {
    static NL: Js = Js::new(r"\r?\n", "");
    split_re(&NL, text)
}

pub fn parse_project_dirs(settings: &str) -> BTreeMap<String, String> {
    static RE: Js = Js::new(
        r#"project\s*\(\s*["'](:?[\w.\-:]+)["']\s*\)\s*\.projectDir\s*=\s*(?:new\s+File\s*\(\s*(?:settingsDir|rootDir|rootProject\.projectDir)\s*,\s*["']([^"']+)["']\s*\)|file\s*\(\s*["']([^"']+)["']\s*\))"#,
        "g",
    );
    let mut out = BTreeMap::new();
    for m in RE.all(&strip_comments(settings)) {
        let key = format!(":{}", g(&m, 1).unwrap_or("").trim_start_matches(':'));
        if let Some(v) = g(&m, 2).or(g(&m, 3)) {
            out.insert(key, v.to_string());
        }
    }
    out
}

pub fn parse_root_name(settings: &str) -> Option<String> {
    static RE: Js = Js::new(r#"rootProject\.name\s*=\s*["']([^"']+)["']"#, "");
    let m = RE.exec(&strip_comments(settings))?;
    let v = g(&m, 1)?.trim();
    (!v.is_empty()).then(|| v.to_string())
}

pub type Catalogs = BTreeMap<String, BTreeMap<String, String>>;

pub fn plugin_ids(build: &str, catalogs: &Catalogs) -> Vec<String> {
    static APPLY_FALSE: Js = Js::new(r"\bapply\s*\(?\s*false\b", "");
    static ID: Js = Js::new(r#"\bid\s*\(?\s*["']([\w.\-]+)["']"#, "g");
    static LEGACY: Js = Js::new(r#"\bapply\s+plugin\s*:\s*["']([\w.\-]+)["']"#, "g");
    static KTS_APPLY: Js = Js::new(r#"\bapply\s*\(\s*plugin\s*=\s*["']([\w.\-]+)["']"#, "g");
    static KMP: Js = Js::new(r#"\bkotlin\s*\(\s*["']multiplatform["']\s*\)"#, "");
    static ALIAS: Js = Js::new(r"\balias\s*\(\s*(\w+)\.plugins\.([\w.]+?)(?:\.get\(\))?\s*\)", "g");
    let mut ids = Vec::new();
    let stripped = strip_comments(build);
    for line in split_lines(&stripped) {
        if APPLY_FALSE.test(line) {
            continue;
        }
        for re in [&ID, &LEGACY, &KTS_APPLY] {
            for m in re.all(line) {
                if let Some(id) = g(&m, 1) {
                    push_unique(&mut ids, id.to_string());
                }
            }
        }
        if KMP.test(line) {
            push_unique(&mut ids, KMP_ID.to_string());
        }
        for m in ALIAS.all(line) {
            if let Some(id) = catalogs.get(g(&m, 1).unwrap_or("")).and_then(|c| c.get(&norm_key(g(&m, 2).unwrap_or("")))) {
                push_unique(&mut ids, id.clone());
            }
        }
    }
    ids
}

pub fn module_type(ids: &[String]) -> &'static str {
    if ids.iter().any(|i| i == APP_ID) {
        "application"
    } else if ids.iter().any(|i| LIBRARY_IDS.contains(&i.as_str())) {
        "library"
    } else {
        "unknown"
    }
}

pub fn has_launcher(manifest: &str) -> bool {
    static COMMENT: Js = Js::new(r"<!--[\s\S]*?-->", "g");
    static ACT: Js = Js::new(r"<(activity-alias|activity)\b[^>]*?(?:\/>|>([\s\S]*?)<\/\1\s*>)", "g");
    static FILTER: Js = Js::new(r"<intent-filter\b[^>]*>([\s\S]*?)<\/intent-filter\s*>", "g");
    static MAIN: Js = Js::new(r#"android:name\s*=\s*["']android\.intent\.action\.MAIN["']"#, "");
    static LAUNCH: Js = Js::new(r#"android:name\s*=\s*["']android\.intent\.category\.(?:LEANBACK_)?LAUNCHER["']"#, "");
    let text = COMMENT.replace_all(manifest, "");
    for m in ACT.all(&text) {
        for f in FILTER.all(g(&m, 2).unwrap_or("")) {
            let body = g(&f, 1).unwrap_or("");
            if MAIN.test(body) && LAUNCH.test(body) {
                return true;
            }
        }
    }
    false
}

pub fn is_flutter_pubspec(text: Option<&str>) -> bool {
    static RE: Js = Js::new(r"^\s*sdk:\s*flutter\b", "m");
    text.is_some_and(|t| RE.test(t))
}

pub fn project_dir_of(gradle_root: &str) -> String {
    if pathx::basename(gradle_root, win()) == "android" {
        pathx::dirname(gradle_root, win())
    } else {
        gradle_root.to_string()
    }
}

pub fn android_framework(gradle_root: &str, kmp: bool) -> &'static str {
    let dir = project_dir_of(gradle_root);
    if is_flutter_pubspec(read_text(&join(&[&dir, "pubspec.yaml"])).as_deref()) {
        return "flutter";
    }
    if dir != gradle_root {
        if let Some(Value::Object(json)) = read_text(&join(&[&dir, "package.json"])).and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            let has = |k: &str| json.get(k).and_then(|d| d.get("react-native")).is_some_and(truthy);
            if has("dependencies") || has("devDependencies") {
                return "react-native";
            }
        }
    }
    for name in ["capacitor.config.json", "capacitor.config.ts", "capacitor.config.js"] {
        if read_text(&join(&[&dir, name])).is_some() {
            return "capacitor";
        }
    }
    if kmp {
        "kmp"
    } else {
        "native"
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn read_first(files: &[String]) -> Option<(String, String)> {
    files.iter().find_map(|f| read_text(f).map(|t| (f.clone(), t)))
}

fn load_catalogs(root: &str) -> Catalogs {
    let mut out = Catalogs::new();
    if let Some(text) = read_text(&join(&[root, "gradle", "libs.versions.toml"])).filter(|t| !t.is_empty()) {
        out.insert("libs".into(), parse_catalog_plugins(&text));
    }
    out
}

pub fn analyze_gradle_root(root: &str) -> Option<Value> {
    let (_, settings) = read_first(&[join(&[root, "settings.gradle.kts"]), join(&[root, "settings.gradle"])])?;
    let catalogs = load_catalogs(root);
    let dirs = parse_project_dirs(&settings);
    let includes = parse_includes(&settings);
    let paths = if includes.is_empty() { vec![":".to_string()] } else { includes };
    let mut modules: Vec<Value> = Vec::new();
    let mut kmp = false;
    let (mut any_app, mut any_unknown_manifest, mut android_like) = (false, false, false);
    for p in &paths {
        let dir = if p == ":" {
            root.to_string()
        } else if let Some(custom) = dirs.get(p) {
            pathx::resolve(root, custom, win())
        } else {
            let mut parts = vec![root];
            parts.extend(p[1..].split(':'));
            join(&parts)
        };
        let build = read_first(&[join(&[&dir, "build.gradle.kts"]), join(&[&dir, "build.gradle"])]);
        let manifest = read_first(&[join(&[&dir, "src", "main", "AndroidManifest.xml"]), join(&[&dir, "src", "androidMain", "AndroidManifest.xml"])]);
        if build.is_none() && manifest.is_none() {
            continue;
        }
        let ids = build.as_ref().map(|(_, t)| plugin_ids(t, &catalogs)).unwrap_or_default();
        if ids.iter().any(|i| i == KMP_ID) {
            kmp = true;
        }
        let kind = module_type(&ids);
        let launcher = manifest.as_ref().is_some_and(|(_, t)| has_launcher(t));
        let guessed = kind == "unknown" && launcher;
        any_app |= kind == "application" || guessed;
        any_unknown_manifest |= kind == "unknown" && manifest.is_some();
        android_like |= kind != "unknown" || manifest.is_some();
        modules.push(json!({"path": p, "dir": dir, "type": kind, "manifest": manifest.map(|(f, _)| f), "launcher": launcher, "guessed": guessed}));
    }
    if !android_like {
        return None;
    }
    let status = if any_app {
        "runnable"
    } else if any_unknown_manifest {
        "unknown"
    } else {
        "no-app"
    };
    let name = parse_root_name(&settings).unwrap_or_else(|| pathx::basename(&project_dir_of(root), win()));
    Some(json!({"kind": "android", "dir": root, "name": name, "framework": android_framework(root, kmp), "status": status, "modules": modules}))
}

fn has_settings(dir: &str) -> bool {
    read_text(&join(&[dir, "settings.gradle.kts"])).is_some() || read_text(&join(&[dir, "settings.gradle"])).is_some()
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.') || SKIP_DIRS.contains(&name)
}

pub fn find_gradle_roots(root: &str) -> Vec<String> {
    fn walk(dir: &str, depth: u32, visited: &mut usize, found: &mut Vec<String>) {
        if *visited >= MAX_DIRS {
            *visited += 1;
            return;
        }
        *visited += 1;
        if has_settings(dir) {
            found.push(dir.to_string());
            return;
        }
        let mut children: Vec<String> = list_dirs(dir).into_iter().filter(|n| !skip_dir(n)).collect();
        children.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        for name in children {
            if depth < 2 || (depth == 2 && name == "android") {
                walk(&join(&[dir, &name]), depth + 1, visited, found);
            }
        }
    }
    let mut found = Vec::new();
    let mut visited = 0;
    walk(root, 0, &mut visited, &mut found);
    found
        .into_iter()
        .filter(|dir| !(pathx::basename(dir, win()) == "android" && is_flutter_pubspec(read_text(&join(&[&pathx::dirname(dir, win()), "pubspec.yaml"])).as_deref())))
        .collect()
}

pub fn detect_android(root: &str) -> Vec<Value> {
    find_gradle_roots(root).iter().filter_map(|d| analyze_gradle_root(d)).collect()
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
