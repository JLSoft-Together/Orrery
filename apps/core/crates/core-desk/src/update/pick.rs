use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct FeedAsset {
    pub name: String,
    pub size: f64,
    pub url: String,
    pub sha256: Option<String>,
}

pub fn pick_asset(assets: &[FeedAsset], version: &str, kind: &str, arch: &str) -> Option<FeedAsset> {
    let (exact, loose) = match kind {
        "nsis" => (format!("Orrery-Setup-{version}.exe"), r"(?i)^Orrery-Setup-[A-Za-z0-9_.-]+\.exe$".to_string()),
        "portable" => (format!("Orrery-Portable-{version}.exe"), r"(?i)^Orrery-Portable-[A-Za-z0-9_.-]+\.exe$".to_string()),
        "mac" if arch == "arm64" || arch == "x64" => (format!("Orrery-{version}-{arch}.dmg"), format!(r"(?i)^Orrery-[A-Za-z0-9_.-]+-{}\.dmg$", regex::escape(arch))),
        _ => return None,
    };
    let re = Regex::new(&loose).ok()?;
    assets.iter().find(|a| a.name == exact).or_else(|| assets.iter().find(|a| re.is_match(&a.name))).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(name: &str) -> FeedAsset {
        FeedAsset { name: name.into(), size: 10.0, url: "https://github.com/x".into(), sha256: None }
    }

    #[test]
    fn picks_by_kind_and_arch() {
        let all = [a("Orrery-Setup-2.5.0.exe"), a("Orrery-Portable-2.5.0.exe"), a("Orrery-2.5.0-arm64.dmg"), a("Orrery-2.5.0-x64.dmg")];
        assert_eq!(pick_asset(&all, "2.5.0", "nsis", "x64").unwrap().name, "Orrery-Setup-2.5.0.exe");
        assert_eq!(pick_asset(&all, "2.5.0", "portable", "x64").unwrap().name, "Orrery-Portable-2.5.0.exe");
        assert_eq!(pick_asset(&all, "2.5.0", "mac", "arm64").unwrap().name, "Orrery-2.5.0-arm64.dmg");
        assert_eq!(pick_asset(&all, "2.5.1", "nsis", "x64").unwrap().name, "Orrery-Setup-2.5.0.exe");
        assert!(pick_asset(&all[1..], "2.5.0", "nsis", "x64").is_none());
        assert!(pick_asset(&all, "2.5.0", "mac", "ia32").is_none());
        assert!(pick_asset(&all, "2.5.0", "dev", "x64").is_none());
    }
}
