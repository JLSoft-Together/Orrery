use super::zip::extract_zip_filtered;
use crate::run::android::jsre::{dynamic, g, Js};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Coordinate {
    pub group: String,
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedArtifact {
    pub group: String,
    pub name: String,
    pub version: String,
    pub file: String,
}

#[derive(Debug, Default)]
pub struct Resolution {
    pub artifacts: Vec<CachedArtifact>,
    pub missing: Vec<String>,
}

const AAR_MAX_TOTAL: u64 = 256 * 1024 * 1024;
const READY: &str = ".ccm-ready";
const READY_VERSION: &str = "2";

pub fn safe_part(s: &str) -> bool {
    let mut cs = s.chars();
    let Some(first) = cs.next() else { return false };
    (first.is_ascii_alphanumeric() || first == '_') && cs.all(|c| c.is_ascii_alphanumeric() || "_.-+".contains(c)) && !s.contains("..")
}

pub fn static_version(v: Option<&str>) -> Option<String> {
    let v = v?;
    (!v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c)) && safe_part(v) && !v.ends_with('+')).then(|| v.to_string())
}

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let pa: Vec<&str> = a.split(['.', '-']).collect();
    let pb: Vec<&str> = b.split(['.', '-']).collect();
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(""), pb.get(i).copied().unwrap_or(""));
        let num = |s: &str| (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse::<f64>().unwrap_or(f64::MAX));
        let d = match (num(x), num(y)) {
            (Some(nx), Some(ny)) => nx.partial_cmp(&ny).unwrap_or(Ordering::Equal),
            _ => x.to_lowercase().cmp(&y.to_lowercase()).then_with(|| y.cmp(x)),
        };
        if d != Ordering::Equal {
            return d;
        }
    }
    Ordering::Equal
}

pub fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.is_dir())
}

pub fn is_file(p: &str) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.is_file())
}

fn join(parts: &[&str]) -> String {
    let mut p = PathBuf::new();
    for x in parts {
        p.push(x);
    }
    p.to_string_lossy().to_string()
}

fn files_root(gradle_home: &str) -> String {
    join(&[gradle_home, "caches", "modules-2", "files-2.1"])
}

fn read_dir_names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn version_files(gradle_home: &str, group: &str, name: &str, version: &str) -> Vec<String> {
    let dir = join(&[&files_root(gradle_home), group, name, version]);
    let mut out = Vec::new();
    for hash in read_dir_names(&dir) {
        for f in read_dir_names(&join(&[&dir, &hash])) {
            out.push(join(&[&dir, &hash, &f]));
        }
    }
    out
}

fn cached_versions(gradle_home: &str, group: &str, name: &str) -> Vec<String> {
    read_dir_names(&join(&[&files_root(gradle_home), group, name])).into_iter().filter(|v| safe_part(v)).collect()
}

fn candidate_versions(gradle_home: &str, c: &Coordinate) -> Vec<String> {
    let mut all = cached_versions(gradle_home, &c.group, &c.name);
    all.sort_by(|a, b| compare_versions(b, a));
    all.truncate(5);
    match &c.version {
        Some(v) => std::iter::once(v.clone()).chain(all.into_iter().filter(|x| x != v)).collect(),
        None => all,
    }
}

struct Metadata {
    deps: Vec<Coordinate>,
    file_name: Option<String>,
}

fn module_metadata(json: &Value, group: &str, version: &str) -> Option<Metadata> {
    let variants = json["variants"].as_array()?;
    let rank = |v: &Value| match v["attributes"]["org.jetbrains.kotlin.platform.type"].as_str() {
        Some("androidJvm") => 0,
        Some("jvm") => 1,
        None => 2,
        _ => 9,
    };
    let mut runtime: Vec<&Value> = variants
        .iter()
        .filter(|v| v["attributes"]["org.gradle.usage"] == "java-runtime" && v["attributes"]["org.gradle.category"].as_str().unwrap_or("library") == "library")
        .filter(|v| rank(v) < 9)
        .collect();
    runtime.sort_by_key(|v| rank(v));
    let best = runtime.first()?;
    let at = &best["available-at"];
    if let Some(module) = at["module"].as_str() {
        let v = at["version"].as_str().unwrap_or(version);
        let group = at["group"].as_str().unwrap_or(group);
        return Some(Metadata { deps: vec![Coordinate { group: group.into(), name: module.into(), version: static_version(Some(v)) }], file_name: None });
    }
    let mut deps = Vec::new();
    for d in best["dependencies"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let (Some(group), Some(module)) = (d["group"].as_str(), d["module"].as_str()) else { continue };
        if matches!(d["attributes"]["org.gradle.category"].as_str(), Some("platform" | "enforced-platform")) {
            continue;
        }
        let ver = ["strictly", "requires", "prefers"].iter().find_map(|k| d["version"].get(*k).filter(|x| !x.is_null()));
        deps.push(Coordinate { group: group.into(), name: module.into(), version: static_version(ver.and_then(Value::as_str)) });
    }
    let file_name = best["files"].as_array().and_then(|a| a.iter().find_map(|f| f["name"].as_str())).map(String::from);
    Some(Metadata { deps, file_name })
}

