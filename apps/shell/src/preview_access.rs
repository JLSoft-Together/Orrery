use core_desk::preview::changes::{common_dir, toplevel};
use core_desk::preview::files::real_roots;
use core_desk::preview::paths::{dirname, is_absolute, real_or_null, root_of};
use serde_json::{json, Map, Value};

use crate::preview_grants::is_fs_root;
use crate::preview_host::{is_dir, Gate, Roots};
use crate::state::lock;

const MAX_DIR_LEN: usize = 1024;

fn facts(root: &str, common: Option<String>) -> Value {
    let mut m = Map::new();
    m.insert("root".into(), json!(root));
    m.insert("repo".into(), json!(true));
    if let Some(c) = common {
        m.insert("common".into(), json!(c));
    }
    Value::Object(m)
}

impl Gate {
    pub fn request_access(&self, roots: Roots, target: &Value, confirm: &dyn Fn(&str) -> bool) -> bool {
        let Some(t) = target.as_str().filter(|t| is_absolute(t)) else { return false };
        if self.allowed(roots, target, false).is_some() {
            return true;
        }
        let Some(real) = real_or_null(t) else { return false };
        let Ok(meta) = std::fs::metadata(&real) else { return false };
        let folder = if meta.is_dir() { real.clone() } else { dirname(&real) };
        if is_fs_root(&folder) {
            return false;
        }
        let _asking = lock(&self.asking);
        if root_of(&folder, &real_roots(&self.grants.list())).is_some() {
            return true;
        }
        let ok = confirm(&folder);
        if ok {
            self.grants.add(&folder);
        }
        ok
    }

    pub fn full_access(&self, on: &Value, confirm: &dyn Fn() -> bool) -> bool {
        if on == &Value::Bool(false) {
            self.full.set(false);
        }
        let _asking = lock(&self.asking);
        let ask = (on == &Value::Bool(true) || (on == "first" && !self.full.asked())) && !self.full.get();
        if ask {
            let yes = confirm();
            if yes || !self.full.asked() {
                self.full.set(yes);
            }
        }
        self.full.get()
    }

    pub fn grant_folder(&self, folder: &str) -> bool {
        if is_fs_root(folder) {
            return false;
        }
        self.grants.add(folder);
        true
    }

    fn root_facts(dir: &Value) -> Value {
        let Some(d) = dir.as_str().filter(|d| d.len() <= MAX_DIR_LEN && is_absolute(d)) else { return Value::Null };
        let Some(real) = real_or_null(d).filter(|r| is_dir(r)) else { return Value::Null };
        match toplevel(&real) {
            Ok(top) => {
                let common = common_dir(&top);
                facts(&top, common)
            }
            Err(_) => json!({"root": real, "repo": false}),
        }
    }

    pub fn project_root(&self, roots: Roots, dir: &Value, remembered: bool) -> Value {
        let Some((real, _)) = self.allowed(roots, dir, false).or_else(|| self.allowed(roots, dir, true)) else {
            return if remembered { Self::root_facts(dir) } else { Value::Null };
        };
        match toplevel(&real) {
            Ok(top) => {
                self.remember_repo(&top, &real);
                let common = common_dir(&top);
                facts(&top, common)
            }
            Err(_) => json!({"root": real, "repo": false}),
        }
    }

    pub fn list_dir_files(&self, roots: Roots, root: &Value) -> Value {
        let Some(dir) = self.repo_of(roots, root).or_else(|| self.allowed(roots, root, false).map(|a| a.0)) else { return Value::Null };
        if !is_dir(&dir) {
            return Value::Null;
        }
        self.files.get(&dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::{Path, PathBuf};

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("orrery-access-{tag}-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
        std::fs::create_dir_all(&d).unwrap();
        PathBuf::from(real_or_null(&d.to_string_lossy()).unwrap())
    }

    fn s(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn request_access_asks_once_for_the_folder_and_grants_it() {
        let base = temp("req");
        let dir = base.join("docs");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.html"), "x").unwrap();
        let gate = Gate::in_memory();
        let none = |_: bool| Vec::new();
        let asked = Cell::new(0);
        let no = |_: &str| {
            asked.set(asked.get() + 1);
            false
        };
        assert!(!gate.request_access(&none, &json!(s(&dir.join("a.html"))), &no));
        let yes = |f: &str| {
            asked.set(asked.get() + 1);
            f == s(&dir)
        };
        assert!(gate.request_access(&none, &json!(s(&dir.join("a.html"))), &yes));
        assert_eq!(asked.get(), 2);
        assert!(gate.request_access(&none, &json!(s(&dir)), &no));
        assert_eq!(asked.get(), 2);
        assert!(gate.allowed(&none, &json!(s(&dir.join("a.html"))), false).is_some());
        assert!(!gate.request_access(&none, &json!("relative"), &yes));
        assert!(!gate.request_access(&none, &json!("/"), &yes));
        gate.grants.clear();
        assert!(gate.allowed(&none, &json!(s(&dir)), false).is_none());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn full_access_follows_the_electron_switch() {
        let gate = Gate::in_memory();
        let asked = Cell::new(0);
        let no = || {
            asked.set(asked.get() + 1);
            false
        };
        let yes = || {
            asked.set(asked.get() + 1);
            true
        };
        assert!(!gate.full_access(&Value::Null, &yes));
        assert_eq!(asked.get(), 0);
        assert!(!gate.full_access(&json!("first"), &no));
        assert!(gate.full.asked());
        assert!(!gate.full_access(&json!("first"), &yes));
        assert_eq!(asked.get(), 1);
        assert!(gate.full_access(&json!(true), &yes));
        assert!(gate.allowed(&|_| Vec::new(), &json!(std::env::temp_dir().to_string_lossy()), false).is_some());
        assert!(!gate.full_access(&json!(false), &yes));
        assert_eq!(asked.get(), 2);
    }

    #[test]
    fn project_root_names_the_repo_or_the_folder() {
        let base = temp("proj");
        let gate = Gate::in_memory();
        let roots = {
            let b = s(&base);
            move |_: bool| vec![b.clone()]
        };
        let none = |_: bool| Vec::new();
        assert_eq!(gate.project_root(&roots, &json!(s(&base)), false), json!({"root": s(&base), "repo": false}));
        assert_eq!(gate.project_root(&none, &json!(s(&base)), false), Value::Null);
        assert_eq!(gate.project_root(&none, &json!(s(&base)), true), json!({"root": s(&base), "repo": false}));
        let repo = base.join("r");
        std::fs::create_dir_all(repo.join("sub")).unwrap();
        let inited = std::process::Command::new("git").args(["init", "-q"]).current_dir(&repo).output().is_ok_and(|o| o.status.success());
        if inited {
            let v = gate.project_root(&roots, &json!(s(&repo.join("sub"))), false);
            assert_eq!(v["root"], json!(s(&repo)));
            assert_eq!(v["repo"], json!(true));
            assert!(v["common"].as_str().is_some_and(|c| c.ends_with(".git")));
            assert_eq!(gate.repo_of(&roots, &json!(s(&repo))), Some(s(&repo)));
            std::fs::write(repo.join("f.txt"), "").unwrap();
            let files = gate.list_dir_files(&roots, &json!(s(&repo)));
            assert_eq!(files["source"], json!("git"));
            assert_eq!(files["files"], json!(["f.txt"]));
        }
        assert_eq!(gate.list_dir_files(&none, &json!(s(&base))), Value::Null);
        let _ = std::fs::remove_dir_all(base);
    }
}
