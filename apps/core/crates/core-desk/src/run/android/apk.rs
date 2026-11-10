use super::detect::parse_project_dirs;
use super::pathx;
use crate::preview::paths::is_within;
use crate::run::parse::parse_output_metadata;
use serde_json::Value;

fn win() -> bool {
    pathx::host_win()
}

fn read_text(file: &str) -> Option<String> {
    std::fs::read_to_string(file).ok()
}

pub fn module_dir(root: &str, module: &str) -> String {
    let rel = module.trim_start_matches(':');
    if rel.is_empty() {
        return root.to_string();
    }
    let settings = read_text(&pathx::join(win(), &[root, "settings.gradle.kts"])).or_else(|| read_text(&pathx::join(win(), &[root, "settings.gradle"])));
    match settings.and_then(|s| parse_project_dirs(&s).get(&format!(":{rel}")).cloned()) {
        Some(custom) => pathx::resolve(root, &custom, win()),
        None => {
            let mut parts = vec![root];
            parts.extend(rel.split(':'));
            pathx::join(win(), &parts)
        }
    }
}

pub fn apk_dirs(variant: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for c in variant.chars() {
        if c.is_ascii_uppercase() || words.is_empty() {
            words.push(String::new());
        }
        words.last_mut().expect("word").push(c);
    }
    let build_type = words.pop().unwrap_or_else(|| variant.to_string()).to_lowercase();
    let flavor = words.concat();
    let out = if flavor.is_empty() { vec![build_type] } else { vec![format!("{flavor}/{build_type}"), build_type, variant.to_string()] };
    let mut uniq: Vec<String> = Vec::new();
    for d in out {
        if !uniq.contains(&d) {
            uniq.push(d);
        }
    }
    uniq
}

#[derive(Debug, Clone, PartialEq)]
pub struct Apk {
    pub apk: String,
    pub application_id: String,
}

fn read_meta(dir: &str) -> (Option<Apk>, Option<String>) {
    let Some(text) = read_text(&pathx::join(win(), &[dir, "output-metadata.json"])) else { return (None, None) };
    let name = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.get("variantName").and_then(Value::as_str).map(String::from));
    (parse_output_metadata(&text, dir).map(|(apk, application_id)| Apk { apk, application_id }), name)
}

fn subdirs(dir: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<String> = rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).map(|e| pathx::join(win(), &[dir, &e.file_name().to_string_lossy()])).collect();
    out.sort();
    out
}

pub fn find_apk(mod_dir: &str, variant: &str) -> Option<Apk> {
    let build_dir = pathx::join(win(), &[mod_dir, "build"]);
    let apk_root = pathx::join(win(), &[&build_dir, "outputs", "apk"]);
    let fits = |meta: &Apk, name: &Option<String>| name.as_deref().is_none_or(|n| n == variant) && is_within(&build_dir, &meta.apk);
    for rel in apk_dirs(variant) {
        let mut parts = vec![apk_root.as_str()];
        parts.extend(rel.split('/'));
        if let (Some(meta), name) = read_meta(&pathx::join(win(), &parts)) {
            if fits(&meta, &name) {
                return Some(meta);
            }
        }
    }
    for a in subdirs(&apk_root) {
        let mut dirs = vec![a.clone()];
        dirs.extend(subdirs(&a));
        for dir in dirs {
            if let (Some(meta), name) = read_meta(&dir) {
                if name.as_deref() == Some(variant) && fits(&meta, &name) {
                    return Some(meta);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::android::testutil::tmp_dir;

    #[test]
    fn variants_map_to_agp_dirs() {
        assert_eq!(apk_dirs("debug"), vec!["debug"]);
        assert_eq!(apk_dirs("freeArmDebug"), vec!["freeArm/debug", "debug", "freeArmDebug"]);
    }

    #[test]
    fn ignores_other_variant_or_outside_build() {
        let dir = tmp_dir("apk");
        let out = pathx::join(win(), &[&dir, "build", "outputs", "apk", "debug"]);
        std::fs::create_dir_all(&out).unwrap();
        let meta = |variant: &str, file: &str| std::fs::write(pathx::join(win(), &[&out, "output-metadata.json"]), format!(r#"{{"applicationId":"com.ex","variantName":"{variant}","elements":[{{"outputFile":"{file}"}}]}}"#)).unwrap();
        meta("release", "app.apk");
        assert_eq!(find_apk(&dir, "debug"), None);
        meta("debug", "../../../../../evil.apk");
        assert_eq!(find_apk(&dir, "debug"), None);
        meta("debug", "app-debug.apk");
        assert_eq!(find_apk(&dir, "debug"), Some(Apk { apk: pathx::join(win(), &[&out, "app-debug.apk"]), application_id: "com.ex".into() }));
    }

    #[test]
    fn module_dir_honours_overrides() {
        let dir = tmp_dir("apk");
        std::fs::write(pathx::join(win(), &[&dir, "settings.gradle"]), "include ':app'\nproject(':app').projectDir = file('mods/app')").unwrap();
        assert_eq!(module_dir(&dir, ":app"), pathx::join(win(), &[&dir, "mods", "app"]));
        assert_eq!(module_dir(&dir, ":core:ui"), pathx::join(win(), &[&dir, "core", "ui"]));
        assert_eq!(module_dir(&dir, ":"), dir);
    }
}
