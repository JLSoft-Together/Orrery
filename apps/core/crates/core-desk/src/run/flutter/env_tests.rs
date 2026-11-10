use super::*;
use crate::run::flutter::deps::ExecResult;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

const VERSION_JSON: &str = r#"{"frameworkVersion":"3.24.3","channel":"stable","dartSdkVersion":"3.5.3","flutterRoot":"/x"}"#;

#[derive(Default)]
struct Fake {
    files: HashSet<String>,
    links: BTreeMap<String, String>,
    dirs: BTreeMap<String, Vec<String>>,
    texts: BTreeMap<String, String>,
    exec: Option<ExecResult>,
    calls: RefCell<Vec<(String, Vec<String>, bool)>>,
}

impl EnvDeps for Fake {
    fn is_file(&self, f: &str) -> bool {
        self.files.contains(f)
    }
    fn read_text(&self, f: &str) -> Option<String> {
        self.texts.get(f).cloned()
    }
    fn readdir(&self, d: &str) -> Vec<String> {
        self.dirs.get(d).cloned().unwrap_or_default()
    }
    fn realpath(&self, f: &str) -> String {
        self.links.get(f).cloned().unwrap_or_else(|| f.to_string())
    }
    fn exec(&self, file: &str, args: &[String], _: u64, _: &Env, verbatim: bool) -> ExecResult {
        self.calls.borrow_mut().push((file.into(), args.to_vec(), verbatim));
        self.exec.clone().unwrap_or(ExecResult { ok: true, stdout: VERSION_JSON.into(), stderr: String::new() })
    }
}

fn fake(files: &[&str]) -> Fake {
    Fake { files: files.iter().map(|s| s.to_string()).collect(), ..Default::default() }
}

