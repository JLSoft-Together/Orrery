use std::collections::HashSet;
use std::sync::Mutex;

use serde_json::{json, Value};

use super::*;
use crate::disk_clean_dev::{avd_images, build_dir_kind, docker_bytes, junk_kind, parse_docker_df, read_build_file, read_wrapper, version_cmp, AndroidUse};
use crate::disk_clean_sys::{installer_name, is_forbidden, parse_lsappinfo, sys_roots, SysIo};

fn tmp_root(tag: &str) -> String {
    let base = crate::config::test_temp_dir().join(format!("ccm-clean-test-{tag}-{}-{}", std::process::id(), crate::disk_map::now_ms()));
    std::fs::create_dir_all(&base).unwrap();
    let real = std::fs::canonicalize(&base).unwrap();
    guard::plain_canonical(&real.to_string_lossy(), Plat::host()).unwrap()
}

fn write(root: &str, rel: &str, n: usize, age_days: u64) -> String {
    let p = std::path::Path::new(root).join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, vec![7u8; n]).unwrap();
    if age_days > 0 {
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(age_days * 86_400);
        std::fs::File::options().write(true).open(&p).unwrap().set_modified(t).unwrap();
    }
    p.to_string_lossy().into_owned()
}

#[derive(Default)]
struct Fake {
    home: String,
    events: Mutex<Vec<Value>>,
    records: Mutex<Vec<Value>>,
    trashed: Mutex<Vec<String>>,
    answer: bool,
}

impl Deps for Fake {
    fn plat(&self) -> Plat {
        Plat::host()
    }
    fn home(&self) -> String {
        self.home.clone()
    }
    fn env(&self, _k: &str) -> Option<String> {
        None
    }
    fn keep(&self) -> Vec<String> {
        Vec::new()
    }
    fn self_dirs(&self) -> Vec<String> {
        Vec::new()
    }
    fn running(&self) -> Vec<Holder> {
        Vec::new()
    }
    fn emit(&self, p: Value) {
        self.events.lock().unwrap().push(p);
    }
    fn confirm(&self, _ask: &Ask, _second: bool) -> bool {
        self.answer
    }
    fn ask_one_tap(&self, _ask: &OneTapAsk) -> OneTap {
        if self.answer {
            OneTap::Clean
        } else {
            OneTap::Cancel
        }
    }
    fn ask_bin(&self, _b: u64, _c: u64) -> bool {
        self.answer
    }
    fn ask_command(&self, _ask: &CommandAsk) -> bool {
        self.answer
    }
    fn trash(&self, p: &str) -> Result<(), String> {
        self.trashed.lock().unwrap().push(p.to_owned());
        std::fs::remove_dir_all(p).or_else(|_| std::fs::remove_file(p)).map_err(|e| e.to_string())
    }
    fn remove(&self, p: &str) -> Result<(), String> {
        self.trash(p)
    }
    fn measure(&self, _p: &str) -> Option<u64> {
        None
    }
    fn record(&self, e: Value) {
        self.records.lock().unwrap().push(e);
    }
    fn tracked(&self, _dir: &str, _name: &str) -> Option<bool> {
        Some(false)
    }
    fn now(&self) -> f64 {
        crate::disk_map::now_ms() as f64
    }
}

fn set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn requests_are_parsed_like_the_shared_parsers() {
    assert!(parse_scan(&json!({ "ids": ["build"] })).is_some_and(|r| r.roots.is_empty() && r.open.is_empty()));
    assert!(parse_scan(&json!({ "roots": [] })).is_none());
    assert!(parse_scan(&json!({ "ids": ["a\u{1}"] })).is_none());
    assert!(parse_scan(&json!([1])).is_none());
    let d = parse_delete(&json!({ "paths": ["C:\\a", "C:\\a"], "permanent": true, "oneTap": true })).unwrap();
    assert_eq!(d.paths, vec!["C:\\a".to_owned()]);
    assert!(d.permanent && d.one_tap);
    assert!(parse_delete(&json!({ "paths": [] })).is_none());
    assert_eq!(parse_command(&json!({ "path": "docker:images" })), Some("docker:images".into()));
    assert_eq!(parse_command(&json!({ "path": "" })), None);
}

