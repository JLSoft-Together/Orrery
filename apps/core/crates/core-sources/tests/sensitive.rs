use core_sources::sensitive::classify::{classify_touch, ClassifyContext, SensitiveHit};
use core_sources::sensitive::settings::default_settings;
use core_store::extra::SensitiveSettings;
use core_transcript::rules::{command_hits, touch_of, Touch};
use serde_json::json;

struct Ctx {
    project: &'static str,
    home: &'static str,
    win: bool,
    safe: Vec<String>,
    settings: SensitiveSettings,
}

fn win() -> Ctx {
    Ctx { project: "F:\\work\\app", home: "C:\\Users\\me", win: true, safe: vec!["C:\\Users\\me\\.claude\\plans".into()], settings: default_settings() }
}

fn posix() -> Ctx {
    Ctx { project: "/home/me/app", home: "/home/me", win: false, safe: Vec::new(), settings: default_settings() }
}

fn classify(touch: &Touch, c: &Ctx) -> Vec<SensitiveHit> {
    classify_touch(touch, &ClassifyContext { project: Some(c.project), home: c.home, win: c.win, safe_dirs: &c.safe, settings: &c.settings, project_key: Some(c.project) })
}

fn rules(cmd: &str, c: &Ctx) -> Vec<String> {
    classify(&Touch::Cmds(command_hits(cmd, 0)), c).into_iter().map(|h| h.rule).collect()
}

fn file(f: &str, c: &Ctx) -> Vec<SensitiveHit> {
    classify(&Touch::File(f.to_owned()), c)
}

fn rule_names(hits: &[SensitiveHit]) -> Vec<&str> {
    hits.iter().map(|h| h.rule.as_str()).collect()
}

#[test]
fn flags_secret_files_by_name_case_insensitively() {
    let w = win();
    assert_eq!(rule_names(&file("F:\\work\\app\\.env.local", &w)), vec![".env*"]);
    let jks = file("F:/work/app/android/app/upload.JKS", &w);
    assert_eq!((jks[0].rule.as_str(), jks[0].path.as_deref()), ("*.jks", Some("android/app/upload.JKS")));
    assert_eq!(file("F:\\work\\app\\android\\key.properties", &w)[0].rule, "key.properties");
    assert!(file("F:\\work\\app\\src\\main.ts", &w).is_empty());
}

#[test]
fn flags_files_outside_the_project_but_not_the_session_plan_folders() {
    let w = win();
    assert_eq!(file("C:\\Users\\me\\.ssh\\config", &w), vec![SensitiveHit { rule: "outside-project".into(), path: Some("~/.ssh/config".into()), file: Some("c:/Users/me/.ssh/config".into()) }]);
    assert!(file("C:\\Users\\me\\.claude\\plans\\p.md", &w).is_empty());
    assert!(file("/f/work/app/README.md", &w).is_empty());
    assert!(file("f:/WORK/App/x.ts", &w).is_empty());
}

#[test]
fn custom_globs_ignore_list_and_group_switches() {
    let mut w = win();
    w.settings.globs = vec!["*.secret".into(), "config/prod/**".into()];
    assert_eq!(rule_names(&file("F:\\work\\app\\a.secret", &w)), vec!["custom:*.secret"]);
    assert_eq!(rule_names(&file("F:\\work\\app\\config\\prod\\db.yml", &w)), vec!["custom:config/prod/**"]);
    let mut ignored = win();
    ignored.settings.ignore.insert("F:\\work\\app".into(), vec![".env*".into()]);
    assert!(file("F:\\work\\app\\.env", &ignored).is_empty());
    let mut off = win();
    off.settings.groups.insert("outside".into(), false);
    assert!(file("D:\\other\\x.ts", &off).is_empty());
}

#[test]
fn git_force_push_reset_hard_and_clean() {
    let p = posix();
    assert_eq!(rules("git push --force origin main", &p), vec!["git-force-push"]);
    assert_eq!(rules("git push -f", &p), vec!["git-force-push"]);
    assert!(rules("git push --force-with-lease", &p).is_empty());
    assert_eq!(rules("git status && git reset --hard HEAD~1", &p), vec!["git-reset-hard"]);
    assert_eq!(rules("git clean -fdx", &p), vec!["git-clean"]);
    assert!(rules("git clean -n", &p).is_empty());
    assert_eq!(rules("git -C ../x push -f", &p), vec!["git-force-push"]);
}

#[test]
fn rm_rf_only_outside_the_project_home_or_a_root() {
    let p = posix();
    assert!(rules("rm -rf build node_modules", &p).is_empty());
    assert_eq!(rules("rm -rf ../other", &p), vec!["rm-rf"]);
    assert_eq!(rules("rm -rf ~", &p), vec!["rm-rf"]);
    assert_eq!(rules("sudo rm -fr /", &p), vec!["rm-rf"]);
    assert_eq!(rules("rm -rf /*", &p), vec!["rm-rf"]);
    assert_eq!(rules("rm -r -f $HOME/x", &p), vec!["rm-rf"]);
    assert!(rules("rm -rf \"$OUT\"", &p).is_empty());
    assert!(rules("rm -f a.txt", &p).is_empty());
    assert_eq!(classify(&Touch::Cmds(command_hits("rm -rf /tmp/cache", 0)), &p), vec![SensitiveHit { rule: "rm-rf".into(), path: Some("/tmp/cache".into()), file: None }]);
}

#[test]
fn windows_deletes_and_drive_roots() {
    let w = win();
    assert_eq!(rules("del /s /q C:\\", &w), vec!["del-s"]);
    assert!(rules("rd /s /q build", &w).is_empty());
    assert_eq!(rules("Remove-Item -Recurse -Force D:\\data", &w), vec!["remove-item-recurse"]);
    assert!(rules("Remove-Item -Path .\\dist -Recurse", &w).is_empty());
    assert_eq!(rules("rm -Recurse -Force $env:USERPROFILE\\Downloads", &w), vec!["remove-item-recurse"]);
    assert_eq!(rules("cmd /c \"rd /s /q E:\\\"", &w), vec!["del-s"]);
    assert_eq!(rules("rm -rf /c/Users", &w), vec!["rm-rf"]);
}

#[test]
fn adb_uninstall_drop_table_and_extraction() {
    let p = posix();
    assert_eq!(rules("adb -s emulator-5554 uninstall com.x", &p), vec!["adb-uninstall"]);
    assert_eq!(rules("adb shell pm uninstall com.x", &p), vec!["adb-uninstall"]);
    assert_eq!(rules("sqlite3 app.db \"drop table users\"", &p), vec!["drop-table"]);
    assert!(rules("echo hello", &p).is_empty());
    assert_eq!(touch_of("Edit", &json!({"file_path": "F:/a/.env", "old_string": "x"})), Some(Touch::File("F:/a/.env".into())));
    assert_eq!(touch_of("NotebookEdit", &json!({"notebook_path": "n.ipynb"})), Some(Touch::File("n.ipynb".into())));
    assert_eq!(touch_of("Read", &json!({"file_path": "F:/a/.env"})), None);
    assert_eq!(touch_of("Bash", &json!({"command": "ls -la"})), None);
}
