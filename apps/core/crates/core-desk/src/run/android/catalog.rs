use super::detect::split_lines;
use super::jsre::{dynamic, g, Js};
use std::collections::BTreeMap;

pub fn norm_key(k: &str) -> String {
    k.to_lowercase().replace(['-', '_', '.'], ".")
}

static COMMENT_TAIL: Js = Js::new(r"\s+#.*$", "");
static HEAD: Js = Js::new(r"^\[([^\[\]]+)\]$", "");
static ENTRY: Js = Js::new(r#"^["']?([\w.\-]+)["']?\s*=\s*(.+)$"#, "");

type TomlLine = (Option<String>, Option<(String, String)>);

fn toml_lines(toml: &str) -> Vec<TomlLine> {
    split_lines(toml)
        .into_iter()
        .map(|raw| {
            let line = COMMENT_TAIL.re().replace(raw, "").trim().to_string();
            if let Some(h) = HEAD.exec(&line) {
                return (Some(g(&h, 1).unwrap_or("").trim().to_string()), None);
            }
            (None, ENTRY.exec(&line).map(|m| (g(&m, 1).unwrap_or("").to_string(), g(&m, 2).unwrap_or("").to_string())))
        })
        .collect()
}

pub fn parse_catalog_plugins(toml: &str) -> BTreeMap<String, String> {
    static ID: Js = Js::new(r#"\bid\s*=\s*["']([^"']+)["']"#, "");
    static SHORT: Js = Js::new(r#"^["']([^"':]+)(?::[^"']*)?["']"#, "");
    let mut out = BTreeMap::new();
    let mut section = String::new();
    for (head, entry) in toml_lines(toml) {
        if let Some(h) = head {
            section = h;
            continue;
        }
        if section != "plugins" {
            continue;
        }
        let Some((key, value)) = entry else { continue };
        let id = ID.exec(&value).and_then(|m| g(&m, 1).map(String::from)).or_else(|| SHORT.exec(&value).and_then(|m| g(&m, 1).map(String::from)));
        if let Some(id) = id {
            out.insert(norm_key(&key), id.trim().to_string());
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogLibrary {
    pub group: String,
    pub name: String,
    pub version: Option<String>,
}

pub struct Catalog {
    pub libraries: BTreeMap<String, CatalogLibrary>,
    pub bundles: BTreeMap<String, Vec<String>>,
}

fn attr(value: &str, name: &str) -> Option<String> {
    let re = dynamic(&format!(r#"(?:^|[\s{{,]){}\s*=\s*["']([^"']+)["']"#, name.replacen('.', "\\.", 1)))?;
    let m = re.find(value)?;
    m.group(1).map(|r| value[r].to_string())
}

pub fn parse_catalog_libraries(toml: &str) -> Catalog {
    static QUOTED: Js = Js::new(r#"^["']([^"']+)["']$"#, "");
    static RICH: Js = Js::new(r#"\b(?:strictly|require|prefer)\s*=\s*["']([^"']+)["']"#, "");
    static BUNDLE: Js = Js::new(r#"["']([\w.\-]+)["']"#, "g");
    static SHORT: Js = Js::new(r#"^["']([^"':]+):([^"':]+)(?::([^"']+))?["']$"#, "");
    static INLINE_RICH: Js = Js::new(r#"\bversion\s*=\s*\{[^}]*\b(?:strictly|require|prefer)\s*=\s*["']([^"']+)["']"#, "");
    let mut versions: BTreeMap<String, String> = BTreeMap::new();
    let mut raw: Vec<(String, String)> = Vec::new();
    let mut bundles = BTreeMap::new();
    let mut section = String::new();
    for (head, entry) in toml_lines(toml) {
        if let Some(h) = head {
            section = h;
            continue;
        }
        let Some((key, value)) = entry else { continue };
        let value = value.trim().to_string();
        match section.as_str() {
            "versions" => {
                let v = QUOTED.exec(&value).and_then(|m| g(&m, 1).map(String::from)).or_else(|| RICH.exec(&value).and_then(|m| g(&m, 1).map(String::from)));
                if let Some(v) = v {
                    versions.insert(key, v);
                }
            }
            "libraries" => raw.push((key, value)),
            "bundles" => {
                bundles.insert(norm_key(&key), BUNDLE.all(&value).iter().map(|b| norm_key(g(b, 1).unwrap_or(""))).collect());
            }
            _ => {}
        }
    }
    let mut libraries = BTreeMap::new();
    for (key, value) in raw {
        let lib = if let Some(s) = SHORT.exec(&value) {
            Some(CatalogLibrary { group: g(&s, 1).unwrap_or("").into(), name: g(&s, 2).unwrap_or("").into(), version: g(&s, 3).map(String::from) })
        } else {
            let module: Option<Vec<String>> = attr(&value, "module").map(|m| m.split(':').map(String::from).collect());
            let group = module.as_ref().and_then(|m| m.first().cloned()).or_else(|| attr(&value, "group"));
            let name = module.as_ref().and_then(|m| m.get(1).cloned()).or_else(|| attr(&value, "name"));
            let version = match attr(&value, "version.ref") {
                Some(r) if !r.is_empty() => versions.get(&r).cloned(),
                _ => attr(&value, "version").or_else(|| INLINE_RICH.exec(&value).and_then(|m| g(&m, 1).map(String::from))),
            };
            match (group, name) {
                (Some(group), Some(name)) if !group.is_empty() && !name.is_empty() => Some(CatalogLibrary { group, name, version }),
                _ => None,
            }
        };
        if let Some(lib) = lib {
            libraries.insert(norm_key(&key), lib);
        }
    }
    Catalog { libraries, bundles }
}