fn strip_blocks(text: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        out.push_str(&rest[..i]);
        match rest[i + open.len()..].find(close) {
            Some(j) => rest = &rest[i + open.len() + j + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn tag(block: &str, name: &str) -> Option<String> {
    let re = dynamic(&format!(r"<{name}>\s*([^<]*?)\s*</{name}>"))?;
    let m = re.find(block)?;
    m.group(1).map(|r| block[r].to_string())
}

fn substitute(s: &str, props: &HashMap<String, String>) -> Option<String> {
    let s = s.trim();
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        match after.find('}') {
            Some(j) if j > 0 && !after[..j].contains('}') => {
                out.push_str(props.get(&after[..j])?);
                rest = &after[j + 1..];
            }
            _ => {
                out.push_str("${");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Some(out)
}

pub fn pom_dependencies(pom: &str, version: &str) -> Vec<Coordinate> {
    static PROPS: Js = Js::new(r"<([\w.\-]+)>\s*([^<]*?)\s*<\/\1>", "g");
    static DEP: Js = Js::new(r"<dependency>([\s\S]*?)<\/dependency>", "g");
    static COMMENT: Js = Js::new(r"<!--[\s\S]*?-->", "g");
    let text = COMMENT.replace_all(pom, "");
    let text = strip_blocks(&strip_blocks(&strip_blocks(&text, "<dependencyManagement>", "</dependencyManagement>"), "<build>", "</build>"), "<profiles>", "</profiles>");
    let mut props: HashMap<String, String> = [("project.version", version), ("version", version), ("pom.version", version)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    if let Some(i) = text.find("<properties>") {
        if let Some(j) = text[i..].find("</properties>") {
            for m in PROPS.all(&text[i + 12..i + j]) {
                props.insert(g(&m, 1).unwrap_or("").into(), g(&m, 2).unwrap_or("").into());
            }
        }
    }
    let list = text.find("<dependencies>").and_then(|i| text[i + 14..].find("</dependencies>").map(|j| text[i + 14..i + 14 + j].to_string())).unwrap_or_default();
    let mut deps = Vec::new();
    for m in DEP.all(&list) {
        let b = g(&m, 1).unwrap_or("");
        let scope = tag(b, "scope").unwrap_or_else(|| "compile".into());
        let scope = scope.trim();
        if scope != "compile" && scope != "runtime" {
            continue;
        }
        if tag(b, "optional").unwrap_or_default().trim() == "true" || tag(b, "type").unwrap_or_else(|| "jar".into()).trim() == "pom" {
            continue;
        }
        let group = tag(b, "groupId").and_then(|s| substitute(&s, &props));
        let name = tag(b, "artifactId").and_then(|s| substitute(&s, &props));
        let ver = tag(b, "version").and_then(|s| substitute(&s, &props));
        if let (Some(group), Some(name)) = (group.filter(|s| !s.is_empty()), name.filter(|s| !s.is_empty())) {
            deps.push(Coordinate { group, name, version: static_version(ver.as_deref()) });
        }
    }
    deps
}

fn base(f: &str) -> &str {
    Path::new(f).file_name().and_then(|n| n.to_str()).unwrap_or("")
}

fn metadata_of(group: &str, name: &str, version: &str, files: &[String]) -> Option<Metadata> {
    if let Some(f) = files.iter().find(|f| base(f) == format!("{name}-{version}.module")) {
        if let Some(json) = std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
            if let Some(m) = module_metadata(&json, group, version) {
                return Some(m);
            }
        }
    }
    let pom = files.iter().find(|f| base(f) == format!("{name}-{version}.pom")).and_then(|f| std::fs::read_to_string(f).ok())?;
    Some(Metadata { deps: pom_dependencies(&pom, version), file_name: None })
}

fn nearest_metadata(gradle_home: &str, group: &str, name: &str, version: &str) -> Option<Metadata> {
    let all: Vec<String> = cached_versions(gradle_home, group, name).into_iter().filter(|v| v != version).collect();
    let mut older: Vec<String> = all.iter().filter(|v| compare_versions(v, version) == Ordering::Less).cloned().collect();
    older.sort_by(|a, b| compare_versions(b, a));
    let mut newer: Vec<String> = all.iter().filter(|v| compare_versions(v, version) == Ordering::Greater).cloned().collect();
    newer.sort_by(|a, b| compare_versions(a, b));
    for v in older.into_iter().chain(newer).take(6) {
        if let Some(m) = metadata_of(group, name, &v, &version_files(gradle_home, group, name, &v)) {
            return Some(Metadata { deps: m.deps, file_name: None });
        }
    }
    None
}

pub fn resolve_cached(gradle_home: &str, roots: &[Coordinate], max: usize) -> Resolution {
    let mut chosen: Vec<(String, String, Option<String>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut missing: Vec<String> = Vec::new();
    let mut queue: std::collections::VecDeque<Coordinate> = roots.iter().cloned().collect();
    let mut steps = 0;
    while let Some(c) = queue.pop_front() {
        if steps >= max * 4 {
            break;
        }
        steps += 1;
        if !safe_part(&c.group) || !safe_part(&c.name) {
            continue;
        }
        let key = format!("{}:{}", c.group, c.name);
        let prev = index.get(&key).map(|i| chosen[*i].1.clone());
        if let Some(pv) = &prev {
            if c.version.as_deref().is_none_or(|v| compare_versions(v, pv) != Ordering::Greater) {
                continue;
            }
        }
        if prev.is_none() && chosen.len() >= max {
            continue;
        }
        let mut hit: Option<(String, Vec<String>, Option<String>)> = None;
        for v in candidate_versions(gradle_home, &c) {
            let files = version_files(gradle_home, &c.group, &c.name, &v);
            if files.is_empty() {
                continue;
            }
            let file = files.iter().find(|f| base(f) == format!("{}-{v}.aar", c.name)).or_else(|| files.iter().find(|f| base(f) == format!("{}-{v}.jar", c.name))).cloned();
            if file.is_some() {
                hit = Some((v, files, file));
                break;
            }
            if hit.is_none() {
                hit = Some((v, files, None));
            }
        }
        let Some((hv, hfiles, hfile)) = hit else {
            if prev.is_none() && !missing.contains(&key) {
                missing.push(key);
            }
            continue;
        };
        if let Some(pv) = &prev {
            if compare_versions(&hv, pv) != Ordering::Greater {
                continue;
            }
        }
        let meta = metadata_of(&c.group, &c.name, &hv, &hfiles).or_else(|| nearest_metadata(gradle_home, &c.group, &c.name, &hv));
        let named = meta.as_ref().and_then(|m| m.file_name.as_ref()).and_then(|n| hfiles.iter().find(|f| base(f) == n.as_str())).cloned();
        let file = named.or(hfile);
        match index.get(&key) {
            Some(i) => chosen[*i] = (key.clone(), hv, file),
            None => {
                index.insert(key.clone(), chosen.len());
                chosen.push((key.clone(), hv, file));
            }
        }
        missing.retain(|m| *m != key);
        if let Some(m) = meta {
            queue.extend(m.deps);
        }
    }
    let artifacts = chosen
        .into_iter()
        .filter_map(|(key, version, file)| {
            let file = file?;
            let (group, name) = key.split_once(':')?;
            Some(CachedArtifact { group: group.into(), name: name.into(), version, file })
        })
        .collect();
    Resolution { artifacts, missing }
}

fn keep_entry(n: &str) -> bool {
    n.starts_with("res/") || n == "R.txt" || n == "AndroidManifest.xml" || n == "classes.jar" || (n.starts_with("libs/") && n.ends_with(".jar") && !n[5..].contains('/'))
}

pub fn extract_aar(file: &str, dest: &str) -> Result<(Option<String>, Vec<String>), String> {
    let marker = join(&[dest, READY]);
    let ready = |m: &str| std::fs::read_to_string(m).is_ok_and(|t| t == READY_VERSION);
    if !ready(&marker) {
        let mut rnd = [0u8; 4];
        let _ = getrandom::fill(&mut rnd);
        let tmp = format!("{dest}.partial-{}", rnd.iter().map(|b| format!("{b:02x}")).collect::<String>());
        let done = (|| -> Result<(), String> {
            std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            extract_zip_filtered(Path::new(file), Path::new(&tmp), AAR_MAX_TOTAL, &keep_entry).map_err(|e| e.code().to_string())?;
            std::fs::write(join(&[&tmp, READY]), READY_VERSION).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_dir_all(dest);
            std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
        })();
        if let Err(err) = done {
            let _ = std::fs::remove_dir_all(&tmp);
            if !ready(&marker) {
                return Err(err);
            }
        }
    }
    let res = join(&[dest, "res"]);
    let mut jars = Vec::new();
    if is_file(&join(&[dest, "classes.jar"])) {
        jars.push(join(&[dest, "classes.jar"]));
    }
    for f in read_dir_names(&join(&[dest, "libs"])) {
        if f.ends_with(".jar") {
            jars.push(join(&[dest, "libs", &f]));
        }
    }
    Ok((is_dir(&res).then_some(res), jars))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_order_numbers_then_text() {
        assert_eq!(compare_versions("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Less);
        assert_eq!(compare_versions("1.0.0-alpha", "1.0.0-beta"), Ordering::Less);
    }

    #[test]
    fn aar_keeps_res_classes_and_libs_and_is_reused() {
        let d = std::env::temp_dir().join(format!("ccm-aar-{}", crate::util::uuid()));
        std::fs::create_dir_all(&d).unwrap();
        let zip = super::super::zip::tests::build_zip(&[("res/values/a.xml", b"<r/>", true), ("classes.jar", b"j", false), ("libs/x.jar", b"x", false), ("libs/sub/y.jar", b"y", false), ("assets/skip.bin", b"s", false), ("R.txt", b"", false)]);
        let aar = d.join("a.aar");
        std::fs::write(&aar, zip).unwrap();
        let dest = d.join("out/g/a-1.0");
        let (res, jars) = extract_aar(&aar.to_string_lossy(), &dest.to_string_lossy()).unwrap();
        assert_eq!(res, Some(dest.join("res").to_string_lossy().to_string()));
        assert_eq!(jars, vec![dest.join("classes.jar").to_string_lossy().to_string(), dest.join("libs").join("x.jar").to_string_lossy().to_string()]);
        assert!(!dest.join("assets").exists() && !dest.join("libs/sub").exists());
        assert_eq!(std::fs::read_to_string(dest.join(READY)).unwrap(), READY_VERSION);
        std::fs::remove_file(&aar).unwrap();
        assert_eq!(extract_aar(&aar.to_string_lossy(), &dest.to_string_lossy()).unwrap().1.len(), 2);
        std::fs::write(&aar, b"not a zip").unwrap();
        std::fs::remove_dir_all(&dest).unwrap();
        assert!(extract_aar(&aar.to_string_lossy(), &dest.to_string_lossy()).is_err());
        assert!(!d.join("out/g").read_dir().unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("partial")));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn static_versions() {
        assert_eq!(static_version(Some("1.2.3")), Some("1.2.3".into()));
        assert_eq!(static_version(Some("1.+")), None);
        assert_eq!(static_version(Some("[1,2)")), None);
        assert_eq!(static_version(None), None);
    }

    #[test]
    fn pom_scopes_and_properties() {
        let pom = "<project><properties><v>2.0</v></properties><dependencies>\
            <dependency><groupId>a</groupId><artifactId>b</artifactId><version>${v}</version></dependency>\
            <dependency><groupId>t</groupId><artifactId>x</artifactId><version>1</version><scope>test</scope></dependency>\
            <dependency><groupId>o</groupId><artifactId>y</artifactId><version>1</version><optional>true</optional></dependency>\
            <dependency><groupId>u</groupId><artifactId>z</artifactId><version>${missing}</version></dependency>\
            </dependencies></project>";
        let deps = pom_dependencies(pom, "9");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], Coordinate { group: "a".into(), name: "b".into(), version: Some("2.0".into()) });
        assert_eq!(deps[1].name, "z");
        assert_eq!(deps[1].version, None);
    }
}
