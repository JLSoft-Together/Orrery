use super::*;
use crate::disk_organize_junk::{lnk_target, raw_items, skipped_path};
use crate::disk_organize_log::{is_run_id, parse_run, summarize};
use crate::disk_organize_rules::{classify, clean_patch, clean_rule, kind_of, with_suffix};

fn tmp(tag: &str) -> PathBuf {
    let d = crate::config::test_temp_dir().join(format!("orrery-org-test-{tag}-{}", hex(4)));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(dir: &Path, rel: &str, n: usize) -> PathBuf {
    let f = dir.join(rel);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(&f, vec![3u8; n]).unwrap();
    f
}

fn org(data: &Path, downloads: &Path, broadcast: Arc<Mutex<Vec<Value>>>) -> Arc<Organize> {
    Organize::new(Config {
        data_dir: data.to_path_buf(),
        plat: Plat::host(),
        home: String::new(),
        downloads: downloads.to_string_lossy().into_owned(),
        desktop: String::new(),
        app_dirs: vec![data.to_string_lossy().into_owned()],
        stable_ms: 0,
        locale: Box::new(|| "vi".into()),
        broadcast: Box::new(move |v| lock(&broadcast).push(v)),
    })
}

#[test]
fn kinds_suffixes_and_rules_match_the_electron_rules() {
    assert_eq!(kind_of("a.JPG"), "image");
    assert_eq!(kind_of("setup.msi"), "installer");
    assert_eq!(kind_of(".bashrc"), "other");
    assert_eq!(with_suffix("a.jpg", 2), "a (2).jpg");
    assert_eq!(with_suffix("README", 3), "README (3)");
    assert_eq!(clean_rule(&json!({ "kind": "ext", "value": "*.PSD", "folder": "Design" })).unwrap().value, "psd");
    assert!(clean_rule(&json!({ "kind": "ext", "value": "x", "folder": "con" })).is_none());
    assert!(clean_rule(&json!({ "kind": "ext", "value": "x", "folder": "a/b" })).is_none());
    assert_eq!(clean_rule(&json!({ "kind": "size", "value": "1.5", "folder": "Big" })).unwrap().value, "1.5");
    let p = clean_patch(&json!({ "mode": "month", "kinds": ["video", "nope", "image"], "auto": true }));
    assert_eq!(p.kinds.unwrap(), vec!["image".to_owned(), "video".to_owned()]);
    let rules = vec![Rule { kind: "size".into(), value: "1".into(), folder: "Big".into() }];
    assert_eq!(classify("x.zip", 2 * 1024 * 1024, 0.0, "type", &[], &rules, "vi").unwrap().folder, "Big");
    assert_eq!(classify("x.pdf", 1, 0.0, "type", &["doc".into()], &[], "vi").unwrap().folder, "Tài liệu");
    assert_eq!(classify("x.pdf", 1, 0.0, "type", &["doc".into()], &[], "en").unwrap().folder, "Documents");
    assert!(classify("x.pdf", 1, 0.0, "type", &["image".into()], &[], "en").is_none());
}

#[test]
fn run_log_parses_like_the_electron_journal() {
    let body = [
        r#"{"t":"run","id":"abcd-0123abcd","at":5,"source":"C:\\D","mode":"type","auto":false}"#,
        r#"{"t":"dir","path":"C:\\D\\Ảnh"}"#,
        r#"{"t":"intent","from":"C:\\D\\a.jpg","to":"C:\\D\\Ảnh\\a.jpg"}"#,
        r#"{"t":"move","from":"C:\\D\\a.jpg","to":"C:\\D\\Ảnh\\a.jpg","size":10,"mtime":1.5}"#,
        r#"{"t":"intent","from":"C:\\D\\b.jpg","to":"C:\\D\\Ảnh\\b.jpg"}"#,
        r#"{"t":"fail","to":"C:\\D\\Ảnh\\b.jpg"}"#,
        "garbage",
    ]
    .join("\n");
    let r = parse_run(&body).unwrap();
    assert_eq!(r.moves.len(), 1);
    assert!(r.moves[0].done);
    assert_eq!(r.dirs.len(), 1);
    let s = summarize(&r);
    assert_eq!(s["moved"], json!(1));
    assert_eq!(s["bytes"], json!(10.0));
    assert!(is_run_id("lzx1abc-deadbeef"));
    assert!(!is_run_id("../x-deadbeef"));
    assert!(parse_run(r#"{"t":"run","id":"bad","at":1,"source":"x"}"#).is_none());
}

#[test]
fn plan_run_and_undo_move_only_settled_top_level_files_and_never_overwrite() {
    let data = tmp("data");
    let dl = tmp("dl");
    put(&dl, "a.jpg", 1000);
    put(&dl, "Ảnh/a.jpg", 10);
    put(&dl, "báo cáo tháng.pdf", 2000);
    put(&dl, "c.zip", 3000);
    put(&dl, "video.mp4.crdownload", 50);
    put(&dl, "desktop.ini", 5);
    std::thread::sleep(Duration::from_millis(5));
    let events = Arc::new(Mutex::new(Vec::new()));
    let o = org(&data, &dl, events.clone());
    let st = o.state();
    assert_eq!(st["settings"]["sources"].as_array().unwrap().len(), 1);
    assert_eq!(st["settings"]["auto"], json!(false));
    assert_eq!(o.run(&json!({ "plan": "nope" }))["error"], json!("expired"));
    assert_eq!(o.plan(&json!({ "source": std::env::temp_dir().to_string_lossy() }))["error"], json!("unknown"));
    let plan = o.plan(&json!({}));
    let mut names: Vec<String> = plan["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap().to_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["a.jpg", "báo cáo tháng.pdf", "c.zip"]);
    let a = plan["items"].as_array().unwrap().iter().find(|i| i["name"] == json!("a.jpg")).unwrap();
    assert_eq!(a["target"], json!("a (2).jpg"));
    let skipped: Vec<&str> = plan["skipped"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(skipped.contains(&"video.mp4.crdownload") && skipped.contains(&"desktop.ini"));
    let zip_id = plan["items"].as_array().unwrap().iter().find(|i| i["name"] == json!("c.zip")).unwrap()["id"].clone();
    let res = o.run(&json!({ "plan": plan["id"], "exclude": [zip_id] }));
    assert_eq!(res["moved"], json!(2), "{res}");
    assert!(dl.join("Ảnh").join("a (2).jpg").exists() && dl.join("Tài liệu").join("báo cáo tháng.pdf").exists());
    assert_eq!(std::fs::metadata(dl.join("Ảnh").join("a.jpg")).unwrap().len(), 10);
    assert!(dl.join("c.zip").exists());
    assert!(lock(&events).is_empty());
    assert_eq!(o.run(&json!({ "plan": plan["id"] }))["error"], json!("expired"));
    let runs = o.state()["runs"].clone();
    assert_eq!(runs.as_array().unwrap().len(), 1);
    let undo = o.undo(&json!({ "run": runs[0]["id"] }));
    assert_eq!(undo["restored"], json!(2), "{undo}");
    assert_eq!(undo["dirs"], json!(1));
    assert_eq!(std::fs::metadata(dl.join("a.jpg")).unwrap().len(), 1000);
    assert!(!dl.join("Tài liệu").exists() && dl.join("Ảnh").join("a.jpg").exists());
    assert_eq!(o.undo(&json!({ "run": runs[0]["id"] }))["error"], json!("done"));
    o.dispose();
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&dl);
}

#[test]
fn auto_pass_moves_only_files_newer_than_the_switch_and_broadcasts() {
    let data = tmp("data2");
    let dl = tmp("dl2");
    put(&dl, "old.png", 10);
    std::thread::sleep(Duration::from_millis(5));
    let events = Arc::new(Mutex::new(Vec::new()));
    let o = org(&data, &dl, events.clone());
    o.patch(&json!({ "auto": true }));
    std::thread::sleep(Duration::from_millis(50));
    let src = o.load().sources[0].clone();
    let _serial_free = o.pass(&src);
    assert!(dl.join("old.png").exists());
    std::thread::sleep(Duration::from_millis(30));
    put(&dl, "new.png", 10);
    let mut moved = false;
    for _ in 0..40 {
        if dl.join("Ảnh").join("new.png").exists() {
            moved = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(moved);
    assert!(dl.join("old.png").exists());
    assert!(!lock(&events).is_empty());
    assert_eq!(o.state()["watching"].as_array().unwrap().len(), 1);
    o.patch(&json!({ "auto": false }));
    assert_eq!(o.state()["watching"].as_array().unwrap().len(), 0);
    o.dispose();
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&dl);
}

#[test]
fn move_file_refuses_an_existing_target() {
    let d = tmp("mv");
    let a = put(&d, "a.txt", 3);
    let b = put(&d, "b.txt", 7);
    assert_eq!(ofs::move_file(&a.to_string_lossy(), &b.to_string_lossy()), Err("EEXIST".into()));
    assert_eq!(std::fs::metadata(&b).unwrap().len(), 7);
    assert_eq!(ofs::move_file(&a.to_string_lossy(), &d.join("c.txt").to_string_lossy()), Ok("rename"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn app_folders_are_refused_as_sources() {
    let data = tmp("data3");
    let o = org(&data, &data, Arc::new(Mutex::new(Vec::new())));
    assert_eq!(o.check_source(&data.to_string_lossy()), Err("protected".into()));
    let _ = std::fs::remove_dir_all(&data);
}

fn lnk(path_ansi: &str) -> Vec<u8> {
    let mut b = vec![0u8; 0x4C];
    b[0] = 0x4C;
    b[0x14] = 0x2;
    let base_off = 0x1C;
    let suffix_off = base_off + path_ansi.len() + 1;
    let size = suffix_off + 1;
    let mut info = vec![0u8; size];
    info[0..4].copy_from_slice(&(size as u32).to_le_bytes());
    info[4..8].copy_from_slice(&0x1Cu32.to_le_bytes());
    info[8..12].copy_from_slice(&1u32.to_le_bytes());
    info[0x10..0x14].copy_from_slice(&(base_off as u32).to_le_bytes());
    info[0x18..0x1C].copy_from_slice(&(suffix_off as u32).to_le_bytes());
    info[base_off..base_off + path_ansi.len()].copy_from_slice(path_ansi.as_bytes());
    b.extend(info);
    b
}

#[test]
fn shortcut_target_is_read_from_link_info() {
    assert_eq!(lnk_target(&lnk("C:\\gone\\target.txt")).as_deref(), Some("C:\\gone\\target.txt"));
    assert_eq!(lnk_target(b"not a link"), None);
    let mut no_info = lnk("C:\\x");
    no_info[0x14] = 0;
    assert_eq!(lnk_target(&no_info), None);
}

#[test]
fn junk_skips_app_and_build_folders_but_allows_temp() {
    let plat = Plat::Win;
    assert!(skipped_path("C:\\Users\\u\\AppData\\Roaming\\x", plat, &[], &[]));
    assert!(!skipped_path("C:\\Users\\u\\AppData\\Local\\Temp\\x", plat, &[], &["C:\\Users\\u\\AppData\\Local\\Temp".into()]));
    assert!(skipped_path("D:\\p\\node_modules\\a", plat, &[], &[]));
    assert!(skipped_path("D:\\p\\.git\\a", plat, &[], &[]));
    assert!(skipped_path("D:\\keep\\a", plat, &["D:\\keep".into()], &[]));
    assert!(!skipped_path("D:\\p\\empty", plat, &[], &[]));
    let items = raw_items(&json!({ "items": [{ "path": "D:\\a", "name": "a", "dir": true, "count": 0 }, { "bad": 1 }] })).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].count, Some(0.0));
}
