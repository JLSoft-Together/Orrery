use crate::run::android::deps::EnvDeps;
use crate::run::android::env::{parse_java_version, studio_jbr, MIN_JDK};
use crate::run::types::Env;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Jdk {
    Ok { home: String, version: u32, java: String },
    Bad { code: &'static str, detail: Option<String> },
}

impl Jdk {
    pub fn public(&self) -> Value {
        match self {
            Jdk::Ok { home, version, .. } => json!({"ok": true, "home": home, "version": version}),
            Jdk::Bad { code, detail: None } => json!({"ok": false, "code": code}),
            Jdk::Bad { code, detail: Some(d) } => json!({"ok": false, "code": code, "detail": d}),
        }
    }
}

fn path_candidates(env: &Env, win: bool) -> Vec<String> {
    let raw = env.get("PATH").or_else(|| env.get("Path")).map(String::as_str).unwrap_or("");
    raw.split(if win { ';' } else { ':' })
        .filter(|d| !d.is_empty() && crate::run::android::pathx::is_absolute(d, win))
        .filter_map(|d| Path::new(d).parent().map(|p| p.to_string_lossy().to_string()))
        .collect()
}

pub fn find_jdk(setting: Option<&str>, env: &Env, win: bool, deps: &dyn EnvDeps) -> Jdk {
    let exe = if win { "java.exe" } else { "java" };
    let mut homes: Vec<Option<String>> = vec![setting.map(String::from), env.get("JAVA_HOME").cloned(), studio_jbr(win, env)];
    homes.extend(path_candidates(env, win).into_iter().map(Some));
    let mut seen = HashSet::new();
    let mut old: Option<String> = None;
    for raw in homes.into_iter().flatten() {
        let home = raw.trim().to_string();
        if home.is_empty() || !crate::run::android::pathx::is_absolute(&home, win) || !seen.insert(home.to_lowercase()) {
            continue;
        }
        let java = crate::run::android::pathx::join(win, &[&home, "bin", exe]);
        if !deps.is_file(&java) {
            continue;
        }
        let Some((out, err)) = deps.exec(&java, &["-version"], 15_000) else { continue };
        let Some(version) = parse_java_version(&format!("{err}\n{out}")) else { continue };
        if version >= MIN_JDK {
            return Jdk::Ok { home, version, java };
        }
        old.get_or_insert(version.to_string());
    }
    match old {
        Some(v) => Jdk::Bad { code: "jdk-old", detail: Some(v) },
        None => Jdk::Bad { code: "jdk-missing", detail: None },
    }
}
