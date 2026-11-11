use super::*;
use crate::preview_grants::{FULL_ACCESS_FILE, GRANTS_FILE};
use std::process::Command;

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("orrery-preview-{tag}-{}-{}", std::process::id(), core_desk::util::now_ms() as u64));
    std::fs::create_dir_all(&d).unwrap();
    real_or_null(&d.to_string_lossy()).map(PathBuf::from).unwrap()
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn fixed(dirs: Vec<String>) -> impl Fn(bool) -> Vec<String> {
    move |_| dirs.clone()
}

#[test]
fn a_path_is_allowed_only_inside_a_root() {
    let base = temp("inside");
    let root = base.join("root");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.txt"), "a").unwrap();
    std::fs::write(base.join("outside.txt"), "o").unwrap();
    let gate = Gate::in_memory();
    let roots = fixed(vec![s(&root)]);
    let (real, list) = gate.allowed(&roots, &json!(s(&root.join("src/a.txt"))), false).unwrap();
    assert_eq!(real, s(&root.join("src").join("a.txt")));
    assert_eq!(list, vec![s(&root)]);
    assert!(gate.allowed(&roots, &json!(s(&base.join("outside.txt"))), false).is_none());
    assert!(gate.allowed(&roots, &json!("relative/a.txt"), false).is_none());
    assert!(gate.allowed(&roots, &json!(42), false).is_none());
    let v = gate.answer(&roots, "preview.allowed", &json!({"path": s(&root.join("src"))})).unwrap();
    assert_eq!(v["real"], json!(s(&root.join("src"))));
    assert_eq!(v["roots"], json!([s(&root)]));
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn dot_dot_and_symlink_escapes_are_refused() {
    let base = temp("escape");
    let root = base.join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(base.join("secret.txt"), "s").unwrap();
    let gate = Gate::in_memory();
    let roots = fixed(vec![s(&root)]);
    let dotted = format!("{}{}..{}secret.txt", s(&root), std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR);
    assert!(gate.allowed(&roots, &json!(dotted), false).is_none());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.join("secret.txt"), root.join("link.txt")).unwrap();
        assert!(gate.allowed(&roots, &json!(s(&root.join("link.txt"))), false).is_none());
        std::os::unix::fs::symlink(&base, root.join("up")).unwrap();
        assert!(gate.explorer_root(&roots, &json!(s(&root.join("up")))).is_none());
    }
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn explorer_root_retries_with_fresh_roots() {
    let base = temp("fresh");
    let gate = Gate::in_memory();
    let dir = s(&base);
    let roots = move |fresh: bool| if fresh { vec![dir.clone()] } else { Vec::new() };
    assert!(gate.allowed(&roots, &json!(s(&base)), false).is_none());
    assert_eq!(gate.explorer_root(&roots, &json!(s(&base))), Some(s(&base)));
    assert_eq!(gate.answer(&roots, "preview.explorerRoot", &json!({"root": s(&base)})).unwrap(), json!(s(&base)));
    assert_eq!(gate.answer(&roots, "preview.checkDir", &json!({"dir": s(&base)})).unwrap(), Value::Null);
    let none = fixed(Vec::new());
    assert_eq!(gate.answer(&none, "preview.explorerRoot", &json!({"root": s(&base)})).unwrap(), Value::Null);
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn editable_file_needs_a_file_and_prefers_the_widest_project_root() {
    let base = temp("edit");
    let sub = base.join("app");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("main.rs"), "fn main() {}").unwrap();
    let gate = Gate::in_memory();
    let roots = fixed(vec![s(&sub), s(&base)]);
    let v = gate.answer(&roots, "preview.editableFile", &json!({"file": s(&sub.join("main.rs"))})).unwrap();
    assert_eq!(v, json!({"real": s(&sub.join("main.rs")), "root": s(&base)}));
    assert_eq!(gate.answer(&roots, "preview.editableFile", &json!({"file": s(&sub)})).unwrap(), Value::Null);
    let only_sub = fixed(vec![s(&sub)]);
    assert_eq!(gate.editable_file(&only_sub, &json!(s(&sub.join("main.rs")))).unwrap().1, s(&sub));
    let other = temp("edit-out");
    std::fs::write(other.join("x.txt"), "x").unwrap();
    assert!(gate.editable_file(&roots, &json!(s(&other.join("x.txt")))).is_none());
    assert!(gate.editable_file(&roots, &json!("x".repeat(5000))).is_none());
    let _ = std::fs::remove_dir_all(base);
    let _ = std::fs::remove_dir_all(other);
}

