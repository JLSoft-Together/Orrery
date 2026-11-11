use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

pub fn tmp_dir(tag: &str) -> String {
    let base = std::fs::canonicalize(std::env::temp_dir()).map(crate::mcp::paths::strip_verbatim).unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(format!("ccm-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.to_string_lossy().to_string()
}

pub fn fixture(tag: &str, files: &[(&str, &str)]) -> String {
    let dir = tmp_dir(tag);
    for (rel, text) in files {
        let file = PathBuf::from(&dir).join(rel);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, text).unwrap();
    }
    dir
}

#[cfg(unix)]
pub fn script(path: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let p = PathBuf::from(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn wait_for(ms: u64, mut f: impl FnMut() -> bool) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    while std::time::Instant::now() < end {
        if f() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    f()
}
