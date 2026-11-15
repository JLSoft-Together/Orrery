use super::*;
use crate::disk_apps_left::{app_key, find_leftovers, last_used_of, parse_plist_strings, publisher_key, Entry};
use crate::disk_apps_reg::{parse_reg_export, parse_reg_value, parse_uninstall, read_windows_apps, RegValue, REG_SOURCES};

const HKCU: &str = "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

fn fixture() -> String {
    [
        "Windows Registry Editor Version 5.00".to_owned(),
        String::new(),
        format!("[{HKCU}]"),
        String::new(),
        format!("[{HKCU}\\OrreryE2EFake]"),
        "\"DisplayName\"=\"Orrery E2E Fake App\"".into(),
        "\"Publisher\"=\"Orrery E2E\"".into(),
        "\"InstallDate\"=\"20261101\"".into(),
        "\"EstimatedSize\"=dword:00001000".into(),
        "\"UninstallString\"=\"\\\"C:\\\\node\\\\node.exe\\\" \\\"C:\\\\t\\\\fake uninst.js\\\" /S\"".into(),
        String::new(),
        format!("[{HKCU}\\OrreryE2EHidden]"),
        "\"DisplayName\"=\"Hidden Runtime\"".into(),
        "\"SystemComponent\"=dword:00000001".into(),
        String::new(),
        format!("[{HKCU}\\KB]"),
        "\"DisplayName\"=\"Security Update for Windows (KB1234567)\"".into(),
        String::new(),
        format!("[{HKCU}\\Multi]"),
        "\"DisplayName\"=hex(2):41,00,42,00,00,00".into(),
        "\"InstallLocation\"=\"C:\\\\Apps\\\\AB\\\\\"".into(),
    ]
    .join("\r\n")
}

#[test]
fn registry_export_lists_visible_apps_only() {
    let env: HashMap<String, String> = HashMap::new();
    let apps = read_windows_apps(Some(vec![(REG_SOURCES[2], fixture())]), &env).unwrap();
    let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["Orrery E2E Fake App", "AB"]);
    let fake = &apps[0];
    assert_eq!(fake.size, Some(4096.0 * 1024.0));
    assert_eq!(fake.size_from, Some("estimate"));
    assert_eq!(fake.uninstall.as_deref(), Some("\"C:\\node\\node.exe\" \"C:\\t\\fake uninst.js\" /S"));
    assert_eq!(fake.scope, "user");
    assert!(fake.installed_at.is_some());
    assert_eq!(apps[1].location.as_deref(), Some("C:\\Apps\\AB"));
    assert_eq!(parse_reg_value("hex(7):41,00,00,00,42,00,00,00,00,00"), Some(RegValue::Multi(vec!["A".into(), "B".into()])));
    assert_eq!(parse_reg_value("dword:0000000a"), Some(RegValue::Num(10.0)));
    assert_eq!(parse_reg_export("[-HKEY_X]\r\n\"a\"=\"b\"").len(), 0);
}

#[test]
fn uninstall_strings_are_parsed_without_a_shell() {
    let env: HashMap<String, String> = [("systemroot".to_owned(), "C:\\Windows".to_owned())].into();
    let files = ["C:\\node\\node.exe", "C:\\Windows\\System32\\msiexec.exe", "C:\\Program Files\\App\\unins000.exe", "C:\\Windows\\System32\\cmd.exe"];
    let exists = |p: &str| files.iter().any(|f| f.eq_ignore_ascii_case(p));
    let p = parse_uninstall(Some("\"C:\\node\\node.exe\" \"C:\\t\\fake uninst.js\" /S"), &env, &exists).unwrap();
    assert_eq!(p.file, "C:\\node\\node.exe");
    assert_eq!(p.args, "\"C:\\t\\fake uninst.js\" /S");
    assert_eq!(parse_uninstall(Some("\"C:\\node\\node.exe\" x & del C:\\x"), &env, &exists), Err("unsafe"));
    assert_eq!(parse_uninstall(Some("C:\\Program Files\\App\\unins000.exe /SILENT"), &env, &exists).unwrap().args, "/SILENT");
    let msi = parse_uninstall(Some("MsiExec.exe /I{12345678-1234-1234-1234-1234567890ab}"), &env, &exists).unwrap();
    assert_eq!(msi.args, "/X{12345678-1234-1234-1234-1234567890AB}");
    assert_eq!(parse_uninstall(Some("msiexec.exe /qn /x{12345678-1234-1234-1234-1234567890ab}"), &env, &exists), Err("msi"));
    assert_eq!(parse_uninstall(Some("cmd.exe /c rd"), &env, &exists), Err("shell"));
    assert_eq!(parse_uninstall(Some("\"C:\\nope.exe\""), &env, &exists), Err("missing"));
    assert_eq!(parse_uninstall(Some(""), &env, &exists), Err("empty"));
}

#[test]
fn app_and_publisher_keys_drop_versions_and_suffixes() {
    assert_eq!(app_key("Orrery E2E Fake App"), "orrerye2efakeapp");
    assert_eq!(app_key("Foo Bar 2.3.1 (x64)"), "foobar");
    assert_eq!(app_key("Zalo version 24.1"), "zalo");
    assert_eq!(app_key("Thing.app"), "thing");
    assert_eq!(publisher_key(Some("Acme, Inc.")), "acme");
    assert_eq!(publisher_key(Some("Công ty TNHH Vina")), "vina");
}

#[test]
fn leftovers_match_the_app_folder_and_skip_shared_vendors() {
    let roots = vec![Root { base: "appdata", dir: "C:\\R".into(), files: false }, Root { base: "localappdata", dir: "C:\\L".into(), files: false }];
    let readdir = |d: &str| -> Vec<Entry> {
        let mk = |n: &str| Entry { name: n.into(), dir: true, link: false };
        match d {
            "C:\\R" => vec![mk("Orrery E2E Fake App"), mk("Other Tool"), Entry { name: "Orrery E2E Fake App link".into(), dir: false, link: true }],
            "C:\\L" => vec![mk("Microsoft")],
            "C:\\L\\Microsoft" => vec![mk("Edge")],
            _ => Vec::new(),
        }
    };
    let me = LeftApp { name: "Orrery E2E Fake App".into(), publisher: Some("Orrery E2E".into()), bundle_id: None, location: None };
    let others = vec![LeftApp { name: "Other Tool".into(), publisher: None, bundle_id: None, location: None }];
    let hits = find_leftovers(&Query { app: &me, installed: false, others: &others }, &roots, Plat::Win, &readdir, &|_| false);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "C:\\R\\Orrery E2E Fake App");
    assert!(hits[0].exact);
}

#[test]
fn mac_plist_and_mdls_parse() {
    let xml = r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.x.y</string><key>Nested</key><dict><key>CFBundleName</key><string>no</string></dict><key>CFBundleName</key><string>A &amp; B</string></dict></plist>"#;
    let m = parse_plist_strings(xml);
    assert_eq!(m.get("CFBundleIdentifier").map(String::as_str), Some("com.x.y"));
    assert_eq!(m.get("CFBundleName").map(String::as_str), Some("A & B"));
    let used = last_used_of(Some("2024-01-02 03:04:05 +0000\0(null)"), 3);
    assert_eq!(used[0], Some(1_704_164_645_000.0));
    assert_eq!(used[1], None);
    assert_eq!(used[2], None);
}

#[test]
fn texts_follow_the_locale() {
    assert_eq!(uninstall_text("en", "X", "c").uninstall_confirm, "Start uninstaller");
    assert!(uninstall_text("vi", "X", "c").uninstall_title.starts_with("Gỡ"));
}
