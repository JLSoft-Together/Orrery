use super::*;
use std::process::Command;

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("orrery-wtcfg-{tag}-{}-{}", std::process::id(), crate::util::now_ms() as u64));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn sh(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_full_config_is_read() {
    let c = parse(r#"{"copy": [".env*", " **/a.json "], "setup": ["npm install", "  ", "npm run gen"], "cleanup": ["docker compose down"]}"#);
    assert_eq!(c.copy, Some(vec![".env*".to_string(), "**/a.json".to_string()]));
    assert_eq!(c.setup, ["npm install", "npm run gen"]);
    assert_eq!(c.cleanup, ["docker compose down"]);
    assert!(c.errors.is_empty());
}

#[test]
fn missing_fields_stay_empty_and_copy_stays_unset() {
    let c = parse(r#"{"setup": ["a"]}"#);
    assert_eq!(c.copy, None);
    assert!(c.cleanup.is_empty());
    assert!(c.errors.is_empty());
    assert_eq!(parse("{}"), WtConfig::default());
}

#[test]
fn a_wrong_typed_field_is_dropped_and_reported_but_others_survive() {
    let c = parse(r#"{"copy": ".env", "setup": ["a", 3], "cleanup": ["x"]}"#);
    assert_eq!(c.copy, None);
    assert!(c.setup.is_empty());
    assert_eq!(c.cleanup, ["x"]);
    assert_eq!(c.errors, ["copy", "setup"]);
}

#[test]
fn bad_json_and_non_objects_report_json_and_keep_defaults() {
    for text in ["{", "[]", "3", ""] {
        let c = parse(text);
        assert_eq!(c.errors, ["json"], "{text}");
        assert!(c.setup.is_empty() && c.copy.is_none());
    }
    assert_eq!(parse("\u{feff}{\"setup\": [\"a\"]}").setup, ["a"]);
}

#[test]
fn lists_and_lines_are_capped() {
    let many: Vec<String> = (0..40).map(|i| format!("\"c{i}\"")).collect();
    let c = parse(&format!("{{\"setup\": [{}, \"{}\"]}}", many.join(","), "x".repeat(900)));
    assert_eq!(c.setup.len(), MAX_CMDS);
    let c = parse(&format!("{{\"setup\": [\"{}\"]}}", "x".repeat(900)));
    assert_eq!(c.setup[0].chars().count(), MAX_CMD_CHARS);
}

#[test]
fn a_missing_file_means_no_config() {
    let dir = tmp("none");
    let (exists, c) = read(&dir);
    assert!(!exists);
    assert_eq!(c, WtConfig::default());
    fs::create_dir_all(dir.join(".orrery")).unwrap();
    fs::write(dir.join(FILE), r#"{"setup": ["a"]}"#).unwrap();
    let (exists, c) = read(&dir);
    assert!(exists);
    assert_eq!(c.setup, ["a"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn approval_follows_the_exact_command_list() {
    let data = tmp("approve");
    let a = vec!["npm install".to_string()];
    let b = vec!["npm install".to_string(), "npm run gen".to_string()];
    assert!(is_approved(&data, "/r/p", "setup", &[]));
    assert!(!is_approved(&data, "/r/p", "setup", &a));
    assert!(approve(&data, "/r/p", "setup", &a));
    assert!(is_approved(&data, "/r/p", "setup", &a));
    assert!(!is_approved(&data, "/r/p", "setup", &b));
    assert!(!is_approved(&data, "/r/p", "cleanup", &a));
    assert!(!is_approved(&data, "/r/other", "setup", &a));
    assert!(approve(&data, "/r/p", "setup", &b));
    assert!(is_approved(&data, "/r/p", "setup", &b));
    assert!(!is_approved(&data, "/r/p", "setup", &a));
    assert_ne!(commands_hash(&["ab".to_string(), "c".to_string()]), commands_hash(&["a".to_string(), "bc".to_string()]));
    let _ = fs::remove_dir_all(&data);
}

#[test]
fn setup_is_guessed_from_the_lockfile() {
    let cases: [(&[&str], &[&str]); 8] = [
        (&["package-lock.json", "package.json"], &["npm ci"]),
        (&["pnpm-lock.yaml", "package.json"], &["pnpm install"]),
        (&["yarn.lock", "package.json"], &["yarn install"]),
        (&["bun.lockb", "package.json"], &["bun install"]),
        (&["package.json"], &["npm install"]),
        (&["pubspec.yaml"], &["flutter pub get"]),
        (&["package-lock.json", "pubspec.yaml"], &["npm ci", "flutter pub get"]),
        (&["Cargo.toml"], &[]),
    ];
    for (files, want) in cases {
        let dir = tmp("guess");
        for f in files {
            fs::write(dir.join(f), "").unwrap();
        }
        assert_eq!(suggest_setup(&dir), want, "{files:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_suggestion_keeps_what_the_file_already_has() {
    let dir = tmp("sugg");
    fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
    let fresh: Value = serde_json::from_str(&suggestion_text(&dir, &WtConfig::default())).unwrap();
    assert_eq!(fresh["setup"], json!(["pnpm install"]));
    assert_eq!(fresh["copy"], json!([".env*", "**/google-services.json"]));
    let have = WtConfig { copy: Some(vec![".npmrc".into()]), setup: vec!["make".into()], cleanup: vec!["make down".into()], errors: vec![] };
    let kept: Value = serde_json::from_str(&suggestion_text(&dir, &have)).unwrap();
    assert_eq!(kept, json!({"copy": [".npmrc"], "setup": ["make"], "cleanup": ["make down"]}));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cleanup_commands_stop_at_the_first_failure_and_time_out() {
    let dir = tmp("run");
    let ok = run_cleanup(&dir, &["echo hi".to_string()], 20_000);
    assert_eq!(ok["ok"], true, "{ok}");
    let bad = run_cleanup(&dir, &["exit 3".to_string(), "echo never".to_string()], 20_000);
    assert_eq!(bad["error"], "cleanup-failed", "{bad}");
    assert_eq!(bad["ran"], 0);
    let sleep = if cfg!(windows) { "Start-Sleep -Seconds 30" } else { "sleep 30" };
    let started = Instant::now();
    let slow = run_cleanup(&dir, &[sleep.to_string()], 700);
    assert_eq!(slow["error"], "timeout", "{slow}");
    assert!(started.elapsed().as_secs() < 15);
    assert_eq!(run_cleanup(&dir, &[], 1000)["ok"], true);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cleanup_is_refused_for_main_unknown_and_dirty_worktrees() {
    if crate::git::git_path().is_none() {
        return;
    }
    let base = tmp("target");
    let main = base.join("proj");
    fs::create_dir_all(&main).unwrap();
    sh(&main, &["init", "-q", "-b", "main"]);
    fs::write(main.join("a.txt"), "a").unwrap();
    sh(&main, &["add", "a.txt"]);
    sh(&main, &["commit", "-q", "-m", "init"]);
    let wt = base.join("wt");
    sh(&main, &["worktree", "add", "-q", "-b", "feat", &wt.to_string_lossy()]);
    fs::create_dir_all(main.join(".orrery")).unwrap();
    fs::write(main.join(FILE), r#"{"cleanup": ["echo bye"]}"#).unwrap();
    let dir = main.to_string_lossy().into_owned();
    assert_eq!(cleanup_target(&dir, &json!("main")).err().map(|e| e.0), Some("main"));
    assert_eq!(cleanup_target(&dir, &json!("nope")).err().map(|e| e.0), Some("unknown"));
    assert_eq!(cleanup_target(&dir, &json!("-x")).err().map(|e| e.0), Some("unknown"));
    let t = cleanup_target(&dir, &json!("feat")).ok().expect("clean worktree");
    assert_eq!(t.cleanup, ["echo bye"]);
    fs::write(wt.join("new.txt"), "x").unwrap();
    assert_eq!(cleanup_target(&dir, &json!("feat")).err().map(|e| e.0), Some("dirty"));
    let _ = fs::remove_dir_all(&base);
}