#[test]
fn caps_have_every_flag_the_desk_reads() {
    let c = caps("/home/u/.claude");
    for k in ["trash", "confirmGit", "confirmRun", "confirmReplace", "confirmPermRules", "confirmDenyRules", "github", "history"] {
        assert_eq!(c[k], json!(true), "{k}");
    }
    assert_eq!(c["claudeConfigDir"], json!("/home/u/.claude"));
    assert_eq!(c.as_object().unwrap().len(), 9);
}

#[test]
fn unknown_methods_are_not_answered_and_constants_are_stable() {
    let gate = Gate::in_memory();
    let roots = fixed(Vec::new());
    assert!(gate.answer(&roots, "preview.nope", &json!({})).is_none());
    assert!(gate.answer(&roots, "preview.caps", &json!({})).is_none());
    assert_eq!(gate.answer(&roots, "preview.sender", &json!({})).unwrap(), json!(SENDER));
    assert_eq!(gate.answer(&roots, "preview.invalidate", &json!({"root": "/x"})).unwrap(), json!(true));
    assert_eq!(gate.answer(&roots, "preview.roots", &json!({})).unwrap(), json!([]));
    assert_eq!(gate.answer(&roots, "preview.registerRepo", &json!({"dir": "/definitely/not/here"})).unwrap(), json!({"error": "denied"}));
    assert_eq!(trash(&json!({"path": "relative"})), json!({"error": "denied"}));
}

#[test]
fn grants_and_full_access_files_feed_the_roots() {
    let base = temp("grants");
    let a = base.join("a");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::write(base.join(GRANTS_FILE), serde_json::to_string(&json!([s(&a), s(&a), "/", "relative", s(&base.join("gone"))])).unwrap()).unwrap();
    std::fs::write(base.join(FULL_ACCESS_FILE), r#"{"full":true}"#).unwrap();
    let gate = Gate::load(&base);
    assert!(gate.allowed(&fixed(Vec::new()), &json!(s(&a)), false).is_some());
    assert!(gate.allowed(&fixed(Vec::new()), &json!(s(&base)), false).is_some());
    let _ = std::fs::remove_dir_all(base);
}

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git").args(args).current_dir(dir).output().is_ok_and(|o| o.status.success())
}

#[test]
fn git_log_registers_the_toplevel_of_a_pane_cwd_and_finds_it_again() {
    let base = temp("repo");
    let repo = base.join("repo");
    let cwd = repo.join("app").join("src");
    std::fs::create_dir_all(&cwd).unwrap();
    if !git(&repo, &["init", "-q"]) {
        let _ = std::fs::remove_dir_all(base);
        return;
    }
    let gate = Gate::in_memory();
    let roots = fixed(vec![s(&cwd)]);
    let top = gate.answer(&roots, "preview.registerRepo", &json!({"dir": s(&cwd)})).unwrap();
    assert_eq!(top, json!(s(&repo)));
    assert_eq!(gate.answer(&roots, "preview.repoOf", &json!({"root": s(&repo)})).unwrap(), json!(s(&repo)));
    assert_eq!(gate.answer(&roots, "preview.registerRepo", &json!({"dir": s(&repo)})).unwrap(), json!(s(&repo)));
    assert_eq!(gate.check_dir(&roots, &json!(s(&repo))), Some(s(&repo)));
    assert_eq!(gate.explorer_root(&roots, &json!(s(&repo))), Some(s(&repo)));
    assert!(gate.write_roots(&roots).contains(&s(&repo)));
    let gone = fixed(Vec::new());
    assert_eq!(gate.answer(&gone, "preview.repoOf", &json!({"root": s(&repo)})).unwrap(), Value::Null);
    let plain = temp("plain");
    let in_plain = fixed(vec![s(&plain)]);
    let err = gate.register_repo(&in_plain, &json!(s(&plain)));
    assert!(err.get("error").is_some_and(|e| e == "not-repo" || e == "no-git"), "{err}");
    let _ = std::fs::remove_dir_all(base);
    let _ = std::fs::remove_dir_all(plain);
}
