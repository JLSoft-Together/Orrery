use super::*;

fn snap(at: u64, size: u64, dirs: &[(&str, u64)]) -> Snapshot {
    Snapshot { at, roots: vec![Root { path: "D:\\w".into(), size, dirs: dirs.iter().map(|(p, n)| ((*p).to_owned(), *n)).collect() }], volumes: vec![] }
}

#[test]
fn snapshot_ids_and_files_follow_the_electron_names() {
    assert!(is_snapshot_id("00000001-1763251200000"));
    assert!(!is_snapshot_id("1-1763251200000"));
    assert!(!is_snapshot_id("00000001-12"));
    assert_eq!(file_seq("00000012-1763251200000.json.gz"), Some(12));
    assert_eq!(file_seq("00000012-1763251200000.json"), None);
}

#[test]
fn the_store_writes_lists_reads_and_keeps_thirty() {
    let dir = std::env::temp_dir().join(format!("orrery-disk-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::new(dir.clone());
    for i in 0..32u64 {
        store.write(&snap(1_763_251_200_000 + i, 100 + i, &[("D:\\w\\a", 50)])).unwrap();
    }
    let list = store.list();
    assert_eq!(list.len(), 30);
    assert_eq!(list[0]["bytes"], json!(131));
    let id = list[0]["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("00000032-"));
    let back = store.read(&id).unwrap();
    assert_eq!(back.roots[0].dirs, vec![("D:\\w\\a".to_owned(), 50)]);
    assert_eq!(store.latest().unwrap().at, 1_763_251_200_031);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn compare_lists_growth_and_new_or_gone_folders() {
    let base = snap(1, 100, &[("D:\\w\\a", 60), ("D:\\w\\a\\x", 50), ("D:\\w\\old", 40)]);
    let cur = snap(2, 300, &[("D:\\w\\a", 250), ("D:\\w\\a\\x", 240), ("D:\\w\\new", 50)]);
    let c = build_compared(&base, &cur, Plat::Win);
    let meta = json!({ "id": "00000001-1", "at": 1, "bytes": 100, "roots": ["D:\\w"] });
    let top = compare_at(&c, "", Plat::Win, &meta);
    assert_eq!(top["rows"][0]["delta"], json!(200));
    let paths: Vec<&str> = top["top"].as_array().unwrap().iter().map(|r| r["path"].as_str().unwrap()).collect();
    assert_eq!(paths, vec!["D:\\w\\a\\x", "D:\\w\\new"]);
    let inside = compare_at(&c, "d:\\W", Plat::Win, &meta);
    assert_eq!(inside["covered"], json!(true));
    let states: Vec<(&str, &str)> = inside["rows"].as_array().unwrap().iter().map(|r| (r["name"].as_str().unwrap(), r["state"].as_str().unwrap())).collect();
    assert_eq!(states, vec![("a", "changed"), ("new", "new"), ("old", "gone")]);
}

#[test]
fn schedules_are_due_after_the_period_and_retry_hourly() {
    let day = 24 * 3_600_000;
    let now = 10 * day;
    let s = |last: Option<u64>, attempt: Option<u64>, freq: &str| ScheduleFile { freq: freq.into(), telegram: true, last_at: last, fail_at: None, attempt_at: attempt };
    assert!(due(&s(None, None, "daily"), now));
    assert!(!due(&s(None, None, "off"), now));
    assert!(!due(&s(Some(now - day + 1), None, "daily"), now));
    assert!(due(&s(Some(now - day), None, "daily"), now));
    assert!(!due(&s(Some(now - day), Some(now - 1000), "daily"), now));
    assert!(due(&s(Some(now + 10 * day), None, "daily"), now));
    assert!(!due(&s(Some(now - 2 * day), None, "weekly"), now));
}

#[test]
fn a_missing_schedule_file_reads_as_daily_with_telegram() {
    let f = read_schedule(Path::new("Z:/missing/schedule.json"));
    assert_eq!(f, ScheduleFile { freq: "daily".into(), telegram: true, last_at: None, fail_at: None, attempt_at: None });
}
