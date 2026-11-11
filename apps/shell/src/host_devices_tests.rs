use super::*;

#[test]
fn every_confirm_kind_has_text_in_both_languages() {
    let reqs = [
        json!({"kind": "remove", "ids": ["a", "b"]}),
        json!({"kind": "wipe", "name": "Pixel_8"}),
        json!({"kind": "fs-delete", "name": "x.txt", "where": "/sdcard/x.txt", "dir": false}),
        json!({"kind": "fs-replace", "name": "x.txt", "where": "/sdcard/x.txt"}),
        json!({"kind": "perf-cold", "pkg": "com.a"}),
        json!({"kind": "pref-write", "pkg": "com.a", "file": "p.xml", "key": "k", "before": "1", "after": "2"}),
        json!({"kind": "pref-restore", "pkg": "com.a", "file": "p.xml"}),
    ];
    for t in [&EN, &VI] {
        for r in &reqs {
            let a = t.ask(r).unwrap_or_else(|| panic!("no text for {r}"));
            assert!(!a.message.is_empty() && !a.detail.is_empty() && !a.ok.is_empty());
        }
    }
}

#[test]
fn unknown_or_malformed_confirms_are_refused() {
    assert!(EN.ask(&json!({"kind": "nope"})).is_none());
    assert!(EN.ask(&json!({})).is_none());
    assert!(EN.ask(&json!({"kind": "remove", "ids": ["a", 1]})).is_none());
    assert!(EN.ask(&json!({"kind": "remove", "ids": "a"})).is_none());
    assert!(EN.ask(&json!({"kind": "wipe"})).is_none());
}

#[test]
fn confirm_text_matches_electron() {
    let one = EN.ask(&json!({"kind": "remove", "ids": ["emulator-5554"]})).unwrap();
    assert_eq!(one.message, "Forget this device?");
    assert_eq!(one.detail, "emulator-5554\n\nOrrery stops listing and auto-connecting them. Nothing on the devices changes.");
    assert_eq!(VI.ask(&json!({"kind": "remove", "ids": ["a", "b"]})).unwrap().message, "Quên 2 thiết bị?");
    let dir = EN.ask(&json!({"kind": "fs-delete", "name": "DCIM", "where": "/sdcard/DCIM", "dir": true})).unwrap();
    assert_eq!(dir.message, "Delete the folder DCIM and everything in it from the device?");
    assert_eq!(dir.detail, "/sdcard/DCIM\n\nThis cannot be undone.");
    let w = EN.ask(&json!({"kind": "pref-write", "pkg": "com.a", "file": "p.xml", "key": "k", "before": 1, "after": null})).unwrap();
    assert!(w.detail.starts_with("p.xml\nk: 1 → \n\n"));
}

#[test]
fn long_pref_values_are_clipped_at_300_chars() {
    let long = "é".repeat(400);
    let c = clip(&long);
    assert_eq!(c.chars().count(), 301);
    assert!(c.ends_with('…'));
    assert_eq!(clip("short"), "short");
}

#[test]
fn device_use_is_throttled_per_serial() {
    let mut last = HashMap::new();
    let t0 = Instant::now();
    assert!(should_report(&mut last, "emulator-5554", t0, USE_GAP));
    assert!(!should_report(&mut last, "emulator-5554", t0 + Duration::from_secs(5), USE_GAP));
    assert!(should_report(&mut last, "192.168.1.2:5555", t0 + Duration::from_secs(5), USE_GAP));
    assert!(should_report(&mut last, "emulator-5554", t0 + Duration::from_secs(16), USE_GAP));
    assert!(!should_report(&mut last, "bad serial", t0, USE_GAP));
    assert!(!should_report(&mut last, "", t0, USE_GAP));
}

#[test]
fn recordings_go_to_the_capture_folder_or_temp() {
    assert_eq!(rec_dir(None), std::env::temp_dir().join("orrery-rec"));
    assert_eq!(rec_dir(Some("/cap")), PathBuf::from("/cap").join("recordings"));
}

#[test]
fn only_files_under_capture_recordings_are_kept() {
    let rec = join("/cap", "recordings");
    assert!(keeps(Some("/cap"), &join(&rec, "orrery-rec-1.mp4")));
    assert!(!keeps(Some("/cap"), "/tmp/orrery-rec/a.mp4"));
    assert!(!keeps(None, &join(&rec, "a.mp4")));
}

#[test]
fn owners_are_scoped_by_window() {
    assert_eq!(owner_key("main", "view"), "main:view");
}
