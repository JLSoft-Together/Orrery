fn package_version(text: &str) -> Option<String> {
    let at = text.find("\"version\"")?;
    let rest = &text[at + 9..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

fn main() {
    let pkg = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("desktop").join("package.json");
    println!("cargo:rerun-if-changed={}", pkg.display());
    let version = std::fs::read_to_string(&pkg).ok().and_then(|t| package_version(&t)).expect("apps/desktop/package.json version");
    println!("cargo:rustc-env=ORRERY_VERSION={version}");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&["ccm", "guest"]))).expect("tauri build");
}
