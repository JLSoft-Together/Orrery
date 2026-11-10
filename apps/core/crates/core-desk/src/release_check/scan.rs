use super::rules::*;
use super::store::{Memory, Store};
use crate::preview::paths::is_within;
use crate::run::android::detect::detect_android;
use crate::run::android::jsre::Js;
use crate::util::basename;
use serde_json::{json, Value};
use std::path::Path;

const TEXT_MAX: u64 = 1024 * 1024;
const WALK_FILES: usize = 5000;
const WALK_BYTES: u64 = 48 * 1024 * 1024;
const APPS_MAX: usize = 8;
const ADMOB_MAX: usize = 40;
const SKIP: [&str; 8] = ["build", "node_modules", ".dart_tool", "Pods", ".gradle", ".git", ".idea", ".cxx"];

fn skip(name: &str) -> bool {
    SKIP.contains(&name) || name == "generated"
}

pub fn read_text(file: &str) -> Option<String> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() > TEXT_MAX {
        return None;
    }
    std::fs::read(file).ok().map(|b| String::from_utf8_lossy(&b).to_string())
}

fn first(files: &[String]) -> Option<(String, String)> {
    files.iter().find_map(|f| read_text(f).map(|t| (f.clone(), t)))
}

struct Entry {
    name: String,
    dir: bool,
    file: bool,
}

fn list_dir(dir: &str) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Entry> = rd
        .flatten()
        .filter_map(|e| {
            let t = e.file_type().ok()?;
            Some(Entry { name: e.file_name().to_string_lossy().to_string(), dir: t.is_dir(), file: t.is_file() })
        })
        .collect();
    out.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    out
}

fn join(a: &str, b: &str) -> String {
    Path::new(a).join(b).to_string_lossy().to_string()
}

fn relative(base: &str, full: &str) -> String {
    Path::new(full).strip_prefix(base).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| full.replace('\\', "/"))
}

fn walk_sources(dirs: &[String], base: &str) -> Vec<(String, String)> {
    static TEST_SET: Js = Js::new(r"(^|\/)src\/(test|androidTest|testFixtures)\w*\/", "");
    let mut out = Vec::new();
    let mut bytes = 0u64;
    fn walk(dir: &str, base: &str, out: &mut Vec<(String, String)>, bytes: &mut u64) {
        for e in list_dir(dir) {
            if out.len() >= WALK_FILES || *bytes >= WALK_BYTES {
                return;
            }
            let full = join(dir, &e.name);
            let rel = relative(base, &full);
            if e.dir {
                if !skip(&e.name) && !e.name.starts_with('.') && !TEST_SET.test(&format!("{rel}/")) {
                    walk(&full, base, out, bytes);
                }
            } else if e.file && lang_of(&e.name).is_some() {
                let Some(size) = std::fs::metadata(&full).ok().map(|m| m.len()) else { continue };
                if size > TEXT_MAX {
                    continue;
                }
                *bytes += size;
                out.push((full, rel));
            }
        }
    }
    for d in dirs {
        walk(d, base, &mut out, &mut bytes);
    }
    out
}

#[derive(Clone)]
pub struct AppTarget {
    pub kind: &'static str,
    pub dir: String,
    pub name: String,
    pub module: String,
    pub gradle_root: String,
}

fn is_flutter_app(pubspec: Option<&str>) -> bool {
    static RE: Js = Js::new(r"^\s*sdk:\s*flutter\b", "m");
    pubspec.is_some_and(|p| RE.test(p))
}

