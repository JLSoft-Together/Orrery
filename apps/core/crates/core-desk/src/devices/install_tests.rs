use super::install::*;
use serde_json::json;

#[test]
fn classifies_all_install_errors() {
    let cases = [
        ("Performing Streamed Install\nSuccess\n", None),
        ("adb: failed to install a.apk: Failure [INSTALL_FAILED_USER_RESTRICTED: Install canceled by user]", Some("restricted")),
        ("Failure [INSTALL_FAILED_INSUFFICIENT_STORAGE]", Some("storage")),
        ("Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: Existing package signatures do not match]", Some("signature")),
        ("Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE]", Some("signature")),
        ("Failure [INSTALL_PARSE_FAILED_INCONSISTENT_CERTIFICATES]", Some("signature")),
        ("Failure [INSTALL_FAILED_VERSION_DOWNGRADE]", Some("downgrade")),
        ("Failure [INSTALL_FAILED_NO_MATCHING_ABIS: Failed to extract native libraries]", Some("abi")),
        ("adb.exe: device offline", Some("offline")),
        ("error: device 'R58M' not found", Some("offline")),
        ("error: no devices/emulators found", Some("offline")),
        ("adb.exe: device unauthorized.\nPlease check the confirmation dialog on your device.", Some("unauthorized")),
        ("Failure [INSTALL_FAILED_OLDER_SDK]", Some("other")),
        ("", Some("other")),
    ];
    for (out, want) in cases {
        assert_eq!(classify_install(out), want, "{out}");
    }
}

#[test]
fn validates_qr() {
    assert_eq!(clean_qr(&json!("orrery://pair?a=1&b='x y'")), Some("orrery://pair?a=1&b='x y'"));
    assert_eq!(clean_qr(&json!("a\nb")), None);
    assert_eq!(clean_qr(&json!("a\u{7}b")), None);
    assert_eq!(clean_qr(&json!("")), None);
    assert_eq!(clean_qr(&json!("x".repeat(640))).map(str::len), Some(640));
    assert_eq!(clean_qr(&json!("x".repeat(641))), None);
    assert_eq!(clean_qr(&json!(5)), None);
}

#[test]
fn quotes_qr_for_the_device_shell() {
    let q = super::toggles::sh_quote("a&b 'c' $x;rm");
    assert_eq!(q, "'a&b '\\''c'\\'' $x;rm'");
}

#[test]
fn validates_port_and_apk() {
    assert_eq!(clean_port(&json!(1)), Some(1));
    assert_eq!(clean_port(&json!(65535)), Some(65535));
    assert_eq!(clean_port(&json!(0)), None);
    assert_eq!(clean_port(&json!(65536)), None);
    assert_eq!(clean_port(&json!(-1)), None);
    assert_eq!(clean_port(&json!("80")), None);
    assert_eq!(clean_apk(&json!("relative.apk")), None);
    assert_eq!(clean_apk(&json!("/no/such/file.apk")), None);
    let dir = std::env::temp_dir().join(format!("ccm-apk-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let apk = dir.join("a.apk");
    let txt = dir.join("a.txt");
    std::fs::write(&apk, b"x").unwrap();
    std::fs::write(&txt, b"x").unwrap();
    assert!(clean_apk(&json!(apk.to_str().unwrap())).is_some());
    assert_eq!(clean_apk(&json!(txt.to_str().unwrap())), None);
    assert_eq!(clean_apk(&json!(dir.to_str().unwrap())), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn parses_dumpsys_version() {
    let out = "Packages:\n  Package [dev.ccm.orrery.remote] (abc):\n    versionCode=42 minSdk=26 targetSdk=35\n    versionName=1.2.3\n    flags=[ HAS_CODE ]\n  Hidden:\n    versionName=0.0.1\n";
    assert_eq!(parse_version(out), Some(("1.2.3".into(), Some(42))));
    assert_eq!(parse_version("Unable to find package: x"), None);
    assert_eq!(parse_version("versionName=2.0"), Some(("2.0".into(), None)));
}
