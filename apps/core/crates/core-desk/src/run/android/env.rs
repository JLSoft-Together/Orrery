pub use super::deps::{EnvDeps, FsEnvDeps};
use super::jdk_pick::{choose_jdk, describe_jdk, JdkPickOpts};
use super::jsre::{g, Js};
use super::pathx;
use crate::preview::format::{cmd_shim, resolve_command, Resolved};
use crate::run::types::{Env, EnvIssue};
use serde_json::{json, Value};

pub const MIN_JDK: u32 = 17;
const JAVA_TIMEOUT: u64 = 15_000;

#[derive(Debug, Clone, Default)]
pub struct AndroidEnvOpts {
    pub sdk_setting: Option<String>,
    pub jdk_setting: Option<String>,
    pub env: Env,
    pub win: bool,
    pub home: String,
    pub trusted: bool,
    pub repo_root: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AndroidEnvInfo {
    pub sdk: Option<String>,
    pub adb: Option<String>,
    pub emulator: Option<String>,
    pub jdk: Option<String>,
    pub jdk_version: Option<u32>,
    pub gradlew: bool,
    pub issues: Vec<EnvIssue>,
    pub jdk_note: Option<String>,
}

impl AndroidEnvInfo {
    pub fn to_value(&self) -> Value {
        let mut v = json!({
            "kind": "android",
            "sdk": self.sdk,
            "adb": self.adb,
            "emulator": self.emulator,
            "jdk": self.jdk,
            "jdkVersion": self.jdk_version,
            "gradlew": self.gradlew,
            "issues": self.issues.iter().map(EnvIssue::to_value).collect::<Vec<_>>(),
        });
        if let Some(n) = &self.jdk_note {
            v["jdkNote"] = json!(n);
        }
        v
    }
}

pub fn exe(name: &str, win: bool) -> String {
    if win {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn env_get<'a>(env: &'a Env, key: &str) -> Option<&'a str> {
    env.get(key).map(String::as_str)
}

fn non_empty(v: Option<&str>) -> Option<&str> {
    v.filter(|s| !s.is_empty())
}

pub fn read_property(text: &str, key: &str) -> Option<String> {
    static LINE: Js = Js::new(r"^((?:\\.|[^:=\s\\])+)\s*[:=]?\s*(.*)$", "");
    static ESC: Js = Js::new(r"\\(.)", "g");
    let unescape = |s: &str| {
        ESC.re().replace_all_with(s, |m| {
            let c = &s[m.group(1).unwrap()];
            match c {
                "t" => "\t".into(),
                "n" => "\n".into(),
                "r" => "\r".into(),
                "f" => "\u{c}".into(),
                other => other.to_string(),
            }
        })
    };
    for raw in super::detect::split_lines(text) {
        let line = raw.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(m) = LINE.exec(line) else { continue };
        if unescape(g(&m, 1).unwrap_or("")) == key {
            return Some(unescape(g(&m, 2).unwrap_or("").trim_end()));
        }
    }
    None
}

pub fn parse_major(full: &str) -> Option<u32> {
    let parts: Vec<&str> = full.split(['.', '+', '_', '-']).collect();
    let pick = if parts.first() == Some(&"1") { parts.get(1) } else { parts.first() };
    pick.filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).and_then(|s| s.parse::<u32>().ok()).filter(|n| *n > 0)
}