fn env(pairs: &[(&str, &str)]) -> Env {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn mac(e: &[(&str, &str)]) -> FlutterEnvOpts {
    FlutterEnvOpts { env: env(e), win: false, home: "/Users/me".into(), ..Default::default() }
}

fn win(extra: &[(&str, &str)]) -> FlutterEnvOpts {
    let mut e = env(&[("SystemRoot", "C:\\Windows"), ("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local")]);
    e.extend(env(extra));
    FlutterEnvOpts { env: e, win: true, home: "C:\\Users\\me".into(), ..Default::default() }
}

fn issue(code: &str, d: &str) -> EnvIssue {
    EnvIssue::with(code, d)
}

#[test]
fn setting_first_accepting_bin() {
    let deps = fake(&["/sdk/a/bin/flutter", "/sdk/env/bin/flutter"]);
    let ok = find_flutter_sdk_with("/p", &FlutterEnvOpts { setting: Some("/sdk/a/bin".into()), ..mac(&[("FLUTTER_ROOT", "/sdk/env")]) }, &deps);
    assert_eq!((ok.sdk.as_deref(), ok.source, ok.issues.len()), (Some("/sdk/a"), Some("setting"), 0));
    let bad = find_flutter_sdk_with("/p", &FlutterEnvOpts { setting: Some("/nope".into()), ..mac(&[("FLUTTER_ROOT", "/sdk/env")]) }, &deps);
    assert_eq!((bad.sdk.as_deref(), bad.source), (Some("/sdk/env"), Some("FLUTTER_ROOT")));
    assert_eq!(bad.issues, vec![issue("flutter-setting-invalid", "/nope")]);
}

#[test]
fn sdk_root_named_flutter() {
    let deps = fake(&["C:\\src\\flutter\\bin\\flutter.bat"]);
    let a = find_flutter_sdk_with("C:\\p", &FlutterEnvOpts { setting: Some("C:\\src\\flutter".into()), ..win(&[]) }, &deps);
    assert_eq!((a.sdk.as_deref(), a.source, a.issues.len()), (Some("C:\\src\\flutter"), Some("setting"), 0));
    let b = find_flutter_sdk_with("C:\\p", &FlutterEnvOpts { setting: Some("C:\\src\\flutter\\bin\\flutter.bat".into()), ..win(&[]) }, &deps);
    assert_eq!((b.sdk.as_deref(), b.source), (Some("C:\\src\\flutter"), Some("setting")));
}

#[test]
fn fvm_pin() {
    let texts: BTreeMap<String, String> = [("/p/.fvmrc".to_string(), r#"{"flutter":"3.22.0"}"#.to_string())].into();
    let deps = Fake { texts: texts.clone(), ..fake(&["/Users/me/fvm/versions/3.22.0/bin/flutter", "/sdk/env/bin/flutter"]) };
    let a = find_flutter_sdk_with("/p", &mac(&[("FLUTTER_ROOT", "/sdk/env")]), &deps);
    assert_eq!((a.sdk.as_deref(), a.source, a.fvm.as_deref()), (Some("/Users/me/fvm/versions/3.22.0"), Some("fvm"), Some("3.22.0")));
    let custom = Fake { texts: texts.clone(), ..fake(&["/cache/versions/3.22.0/bin/flutter"]) };
    assert_eq!(find_flutter_sdk_with("/p", &mac(&[("FVM_CACHE_PATH", "/cache")]), &custom).sdk.as_deref(), Some("/cache/versions/3.22.0"));
    let missing = Fake { texts, ..fake(&["/sdk/env/bin/flutter"]) };
    let m = find_flutter_sdk_with("/p", &mac(&[("FLUTTER_ROOT", "/sdk/env")]), &missing);
    assert_eq!((m.sdk.as_deref(), m.source, m.issues), (Some("/sdk/env"), Some("FLUTTER_ROOT"), vec![issue("fvm-version-missing", "3.22.0")]));
    let link = find_flutter_sdk_with("/p", &mac(&[]), &fake(&["/p/.fvm/flutter_sdk/bin/flutter"]));
    assert_eq!((link.sdk.as_deref(), link.source, link.fvm), (Some("/p/.fvm/flutter_sdk"), Some("fvm"), None));
}

#[test]
fn path_and_defaults() {
    let brew = "/opt/homebrew/Caskroom/flutter/3.24.3/flutter/bin/flutter";
    let deps = Fake { links: [("/opt/homebrew/bin/flutter".to_string(), brew.to_string())].into(), ..fake(&["/opt/homebrew/bin/flutter", brew]) };
    let p = find_flutter_sdk_with("/p", &mac(&[("PATH", "relative:/usr/bin:/opt/homebrew/bin")]), &deps);
    assert_eq!((p.sdk.as_deref(), p.source), (Some("/opt/homebrew/Caskroom/flutter/3.24.3/flutter"), Some("path")));
    let w = find_flutter_sdk_with("C:\\p", &win(&[("Path", "C:\\Windows;D:\\tools\\flutter\\bin")]), &fake(&["D:\\tools\\flutter\\bin\\flutter.bat"]));
    assert_eq!((w.sdk.as_deref(), w.source), (Some("D:\\tools\\flutter"), Some("path")));
    assert_eq!(find_flutter_sdk_with("C:\\p", &win(&[]), &fake(&["C:\\Users\\me\\AppData\\Local\\flutter\\bin\\flutter.bat"])).source, Some("default"));
    let cask = Fake { dirs: [("/opt/homebrew/Caskroom/flutter".to_string(), vec!["3.19.0".to_string(), "3.24.3".to_string()])].into(), ..fake(&[brew]) };
    assert_eq!(find_flutter_sdk_with("/p", &mac(&[]), &cask).sdk.as_deref(), Some("/opt/homebrew/Caskroom/flutter/3.24.3/flutter"));
    assert_eq!(find_flutter_sdk_with("/p", &mac(&[]), &fake(&[])), FlutterSdkLookup { sdk: None, source: None, fvm: None, issues: vec![EnvIssue::new("flutter-missing")] });
}

#[test]
fn project_setting_wins() {
    let deps = Fake { files: ["/pinned/bin/flutter", "/global/bin/flutter"].iter().map(|s| s.to_string()).collect(), ..Default::default() };
    let mut settings = RunSettings::new();
    settings.flutter_sdk = Some("/global".into());
    settings.flutter_sdk_projects.insert(project_sdk_key("/proj", false), "/pinned".into());
    let opts = |dir: &str, s: &RunSettings| {
        let (project_setting, setting) = sdk_settings(s, dir, false);
        FlutterEnvOpts { project_setting, setting, home: "/h".into(), ..Default::default() }
    };
    let a = find_flutter_sdk_with("/proj", &opts("/proj", &settings), &deps);
    assert_eq!((a.sdk.as_deref(), a.source), (Some("/pinned"), Some("project")));
    let b = find_flutter_sdk_with("/other", &opts("/other", &settings), &deps);
    assert_eq!((b.sdk.as_deref(), b.source), (Some("/global"), Some("setting")));
    settings.flutter_sdk_projects.insert("/proj".into(), "/missing".into());
    let c = find_flutter_sdk_with("/proj", &opts("/proj", &settings), &deps);
    assert_eq!((c.sdk.as_deref(), c.issues), (Some("/global"), vec![issue("flutter-project-invalid", "/missing")]));
    assert_eq!(project_sdk_key("C:\\Users\\Me\\App\\", true), "c:\\users\\me\\app");
    assert_eq!(project_sdk_key("/a/b/../c/", false), "/a/c");
}

#[test]
fn env_runs_version() {
    let deps = fake(&["/sdk/bin/flutter"]);
    let v = flutter_env_with("/p", &mac(&[("FLUTTER_ROOT", "/sdk")]), &deps);
    assert_eq!(crate::js::stringify(&v), r#"{"kind":"flutter","sdk":"/sdk","source":"FLUTTER_ROOT","version":"3.24.3","dart":"3.5.3","channel":"stable","fvm":false,"issues":[]}"#);
    assert_eq!(deps.calls.borrow()[0], ("/sdk/bin/flutter".to_string(), vec!["--version".to_string(), "--machine".to_string()], false));
    let failed = Fake { exec: Some(ExecResult { ok: false, stdout: String::new(), stderr: "boom".into() }), ..fake(&["/sdk/bin/flutter"]) };
    let f = flutter_env_with("/p", &mac(&[("FLUTTER_ROOT", "/sdk")]), &failed);
    assert_eq!((f["version"].clone(), f["issues"].clone()), (Value::Null, json!([{"code": "flutter-version-failed", "detail": "/sdk"}])));
    let none = fake(&[]);
    assert_eq!(flutter_env_with("/p", &mac(&[]), &none)["issues"], json!([{"code": "flutter-missing"}]));
    assert!(none.calls.borrow().is_empty());
    let w = fake(&["C:\\src\\flutter\\bin\\flutter.bat"]);
    assert_eq!(flutter_env_with("C:\\p", &win(&[]), &w)["source"], "default");
    let call = w.calls.borrow()[0].clone();
    assert_eq!((call.0.as_str(), call.2), ("C:\\Windows\\System32\\cmd.exe", true));
}

#[test]
fn local_sdk_is_not_executed_unless_trusted() {
    let deps = fake(&["/p/.fvm/flutter_sdk/bin/flutter"]);
    let v = flutter_env_with("/p", &mac(&[]), &deps);
    assert_eq!((v["sdk"].as_str(), v["source"].as_str(), v["version"].clone()), (Some("/p/.fvm/flutter_sdk"), Some("fvm"), Value::Null));
    assert_eq!(v["issues"], json!([{"code": "fvm-local-unverified", "detail": "/p/.fvm/flutter_sdk"}]));
    assert!(deps.calls.borrow().is_empty());
    assert_eq!(flutter_env_with("/p", &FlutterEnvOpts { trusted: true, ..mac(&[]) }, &deps)["version"], "3.24.3");
    assert_eq!(deps.calls.borrow().len(), 1);
    let linked = Fake {
        links: [("/cache/versions/3.22.0".to_string(), "/repo/tools/flutter".to_string())].into(),
        texts: [("/repo/app/.fvmrc".to_string(), r#"{"flutter":"3.22.0"}"#.to_string())].into(),
        ..fake(&["/cache/versions/3.22.0/bin/flutter"])
    };
    let opts = mac(&[("FVM_CACHE_PATH", "/cache")]);
    let r = flutter_env_with("/repo/app", &FlutterEnvOpts { repo_root: Some("/repo".into()), ..opts.clone() }, &linked);
    assert_eq!(r["issues"], json!([{"code": "fvm-local-unverified", "detail": "/cache/versions/3.22.0"}]));
    assert!(linked.calls.borrow().is_empty());
    assert_eq!(flutter_env_with("/repo/app", &opts, &linked)["version"], "3.24.3");
}

#[test]
fn version_parsing_and_spawn() {
    assert_eq!(parse_flutter_version(&format!("Welcome to Flutter!\n{VERSION_JSON}\n")), Some(FlutterVersion { version: "3.24.3".into(), dart: Some("3.5.3".into()), channel: Some("stable".into()) }));
    assert_eq!(parse_flutter_version(r#"{"dartSdkVersion":"3.7.0 (build 3.7.0-1.0.dev)","frameworkVersion":"3.29.0"}"#), Some(FlutterVersion { version: "3.29.0".into(), dart: Some("3.7.0".into()), channel: None }));
    assert_eq!(parse_flutter_version("nope"), None);
    let posix = flutter_spawn("/sdk", &["run".into(), "-d".into(), "my phone".into()], false, &Env::new()).unwrap();
    assert_eq!(posix, Resolved { file: "/sdk/bin/flutter".into(), args: vec!["run".into(), "-d".into(), "my phone".into()], verbatim: false });
    let e = env(&[("SystemRoot", "C:\\Windows")]);
    let w = flutter_spawn("C:\\Program Files\\flutter", &["run".into(), "--dart-define=A=b c".into(), "x&calc".into()], true, &e).unwrap();
    assert_eq!((w.file.as_str(), w.verbatim), ("C:\\Windows\\System32\\cmd.exe", true));
    assert_eq!(&w.args[..3], &["/d", "/s", "/c"]);
    assert!(w.args[3].starts_with("\"C:\\Program^ Files\\flutter\\bin\\flutter.bat "));
    assert!(w.args[3].contains("^^^\"x^^^&calc^^^\""));
    let q = crate::preview::format::quote_cmd_arg("x&y").unwrap();
    assert!(flutter_spawn("C:\\a b", &["x&y".into()], true, &e).unwrap().args[3].ends_with(&format!("{q}\"")));
    assert_eq!(flutter_spawn("/sdk", &["a\nb".into()], true, &e), None);
}
