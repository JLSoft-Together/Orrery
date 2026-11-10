use super::deps::{EnvDeps, FsEnvDeps};
use super::detect::parse_fvm_config;
use super::pth;
use crate::preview::format::{cmd_shim, Resolved};
use crate::run::types::{Env, EnvIssue, RunSettings};
use serde_json::{json, Value};

const VERSION_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, Default)]
pub struct FlutterEnvOpts {
    pub project_setting: Option<String>,
    pub setting: Option<String>,
    pub env: Env,
    pub win: bool,
    pub home: String,
    pub trusted: bool,
    pub repo_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlutterSdkLookup {
    pub sdk: Option<String>,
    pub source: Option<&'static str>,
    pub fvm: Option<String>,
    pub issues: Vec<EnvIssue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlutterVersion {
    pub version: String,
    pub dart: Option<String>,
    pub channel: Option<String>,
}

pub fn flutter_executable(sdk: &str, win: bool) -> String {
    pth::join(&[sdk, "bin", if win { "flutter.bat" } else { "flutter" }], win)
}

fn is_sdk(dir: &str, win: bool, deps: &dyn EnvDeps) -> bool {
    deps.is_file(&flutter_executable(dir, win))
}

pub fn flutter_spawn(sdk: &str, args: &[String], win: bool, env: &Env) -> Option<Resolved> {
    let exe = flutter_executable(sdk, win);
    if !win {
        return Some(Resolved { file: exe, args: args.to_vec(), verbatim: false });
    }
    cmd_shim(&exe, args, env)
}

fn normalize_setting(setting: &str, win: bool, deps: &dyn EnvDeps) -> String {
    let given = pth::normalize(setting.trim(), win);
    if is_sdk(&given, win, deps) {
        return given;
    }
    let mut dir = given;
    let base = pth::basename(&dir, win).to_lowercase();
    if base == "flutter" || base == "flutter.bat" || base == "flutter.exe" {
        dir = pth::dirname(&dir, win);
    }
    if pth::basename(&dir, win).to_lowercase() == "bin" {
        dir = pth::dirname(&dir, win);
    }
    dir
}

fn fvm_candidates(project_dir: &str, version: &str, opts: &FlutterEnvOpts) -> Vec<String> {
    let w = opts.win;
    let mut out = vec![pth::join(&[project_dir, ".fvm", "flutter_sdk"], w), pth::join(&[project_dir, ".fvm", "versions", version], w)];
    if let Some(cache) = opts.env.get("FVM_CACHE_PATH").or_else(|| opts.env.get("FVM_HOME")) {
        out.push(pth::join(&[cache, "versions", version], w));
        out.push(pth::join(&[cache, version], w));
    }
    out.push(pth::join(&[&opts.home, "fvm", "versions", version], w));
    if w {
        if let Some(local) = opts.env.get("LOCALAPPDATA") {
            out.push(pth::join(&[local, "fvm", "versions", version], w));
        }
    }
    out
}

fn on_path(opts: &FlutterEnvOpts, deps: &dyn EnvDeps) -> Option<String> {
    let w = opts.win;
    let name = if w { "flutter.bat" } else { "flutter" };
    let raw = opts.env.get("PATH").or_else(|| opts.env.get("Path")).cloned().unwrap_or_default();
    for dir in raw.split(if w { ';' } else { ':' }).filter(|d| !d.is_empty() && pth::is_absolute(d, w)) {
        let file = pth::join(&[dir, name], w);
        if !deps.is_file(&file) {
            continue;
        }
        let sdk = pth::dirname(&pth::dirname(&deps.realpath(&file), w), w);
        if is_sdk(&sdk, w, deps) {
            return Some(sdk);
        }
    }
    None
}

fn os_defaults(opts: &FlutterEnvOpts, deps: &dyn EnvDeps) -> Vec<String> {
    let w = opts.win;
    if w {
        let profile = opts.env.get("USERPROFILE").cloned().unwrap_or_else(|| opts.home.clone());
        let mut out = vec!["C:\\src\\flutter".to_string(), pth::join(&[&profile, "flutter"], w)];
        if let Some(local) = opts.env.get("LOCALAPPDATA") {
            out.push(pth::join(&[local, "flutter"], w));
        }
        return out;
    }
    let mut out = vec![pth::join(&[&opts.home, "development", "flutter"], w), pth::join(&[&opts.home, "flutter"], w)];
    for cask in ["/opt/homebrew/Caskroom/flutter", "/usr/local/Caskroom/flutter"] {
        let mut versions = deps.readdir(cask);
        versions.sort();
        for v in versions.iter().rev() {
            out.push(pth::join(&[cask, v, "flutter"], w));
        }
    }
    out
}

pub fn project_fvm_version(project_dir: &str, win: bool, deps: &dyn EnvDeps) -> Option<String> {
    for file in [pth::join(&[project_dir, ".fvmrc"], win), pth::join(&[project_dir, ".fvm", "fvm_config.json"], win)] {
        if let Some(v) = deps.read_text(&file).and_then(|t| parse_fvm_config(&t)) {
            return Some(v);
        }
    }
    None
}

pub fn project_sdk_key(dir: &str, win: bool) -> String {
    let resolved = pth::resolve(dir, win);
    if win {
        resolved.to_lowercase()
    } else {
        resolved
    }
}

pub fn sdk_settings(settings: &RunSettings, dir: &str, win: bool) -> (Option<String>, Option<String>) {
    let project = settings.flutter_sdk_projects.get(&project_sdk_key(dir, win)).cloned();
    (project, settings.flutter_sdk.clone())
}

pub fn env_opts(settings: &RunSettings, dir: &str, env: &Env, win: bool, home: &str) -> FlutterEnvOpts {
    let (project_setting, setting) = sdk_settings(settings, dir, win);
    FlutterEnvOpts { project_setting, setting, env: env.clone(), win, home: home.to_string(), ..Default::default() }
}

pub fn find_flutter_sdk(project_dir: &str, opts: &FlutterEnvOpts) -> FlutterSdkLookup {
    find_flutter_sdk_with(project_dir, opts, &FsEnvDeps)
}

pub fn find_flutter_sdk_with(project_dir: &str, opts: &FlutterEnvOpts, deps: &dyn EnvDeps) -> FlutterSdkLookup {
    let w = opts.win;
    let mut issues = Vec::new();
    let fvm = project_fvm_version(project_dir, w, deps);
    let hit = |sdk: String, source: &'static str, issues: Vec<EnvIssue>| FlutterSdkLookup { sdk: Some(sdk), source: Some(source), fvm: fvm.clone(), issues };
    if let Some(p) = opts.project_setting.as_deref().filter(|s| !s.trim().is_empty()) {
        let dir = normalize_setting(p, w, deps);
        if is_sdk(&dir, w, deps) {
            return hit(dir, "project", issues);
        }
        issues.push(EnvIssue::with("flutter-project-invalid", dir));
    }
    if let Some(s) = opts.setting.as_deref().filter(|s| !s.trim().is_empty()) {
        let dir = normalize_setting(s, w, deps);
        if is_sdk(&dir, w, deps) {
            return hit(dir, "setting", issues);
        }
        issues.push(EnvIssue::with("flutter-setting-invalid", dir));
    }
    let fvm_dirs = match &fvm {
        Some(v) => fvm_candidates(project_dir, v, opts),
        None => vec![pth::join(&[project_dir, ".fvm", "flutter_sdk"], w)],
    };
    if let Some(d) = fvm_dirs.into_iter().find(|d| is_sdk(d, w, deps)) {
        return hit(d, "fvm", issues);
    }
    if let Some(v) = &fvm {
        issues.push(EnvIssue::with("fvm-version-missing", v.clone()));
    }
    if let Some(root) = opts.env.get("FLUTTER_ROOT").filter(|r| !r.is_empty() && is_sdk(r, w, deps)) {
        return hit(root.clone(), "FLUTTER_ROOT", issues);
    }
    if let Some(p) = on_path(opts, deps) {
        return hit(p, "path", issues);
    }
    if let Some(d) = os_defaults(opts, deps).into_iter().find(|d| is_sdk(d, w, deps)) {
        return hit(d, "default", issues);
    }
    issues.push(EnvIssue::new("flutter-missing"));
    FlutterSdkLookup { sdk: None, source: None, fvm, issues }
}

fn first_word(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?.trim();
    if s.is_empty() {
        return None;
    }
    s.split_whitespace().next().map(String::from)
}

pub fn parse_flutter_version(stdout: &str) -> Option<FlutterVersion> {
    let start = stdout.find('{')?;
    let end = stdout.rfind('}')?;
    if end <= start {
        return None;
    }
    let json: Value = serde_json::from_str(&stdout[start..=end]).ok()?;
    let obj = json.as_object()?;
    let version = first_word(obj.get("frameworkVersion"))?;
    Some(FlutterVersion { version, dart: first_word(obj.get("dartSdkVersion")), channel: first_word(obj.get("channel")) })
}

pub fn sdk_inside_project(sdk: &str, project_dir: &str, opts: &FlutterEnvOpts, deps: &dyn EnvDeps) -> bool {
    let real = deps.realpath(sdk);
    [Some(project_dir), opts.repo_root.as_deref()].into_iter().flatten().filter(|r| !r.is_empty()).any(|root| pth::is_within(&deps.realpath(root), &real, opts.win))
}

pub fn flutter_env(project_dir: &str, opts: &FlutterEnvOpts) -> Value {
    flutter_env_with(project_dir, opts, &FsEnvDeps)
}

pub fn flutter_env_with(project_dir: &str, opts: &FlutterEnvOpts, deps: &dyn EnvDeps) -> Value {
    let found = find_flutter_sdk_with(project_dir, opts, deps);
    let mut issues = found.issues.clone();
    let shape = |version: Option<FlutterVersion>, issues: &[EnvIssue]| {
        json!({
            "kind": "flutter",
            "sdk": found.sdk,
            "source": found.source,
            "version": version.as_ref().map(|v| v.version.clone()),
            "dart": version.as_ref().and_then(|v| v.dart.clone()),
            "channel": version.as_ref().and_then(|v| v.channel.clone()),
            "fvm": found.source == Some("fvm"),
            "issues": issues.iter().map(EnvIssue::to_value).collect::<Vec<_>>(),
        })
    };
    let Some(sdk) = found.sdk.clone() else {
        return shape(None, &issues);
    };
    if !opts.trusted && found.source != Some("setting") && found.source != Some("project") && sdk_inside_project(&sdk, project_dir, opts, deps) {
        issues.push(EnvIssue::with("fvm-local-unverified", sdk));
        return shape(None, &issues);
    }
    let Some(spec) = flutter_spawn(&sdk, &["--version".into(), "--machine".into()], opts.win, &opts.env) else {
        issues.push(EnvIssue::with("flutter-version-failed", sdk));
        return shape(None, &issues);
    };
    let res = deps.exec(&spec.file, &spec.args, VERSION_TIMEOUT_MS, &opts.env, spec.verbatim);
    match res.ok.then(|| parse_flutter_version(&res.stdout)).flatten() {
        Some(v) => shape(Some(v), &issues),
        None => {
            issues.push(EnvIssue::with("flutter-version-failed", sdk));
            shape(None, &issues)
        }
    }
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
