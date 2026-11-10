use super::download::{compare_versions, Bundle, KIT_JAR, READY_MARKER};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let Ok(t) = e.file_type() else { continue };
        if t.is_dir() {
            total += dir_size(&e.path());
        } else if t.is_file() {
            total += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

pub fn installed_kit_version(base: &str) -> Option<String> {
    let dir = Path::new(base).join("daemon");
    let mut ready: Vec<String> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.contains(".partial-") && dir.join(n).join(READY_MARKER).exists() && dir.join(n).join(KIT_JAR).exists())
        .collect();
    ready.sort_by(|a, b| compare_versions(b, a));
    ready.into_iter().next()
}

pub fn install_status(b: &Bundle) -> Value {
    let pinned = b.cfg.kit.as_ref();
    let available = pinned.is_some() || b.overridden();
    json!({
        "available": available,
        "installed": available && b.is_ready(),
        "version": installed_kit_version(&b.cfg.base),
        "pinnedVersion": pinned.map(|k| k.version.clone()),
        "sizeBytes": dir_size(Path::new(&b.cfg.base)),
        "downloadBytes": b.download_bytes(),
        "bundle": b.status(),
    })
}

pub fn uninstall(b: &Bundle, stop: impl FnOnce()) -> Value {
    if b.busy() {
        return json!({"ok": false, "code": "busy", "status": install_status(b)});
    }
    stop();
    let removed = match fs::remove_dir_all(&b.cfg.base) {
        Ok(()) => true,
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    };
    b.clear_error();
    let mut out = json!({"ok": removed, "status": install_status(b)});
    if !removed {
        out["code"] = json!("io");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_size_sums_nested_files() {
        let dir = std::env::temp_dir().join(format!("ccm-install-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("x"), [0u8; 10]).unwrap();
        fs::write(dir.join("a/b/y"), [0u8; 5]).unwrap();
        assert_eq!(dir_size(&dir), 15);
        assert_eq!(dir_size(&dir.join("nope")), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_newest_ready_kit_wins_and_partials_are_ignored() {
        let dir = std::env::temp_dir().join(format!("ccm-install-ver-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for v in ["1", "3.5.0", "2.partial-ab"] {
            fs::create_dir_all(dir.join("daemon").join(v)).unwrap();
            fs::write(dir.join("daemon").join(v).join(READY_MARKER), "x").unwrap();
            fs::write(dir.join("daemon").join(v).join(KIT_JAR), "x").unwrap();
        }
        fs::create_dir_all(dir.join("daemon/9")).unwrap();
        assert_eq!(installed_kit_version(&dir.to_string_lossy()).as_deref(), Some("3.5.0"));
        assert_eq!(installed_kit_version(&dir.join("none").to_string_lossy()), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
