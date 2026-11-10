use super::gradle_cache::{extract_aar, is_dir, is_file, resolve_cached, safe_part, static_version, CachedArtifact, Coordinate};
use super::gradle_graph::{closure, read_build_script, script_graph, Module, Project};
use super::project_model::{Payload, CAP_AAR_RES_DIRS, CAP_ACTIVITY_THEMES, CAP_CLASSPATH, CAP_RES_DIRS, PACKAGE_RE, THEME_RE};
use crate::run::android::catalog::parse_catalog_libraries;
use crate::run::android::detect::strip_comments;
use crate::run::android::jsre::{dynamic, g, Js};
use crate::run::android::pathx;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const MAX_LIBS: usize = 800;
const AAR_WORKERS: usize = 4;

pub struct SourceOpts<'a> {
    pub win: bool,
    pub gradle_home: &'a str,
    pub aar_cache_dir: &'a str,
    pub kit: Option<&'a Payload>,
    pub prebuilt: bool,
}

#[derive(Debug, Clone)]
pub struct SourceModel {
    pub payload: Payload,
    pub themes: Vec<String>,
    pub warnings: Vec<String>,
    pub prebuilt_at: Option<f64>,
}

fn artifact_of(kit_name: &str) -> String {
    static R: Js = Js::new(r"-\d[\w.\-]*$", "");
    R.re().replace(kit_name, "")
}