#[test]
fn junk_needs_a_project_marker_next_to_it() {
    assert_eq!(junk_kind("node_modules", &set(&["package.json"])), Some("node"));
    assert_eq!(junk_kind("build", &set(&["readme.md"])), None);
    assert_eq!(junk_kind("build", &set(&["pubspec.yaml"])), Some("flutter"));
    assert_eq!(junk_kind("target", &set(&["cargo.toml"])), Some("rust"));
    assert_eq!(junk_kind(".venv", &set(&["requirements.txt"])), Some("python"));
    assert_eq!(junk_kind("dist", &set(&["build.gradle"])), None);
    assert_eq!(build_dir_kind("out", &set(&[]), true), Some("other"));
}

#[test]
fn versions_compare_by_number_then_text() {
    let mut v = vec!["35.0.0", "30.0.3", "34.0.0-rc1", "34.0.0"];
    v.sort_by(|a, b| version_cmp(a, b));
    assert_eq!(v, vec!["30.0.3", "34.0.0-rc1", "34.0.0", "35.0.0"]);
}

#[test]
fn build_files_and_wrappers_are_read() {
    let mut u = AndroidUse::default();
    read_build_file("plugins { id(\"com.android.application\") }\nandroid {\n  compileSdk = 34\n  buildToolsVersion = \"35.0.0\"\n}\n", &mut u);
    assert!(u.android && u.compile.contains(&34) && u.tools.contains("35.0.0") && !u.compile_unsure && !u.tools_unsure);
    let mut g = AndroidUse::default();
    read_build_file("android {\n compileSdkVersion libs.versions.sdk\n}", &mut g);
    assert!(g.compile_unsure && g.tools_unsure);
    assert_eq!(read_wrapper("distributionUrl=https\\://services.gradle.org/distributions/gradle-8.9-bin.zip\n"), Some("gradle-8.9-bin".into()));
    assert_eq!(read_wrapper("distributionUrl=x.zip"), None);
}

#[test]
fn docker_df_lines_become_rows() {
    let rows = parse_docker_df("{\"Type\":\"Images\",\"TotalCount\":\"6\",\"Active\":\"2\",\"Size\":\"3GB\",\"Reclaimable\":\"1.50GB (50%)\"}\n{\"Type\":\"Local Volumes\",\"TotalCount\":\"2\",\"Active\":\"0\",\"Size\":\"300MB\",\"Reclaimable\":\"300MB (100%)\"}\nnoise\n");
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].kind, rows[0].total, rows[0].active, rows[0].reclaimable), ("images", 6, 2, 1_500_000_000));
    assert_eq!(rows[1].kind, "volumes");
    assert_eq!(docker_bytes(&json!("2KiB")), 2048);
    assert_eq!(docker_bytes(&json!(5)), 0);
}

