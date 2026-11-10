use crate::release_check::xml::xml_attrs;
use crate::run::android::jsre::{g, Js};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

const MANIFEST_MAX: u64 = 2 * 1024 * 1024;
const LINKS_MAX: usize = 24;
pub const MANIFEST_CANDIDATES: [&str; 3] = ["app/src/main/AndroidManifest.xml", "src/main/AndroidManifest.xml", "android/app/src/main/AndroidManifest.xml"];

fn attr(a: &HashMap<String, String>, name: &str) -> Option<String> {
    a.get(&format!("android:{name}")).or_else(|| a.get(name)).cloned()
}

pub fn qualify(name: &str, pkg: Option<&str>) -> String {
    if name.starts_with('.') {
        return pkg.map(|p| format!("{p}{name}")).unwrap_or_else(|| name.to_string());
    }
    if !name.contains('.') {
        if let Some(p) = pkg.filter(|p| !p.is_empty()) {
            return format!("{p}.{name}");
        }
    }
    name.to_string()
}

#[derive(Default, Clone)]
struct DataTag {
    scheme: Option<String>,
    host: Option<String>,
    port: Option<String>,
    path: Option<String>,
    path_prefix: Option<String>,
    path_pattern: Option<String>,
}

fn uniq(v: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for x in v {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

fn nonempty(s: &Option<String>) -> Option<String> {
    s.clone().filter(|x| !x.is_empty())
}

fn filter_links(data: &[DataTag]) -> Vec<String> {
    static PATTERN: Js = Js::new(r"\.\*|\\\.", "g");
    let schemes = uniq(data.iter().filter_map(|d| nonempty(&d.scheme)));
    let hosts = uniq(data.iter().filter_map(|d| nonempty(&d.host).map(|h| match nonempty(&d.port) { Some(p) => format!("{h}:{p}"), None => h })));
    let paths = uniq(data.iter().filter_map(|d| {
        d.path
            .clone()
            .or_else(|| d.path_prefix.clone())
            .or_else(|| {
                nonempty(&d.path_pattern).map(|p| {
                    let mut out = String::new();
                    let mut last = 0;
                    for m in PATTERN.re().find_iter(&p) {
                        let r = m.range();
                        out.push_str(&p[last..r.start]);
                        out.push_str(if &p[r.clone()] == ".*" { "" } else { "." });
                        last = r.end;
                    }
                    out.push_str(&p[last..]);
                    out
                })
            })
            .filter(|p| !p.is_empty())
    }));
    let mut out = Vec::new();
    for s in &schemes {
        if hosts.is_empty() {
            out.push(format!("{s}://"));
            continue;
        }
        for h in &hosts {
            let host = h.strip_prefix("*.").unwrap_or(h);
            if paths.is_empty() {
                out.push(format!("{s}://{host}"));
            }
            for p in &paths {
                out.push(format!("{s}://{host}{}", if p.starts_with('/') { p.clone() } else { format!("/{p}") }));
            }
        }
    }
    out
}

pub fn parse_manifest(xml: &str) -> (Option<String>, Vec<Value>) {
    static COMMENT: Js = Js::new(r"<!--[\s\S]*?-->", "g");
    static ROOT: Js = Js::new(r"<manifest\b([^>]*)>", "");
    static ACT: Js = Js::new(r"<(activity|activity-alias)\b([^>]*?)(\/>|>([\s\S]*?)<\/\1\s*>)", "g");
    static FILTER: Js = Js::new(r"<intent-filter\b[^>]*>([\s\S]*?)<\/intent-filter\s*>", "g");
    static DATA: Js = Js::new(r"<data\b([^>]*)\/?>", "g");
    let text = COMMENT.replace_all(xml, "");
    let pkg = ROOT.exec(&text).and_then(|m| xml_attrs(g(&m, 1).unwrap_or("")).get("package").cloned());
    let mut activities = Vec::new();
    for m in ACT.all(&text) {
        let a = xml_attrs(g(&m, 2).unwrap_or(""));
        let Some(name) = attr(&a, "name").filter(|n| !n.is_empty()) else { continue };
        let body = g(&m, 4).unwrap_or("");
        let filters: Vec<String> = FILTER.all(body).iter().map(|f| g(f, 1).unwrap_or("").to_string()).collect();
        let names = |f: &str, tag: &str| -> Vec<String> {
            let re = regress::Regex::new(&format!(r"<{tag}\b([^>]*)/?>")).expect("regex");
            re.find_iter(f).map(|x| x.group(1).map(|r| attr(&xml_attrs(&f[r]), "name").unwrap_or_default()).unwrap_or_default()).collect()
        };
        let mut launcher = false;
        let mut links: Vec<String> = Vec::new();
        for f in &filters {
            let actions = names(f, "action");
            let cats = names(f, "category");
            if actions.iter().any(|a| a == "android.intent.action.MAIN") && cats.iter().any(|c| c == "android.intent.category.LAUNCHER") {
                launcher = true;
            }
            if !actions.iter().any(|a| a == "android.intent.action.VIEW") {
                continue;
            }
            let data: Vec<DataTag> = DATA
                .all(f)
                .iter()
                .map(|d| {
                    let x = xml_attrs(g(d, 1).unwrap_or(""));
                    DataTag { scheme: attr(&x, "scheme"), host: attr(&x, "host"), port: attr(&x, "port"), path: attr(&x, "path"), path_prefix: attr(&x, "pathPrefix"), path_pattern: attr(&x, "pathPattern") }
                })
                .collect();
            for l in filter_links(&data) {
                if !links.contains(&l) && links.len() < LINKS_MAX {
                    links.push(l);
                }
            }
        }
        let exported = match attr(&a, "exported") {
            None => !filters.is_empty(),
            Some(e) => e == "true",
        };
        activities.push(json!({"activity": qualify(&name, pkg.as_deref()), "exported": exported, "launcher": launcher, "links": links}));
    }
    (pkg, activities)
}

pub fn gradle_app_id(text: &str) -> Option<String> {
    static BLOCK: Js = Js::new(r"\/\*[\s\S]*?\*\/", "g");
    static LINE: Js = Js::new(r"^\s*\/\/.*$", "gm");
    static APP: Js = Js::new(r#"\bapplicationId\s*=?\s*["']([\w.]+)["']"#, "");
    static NS: Js = Js::new(r#"\bnamespace\s*=?\s*["']([\w.]+)["']"#, "");
    let clean = LINE.replace_all(&BLOCK.replace_all(text, ""), "");
    APP.exec(&clean).and_then(|m| g(&m, 1).map(String::from)).or_else(|| NS.exec(&clean).and_then(|m| g(&m, 1).map(String::from)))
}

pub fn values_locale(dir: &str) -> Option<String> {
    static VALUES: Js = Js::new(r"^values-(.+)$", "");
    static BCP: Js = Js::new(r"^b\+([a-zA-Z]{2,3}(?:\+[a-zA-Z0-9]{2,8})*)$", "");
    static SIMPLE: Js = Js::new(r"^([a-z]{2,3})(?:-r([A-Z]{2}))?$", "");
    let q = VALUES.exec(dir).and_then(|m| g(&m, 1).map(String::from))?;
    if let Some(b) = BCP.exec(&q) {
        return g(&b, 1).map(|s| s.replace('+', "-"));
    }
    let m = SIMPLE.exec(&q)?;
    let lang = g(&m, 1)?;
    Some(match g(&m, 2) {
        Some(r) => format!("{lang}-{r}"),
        None => lang.to_string(),
    })
}

fn read_small(file: &Path) -> Option<String> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() > MANIFEST_MAX {
        return None;
    }
    std::fs::read(file).ok().map(|b| String::from_utf8_lossy(&b).to_string())
}

fn list_dir(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    v.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    v
}

pub fn read_project_manifest(root: &str) -> Value {
    static ARB: Js = Js::new(r"^\w+?_([a-z]{2,3}(?:_[A-Z]{2})?)\.arb$", "");
    for rel in MANIFEST_CANDIDATES {
        let file = Path::new(root).join(rel);
        let Some(xml) = read_small(&file) else { continue };
        let (mut pkg, activities) = parse_manifest(&xml);
        let main = file.parent().unwrap_or(Path::new(""));
        let module = main.parent().and_then(Path::parent).unwrap_or(Path::new(""));
        if pkg.is_none() {
            for gname in ["build.gradle.kts", "build.gradle"] {
                if pkg.is_none() {
                    pkg = gradle_app_id(&read_small(&module.join(gname)).unwrap_or_default());
                }
            }
        }
        let mut locales: Vec<String> = Vec::new();
        for d in list_dir(&main.join("res")) {
            if let Some(l) = values_locale(&d) {
                if !locales.contains(&l) {
                    locales.push(l);
                }
            }
        }
        for f in list_dir(&Path::new(root).join("lib").join("l10n")) {
            if let Some(l) = ARB.exec(&f).and_then(|m| g(&m, 1).map(|s| s.replacen('_', "-", 1))) {
                if !locales.contains(&l) {
                    locales.push(l);
                }
            }
        }
        locales.sort();
        let acts: Vec<Value> = activities
            .into_iter()
            .map(|mut a| {
                let q = qualify(a["activity"].as_str().unwrap_or(""), pkg.as_deref());
                a["activity"] = json!(q);
                a
            })
            .collect();
        return json!({"ok": true, "manifest": rel, "pkg": pkg, "activities": acts, "locales": locales});
    }
    json!({"ok": false, "error": "no-manifest"})
}
