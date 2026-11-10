use core_desk::backup::format;
use core_desk::libs::checker::LibChecker;
use core_desk::libs::http::{Fetch, Fetched};
use core_desk::{Ctx, Desk, DeskEnv, DeskError, DeskIo};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("core-desk-{name}-{}", core_desk::util::uuid()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Host {
    save_to: PathBuf,
    pick: PathBuf,
    answer: i64,
    events: Mutex<Vec<(String, Value)>>,
    calls: Mutex<Vec<String>>,
}

impl DeskIo for Host {
    fn emit(&self, event: &str, payload: Value) {
        self.events.lock().unwrap().push((event.to_string(), payload));
    }

    fn host(&self, _req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        self.calls.lock().unwrap().push(method.to_string());
        Ok(match method {
            "backup.readLocal" => json!({"ccm.theme": "dark", "ccm.prompts": "sk-ant-api03-abcdefghijk", "ccm.auth": "x"}),
            "backup.github" => json!([{"login": "octo", "name": "Octo"}]),
            "backup.saveDialog" => {
                assert!(params["defaultPath"].as_str().unwrap().contains("orrery-backup-"));
                json!(self.save_to.to_string_lossy())
            }
            "backup.openDialog" => json!(self.pick.to_string_lossy()),
            "backup.confirmApply" => json!(true),
            "buildClean.confirm" => json!(self.answer),
            "buildClean.running" | "buildClean.daemons" => json!([]),
            _ => Value::Null,
        })
    }
}

fn host(dir: &Path) -> Host {
    Host { save_to: dir.join("out.json"), pick: dir.join("out.json"), answer: 0, events: Mutex::new(vec![]), calls: Mutex::new(vec![]) }
}

#[test]
fn backup_sections_and_scrub() {
    assert_eq!(format::local_section("ccm.theme"), Some("appearance"));
    assert_eq!(format::local_section("ccm.mapLayout.v53"), Some("workspace"));
    assert_eq!(format::local_section("ccm.bookmarks"), Some("notes"));
    assert_eq!(format::local_section("ccm.notify.waiting"), Some("settings"));
    assert_eq!(format::local_section("ccm.github.token"), None);
    assert_eq!(format::local_section("ccm.inbox"), None);
    assert_eq!(format::file_section("adb-devices.json"), Some("devices"));
    assert_eq!(format::file_section("notes/x.md"), None);
    let v = format::scrub_json(&json!({"name": "a", "token": "x", "env": {"DB": "p"}, "n": [{"password": "p", "note": "use ghp_abcdefghijklmnopqrstuvwx1234 here"}]}), 0);
    assert_eq!(v, json!({"name": "a", "env": {}, "n": [{"note": "use [removed] here"}]}));
    assert_eq!(format::clean_section_ids(&json!(["names", "names"])), Some(vec!["names".to_string()]));
    assert_eq!(format::clean_section_ids(&json!(["x"])), None);
    assert_eq!(format::export_name((2026, 3, 7)), "orrery-backup-2026-03-07.json");
    let list = vec![("pre-import-a.json".to_string(), 1.0), ("pre-import-b.json".to_string(), 3.0), ("pre-import-c.json".to_string(), 2.0), ("x.json".to_string(), 9.0)];
    assert_eq!(format::prune_list(&list, "pre-import-", 2), vec!["pre-import-a.json".to_string()]);
    assert!(format::parse_backup("{\"format\":\"orrery-backup\",\"version\":2}").is_none());
    assert_eq!(core_desk::util::to_fixed1(1.25), "1.3");
    assert_eq!(core_desk::util::to_fixed1(1.24), "1.2");
}

#[test]
fn backup_export_pick_apply_through_the_desk() {
    let dir = tmp("backup");
    std::fs::write(dir.join("aliases.json"), "{\"a\":{\"name\":\"A\",\"token\":\"t\"}}").unwrap();
    let env = DeskEnv { data_dir: dir.clone(), app_version: "1.2.3".into(), documents_dir: dir.clone(), ..DeskEnv::default() };
    let desk = Desk::standard(&env);
    let h = host(&dir);
    let ctx = Ctx::new(&h, 1);
    let out = desk.dispatch_with(&ctx, "ccm:backup-export", json!([])).unwrap();
    assert_eq!(out["ok"], json!(true));
    let text = std::fs::read_to_string(dir.join("out.json")).unwrap();
    assert!(text.starts_with("{\n \"format\": \"orrery-backup\",\n \"version\": 1,\n \"app\": \"1.2.3\""));
    assert!(!text.contains("ccm.auth") && text.contains("[removed]"));
    let preview = desk.dispatch_with(&ctx, "ccm:backup-pick", json!([])).unwrap();
    assert_eq!(preview["file"], json!("out.json"));
    let token = preview["token"].as_str().unwrap().to_string();
    let applied = desk.dispatch_with(&ctx, "ccm:backup-apply", json!([token, ["names", "github"]])).unwrap();
    assert_eq!(applied, json!({"ok": true, "local": {}, "github": ["octo"]}));
    assert!(dir.join("backups").join("pending-import.json").exists());
    std::fs::remove_file(dir.join("aliases.json")).unwrap();
    assert_eq!(desk.dispatch("desk:backup-apply-pending", json!([])).unwrap(), json!(1));
    assert_eq!(std::fs::read_to_string(dir.join("aliases.json")).unwrap(), "{\"a\":{\"name\":\"A\"}}");
    assert_eq!(desk.dispatch_with(&ctx, "ccm:backup-apply", json!([token, ["names"]])).unwrap(), json!({"ok": false, "error": "expired"}));
    assert!(matches!(desk.dispatch("ccm:nope", json!([])), Err(DeskError::UnknownChannel(_))));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn build_clean_scans_and_deletes_offered_folders_only() {
    let dir = tmp("clean");
    let proj = dir.join("proj");
    std::fs::create_dir_all(proj.join("node_modules/a")).unwrap();
    std::fs::write(proj.join("package.json"), "{}").unwrap();
    std::fs::write(proj.join("node_modules/a/i.js"), "12345").unwrap();
    std::fs::create_dir_all(proj.join("src")).unwrap();
    let desk = Desk::standard(&DeskEnv { data_dir: dir.clone(), ..DeskEnv::default() });
    let h = host(&dir);
    let ctx = Ctx::new(&h, 1);
    let root = proj.to_string_lossy().to_string();
    let scan = desk.dispatch_with(&ctx, "ccm:build-scan", json!([[root]])).unwrap();
    assert_eq!(scan[0]["dirs"][0]["rel"], json!("node_modules"));
    assert_eq!(scan[0]["bytes"], json!(5.0));
    let bad = desk.dispatch_with(&ctx, "ccm:build-clean", json!([{"items": [{"root": root, "rel": "build"}]}])).unwrap();
    assert_eq!(bad, json!({"ok": false, "error": "invalid"}));
    let done = desk.dispatch_with(&ctx, "ccm:build-clean", json!([{"items": [{"root": root, "rel": "node_modules"}]}])).unwrap();
    assert_eq!(done["ok"], json!(true));
    assert_eq!(done["dirs"], json!(1.0));
    assert!(!proj.join("node_modules").exists() && proj.join("src").exists());
    assert!(h.events.lock().unwrap().iter().all(|(e, _)| e == "ccm:build-event"));
    assert!(h.calls.lock().unwrap().contains(&"buildClean.confirm".to_string()));
    assert_eq!(desk.dispatch("ccm:build-cancel", json!([])).unwrap(), json!(false));
    let _ = std::fs::remove_dir_all(dir);
}

struct FakeFetch(Mutex<Vec<String>>);

impl Fetch for FakeFetch {
    fn fetch(&self, url: &str, _accept: &str, body: Option<&str>) -> Fetched {
        self.0.lock().unwrap().push(url.to_string());
        if url.ends_with("/v1/querybatch") {
            assert!(body.unwrap().contains("\"ecosystem\":\"npm\""));
            return Fetched::Text("{\"results\":[{\"vulns\":[{\"id\":\"GHSA-1\"}]}]}".into());
        }
        if url.contains("/v1/vulns/") {
            return Fetched::Status(500);
        }
        if url.ends_with("/npm/vue") {
            return Fetched::Text("{\"versions\":{\"3.0.0\":{},\"3.1.0-beta.1\":{}}}".into());
        }
        Fetched::Status(404)
    }
}

#[test]
fn lib_checker_caches_lookups() {
    let dir = tmp("libs");
    let fetch = Box::new(FakeFetch(Mutex::new(vec![])));
    let checker = LibChecker::new(fetch, dir.clone(), Some("http://127.0.0.1:1".into()), Some("http://127.0.0.1:1".into()));
    let r = checker.check(&json!([{"eco": "npm", "name": "vue"}, {"eco": "pub", "name": "nope"}]), false);
    assert_eq!(r["results"][0]["stable"], json!("3.0.0"));
    assert_eq!(r["results"][0]["pre"], json!("3.1.0-beta.1"));
    assert_eq!(r["results"][1], json!({"eco": "pub", "name": "nope", "ok": false, "error": "not-found"}));
    let again = checker.check(&json!([{"eco": "npm", "name": "vue"}]), false);
    assert_eq!(again["results"][0]["checkedAt"], r["results"][0]["checkedAt"]);
    assert!(dir.join("lib-versions-cache.json").exists());
    let v = checker.vulns(&json!([{"eco": "npm", "name": "vue", "version": "3.0.0"}]), false);
    assert_eq!(v["results"][0]["vulns"], json!([{"id": "GHSA-1", "aliases": [], "summary": "", "severity": "unknown", "fixed": null}]));
    assert_eq!(checker.check(&json!("x"), false), json!({"ok": false, "error": "invalid"}));
    let _ = std::fs::remove_dir_all(dir);
}
