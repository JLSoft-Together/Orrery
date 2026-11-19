use super::*;

fn win() -> GuardContext {
    GuardContext {
        home: "C:\\Users\\dev".into(),
        keep: vec!["C:\\Users\\dev\\Documents".into(), "C:\\Users\\dev\\Desktop".into(), "C:\\Users\\dev\\AppData\\Local\\ccm".into()],
        roots: vec!["C:\\".into(), "D:\\work".into()],
        volumes: vec![],
    }
}

fn mac() -> GuardContext {
    GuardContext { home: "/Users/dev".into(), keep: vec!["/Users/dev/Documents".into(), "/Users/dev/Desktop".into()], roots: vec!["/".into()], volumes: vec![] }
}

fn linux() -> GuardContext {
    GuardContext { home: "/home/dev".into(), keep: vec!["/home/dev/Documents".into()], roots: vec!["/".into()], volumes: vec![] }
}

fn w(p: &str) -> Option<&'static str> {
    check_deletable(p, &win(), Plat::Win)
}

#[test]
fn windows_rejects_bad_input_relative_paths_and_roots() {
    assert_eq!(w(""), Some("invalid"));
    assert_eq!(w("C:\\a\0b"), Some("invalid"));
    assert_eq!(w("foo\\bar"), Some("relative"));
    assert_eq!(w("\\foo"), Some("relative"));
    assert_eq!(w("C:foo"), Some("relative"));
    assert_eq!(w("C:\\"), Some("root"));
    assert_eq!(w("c:/"), Some("root"));
    assert_eq!(w("D:\\"), Some("root"));
    assert_eq!(w("\\\\server\\share"), Some("root"));
    assert_eq!(w("\\\\server\\share\\"), Some("root"));
    assert_eq!(w("D:\\work\\bad name."), Some("invalid"));
}

#[test]
fn windows_rejects_system_folders() {
    assert_eq!(w("C:\\Windows"), Some("system"));
    assert_eq!(w("c:\\windows\\System32\\drivers"), Some("system"));
    assert_eq!(w("C:\\work\\..\\Windows\\Temp"), Some("system"));
    assert_eq!(w("C:\\Program Files"), Some("system"));
    assert_eq!(w("C:\\Program Files (x86)\\"), Some("system"));
    assert_eq!(w("C:\\ProgramData"), Some("system"));
    assert_eq!(w("C:\\$Recycle.Bin\\S-1-5"), Some("system"));
    assert_eq!(w("C:\\Users"), Some("system"));
    assert_eq!(w("C:\\Program Files\\OldApp"), None);
    assert_eq!(w("\\\\pc\\c$\\Windows\\x"), Some("system"));
}