pub fn script_dependencies(script: &str, catalog_toml: Option<&str>) -> Vec<Coordinate> {
    static RE: Js = Js::new(r#"\b([A-Za-z]+)\s*\(?\s*(?:["']([^"'\s]+)["']|\blibs\.([\w.]+))"#, "g");
    static CONFIG: Js = Js::new(r"^(?:[a-z][A-Za-z0-9]*)?(?:implementation|Implementation|api|Api|compileOnly|CompileOnly)$", "");
    static SKIP: Js = Js::new(r"^(?:test|androidTest|testFixtures|lint)", "");
    let catalog = catalog_toml.map(parse_catalog_libraries);
    let mut out = Vec::new();
    let mut add = |group: &str, name: &str, version: Option<&str>| {
        if safe_part(group) && safe_part(name) {
            out.push(Coordinate { group: group.into(), name: name.into(), version: static_version(version) });
        }
    };
    for m in RE.all(&strip_comments(script)) {
        let config = g(&m, 1).unwrap_or("");
        if !CONFIG.test(config) || SKIP.test(config) {
            continue;
        }
        if let Some(coord) = g(&m, 2) {
            let mut it = coord.split('@').next().unwrap_or("").split(':');
            if let (Some(group), Some(name)) = (it.next().filter(|s| !s.is_empty()), it.next().filter(|s| !s.is_empty())) {
                add(group, name, it.next());
            }
            continue;
        }
        let Some(cat) = &catalog else { continue };
        let raw = g(&m, 3).unwrap_or("");
        let key = raw.strip_suffix(".get").unwrap_or(raw).to_lowercase().replace(['-', '_', '.'], ".");
        if let Some(b) = key.strip_prefix("bundles.") {
            for k in cat.bundles.get(b).map(Vec::as_slice).unwrap_or(&[]) {
                if let Some(l) = cat.libraries.get(k) {
                    add(&l.group, &l.name, l.version.as_deref());
                }
            }
        } else if !key.starts_with("plugins.") && !key.starts_with("versions.") {
            if let Some(l) = cat.libraries.get(&key) {
                add(&l.group, &l.name, l.version.as_deref());
            }
        }
    }
    out
}

struct Facts {
    pkg: Option<String>,
    app_theme: Option<String>,
    activity_themes: Vec<(String, String)>,
}

fn manifest_facts(text: &str, namespace: Option<&str>) -> Facts {
    static PKG: Js = Js::new(r#"<manifest\b[^>]*?\bpackage\s*=\s*["']([^"']+)["']"#, "");
    static TAGS: Js = Js::new(r"<(application|activity|activity-alias)\b([^>]*)>", "g");
    let attr = |tag: &str, name: &str| -> Option<String> {
        let re = dynamic(&format!(r#"\bandroid:{name}\s*=\s*["']([^"']+)["']"#))?;
        let m = re.find(tag)?;
        m.group(1).map(|r| tag[r].trim().to_string())
    };
    let pkg = PKG.exec(text).and_then(|m| g(&m, 1).map(String::from));
    let ns = namespace.map(String::from).or_else(|| pkg.clone());
    let mut app_theme = None;
    let mut activity_themes: Vec<(String, String)> = Vec::new();
    for m in TAGS.all(text) {
        let body = g(&m, 2).unwrap_or("");
        let Some(theme) = attr(body, "theme").filter(|t| THEME_RE.test(t)) else { continue };
        if g(&m, 1) == Some("application") {
            app_theme.get_or_insert(theme);
            continue;
        }
        let raw = attr(body, "name");
        let name = match (&raw, &ns) {
            (Some(r), Some(ns)) if r.starts_with('.') || !r.contains('.') => Some(format!("{ns}{}{r}", if r.starts_with('.') { "" } else { "." })),
            _ => raw,
        };
        if let Some(n) = name.filter(|n| PACKAGE_RE.test(n)) {
            if activity_themes.len() < CAP_ACTIVITY_THEMES {
                match activity_themes.iter_mut().find(|(k, _)| *k == n) {
                    Some(e) => e.1 = theme,
                    None => activity_themes.push((n, theme)),
                }
            }
        }
    }
    Facts { pkg: pkg.filter(|p| PACKAGE_RE.test(p)), app_theme, activity_themes }
}

fn mtime_of(p: &str) -> f64 {
    std::fs::metadata(p).map(|m| crate::util::mtime_ms(&m)).unwrap_or(0.0)
}

fn subdirs(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().filter(|e| std::fs::metadata(e.path()).is_ok_and(|m| m.is_dir())).map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn pick_variant(found: &[(String, f64)], want: Option<&str>) -> Option<String> {
    if found.is_empty() {
        return None;
    }
    if let Some(w) = want {
        if found.iter().any(|f| f.0 == w) {
            return Some(w.to_string());
        }
    }
    let debug: Vec<&(String, f64)> = found.iter().filter(|f| f.0.to_lowercase().ends_with("debug")).collect();
    let pool: Vec<&(String, f64)> = if debug.is_empty() { found.iter().collect() } else { debug };
    let mut best = pool[0];
    for f in &pool {
        if f.1 > best.1 {
            best = f;
        }
    }
    Some(best.0.clone())
}

fn join(parts: &[&str]) -> String {
    pathx::join(pathx::host_win(), parts)
}

pub struct RJar {
    pub file: String,
    pub variant: String,
    pub mtime: f64,
}

pub fn app_r_jar(app_dir: &str) -> Option<RJar> {
    let base = join(&[app_dir, "build", "intermediates", "compile_and_runtime_not_namespaced_r_class_jar"]);
    let mut found: Vec<RJar> = Vec::new();
    for variant in subdirs(&base) {
        let vdir = join(&[&base, &variant]);
        let mut files = vec![join(&[&vdir, "R.jar"])];
        files.extend(subdirs(&vdir).iter().map(|t| join(&[&vdir, t, "R.jar"])));
        for file in files {
            if is_file(&file) {
                let mtime = mtime_of(&file);
                found.push(RJar { file, variant: variant.clone(), mtime });
            }
        }
    }
    let list: Vec<(String, f64)> = found.iter().map(|f| (f.variant.clone(), f.mtime)).collect();
    let v = pick_variant(&list, None)?;
    let mut mine: Vec<RJar> = found.into_iter().filter(|f| f.variant == v).collect();
    mine.sort_by(|a, b| b.mtime.partial_cmp(&a.mtime).unwrap_or(std::cmp::Ordering::Equal));
    mine.into_iter().next()
}

pub fn module_classes(module_dir: &str, want: Option<&str>) -> Vec<String> {
    let build = join(&[module_dir, "build"]);
    let kotlin = join(&[&build, "tmp", "kotlin-classes"]);
    let javac = join(&[&build, "intermediates", "javac"]);
    let built_in = join(&[&build, "intermediates", "built_in_kotlinc"]);
    let mut found: Vec<(String, f64)> = Vec::new();
    for root in [&kotlin, &javac, &built_in] {
        for v in subdirs(root) {
            let m = mtime_of(&join(&[root, &v]));
            found.push((v, m));
        }
    }
    let mut out = Vec::new();
    if let Some(v) = pick_variant(&found, want) {
        if is_dir(&join(&[&kotlin, &v])) {
            out.push(join(&[&kotlin, &v]));
        }
        for root in [&javac, &built_in] {
            for task in subdirs(&join(&[root, &v])) {
                let classes = join(&[root, &v, &task, "classes"]);
                if is_dir(&classes) {
                    out.push(classes);
                }
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    let jars = join(&[&build, "intermediates", "runtime_library_classes_jar"]);
    let jar_found: Vec<(String, f64)> = subdirs(&jars).into_iter().map(|v| (v.clone(), mtime_of(&join(&[&jars, &v])))).collect();
    let Some(jv) = pick_variant(&jar_found, want) else { return Vec::new() };
    for task in subdirs(&join(&[&jars, &jv])) {
        let f = join(&[&jars, &jv, &task, "classes.jar"]);
        if is_file(&f) {
            out.push(f);
        }
    }
    out
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    v.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

fn extract_all(libs: &[CachedArtifact], cache_dir: &str) -> (Vec<(String, String)>, Vec<String>, Vec<String>) {
    type One = (Option<(String, String)>, Vec<String>, Option<String>);
    let slots: Mutex<Vec<Option<One>>> = Mutex::new((0..libs.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..AAR_WORKERS.min(libs.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(a) = libs.get(i) else { return };
                let one: One = if a.file.ends_with(".jar") {
                    (None, vec![a.file.clone()], None)
                } else {
                    match extract_aar(&a.file, &join(&[cache_dir, &a.group, &format!("{}-{}", a.name, a.version)])) {
                        Ok((res, jars)) => (res.map(|d| (d, format!("{}:{}:{}", a.group, a.name, a.version))), jars.into_iter().take(8).collect(), None),
                        Err(err) => (None, Vec::new(), Some(format!("AAR {}:{}:{} unreadable: {err}", a.group, a.name, a.version))),
                    }
                };
                slots.lock().unwrap()[i] = Some(one);
            });
        }
    });
    let (mut res, mut jars, mut warnings) = (Vec::new(), Vec::new(), Vec::new());
    for one in slots.into_inner().unwrap().into_iter().flatten() {
        if let Some(r) = one.0 {
            res.push(r);
        }
        jars.extend(one.1);
        warnings.extend(one.2);
    }
    (res, jars, warnings)
}

pub fn build_source_model(project: &Project, app: &str, file: &str, opts: &SourceOpts) -> Option<SourceModel> {
    let win = opts.win;
    let mut warnings: Vec<String> = Vec::new();
    let by_path: BTreeMap<&str, &Module> = project.modules.iter().map(|m| (m.path.as_str(), m)).collect();
    let mut owner: Option<&Module> = None;
    for m in &project.modules {
        if pathx::is_within(&m.dir, file, win) && owner.is_none_or(|o| m.dir.len() > o.dir.len()) {
            owner = Some(m);
        }
    }
    let graph = script_graph(project);
    let mut order = if by_path.contains_key(app) { closure(&graph, app) } else { Vec::new() };
    if let Some(o) = owner {
        if !order.contains(&o.path) {
            order.push(o.path.clone());
        }
    }
    let mods: Vec<&Module> = order.iter().filter_map(|p| by_path.get(p.as_str()).copied()).collect();
    let mut res_dirs: Vec<String> = Vec::new();
    for m in mods.iter().rev() {
        let dir = join(&[&m.dir, "src", "main", "res"]);
        if is_dir(&dir) {
            res_dirs.push(dir);
        }
    }
    if let Some(o) = owner {
        let rel: Vec<String> = crate::preview::paths::relative(&o.dir, file).split(['/', '\\']).map(String::from).collect();
        if rel.len() > 2 && rel[0] == "src" && !rel[1].is_empty() && rel[1] != "main" && rel[2] == "res" {
            let dir = join(&[&o.dir, "src", &rel[1], "res"]);
            if is_dir(&dir) {
                res_dirs.push(dir);
            }
        }
    }
    if res_dirs.is_empty() {
        return None;
    }
    let app_module = by_path.get(app).copied().or(owner);
    let r_jar = if opts.prebuilt { app_module.and_then(|m| app_r_jar(&m.dir)) } else { None };
    let catalog = std::fs::read_to_string(join(&[&project.dir, "gradle", "libs.versions.toml"])).ok();
    let scripts: Vec<Option<String>> = mods.iter().map(|m| read_build_script(&m.dir)).collect();
    let mut roots: Vec<Coordinate> = Vec::new();
    for s in scripts.iter().flatten() {
        roots.extend(script_dependencies(s, catalog.as_deref()));
    }
    let kit_res: &[(String, String)] = opts.kit.map(|k| k.aar_res_dirs.as_slice()).unwrap_or(&[]);
    let kit_artifacts: HashSet<String> = kit_res.iter().map(|(_, n)| artifact_of(n)).collect();
    let resolved = resolve_cached(opts.gradle_home, &roots, MAX_LIBS);
    let libs: Vec<CachedArtifact> = if r_jar.is_some() { resolved.artifacts.clone() } else { resolved.artifacts.iter().filter(|a| !kit_artifacts.contains(&a.name)).cloned().collect() };
    if !resolved.missing.is_empty() {
        let shown: Vec<&str> = resolved.missing.iter().take(10).map(String::as_str).collect();
        warnings.push(format!("not in the Gradle cache (sync or build the project once): {}{}", shown.join(", "), if resolved.missing.len() > 10 { ", …" } else { "" }));
    }
    let (project_res, lib_jars, aar_warnings) = extract_all(&libs, opts.aar_cache_dir);
    warnings.extend(aar_warnings);
    let mut class_dirs: Vec<String> = Vec::new();
    if let Some(r) = &r_jar {
        for m in &mods {
            class_dirs.extend(module_classes(&m.dir, Some(&r.variant)));
        }
    }
    let app_script = app_module
        .map(|am| {
            let own = mods.iter().position(|m| m.path == am.path).and_then(|i| scripts[i].clone());
            strip_comments(&own.or_else(|| read_build_script(&am.dir)).unwrap_or_default())
        })
        .unwrap_or_default();
    static NS: Js = Js::new(r#"\bnamespace\s*=?\s*["']([\w.]+)["']"#, "");
    let ns_raw = NS.exec(&app_script).and_then(|m| g(&m, 1).map(String::from));
    let manifest_text = app_module.and_then(|m| m.manifest.as_ref()).and_then(|f| std::fs::read(f).ok()).map(|b| String::from_utf8_lossy(&b).into_owned());
    let facts = manifest_text.map(|t| manifest_facts(&t, ns_raw.as_deref())).unwrap_or(Facts { pkg: None, app_theme: None, activity_themes: Vec::new() });
    let namespace = ns_raw.clone().filter(|n| PACKAGE_RE.test(n)).or_else(|| facts.pkg.clone()).or_else(|| opts.kit.map(|k| k.package_name.clone())).unwrap_or_else(|| "dev.ccm.source".into());
    let sdk_of = |name: &str| -> Option<u32> {
        let re = dynamic(&format!(r"\b{name}(?:Version)?\s*[=(]?\s*(\d{{1,3}})\b"))?;
        let m = re.find(&app_script)?;
        let v: u32 = app_script[m.group(1)?].parse().ok()?;
        (v > 0).then_some(v)
    };
    let have: HashSet<&str> = libs.iter().map(|a| a.name.as_str()).collect();
    let kit_rest: Vec<(String, String)> = kit_res.iter().filter(|(_, n)| !have.contains(artifact_of(n).as_str())).cloned().collect();
    let aar_all: Vec<(String, String)> = if r_jar.is_some() { project_res.into_iter().chain(kit_rest).collect() } else { kit_rest.into_iter().chain(project_res).collect() };
    let mut classpath: Vec<String> = class_dirs;
    classpath.extend(lib_jars);
    if let Some(k) = opts.kit {
        classpath.extend(k.classpath.iter().cloned());
    }
    let mut classpath = dedup(classpath);
    classpath.truncate(CAP_CLASSPATH);
    res_dirs.truncate(CAP_RES_DIRS);
    let mut aar_all = aar_all;
    aar_all.truncate(CAP_AAR_RES_DIRS);
    let mut payload = Payload { res_dirs, aar_res_dirs: aar_all, classpath, package_name: namespace.clone(), namespace, activity_themes: facts.activity_themes.clone(), ..Default::default() };
    payload.r_jar = r_jar.as_ref().map(|r| r.file.clone()).or_else(|| opts.kit.and_then(|k| k.r_jar.clone()));
    payload.app_theme = facts.app_theme.clone().or_else(|| opts.kit.and_then(|k| k.app_theme.clone()));
    payload.min_sdk = sdk_of("minSdk");
    payload.target_sdk = sdk_of("targetSdk");
    let mut sample: Vec<String> = Vec::new();
    for m in [owner, app_module].into_iter().flatten() {
        let d = join(&[&m.dir, "sampledata"]);
        if is_dir(&d) && !sample.contains(&d) {
            sample.push(d);
        }
    }
    payload.sample_data_dirs = sample;
    let mut themes: Vec<String> = Vec::new();
    for t in facts.app_theme.iter().chain(facts.activity_themes.iter().map(|(_, t)| t)) {
        if !themes.contains(t) {
            themes.push(t.clone());
        }
    }
    Some(SourceModel { payload, themes, warnings, prebuilt_at: r_jar.map(|r| r.mtime) })
}
