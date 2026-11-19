use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::disk_guard::{is_under, Plat};

const FILE: &str = "picked-roots.json";

pub struct Picked {
    file: PathBuf,
}

impl Picked {
    pub fn new(dir: &Path) -> Self {
        Picked { file: dir.join(FILE) }
    }

    pub fn load(&self) -> Option<Vec<String>> {
        let raw = std::fs::read_to_string(&self.file).ok()?;
        let v: Value = serde_json::from_str(&raw).ok()?;
        let list: Vec<String> = v.as_array()?.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        if list.is_empty() {
            None
        } else {
            Some(list)
        }
    }

    pub fn save(&self, roots: &[String]) {
        if roots.is_empty() {
            return;
        }
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&self.file, Value::from(roots.to_vec()).to_string());
    }
}

pub fn restrict(indexed: &[String], picked: Option<&[String]>, plat: Plat) -> Vec<String> {
    match picked {
        None => indexed.to_vec(),
        Some(p) => indexed.iter().filter(|r| p.iter().any(|x| is_under(r, x, plat) || is_under(x, r, plat))).cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn unpicked_roots_are_dropped_from_background_scans() {
        let indexed = s(&["/data/c", "/data/d"]);
        assert_eq!(restrict(&indexed, Some(&s(&["/data/c"])), Plat::Mac), s(&["/data/c"]));
        assert_eq!(restrict(&indexed, Some(&s(&["/data/c", "/data/d/"])), Plat::Mac), indexed);
        assert_eq!(restrict(&indexed, None, Plat::Mac), indexed);
        assert!(restrict(&indexed, Some(&s(&["/other"])), Plat::Mac).is_empty());
    }

    #[test]
    fn windows_drives_match_without_case_or_slash() {
        let indexed = s(&["C:\\", "D:\\"]);
        assert_eq!(restrict(&indexed, Some(&s(&["c:"])), Plat::Win), s(&["C:\\"]));
        assert_eq!(restrict(&indexed, Some(&s(&["D:\\"])), Plat::Win), s(&["D:\\"]));
    }

    #[test]
    fn picked_roots_round_trip() {
        let dir = std::env::temp_dir().join(format!("ccm-pick-{}", std::process::id()));
        let p = Picked::new(&dir);
        assert_eq!(p.load(), None);
        p.save(&s(&["C:\\"]));
        assert_eq!(p.load(), Some(s(&["C:\\"])));
        p.save(&[]);
        assert_eq!(p.load(), Some(s(&["C:\\"])));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
