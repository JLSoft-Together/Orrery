use super::rules::{is_debug_subject, parse_apksigner, parse_keytool};
use crate::libs::versions::compare_lib_versions;
use crate::run::android::deps::{EnvDeps, FsEnvDeps};
use crate::run::android::detect::list_dirs;
use crate::run::android::env::{default_sdk_dir, env_get, exe, parse_java_version, studio_jbr};
use crate::run::android::jsre::Js;
use crate::run::android::pathx;
use crate::run::types::Env;

const SIGN_TIMEOUT: u64 = 60_000;
const MIN_JDK: u32 = 17;

#[derive(Clone, Default)]
pub struct SignTools {
    pub java: Option<String>,
    pub keytool: Option<String>,
    pub apksigner_jar: Option<String>,
}

pub struct ToolsOpts<'a> {
    pub jdk_setting: Option<&'a str>,
    pub sdk_setting: Option<&'a str>,
    pub env: &'a Env,
    pub win: bool,
    pub home: &'a str,
}

fn find_jdk(setting: Option<&str>, env: &Env, win: bool, deps: &dyn EnvDeps) -> Option<(String, String)> {
    let mut homes: Vec<Option<String>> = vec![setting.map(String::from), env_get(env, "JAVA_HOME").map(String::from), studio_jbr(win, env)];
    let raw = env_get(env, "PATH").or_else(|| env_get(env, "Path")).unwrap_or("");
    for d in raw.split(if win { ';' } else { ':' }) {
        if !d.is_empty() && pathx::is_absolute(d, win) {
            homes.push(Some(pathx::dirname(d, win)));
        }
    }
    let mut seen: Vec<String> = Vec::new();
    for raw in homes.into_iter().flatten() {
        let home = raw.trim().to_string();
        if home.is_empty() || !pathx::is_absolute(&home, win) || seen.contains(&home.to_lowercase()) {
            continue;
        }
        seen.push(home.to_lowercase());
        let java = pathx::join(win, &[&home, "bin", &exe("java", win)]);
        if !deps.is_file(&java) {
            continue;
        }
        let Some((stdout, stderr)) = deps.exec(&java, &["-version"], 15_000) else { continue };
        let Some(version) = parse_java_version(&format!("{stderr}\n{stdout}")) else { continue };
        if version >= MIN_JDK {
            return Some((home, java));
        }
    }
    None
}

pub fn find_sign_tools(opts: &ToolsOpts, deps: &dyn EnvDeps) -> SignTools {
    static VERSION: Js = Js::new(r"^\d[\w.-]*$", "");
    let jdk = find_jdk(opts.jdk_setting, opts.env, opts.win, deps);
    let keytool = jdk.as_ref().map(|(home, _)| pathx::join(opts.win, &[home, "bin", &exe("keytool", opts.win)]));
    let mut jar: Option<String> = None;
    let sdks = [opts.sdk_setting.map(String::from), env_get(opts.env, "ANDROID_HOME").map(String::from), env_get(opts.env, "ANDROID_SDK_ROOT").map(String::from), default_sdk_dir(opts.env, opts.win, opts.home)];
    for sdk in sdks.iter().flatten() {
        let sdk = sdk.trim();
        if sdk.is_empty() || !pathx::is_absolute(sdk, opts.win) || jar.is_some() {
            continue;
        }
        let bt = pathx::join(opts.win, &[sdk, "build-tools"]);
        let mut versions: Vec<String> = list_dirs(&bt).into_iter().filter(|v| VERSION.test(v)).collect();
        versions.sort_by(|a, b| compare_lib_versions(b, a));
        for v in versions {
            let candidate = pathx::join(opts.win, &[&bt, &v, "lib", "apksigner.jar"]);
            if deps.is_file(&candidate) {
                jar = Some(candidate);
                break;
            }
        }
    }
    SignTools { java: jdk.map(|(_, j)| j), keytool: keytool.filter(|k| deps.is_file(k)), apksigner_jar: jar }
}

pub enum Outcome {
    Ok { sha256: String, subject: Option<String>, debug_key: bool },
    Err(&'static str),
}

pub fn read_signer(file: &str, tools: &SignTools, deps: &dyn EnvDeps) -> Outcome {
    static UNSIGNED: Js = Js::new(r"not a signed jar|DOES NOT VERIFY|No JAR signatures|unsigned", "i");
    let aab = file.to_ascii_lowercase().ends_with(".aab");
    if aab && tools.keytool.is_none() || !aab && tools.java.is_none() {
        return Outcome::Err("no-jdk");
    }
    let Some(jar) = tools.apksigner_jar.as_deref().or(if aab { Some("") } else { None }) else { return Outcome::Err("no-apksigner") };
    let res = if aab {
        deps.exec(tools.keytool.as_deref().unwrap_or(""), &["-J-Duser.language=en", "-printcert", "-jarfile", file], SIGN_TIMEOUT)
    } else {
        deps.exec(tools.java.as_deref().unwrap_or(""), &["-jar", jar, "verify", "--print-certs", file], SIGN_TIMEOUT)
    };
    let Some((stdout, stderr)) = res else { return Outcome::Err("failed") };
    let out = format!("{stdout}\n{stderr}");
    match if aab { parse_keytool(&out) } else { parse_apksigner(&out) } {
        Some(p) => {
            let debug_key = is_debug_subject(p.subject.as_deref());
            Outcome::Ok { sha256: p.sha256, subject: p.subject, debug_key }
        }
        None => Outcome::Err(if UNSIGNED.test(&out) { "unsigned" } else { "failed" }),
    }
}

pub fn fs_deps() -> FsEnvDeps {
    FsEnvDeps
}