#[test]
fn windows_protects_home_and_kept_folders_but_not_their_contents() {
    assert_eq!(w("C:\\Users\\dev"), Some("protected"));
    assert_eq!(w("c:\\users\\DEV\\"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\Documents"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\AppData"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\AppData\\Local\\ccm"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\.ssh\\id"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\OneDrive - Work"), Some("protected"));
    assert_eq!(w("C:\\Users\\dev\\Documents\\old.zip"), None);
    assert_eq!(w("C:\\Users\\dev\\Desktop\\Ảnh cũ 🎉"), None);
    assert_eq!(w("C:\\Users\\dev\\node_modules"), None);
}

#[test]
fn windows_requires_an_indexed_root() {
    assert_eq!(w("D:\\work\\big"), None);
    assert_eq!(w("D:\\workspace\\big"), Some("outside"));
    assert_eq!(w("E:\\x"), Some("outside"));
    let mut ctx = win();
    ctx.roots.clear();
    assert_eq!(check_deletable("C:\\x", &ctx, Plat::Win), Some("outside"));
}

#[test]
fn windows_paths_normalize_and_nest_case_insensitively() {
    assert_eq!(normalize("c:\\a\\b\\\\", Plat::Win), "C:\\a\\b");
    assert_eq!(normalize("c:/a/./b/../c", Plat::Win), "C:\\a\\c");
    assert!(is_under("C:\\A\\b", "c:\\a", Plat::Win));
    assert!(!is_under("C:\\ab", "C:\\a", Plat::Win));
    let top = top_level(&["D:\\w\\a\\x".into(), "D:\\w\\a".into(), "d:\\W\\A".into(), "D:\\w\\b".into()], Plat::Win);
    assert_eq!(top, vec!["D:\\w\\a".to_owned(), "D:\\w\\b".to_owned()]);
    assert_eq!(parent("C:\\a\\b", Plat::Win), "C:\\a");
    assert_eq!(parent("C:\\a", Plat::Win), "C:\\");
    assert_eq!(base_name("C:\\a\\b.txt", Plat::Win), "b.txt");
}

#[test]
fn absolute_paths_follow_the_platform() {
    assert!(is_absolute("C:\\x", Plat::Win));
    assert!(is_absolute("\\\\srv\\share\\x", Plat::Win));
    assert!(!is_absolute("\\\\?\\C:\\x", Plat::Win));
    assert!(!is_absolute("/x", Plat::Win));
    assert!(is_absolute("/x", Plat::Mac));
    assert!(!is_absolute("x", Plat::Linux));
}

#[test]
fn mac_and_linux_rules() {
    let m = |p: &str| check_deletable(p, &mac(), Plat::Mac);
    assert_eq!(m("/"), Some("root"));
    assert_eq!(m("/System/Library"), Some("system"));
    assert_eq!(m("/Library"), Some("system"));
    assert_eq!(m("/Library/Caches/foo"), None);
    assert_eq!(m("/users/DEV"), Some("protected"));
    assert_eq!(m("/Users/dev/Library/Caches/x"), None);
    assert_eq!(m("relative/x"), Some("relative"));
    let l = |p: &str| check_deletable(p, &linux(), Plat::Linux);
    assert_eq!(l("/etc/passwd"), Some("system"));
    assert_eq!(l("/home"), Some("system"));
    assert_eq!(l("/home/Dev"), Some("protected"));
    assert_eq!(l("/home/Dev/x"), None);
    assert_eq!(l("/home/dev/.cache/pip"), None);
}

#[test]
fn verbatim_canonical_paths_lose_their_prefix() {
    assert_eq!(plain_canonical("\\\\?\\C:\\a", Plat::Win).as_deref(), Some("C:\\a"));
    assert_eq!(plain_canonical("\\\\?\\UNC\\srv\\share\\a", Plat::Win).as_deref(), Some("\\\\srv\\share\\a"));
    assert_eq!(plain_canonical("\\\\?\\Volume{x}\\a", Plat::Win), None);
    assert_eq!(plain_canonical("C:\\a", Plat::Win).as_deref(), Some("C:\\a"));
}

#[test]
fn short_names_are_spotted() {
    assert!(short_name("PROGRA~1"));
    assert!(short_name("ABC~12.TXT"));
    assert!(!short_name("normal"));
    assert!(!short_name("toolongname~1"));
    assert!(!short_name("A~1.LONG"));
}

struct FakeIo;

impl CanonicalIo for FakeIo {
    fn is_link(&self, p: &str) -> std::io::Result<bool> {
        match p {
            "D:\\work\\gone" => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
            "D:\\work\\link" => Ok(true),
            _ => Ok(false),
        }
    }

    fn realpath(&self, p: &str) -> std::io::Result<String> {
        Ok(match p {
            "D:\\work\\escape" => "\\\\?\\C:\\Windows\\System32".to_owned(),
            other => format!("\\\\?\\{other}"),
        })
    }
}

#[test]
fn the_canonical_check_follows_links_out_of_the_root() {
    let ctx = win();
    assert_eq!(resolve_deletable("D:\\work\\escape", &ctx, Plat::Win, &FakeIo), Err("system"));
    let link = resolve_deletable("D:\\work\\link", &ctx, Plat::Win, &FakeIo).unwrap();
    assert!(link.link && link.path == "D:\\work\\link");
    let gone = resolve_deletable("D:\\work\\gone", &ctx, Plat::Win, &FakeIo).unwrap();
    assert!(gone.missing);
    let plain = resolve_deletable("D:\\work\\big", &ctx, Plat::Win, &FakeIo).unwrap();
    assert!(same_canonical(&plain, &Canonical { path: "d:\\WORK\\big".into(), link: false, missing: false }, Plat::Win));
}