#[test]
fn avd_config_lists_used_images() {
    let root = tmp_root("avd");
    write(&root, "Pixel.avd/config.ini", 0, 0);
    std::fs::write(std::path::Path::new(&root).join("Pixel.avd").join("config.ini"), "image.sysdir.1=system-images\\android-34\\google_apis\\x86_64\\\n").unwrap();
    let (used, readable) = avd_images(&root);
    assert!(readable && used.contains("android-34/google_apis/x86_64"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn locks_follow_open_panes_running_holders_and_the_app() {
    let plat = Plat::Win;
    let open = vec!["C:\\p\\open".to_owned()];
    let run = vec![Holder { cwd: "C:\\p\\srv".into(), lock: "server" }];
    let me = vec!["C:\\app".to_owned()];
    assert_eq!(lock_of("C:\\p\\open\\build", Some("C:\\p\\open"), &open, &run, &me, plat), Some("open"));
    assert_eq!(lock_of("C:\\p\\srv\\dist", Some("C:\\p\\srv"), &open, &run, &me, plat), Some("server"));
    assert_eq!(lock_of("C:\\p\\srv\\dist", None, &open, &run, &me, plat), None);
    assert_eq!(lock_of("C:\\app\\x", None, &open, &run, &me, plat), Some("self"));
    assert_eq!(lock_of("C:\\p\\old\\build", None, &open, &run, &me, plat), None);
    assert!(refused_root("C:\\", "C:\\Users\\me", plat));
    assert!(refused_root("c:\\users\\me\\", "C:\\Users\\me", plat));
    assert!(!refused_root("C:\\Users\\me\\code", "C:\\Users\\me", plat));
}

#[test]
fn system_rules_keep_documents_and_the_home_itself() {
    let r = sys_roots(Plat::Win, "C:\\Users\\me", &|_| None, None, "", Some("D:\\fake"));
    assert!(is_forbidden("D:\\fake\\home\\Documents\\a.docx", &r, &[]));
    assert!(is_forbidden("D:\\fake\\Windows\\Temp", &r, &[]));
    assert!(is_forbidden("D:\\fake", &r, &[]));
    assert!(!is_forbidden("D:\\fake\\Temp\\old.log", &r, &[]));
    assert!(installer_name("Setup-1.0.exe", Plat::Win) && installer_name("tool-install.zip", Plat::Win) && !installer_name("photos.zip", Plat::Win));
    assert_eq!(parse_lsappinfo("1) \"Safari\" ASN:0x0\n bundleID=\"com.apple.Safari\"\n"), vec!["com.apple.Safari".to_owned(), "Safari".to_owned()]);
}

struct NoSys;

impl SysIo for NoSys {
    fn procs(&self) -> Vec<String> {
        vec!["msedge.exe".into()]
    }
    fn bin_size(&self) -> Option<(u64, u64)> {
        Some((10, 1))
    }
    fn bin_empty(&self) -> Result<(), String> {
        Ok(())
    }
    fn busy(&self, file: &str) -> bool {
        file.ends_with("locked.tmp")
    }
}

#[test]
fn scan_finds_project_junk_and_delete_only_takes_known_unlocked_paths() {
    let root = tmp_root("scan");
    let home = guard::join(&root, "home", Plat::host());
    let j = |rel: &str| guard::join(&root, rel, Plat::host());
    write(&root, "old/package.json", 20, 60);
    write(&root, "old/package-lock.json", 20, 60);
    write(&root, "old/node_modules/lib/index.js", 3000, 0);
    write(&root, "old/dist/app.js", 500, 0);
    write(&root, "new/package.json", 20, 0);
    write(&root, "new/pnpm-lock.yaml", 20, 0);
    write(&root, "new/node_modules/lib/index.js", 1200, 0);
    write(&root, "open/pubspec.yaml", 20, 90);
    write(&root, "open/build/app.bin", 2000, 0);
    write(&root, "docs/build/page.html", 100, 0);
    let svc = Service::new(Fake { home, answer: true, ..Default::default() }, None, None, None);
    let ids: Vec<Value> = svc.categories().as_array().unwrap().iter().map(|c| c["id"].clone()).collect();
    assert!(ids.contains(&json!("node_modules")) && ids.contains(&json!("build")) && ids.contains(&json!("cache")));
    let res = svc.scan(&ScanReq { ids: vec!["node_modules".into(), "build".into()], roots: vec![root.clone()], open: vec![j("open")] });
    let items = res["items"].as_array().unwrap();
    let paths: HashSet<String> = items.iter().map(|i| i["path"].as_str().unwrap().to_owned()).collect();
    assert_eq!(paths, [j("old/node_modules"), j("old/dist"), j("new/node_modules"), j("open/build")].into_iter().map(|p| guard::normalize(&p, Plat::host())).collect());
    let by = |p: &str| items.iter().find(|i| i["path"] == json!(guard::normalize(&j(p), Plat::host()))).unwrap().clone();
    assert_eq!(by("open/build")["locked"], json!("open"));
    assert_eq!(by("new/node_modules")["restore"], json!("pnpm install"));
    assert_eq!(by("old/node_modules")["restore"], json!("npm install"));
    assert_eq!(by("open/build")["restore"], json!("flutter pub get"));
    assert_eq!(by("old/node_modules")["size"], json!(3000));
    assert!(svc.deps.events.lock().unwrap().iter().any(|e| e["state"] == json!("done") && e["category"] == json!("build")));

    let forged = svc.remove(&DeleteReq { paths: vec![j("docs")], permanent: true, open: vec![], one_tap: false });
    assert_eq!(forged["failed"][0]["reason"], json!("invalid"));
    let locked = svc.remove(&DeleteReq { paths: vec![j("open/build")], permanent: true, open: vec![], one_tap: false });
    assert_eq!(locked["failed"][0]["reason"], json!("locked:open"));
    let done = svc.remove(&DeleteReq { paths: vec![j("old/node_modules"), j("old/dist")], permanent: false, open: vec![], one_tap: false });
    assert_eq!(done["done"].as_array().unwrap().len(), 2, "{done}");
    assert_eq!(done["freed"], json!(3500));
    assert!(!std::path::Path::new(&j("old/node_modules")).exists() && std::path::Path::new(&j("new/node_modules")).exists());
    let rec = svc.deps.records.lock().unwrap()[0].clone();
    assert_eq!((rec["action"].clone(), rec["count"].clone()), (json!("trash"), json!(2)));
    let again = svc.remove(&DeleteReq { paths: vec![j("old/dist")], permanent: false, open: vec![], one_tap: false });
    assert_eq!(again["failed"][0]["reason"], json!("invalid"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn temp_members_skip_new_and_locked_files_and_one_tap_deletes_them() {
    let root = tmp_root("sys");
    let fake = guard::join(&root, "fake", Plat::host());
    let t = if cfg!(target_os = "macos") { "var/T" } else { "Temp" };
    let old = write(&fake, &format!("{t}/old.log"), 400, 3);
    let newer = write(&fake, &format!("{t}/new.log"), 100, 0);
    let held = write(&fake, &format!("{t}/locked.tmp"), 100, 4);
    let roots = sys_roots(Plat::host(), "C:\\Users\\nobody", &|_| None, None, "", Some(&fake));
    let system = SysDeps { roots, io: Box::new(NoSys) };
    let svc = Service::new(Fake { home: "C:\\Users\\nobody".into(), answer: true, ..Default::default() }, Some(system), None, None);
    let res = svc.scan(&ScanReq { ids: vec!["temp".into(), "bin".into()], roots: vec![], open: vec![] });
    let temp = res["items"].as_array().unwrap().iter().find(|i| i["category"] == json!("temp")).cloned().unwrap();
    assert_eq!((temp["files"].clone(), temp["size"].clone(), temp["action"].clone()), (json!(1), json!(400), json!("files")));
    let bin = res["items"].as_array().unwrap().iter().find(|i| i["category"] == json!("bin")).cloned().unwrap();
    assert_eq!((bin["size"].clone(), bin["action"].clone()), (json!(10), json!("bin")));
    let tap = svc.remove(&DeleteReq { paths: vec![temp["path"].as_str().unwrap().to_owned()], permanent: false, open: vec![], one_tap: true });
    assert_eq!(tap["done"][0]["path"], temp["path"], "{tap}");
    assert_eq!(tap["permanent"], json!(true));
    assert!(!std::path::Path::new(&old).exists() && std::path::Path::new(&newer).exists() && std::path::Path::new(&held).exists());
    assert_eq!(svc.empty_bin()["done"], json!(true));
    assert_eq!(svc.run_command("docker:images")["error"], json!("invalid"));
    let _ = std::fs::remove_dir_all(&root);
}