fn find_flutter(root: &str) -> Vec<AppTarget> {
    fn visit(dir: &str, depth: usize, out: &mut Vec<AppTarget>) {
        if out.len() >= APPS_MAX {
            return;
        }
        let pubspec = read_text(&join(dir, "pubspec.yaml"));
        let module = join(&join(dir, "android"), "app");
        if is_flutter_app(pubspec.as_deref()) && first(&[join(&module, "build.gradle.kts"), join(&module, "build.gradle")]).is_some() {
            let name = pubspec.as_deref().and_then(pubspec_name).unwrap_or_else(|| basename(dir));
            out.push(AppTarget { kind: "flutter", dir: dir.to_string(), name, module, gradle_root: join(dir, "android") });
            return;
        }
        if depth >= 2 {
            return;
        }
        for e in list_dir(dir) {
            if e.dir && !skip(&e.name) && !e.name.starts_with('.') {
                visit(&join(dir, &e.name), depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(root, 0, &mut out);
    out
}

pub fn find_apps(root: &str) -> Vec<AppTarget> {
    let mut apps = find_flutter(root);
    for p in detect_android(root) {
        let (pdir, pname) = (p["dir"].as_str().unwrap_or("").to_string(), p["name"].as_str().unwrap_or("").to_string());
        for m in p["modules"].as_array().cloned().unwrap_or_default() {
            if m["type"].as_str() != Some("application") && m["guessed"] != json!(true) {
                continue;
            }
            let mdir = m["dir"].as_str().unwrap_or("").to_string();
            if !is_within(root, &mdir) || apps.iter().any(|a| is_within(&a.dir, &mdir)) {
                continue;
            }
            let path = m["path"].as_str().unwrap_or("");
            apps.push(AppTarget { kind: "android", dir: mdir.clone(), name: if path == ":" { pname.clone() } else { path.to_string() }, module: mdir, gradle_root: pdir.clone() });
        }
    }
    apps.truncate(APPS_MAX);
    apps
}

fn dep(pubspec: Option<&str>, pubspec_file: &str, build: Option<&(String, String)>, toml: Option<&str>, toml_file: &str, gradle_re: &Js, pub_re: &Js) -> Option<Located> {
    pubspec
        .and_then(|p| locate(p, pubspec_file, Lang::Yaml, pub_re))
        .or_else(|| build.and_then(|(f, t)| locate(t, f, Lang::C, gradle_re)))
        .or_else(|| toml.and_then(|t| locate(t, toml_file, Lang::Yaml, gradle_re)))
}

fn scan_app(app: &AppTarget, store: &Store) -> Value {
    static DEBUG_SET: Js = Js::new(r"(^|\/)src\/\w*[dD]ebug\/", "");
    static REVENUE: Js = Js::new(r"\bpurchases_flutter\s*:|com\.revenuecat", "");
    let flutter = app.kind == "flutter";
    let build = first(&[join(&app.module, "build.gradle.kts"), join(&app.module, "build.gradle")]);
    let gradle = parse_gradle_app(build.as_ref().map(|b| b.1.as_str()).unwrap_or(""));
    let build_file = build.as_ref().map(|b| b.0.clone()).unwrap_or_else(|| join(&app.module, "build.gradle"));
    let pubspec_file = join(&app.dir, "pubspec.yaml");
    let pubspec = if flutter { read_text(&pubspec_file) } else { None };
    let memory: Memory = store.get(&app.dir);
    let mut findings: Vec<Value> = Vec::new();

    let pv = pubspec.as_deref().and_then(parse_pubspec_version);
    let use_gradle = !flutter || gradle.version_code.as_ref().is_some_and(|c| c.value.is_some());
    let version = if use_gradle {
        VersionInput { code: gradle.version_code.as_ref().and_then(|c| c.value), code_raw: gradle.version_code.as_ref().map(|c| c.raw.clone()), name: gradle.version_name.as_ref().and_then(|n| n.value.clone()), file: Some(build_file.clone()), code_line: gradle.version_code.as_ref().map(|c| c.line), name_line: gradle.version_name.as_ref().map(|n| n.line) }
    } else {
        VersionInput { code: pv.as_ref().and_then(|p| p.code), code_raw: pv.as_ref().map(|p| p.raw.clone()), name: pv.as_ref().and_then(|p| p.name.clone()), file: Some(pubspec_file.clone()), code_line: pv.as_ref().map(|p| p.line), name_line: pv.as_ref().map(|p| p.line) }
    };
    let released = memory.released.as_ref().map(|r| Released { code: r["code"].as_f64().map(|c| c as i64), name: r["name"].as_str().map(String::from) });
    findings.extend(version_findings(&version, released.as_ref()));
    findings.extend(build_findings(&gradle, &build_file, flutter));
    findings.extend(sdk_findings(&gradle, &build_file, flutter));

    let main_manifest = join(&join(&join(&app.module, "src"), "main"), "AndroidManifest.xml");
    for (file, main) in [(main_manifest, true), (join(&join(&join(&app.module, "src"), "release"), "AndroidManifest.xml"), false)] {
        if let Some(xml) = read_text(&file) {
            findings.extend(manifest_findings(&xml, &file, main));
        }
    }

    let roots = if flutter { vec![join(&app.dir, "lib"), join(&app.module, "src"), join(&join(&app.dir, "ios"), "Runner")] } else { vec![join(&app.module, "src")] };
    let (mut admob, mut consent, mut acknowledge) = (0usize, false, false);
    for (file, rel) in walk_sources(&roots, &app.dir) {
        let Some(text) = read_text(&file) else { continue };
        if admob < ADMOB_MAX {
            let mut found = admob_findings(&text, &file, DEBUG_SET.test(&rel));
            found.truncate(ADMOB_MAX - admob);
            admob += found.len();
            findings.extend(found);
        }
        consent = consent || CONSENT_SOURCE.test(&text);
        acknowledge = acknowledge || ACK_SOURCE.test(&text);
    }

    let toml_file = join(&join(&app.gradle_root, "gradle"), "libs.versions.toml");
    let toml = read_text(&toml_file);
    let d = |g: &Js, p: &Js| dep(pubspec.as_deref(), &pubspec_file, build.as_ref(), toml.as_deref(), &toml_file, g, p);
    let billing = d(&BILLING_GRADLE, &BILLING_PUBSPEC);
    let combined = format!("{}\n{}\n{}", pubspec.as_deref().unwrap_or(""), build.as_ref().map(|b| b.1.as_str()).unwrap_or(""), toml.as_deref().unwrap_or(""));
    let revenue_cat = REVENUE.test(&combined);
    let src = join(&app.module, "src");
    let mut gs_candidates = vec![join(&app.module, "google-services.json")];
    gs_candidates.extend(list_dir(&src).into_iter().filter(|e| e.dir).map(|e| join(&join(&src, &e.name), "google-services.json")));
    let gs_json = first(&gs_candidates).map(|f| f.0);
    findings.extend(extras_findings(&ExtrasFacts {
        ads: d(&ADS_GRADLE, &ADS_PUBSPEC),
        consent,
        billing: billing.map(|b| (b, revenue_cat)),
        acknowledge,
        google_services_json: gs_json,
        gms_plugin: build.as_ref().and_then(|(f, t)| locate(t, f, Lang::C, &GMS_PLUGIN)),
    }));
    findings.push(cert_finding(None, memory.cert.as_ref().and_then(|c| c["sha256"].as_str()), false, None));

    json!({
        "kind": app.kind,
        "dir": app.dir,
        "name": app.name,
        "version": {"code": version.code, "name": version.name},
        "released": memory.released,
        "cert": memory.cert,
        "findings": findings,
    })
}

pub fn scan_release(root: &str, store: &Store) -> Vec<Value> {
    find_apps(root).iter().map(|a| scan_app(a, store)).collect()
}

pub fn newest_artifact(kind: &str, dir: &str) -> Option<String> {
    let bases = if kind == "flutter" { vec![join(&join(&join(dir, "build"), "app"), "outputs"), join(&join(&join(&join(dir, "android"), "app"), "build"), "outputs")] } else { vec![join(&join(dir, "build"), "outputs")] };
    let mut found: Vec<(String, f64, bool)> = Vec::new();
    fn walk(dir: &str, d: &str, depth: usize, found: &mut Vec<(String, f64, bool)>) {
        for e in list_dir(dir) {
            let full = join(dir, &e.name);
            if e.dir && depth < 5 {
                walk(&full, d, depth + 1, found);
            } else if e.file && { let l = e.name.to_ascii_lowercase(); l.ends_with(".apk") || l.ends_with(".aab") } && found.len() < 200 {
                if let Ok(meta) = std::fs::metadata(&full) {
                    let release = relative(d, &full).to_ascii_lowercase().contains("release");
                    found.push((full, crate::util::mtime_ms(&meta), release));
                }
            }
        }
    }
    for b in &bases {
        walk(b, dir, 0, &mut found);
    }
    found.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)));
    found.into_iter().next().map(|f| f.0)
}
