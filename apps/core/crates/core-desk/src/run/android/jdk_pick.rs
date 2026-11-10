use super::deps::EnvDeps;
use super::env::{env_get, exe, parse_major, read_property, studio_jbr};
use super::jsre::{g, Js};
use super::pathx;
use crate::run::types::{Env, EnvIssue};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub struct GradleVersion {
    pub text: String,
    pub parts: [u32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct JdkInfo {
    pub home: String,
    pub version: u32,
    pub full: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JdkSource {
    GradleProperties,
    Setting,
    Idea,
    Discovered,
}

impl JdkSource {
    pub fn name(self) -> &'static str {
        match self {
            JdkSource::GradleProperties => "gradle-properties",
            JdkSource::Setting => "setting",
            JdkSource::Idea => "idea",
            JdkSource::Discovered => "discovered",
        }
    }

    fn note(self) -> &'static str {
        match self {
            JdkSource::GradleProperties => " from org.gradle.java.home",
            JdkSource::Setting => " from Run settings",
            JdkSource::Idea => " from .idea/gradle.xml",
            JdkSource::Discovered => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JdkPick {
    pub home: String,
    pub version: Option<u32>,
    pub label: String,
    pub source: JdkSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JdkChoice {
    pub gradle: Option<GradleVersion>,
    pub pick: Option<JdkPick>,
    pub min_jdk: u32,
    pub issue: Option<EnvIssue>,
}

#[derive(Debug, Clone, Default)]
pub struct JdkPickOpts {
    pub setting: Option<String>,
    pub env: Env,
    pub win: bool,
    pub home: String,
}

pub const ANY_JAVA: u32 = u32::MAX;
const GRADLE_MAX_JAVA: [[u32; 3]; 15] = [[9, 1, 25], [8, 14, 24], [8, 10, 23], [8, 8, 22], [8, 5, 21], [8, 3, 20], [7, 6, 19], [7, 5, 18], [7, 3, 17], [7, 0, 16], [6, 7, 15], [6, 3, 14], [6, 0, 13], [5, 4, 12], [5, 0, 11]];
const NEWEST_KNOWN: [u32; 2] = [9, 1];
const PREFERRED_JDK: u32 = 17;

fn cmp(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

fn full_parts(full: &str) -> Vec<u32> {
    full.split(['.', '+', '_', '-']).map(|s| s.parse::<u32>().unwrap_or(0)).collect()
}

pub fn parse_gradle_version(wrapper_props: &str) -> Option<GradleVersion> {
    static RE: Js = Js::new(r"gradle-(\d+)\.(\d+)(?:\.(\d+))?(?:-[^/]*?)?-(?:bin|all)\.zip$", "");
    let url = read_property(wrapper_props, "distributionUrl")?;
    let m = RE.exec(url.trim())?;
    let n = |i: usize| g(&m, i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let text = match g(&m, 3) {
        Some(p) => format!("{}.{}.{p}", n(1), n(2)),
        None => format!("{}.{}", n(1), n(2)),
    };
    Some(GradleVersion { text, parts: [n(1), n(2), n(3)] })
}

pub fn max_java_for(g: &GradleVersion) -> u32 {
    if cmp(&g.parts[..2], &NEWEST_KNOWN) == std::cmp::Ordering::Greater {
        return ANY_JAVA;
    }
    for [maj, min, java] in GRADLE_MAX_JAVA {
        if cmp(&g.parts, &[maj, min, 0]) != std::cmp::Ordering::Less {
            return java;
        }
    }
    8
}

pub fn min_java_for(g: Option<&GradleVersion>) -> u32 {
    match g {
        None => PREFERRED_JDK,
        Some(g) if g.parts[0] >= 8 => PREFERRED_JDK,
        Some(g) if g.parts[0] >= 7 => 11,
        Some(_) => 8,
    }
}

pub fn pick_for(gradle: &GradleVersion, jdks: &[JdkInfo]) -> Option<JdkInfo> {
    let max = max_java_for(gradle);
    let mut ok: Vec<&JdkInfo> = jdks.iter().filter(|j| j.version <= max).collect();
    ok.sort_by(|a, b| b.version.cmp(&a.version).then_with(|| cmp(&full_parts(&b.full), &full_parts(&a.full))));
    ok.iter().find(|j| j.version >= PREFERRED_JDK).or_else(|| ok.iter().find(|j| j.version >= 11)).or(ok.first()).map(|j| (*j).clone())
}

pub fn parse_release(text: &str) -> Option<(u32, String)> {
    static RE: Js = Js::new(r#"^JAVA_VERSION\s*=\s*"?([^"\r\n]+)"?"#, "m");
    let m = RE.exec(text)?;
    let full = g(&m, 1)?.trim().to_string();
    parse_major(&full).map(|v| (v, full))
}

fn label_of(home: &str, win: bool) -> String {
    let base = pathx::basename(home, win);
    let parent = pathx::dirname(home, win);
    if base == "Home" && pathx::basename(&parent, win) == "Contents" {
        let name = pathx::basename(&pathx::dirname(&parent, win), win);
        return name.strip_suffix(".jdk").map(String::from).unwrap_or(name);
    }
    base
}

fn read_jdk(home: &str, win: bool, deps: &dyn EnvDeps) -> Option<JdkInfo> {
    if !deps.is_file(&pathx::join(win, &[home, "bin", &exe("java", win)])) {
        return None;
    }
    let (version, full) = deps.read_text(&pathx::join(win, &[home, "release"])).filter(|t| !t.is_empty()).and_then(|t| parse_release(&t))?;
    Some(JdkInfo { home: home.to_string(), version, full, label: label_of(home, win) })
}

fn gradle_user_home(opts: &JdkPickOpts) -> String {
    env_get(&opts.env, "GRADLE_USER_HOME").map(str::trim).filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| pathx::join(opts.win, &[&opts.home, ".gradle"]))
}

fn scan_roots(opts: &JdkPickOpts) -> Vec<(String, bool)> {
    let w = opts.win;
    let mut roots = vec![(pathx::join(w, &[&opts.home, ".jdks"]), false), (pathx::join(w, &[&gradle_user_home(opts), "jdks"]), true)];
    if w {
        let pf = env_get(&opts.env, "ProgramFiles").filter(|s| !s.is_empty()).unwrap_or("C:\\Program Files").to_string();
        for v in ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "Amazon Corretto"] {
            roots.push((pathx::join(true, &[&pf, v]), false));
        }
    } else if cfg!(target_os = "macos") {
        roots.push(("/Library/Java/JavaVirtualMachines".into(), false));
        roots.push((pathx::join(false, &[&opts.home, "Library", "Java", "JavaVirtualMachines"]), false));
    } else {
        roots.push(("/usr/lib/jvm".into(), false));
    }
    roots
}

fn discover(opts: &JdkPickOpts, deps: &dyn EnvDeps) -> Vec<JdkInfo> {
    let w = opts.win;
    let mut homes: Vec<String> = Vec::new();
    for h in [env_get(&opts.env, "JAVA_HOME").map(String::from), studio_jbr(w, &opts.env)].into_iter().flatten() {
        let t = h.trim();
        if !t.is_empty() && pathx::is_absolute(t, w) {
            homes.push(t.to_string());
        }
    }
    if w {
        let pf = env_get(&opts.env, "ProgramFiles").filter(|s| !s.is_empty()).unwrap_or("C:\\Program Files").to_string();
        homes.push(pathx::join(true, &[&pf, "Android", "Android Studio", "jre"]));
    }
    for (root, nested) in scan_roots(opts) {
        let Some(names) = deps.list_dirs(&root) else { break };
        for name in names {
            let dir = pathx::join(w, &[&root, &name]);
            homes.push(dir.clone());
            homes.push(pathx::join(w, &[&dir, "Contents", "Home"]));
            if nested {
                for sub in deps.list_dirs(&dir).unwrap_or_default() {
                    homes.push(pathx::join(w, &[&dir, &sub]));
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for home in homes {
        let Some(info) = read_jdk(&home, w, deps) else { continue };
        let real = deps.realpath(&home);
        let key = if w { real.to_lowercase() } else { real };
        if seen.insert(key) {
            out.push(info);
        }
    }
    out
}

type Cache = HashMap<(u64, String), Vec<JdkInfo>>;

fn cache() -> &'static Mutex<Cache> {
    static C: std::sync::OnceLock<Mutex<Cache>> = std::sync::OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_jdk_cache() {
    cache().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

pub fn discover_jdks(opts: &JdkPickOpts, deps: &dyn EnvDeps) -> Vec<JdkInfo> {
    let Some(id) = deps.cache_key() else { return discover(opts, deps) };
    let e = |k: &str| env_get(&opts.env, k).unwrap_or("\u{0}").to_string();
    let key = format!("{}|{}|{}|{}|{}", opts.win, opts.home, e("JAVA_HOME"), e("ProgramFiles"), e("GRADLE_USER_HOME"));
    if let Some(hit) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(&(id, key.clone())) {
        return hit.clone();
    }
    let found = discover(opts, deps);
    cache().lock().unwrap_or_else(|e| e.into_inner()).insert((id, key), found.clone());
    found
}

fn explicit_home(home: Option<&str>, source: JdkSource, opts: &JdkPickOpts, deps: &dyn EnvDeps) -> Option<JdkPick> {
    let w = opts.win;
    let dir = home?.trim();
    if dir.is_empty() || !pathx::is_absolute(dir, w) || !deps.is_file(&pathx::join(w, &[dir, "bin", &exe("java", w)])) {
        return None;
    }
    let version = deps.read_text(&pathx::join(w, &[dir, "release"])).filter(|t| !t.is_empty()).and_then(|t| parse_release(&t)).map(|r| r.0);
    Some(JdkPick { home: dir.to_string(), version, label: label_of(dir, w), source })
}

pub fn parse_gradle_jvm(xml: &str) -> Option<String> {
    static RE: Js = Js::new(r#"<option\s+name="gradleJvm"\s+value="([^"]*)""#, "");
    let m = RE.exec(xml)?;
    let v = g(&m, 1).unwrap_or("").replace("&amp;", "&").replace("&quot;", "\"").trim().to_string();
    (!v.is_empty()).then_some(v)
}

fn matches_name(name: &str, jdk: &JdkInfo) -> bool {
    static NUM: Js = Js::new(r"^\d+$", "");
    static VENDOR: Js = Js::new(r"^(.+)-(\d+)$", "");
    let n = name.to_lowercase();
    let label = jdk.label.to_lowercase();
    if label == n || label.starts_with(&format!("{n}.")) {
        return true;
    }
    if NUM.test(&n) {
        return n.parse::<u32>().ok() == Some(jdk.version);
    }
    VENDOR.exec(&n).is_some_and(|m| g(&m, 2).and_then(|v| v.parse::<u32>().ok()) == Some(jdk.version) && label.starts_with(g(&m, 1).unwrap_or("")))
}

fn idea_pick(root: &str, jdks: &[JdkInfo], max: u32, opts: &JdkPickOpts, deps: &dyn EnvDeps) -> Option<JdkPick> {
    let w = opts.win;
    let mut value = deps.read_text(&pathx::join(w, &[root, ".idea", "gradle.xml"])).filter(|t| !t.is_empty()).and_then(|x| parse_gradle_jvm(&x))?;
    if value == "#GRADLE_LOCAL_JAVA_HOME" {
        value = deps.read_text(&pathx::join(w, &[root, ".gradle", "config.properties"])).filter(|t| !t.is_empty()).and_then(|p| read_property(&p, "java.home")).filter(|v| !v.is_empty())?;
    } else if value.starts_with('#') {
        return None;
    }
    let value = value.replace("$USER_HOME$", &opts.home).replace("$PROJECT_DIR$", root);
    let fits = |v: Option<u32>| v.is_none_or(|v| v <= max);
    if pathx::is_absolute(&value, w) {
        return explicit_home(Some(&value), JdkSource::Idea, opts, deps).filter(|p| fits(p.version));
    }
    let mut named: Vec<&JdkInfo> = jdks.iter().filter(|j| matches_name(&value, j) && fits(Some(j.version))).collect();
    named.sort_by(|a, b| cmp(&full_parts(&b.full), &full_parts(&a.full)));
    named.first().map(|j| JdkPick { home: j.home.clone(), version: Some(j.version), label: j.label.clone(), source: JdkSource::Idea })
}

pub fn choose_jdk(root: &str, opts: &JdkPickOpts, deps: &dyn EnvDeps) -> JdkChoice {
    let w = opts.win;
    let gradle = deps.read_text(&pathx::join(w, &[root, "gradle", "wrapper", "gradle-wrapper.properties"])).filter(|t| !t.is_empty()).and_then(|t| parse_gradle_version(&t));
    let min_jdk = min_java_for(gradle.as_ref());
    let done = |pick: Option<JdkPick>, gradle: Option<GradleVersion>| JdkChoice { gradle, pick, min_jdk, issue: None };
    for file in [pathx::join(w, &[&gradle_user_home(opts), "gradle.properties"]), pathx::join(w, &[root, "gradle.properties"])] {
        let raw = deps.read_text(&file).filter(|t| !t.is_empty()).and_then(|t| read_property(&t, "org.gradle.java.home")).filter(|r| !r.is_empty());
        if let Some(pick) = raw.and_then(|r| explicit_home(Some(&pathx::resolve(root, &r, w)), JdkSource::GradleProperties, opts, deps)) {
            return done(Some(pick), gradle);
        }
    }
    if let Some(pick) = explicit_home(opts.setting.as_deref(), JdkSource::Setting, opts, deps) {
        return done(Some(pick), gradle);
    }
    let Some(gv) = gradle.clone() else { return done(None, None) };
    let jdks = discover_jdks(opts, deps);
    let max = max_java_for(&gv);
    if let Some(idea) = idea_pick(root, &jdks, max, opts, deps) {
        return done(Some(idea), gradle);
    }
    if let Some(best) = pick_for(&gv, &jdks) {
        return done(Some(JdkPick { home: best.home, version: Some(best.version), label: best.label, source: JdkSource::Discovered }), gradle);
    }
    if jdks.is_empty() {
        return done(None, gradle);
    }
    let mut sorted = jdks.clone();
    sorted.sort_by_key(|j| j.version);
    let found = sorted.iter().map(|j| format!("{} ({})", j.version, j.label)).collect::<Vec<_>>().join(", ");
    let range = if max == ANY_JAVA { "any".to_string() } else { format!("≤ {max}") };
    JdkChoice { gradle, pick: None, min_jdk, issue: Some(EnvIssue::with("jdk-incompatible", format!("Gradle {} runs on JDK {range}; found {found}", gv.text))) }
}

pub fn describe_jdk(choice: &JdkChoice, home: Option<&str>, version: Option<u32>, win: bool) -> Option<String> {
    let home = home?;
    let label = choice.pick.as_ref().map(|p| p.label.clone()).unwrap_or_else(|| label_of(home, win));
    let v = version.or(choice.pick.as_ref().and_then(|p| p.version)).map(|v| v.to_string()).unwrap_or_else(|| "?".into());
    let gradle = choice.gradle.as_ref().map(|g| format!(" for Gradle {}", g.text)).unwrap_or_default();
    let note = choice.pick.as_ref().map(|p| p.source.note()).unwrap_or("");
    Some(format!("JDK {v} ({label}){gradle}{note}"))
}

#[cfg(test)]
#[path = "jdk_pick_tests.rs"]
mod tests;
