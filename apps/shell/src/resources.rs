use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub const KIT_RELEASE_BASE: &str = "https://github.com/JLSoft-Together/Orrery/releases/download";

pub fn resource_dirs(exe_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![exe_dir.to_path_buf()];
    if let Some(contents) = exe_dir.parent().filter(|_| exe_dir.ends_with("MacOS")) {
        dirs.insert(0, contents.join("Resources"));
    }
    dirs
}

pub fn find_resource(exe_dir: Option<&Path>, rel: &str) -> Option<PathBuf> {
    resource_dirs(exe_dir?).into_iter().map(|d| d.join(rel)).find(|p| p.exists())
}

pub fn embedded_kit(raw: Option<&str>) -> Option<Value> {
    let v: Value = serde_json::from_str(raw?.trim()).ok()?;
    let version = v.get("version")?.as_str()?.to_owned();
    let size = v.get("size")?.as_u64()?;
    let sha = v.get("sha256")?.as_str()?.to_owned();
    let url = format!("{KIT_RELEASE_BASE}/v{version}/layout-daemon-{version}.zip");
    Some(json!({ "version": version, "size": size, "sha256": sha, "url": url }))
}

pub fn layout_env(data_dir: &Path, exe_dir: Option<&Path>, repo: &Path, kit: Option<Value>) -> Vec<(&'static str, String)> {
    let s = |p: PathBuf| p.to_string_lossy().into_owned();
    let init = find_resource(exe_dir, "layout-daemon/ccm-project-model.gradle").unwrap_or_else(|| repo.join("apps/layout-daemon/init/ccm-project-model.gradle"));
    let daemon = repo.join("apps").join("layout-daemon").join("build");
    let mut env = vec![("CCM_LAYOUT_INIT", s(init)), ("CCM_LAYOUT_LOG", s(data_dir.join("layout.log")))];
    match kit {
        Some(kit) => {
            let version = kit["version"].as_str().unwrap_or_default().to_owned();
            let dir = data_dir.join("layoutlib").join("daemon").join(version);
            env.push(("CCM_LAYOUT_JAR", s(dir.join("layout-daemon.jar"))));
            env.push(("CCM_LAYOUT_LOOSEKIT", s(dir.join("loose-kit"))));
            env.push(("CCM_LAYOUT_KIT", kit.to_string()));
            env.push(("CCM_LAYOUT_OVERRIDE", String::new()));
        }
        None => {
            env.push(("CCM_LAYOUT_JAR", s(daemon.join("libs").join("layout-daemon.jar"))));
            env.push(("CCM_LAYOUT_LOOSEKIT", s(daemon.join("loose-kit"))));
            env.push(("CCM_LAYOUT_OVERRIDE", s(daemon.join("layoutlib-dist"))));
        }
    }
    env
}

pub fn apply_layout_env(data_dir: &Path, exe_dir: Option<&Path>, repo: &Path) {
    let kit = embedded_kit(option_env!("ORRERY_DAEMON_KIT"));
    for (k, v) in layout_env(data_dir, exe_dir, repo, kit) {
        if std::env::var_os(k).is_none() {
            std::env::set_var(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mac_bundle_reads_resources_next_to_macos() {
        let dirs = resource_dirs(Path::new("/Applications/Orrery.app/Contents/MacOS"));
        assert_eq!(dirs[0], PathBuf::from("/Applications/Orrery.app/Contents/Resources"));
        assert_eq!(dirs[1], PathBuf::from("/Applications/Orrery.app/Contents/MacOS"));
        assert_eq!(resource_dirs(Path::new("C:/Orrery")), vec![PathBuf::from("C:/Orrery")]);
        assert!(find_resource(None, "web").is_none());
    }

    #[test]
    fn the_embedded_kit_gets_its_release_url() {
        let kit = embedded_kit(Some(r#"{"version":"3.5.0","size":12,"sha256":"ab"}"#)).unwrap();
        assert_eq!(kit["url"], "https://github.com/JLSoft-Together/Orrery/releases/download/v3.5.0/layout-daemon-3.5.0.zip");
        assert!(embedded_kit(Some("{}")).is_none());
        assert!(embedded_kit(None).is_none());
    }

    #[test]
    fn a_packaged_build_unpacks_the_kit_in_the_data_dir() {
        let kit = embedded_kit(Some(r#"{"version":"3.5.0","size":12,"sha256":"ab"}"#));
        let env = layout_env(Path::new("/d"), None, Path::new("/repo"), kit);
        let get = |k: &str| env.iter().find(|(n, _)| *n == k).map(|(_, v)| PathBuf::from(v));
        assert_eq!(get("CCM_LAYOUT_JAR"), Some(Path::new("/d/layoutlib/daemon/3.5.0").join("layout-daemon.jar")));
        assert_eq!(get("CCM_LAYOUT_OVERRIDE"), Some(PathBuf::new()));
        let dev = layout_env(Path::new("/d"), None, Path::new("/repo"), None);
        assert!(dev.iter().any(|(k, v)| *k == "CCM_LAYOUT_OVERRIDE" && v.ends_with("layoutlib-dist")));
        assert!(dev.iter().all(|(k, _)| *k != "CCM_LAYOUT_KIT"));
    }
}