pub fn parse_java_version(output: &str) -> Option<u32> {
    static RE: Js = Js::new(r#"version\s+"([^"]+)""#, "");
    let m = RE.exec(output)?;
    parse_major(g(&m, 1)?)
}

fn find_on_path(name: &str, env: &Env, win: bool, deps: &dyn EnvDeps) -> Option<String> {
    let raw = env_get(env, "PATH").or_else(|| env_get(env, "Path")).unwrap_or("");
    for dir in raw.split(if win { ';' } else { ':' }) {
        if dir.is_empty() || !pathx::is_absolute(dir, win) {
            continue;
        }
        let file = pathx::join(win, &[dir, name]);
        if deps.is_file(&file) {
            return Some(file);
        }
    }
    None
}

fn mac() -> bool {
    cfg!(target_os = "macos")
}

pub fn default_sdk_dir(env: &Env, win: bool, home: &str) -> Option<String> {
    if win {
        let local = non_empty(env_get(env, "LOCALAPPDATA")).map(String::from).unwrap_or_else(|| pathx::join(true, &[home, "AppData", "Local"]));
        return Some(pathx::join(true, &[&local, "Android", "Sdk"]));
    }
    mac().then(|| pathx::join(false, &[home, "Library", "Android", "sdk"]))
}

pub fn studio_jbr(win: bool, env: &Env) -> Option<String> {
    if win {
        let pf = non_empty(env_get(env, "ProgramFiles")).unwrap_or("C:\\Program Files");
        return Some(pathx::join(true, &[pf, "Android", "Android Studio", "jbr"]));
    }
    mac().then(|| "/Applications/Android Studio.app/Contents/jbr/Contents/Home".to_string())
}

fn sdk_candidates(root: &str, opts: &AndroidEnvOpts, deps: &dyn EnvDeps) -> Vec<String> {
    let mut out: Vec<Option<String>> = vec![opts.sdk_setting.clone(), env_get(&opts.env, "ANDROID_HOME").map(String::from), env_get(&opts.env, "ANDROID_SDK_ROOT").map(String::from)];
    let props = deps.read_text(&pathx::join(opts.win, &[root, "local.properties"])).filter(|t| !t.is_empty());
    if let Some(dir) = props.and_then(|p| read_property(&p, "sdk.dir")).filter(|d| !d.is_empty()) {
        out.push(Some(pathx::resolve(root, &dir, opts.win)));
    }
    out.push(default_sdk_dir(&opts.env, opts.win, &opts.home));
    out.into_iter().flatten().filter(|d| !d.trim().is_empty()).map(|d| d.trim().to_string()).collect()
}

fn inside_project(file: &str, gradle_root: &str, opts: &AndroidEnvOpts, deps: &dyn EnvDeps) -> bool {
    let target = deps.realpath(file);
    [Some(gradle_root.to_string()), opts.repo_root.clone()].into_iter().flatten().filter(|r| !r.is_empty()).any(|r| pathx::is_within(&deps.realpath(&r), &target, opts.win))
}

pub fn gradlew_file(root: &str, win: bool) -> String {
    pathx::join(win, &[root, if win { "gradlew.bat" } else { "gradlew" }])
}

pub fn android_env(gradle_root: &str, opts: &AndroidEnvOpts) -> Value {
    android_env_with(gradle_root, opts, &FsEnvDeps).to_value()
}

pub fn android_env_with(gradle_root: &str, opts: &AndroidEnvOpts, deps: &dyn EnvDeps) -> AndroidEnvInfo {
    let win = opts.win;
    let env = &opts.env;
    let mut issues = Vec::new();
    let sdk = sdk_candidates(gradle_root, opts, deps).into_iter().find(|d| deps.is_file(&pathx::join(win, &[d, "platform-tools", &exe("adb", win)])));
    let (adb, emulator) = match &sdk {
        Some(dir) => {
            let emu = pathx::join(win, &[dir, "emulator", &exe("emulator", win)]);
            (Some(pathx::join(win, &[dir, "platform-tools", &exe("adb", win)])), deps.is_file(&emu).then_some(emu))
        }
        None => {
            issues.push(EnvIssue::new("sdk-missing"));
            (find_on_path(&exe("adb", win), env, win, deps), None)
        }
    };
    let choice = choose_jdk(gradle_root, &JdkPickOpts { setting: opts.jdk_setting.clone(), env: env.clone(), win, home: opts.home.clone() }, deps);
    let mut jdk = choice.pick.as_ref().map(|p| p.home.clone());
    let mut java = jdk.as_ref().map(|j| pathx::join(win, &[j, "bin", &exe("java", win)]));
    if java.is_none() {
        for dir in [opts.jdk_setting.clone(), env_get(env, "JAVA_HOME").map(String::from), studio_jbr(win, env)].into_iter().flatten() {
            let dir = dir.trim();
            if dir.is_empty() {
                continue;
            }
            let file = pathx::join(win, &[dir, "bin", &exe("java", win)]);
            if deps.is_file(&file) {
                jdk = Some(dir.to_string());
                java = Some(file);
                break;
            }
        }
    }
    if java.is_none() {
        java = find_on_path(&exe("java", win), env, win, deps);
        if let Some(j) = &java {
            jdk = Some(pathx::dirname(&pathx::dirname(j, win), win));
        }
    }
    let mut jdk_version = None;
    match &java {
        Some(j) if !opts.trusted && inside_project(j, gradle_root, opts, deps) => {
            issues.push(EnvIssue::with("jdk-local-unverified", jdk.clone().unwrap_or_else(|| j.clone())));
        }
        Some(j) => {
            jdk_version = deps.exec(j, &["-version"], JAVA_TIMEOUT).and_then(|(out, err)| parse_java_version(&format!("{err}\n{out}")));
            if let Some(v) = jdk_version.filter(|v| *v < choice.min_jdk) {
                issues.push(EnvIssue::with("jdk-old", v.to_string()));
            }
        }
        None => issues.push(EnvIssue::new("jdk-missing")),
    }
    if let Some(i) = &choice.issue {
        issues.push(i.clone());
    }
    let jdk_note = describe_jdk(&choice, jdk.as_deref(), jdk_version, win);
    let wrapper = gradlew_file(gradle_root, win);
    let gradlew = deps.is_file(&wrapper);
    if !gradlew {
        issues.push(EnvIssue::new("gradlew-missing"));
    } else if !win {
        if let Some(mode) = deps.mode(&wrapper) {
            if mode & 0o111 == 0 {
                issues.push(EnvIssue::new("gradlew-not-executable"));
            }
        }
    }
    AndroidEnvInfo { sdk, adb, emulator, jdk, jdk_version, gradlew, issues, jdk_note: if choice.gradle.is_some() { jdk_note } else { None } }
}

pub fn gradlew_spawn(root: &str, args: &[String], win: bool, env: &Env) -> Option<Resolved> {
    let wrapper = gradlew_file(root, win);
    if !win {
        return Some(Resolved { file: wrapper, args: args.to_vec(), verbatim: false });
    }
    cmd_shim(&wrapper, args, env)
}

pub fn find_adb_tools(android_sdk: Option<&str>, env: &Env, win: bool, home: &str) -> (Option<String>, Option<String>) {
    find_adb_tools_with(android_sdk, env, win, home, &FsEnvDeps)
}

pub fn find_adb_tools_with(android_sdk: Option<&str>, env: &Env, win: bool, home: &str, deps: &dyn EnvDeps) -> (Option<String>, Option<String>) {
    let sdks = [android_sdk.map(String::from), env_get(env, "ANDROID_HOME").map(String::from), env_get(env, "ANDROID_SDK_ROOT").map(String::from), default_sdk_dir(env, win, home)];
    for sdk in sdks.into_iter().flatten() {
        let sdk = sdk.trim();
        if sdk.is_empty() {
            continue;
        }
        let adb = pathx::join(win, &[sdk, "platform-tools", &exe("adb", win)]);
        if !deps.is_file(&adb) {
            continue;
        }
        let emu = pathx::join(win, &[sdk, "emulator", &exe("emulator", win)]);
        return (Some(adb), deps.is_file(&emu).then_some(emu));
    }
    let on_path = resolve_command("adb", home, env, win);
    (on_path.filter(|r| !r.verbatim).map(|r| r.file), None)
}

#[cfg(test)]
#[path = "env_tests.rs"]
mod tests;
