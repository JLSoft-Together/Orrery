use super::*;
use crate::run::android::fake_deps::FakeDeps;

fn env(pairs: &[(&str, &str)]) -> Env {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

fn mac(e: &[(&str, &str)]) -> AndroidEnvOpts {
    AndroidEnvOpts { env: env(e), home: "/Users/me".into(), ..Default::default() }
}

const ROOT: &str = "/work/app";
const JBR: &str = "/Applications/Android Studio.app/Contents/jbr/Contents/Home";

#[test]
fn read_property_unescapes() {
    assert_eq!(read_property("## generated\nsdk.dir=C\\:\\\\Users\\\\me\\\\AppData\\\\Local\\\\Android\\\\Sdk\n", "sdk.dir").as_deref(), Some("C:\\Users\\me\\AppData\\Local\\Android\\Sdk"));
    assert_eq!(read_property("sdk.dir = /Users/me/Library/Android/sdk", "sdk.dir").as_deref(), Some("/Users/me/Library/Android/sdk"));
    assert_eq!(read_property("ndk.dir=/x", "sdk.dir"), None);
}

#[test]
fn java_version_majors() {
    assert_eq!(parse_java_version("java version \"1.8.0_381\"\nJava(TM) SE Runtime Environment"), Some(8));
    assert_eq!(parse_java_version("openjdk version \"11.0.21\" 2023-10-17"), Some(11));
    assert_eq!(parse_java_version("openjdk version \"17.0.9\" 2023-10-17\nOpenJDK Runtime Environment"), Some(17));
    assert_eq!(parse_java_version("openjdk version \"21\" 2023-09-19"), Some(21));
    assert_eq!(parse_java_version("garbage"), None);
}

#[cfg(target_os = "macos")]
#[test]
fn setting_wins_and_json_shape() {
    let deps = FakeDeps::new(&[("/opt/sdk/platform-tools/adb", ""), ("/opt/sdk/emulator/emulator", ""), ("/env/sdk/platform-tools/adb", ""), ("/jdk17/bin/java", ""), (&format!("{JBR}/bin/java"), ""), ("/work/app/gradlew", "")]).java("/jdk17/bin/java", "17.0.9");
    let opts = AndroidEnvOpts { sdk_setting: Some("/opt/sdk".into()), ..mac(&[("ANDROID_HOME", "/env/sdk"), ("JAVA_HOME", "/jdk17")]) };
    let v = android_env_with(ROOT, &opts, &deps).to_value();
    assert_eq!(crate::js::stringify(&v), r#"{"kind":"android","sdk":"/opt/sdk","adb":"/opt/sdk/platform-tools/adb","emulator":"/opt/sdk/emulator/emulator","jdk":"/jdk17","jdkVersion":17,"gradlew":true,"issues":[]}"#);
    assert_eq!(deps.calls(), vec!["/jdk17/bin/java -version"]);
}

#[test]
fn passive_check_never_runs_project_java() {
    let files = [("/opt/sdk/platform-tools/adb", ""), ("/work/app/tools/jdk/bin/java", ""), ("/work/app/gradlew", "")];
    let deps = FakeDeps::new(&files).java("/work/app/tools/jdk/bin/java", "17.0.9");
    let e = android_env_with(ROOT, &mac(&[("ANDROID_HOME", "/opt/sdk"), ("JAVA_HOME", "/work/app/tools/jdk")]), &deps);
    assert_eq!((e.jdk.as_deref(), e.jdk_version), (Some("/work/app/tools/jdk"), None));
    assert_eq!(e.issues, vec![EnvIssue::with("jdk-local-unverified", "/work/app/tools/jdk")]);
    assert!(deps.calls().is_empty());
    let mut linked = FakeDeps::new(&files).java("/jdk/bin/java", "17.0.9");
    linked.add("/jdk/bin/java", "");
    linked.real.insert("/jdk/bin/java".into(), "/work/vendor/java".into());
    let opts = AndroidEnvOpts { repo_root: Some("/work".into()), ..mac(&[("ANDROID_HOME", "/opt/sdk"), ("JAVA_HOME", "/jdk")]) };
    assert_eq!(android_env_with(ROOT, &opts, &linked).jdk_version, None);
    assert!(linked.calls().is_empty());
    let trusted = AndroidEnvOpts { trusted: true, ..mac(&[("ANDROID_HOME", "/opt/sdk"), ("JAVA_HOME", "/work/app/tools/jdk")]) };
    assert_eq!(android_env_with(ROOT, &trusted, &deps).jdk_version, Some(17));
}

#[cfg(target_os = "macos")]
#[test]
fn falls_through_sdk_candidates() {
    let java = format!("{JBR}/bin/java");
    let base = [(java.as_str(), ""), ("/work/app/gradlew", "")];
    let mut with_root = base.to_vec();
    with_root.push(("/r/sdk/platform-tools/adb", ""));
    let e = android_env_with(ROOT, &mac(&[("ANDROID_HOME", "/nope"), ("ANDROID_SDK_ROOT", "/r/sdk")]), &FakeDeps::new(&with_root).java(&java, "21.0.3"));
    assert_eq!((e.sdk.as_deref(), e.jdk.as_deref(), e.jdk_version, e.emulator), (Some("/r/sdk"), Some(JBR), Some(21), None));
    let mut props = base.to_vec();
    props.extend([("/work/app/local.properties", "sdk.dir=/p/sdk"), ("/p/sdk/platform-tools/adb", "")]);
    assert_eq!(android_env_with(ROOT, &mac(&[]), &FakeDeps::new(&props)).sdk.as_deref(), Some("/p/sdk"));
    let mut def = base.to_vec();
    def.push(("/Users/me/Library/Android/sdk/platform-tools/adb", ""));
    assert_eq!(android_env_with(ROOT, &mac(&[]), &FakeDeps::new(&def)).sdk.as_deref(), Some("/Users/me/Library/Android/sdk"));
}

#[test]
fn windows_paths() {
    let java = "C:\\Program Files\\Android\\Android Studio\\jbr\\bin\\java.exe";
    let deps = FakeDeps::new(&[("C:\\proj\\local.properties", "sdk.dir=D\\:\\\\Android\\\\Sdk"), ("D:\\Android\\Sdk\\platform-tools\\adb.exe", ""), ("D:\\Android\\Sdk\\emulator\\emulator.exe", ""), (java, ""), ("C:\\proj\\gradlew.bat", "")]).java(java, "17.0.11");
    let opts = AndroidEnvOpts { win: true, home: "C:\\Users\\me".into(), ..Default::default() };
    let e = android_env_with("C:\\proj", &opts, &deps);
    assert_eq!(e.sdk.as_deref(), Some("D:\\Android\\Sdk"));
    assert_eq!(e.adb.as_deref(), Some("D:\\Android\\Sdk\\platform-tools\\adb.exe"));
    assert_eq!(e.emulator.as_deref(), Some("D:\\Android\\Sdk\\emulator\\emulator.exe"));
    assert_eq!((e.jdk.as_deref(), e.jdk_version, e.gradlew, e.issues.len()), (Some("C:\\Program Files\\Android\\Android Studio\\jbr"), Some(17), true, 0));
    let opts = AndroidEnvOpts { env: env(&[("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local")]), ..opts };
    let def = android_env_with("C:\\proj", &opts, &FakeDeps::new(&[("C:\\Users\\me\\AppData\\Local\\Android\\Sdk\\platform-tools\\adb.exe", "")]));
    assert_eq!(def.sdk.as_deref(), Some("C:\\Users\\me\\AppData\\Local\\Android\\Sdk"));
}

#[test]
fn reports_missing_sdk_old_jdk_missing_gradlew() {
    let deps = FakeDeps::new(&[("/usr/local/bin/adb", ""), ("/usr/lib/jvm/bin/java", "")]).java("/usr/lib/jvm/bin/java", "1.8.0_381");
    let e = android_env_with(ROOT, &mac(&[("PATH", "relative/bin:/usr/local/bin:/usr/lib/jvm/bin")]), &deps);
    assert_eq!((e.sdk.as_deref(), e.adb.as_deref(), e.emulator.as_deref(), e.jdk.as_deref(), e.jdk_version, e.gradlew), (None, Some("/usr/local/bin/adb"), None, Some("/usr/lib/jvm"), Some(8), false));
    assert_eq!(e.issues, vec![EnvIssue::new("sdk-missing"), EnvIssue::with("jdk-old", "8"), EnvIssue::new("gradlew-missing")]);
    let mut deps = FakeDeps::new(&[("/work/app/gradlew", "")]);
    deps.modes.insert("/work/app/gradlew".into(), 0o644);
    let e = android_env_with(ROOT, &AndroidEnvOpts { home: "/nonexistent-home".into(), ..Default::default() }, &deps);
    let codes: Vec<&str> = e.issues.iter().map(|i| i.code.as_str()).collect();
    assert_eq!(codes, vec!["sdk-missing", "jdk-missing", "gradlew-not-executable"]);
    assert_eq!(e.jdk, None);
}

#[test]
fn gradlew_spawn_posix_and_windows() {
    let args: Vec<String> = vec![":app:assembleDebug".into(), "--console=plain".into()];
    assert_eq!(gradlew_spawn("/work/my app", &args, false, &Env::new()), Some(Resolved { file: "/work/my app/gradlew".into(), args: args.clone(), verbatim: false }));
    let s = gradlew_spawn("C:\\My Apps\\demo", &[":app:assembleDebug".into(), "-Pname=a b&c".into()], true, &env(&[("SystemRoot", "C:\\Windows")])).unwrap();
    assert_eq!(s.file, "C:\\Windows\\System32\\cmd.exe");
    assert!(s.verbatim);
    assert_eq!(&s.args[..3], &["/d", "/s", "/c"]);
    assert_eq!(s.args[3], "\"C:\\My^ Apps\\demo\\gradlew.bat ^^^\":app:assembleDebug^^^\" ^^^\"-Pname=a^^^ b^^^&c^^^\"\"");
    assert_eq!(crate::preview::format::quote_cmd_arg("plain").as_deref(), Some("^^^\"plain^^^\""));
    assert_eq!(gradlew_spawn("C:\\p", &["a\nb".into()], true, &Env::new()), None);
}

#[test]
fn adb_tools_from_sdk() {
    let deps = FakeDeps::new(&[("/sdk/platform-tools/adb", ""), ("/sdk/emulator/emulator", "")]);
    assert_eq!(find_adb_tools_with(Some("/sdk"), &Env::new(), false, "/h", &deps), (Some("/sdk/platform-tools/adb".into()), Some("/sdk/emulator/emulator".into())));
    let only_adb = FakeDeps::new(&[("/e/platform-tools/adb", "")]);
    assert_eq!(find_adb_tools_with(None, &env(&[("ANDROID_HOME", "/e")]), false, "/h", &only_adb), (Some("/e/platform-tools/adb".into()), None));
}
