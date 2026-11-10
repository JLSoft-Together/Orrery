use super::*;
use crate::run::android::env::{android_env_with, AndroidEnvOpts};
use crate::run::android::fake_deps::FakeDeps;
use std::sync::atomic::{AtomicU64, Ordering};

const ROOT: &str = "/work/app";
const HOME: &str = "/Users/me";
const JBR: &str = "/Applications/Android Studio.app/Contents/jbr/Contents/Home";

fn jdk(home: &str, full: &str) -> Vec<(String, String)> {
    vec![(format!("{home}/bin/java"), String::new()), (format!("{home}/release"), format!("IMPLEMENTOR=\"x\"\nJAVA_VERSION=\"{full}\"\n"))]
}

fn wrapper(v: &str) -> Vec<(String, String)> {
    vec![(format!("{ROOT}/gradle/wrapper/gradle-wrapper.properties"), format!("distributionUrl=https\\://services.gradle.org/distributions/gradle-{v}-bin.zip\n"))]
}

fn jdk_of(v: u32) -> Vec<(String, String)> {
    match v {
        25 => jdk(&format!("{HOME}/.gradle/jdks/eclipse_adoptium-25-amd64"), "25.0.1"),
        21 => jdk(JBR, "21.0.10"),
        17 => jdk(&format!("{HOME}/.jdks/corretto-17.0.15"), "17.0.15"),
        _ => jdk(&format!("{HOME}/.jdks/jdk-11"), "11.0.21"),
    }
}

fn with(parts: Vec<Vec<(String, String)>>) -> Vec<(String, String)> {
    parts.into_iter().flatten().collect()
}

fn fake(files: &[(String, String)]) -> FakeDeps {
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    FakeDeps::new(&refs).listing('/')
}

fn opts(setting: Option<&str>, env: &[(&str, &str)]) -> JdkPickOpts {
    JdkPickOpts { setting: setting.map(String::from), env: env.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), win: false, home: HOME.into() }
}

fn choose(files: &[(String, String)], setting: Option<&str>, env: &[(&str, &str)]) -> JdkChoice {
    choose_jdk(ROOT, &opts(setting, env), &fake(files))
}

fn gv(v: &str) -> GradleVersion {
    parse_gradle_version(&format!("distributionUrl=gradle-{v}-bin.zip")).unwrap()
}

fn info(version: u32, full: &str) -> JdkInfo {
    JdkInfo { home: format!("/jdk{full}"), version, full: full.into(), label: format!("jdk-{full}") }
}

#[test]
fn parses_wrapper_and_release() {
    assert_eq!(parse_gradle_version("distributionUrl=https\\://services.gradle.org/distributions/gradle-7.3.3-bin.zip"), Some(GradleVersion { text: "7.3.3".into(), parts: [7, 3, 3] }));
    assert_eq!(parse_gradle_version("distributionUrl=https\\://x/gradle-8.0-rc-1-all.zip").map(|g| g.parts), Some([8, 0, 0]));
    assert_eq!(parse_gradle_version("distributionUrl=https\\://x/gradle-7.5-bin.zip").map(|g| g.text), Some("7.5".into()));
    assert_eq!(parse_gradle_version("foo=bar"), None);
    assert_eq!(parse_release("JAVA_VERSION=\"1.8.0_392\""), Some((8, "1.8.0_392".into())));
    assert_eq!(parse_release("JAVA_VERSION=\"21\"").map(|r| r.0), Some(21));
    assert_eq!(parse_gradle_jvm("<option name=\"gradleJvm\" value=\"corretto-17\" />").as_deref(), Some("corretto-17"));
}

#[test]
fn max_java_matrix_and_pick() {
    for (v, max) in [("6.5", 14), ("6.7.1", 15), ("7.3.3", 17), ("7.5", 18), ("7.6.4", 19), ("8.2", 19), ("8.7", 21), ("8.14.3", 24), ("9.1.0", 25), ("4.10", 8)] {
        assert_eq!(max_java_for(&gv(v)), max, "{v}");
    }
    assert_eq!(max_java_for(&gv("9.4")), ANY_JAVA);
    assert_eq!(pick_for(&gv("9.4"), &[info(17, "17.0.1"), info(25, "25.0.1"), info(21, "21.0.1")]).map(|j| j.version), Some(25));
    let cases: [(&str, &[u32], u32); 7] = [("7.3.3", &[21, 17, 11], 17), ("7.3.3", &[21, 11], 11), ("7.5", &[21, 18, 17], 18), ("8.7", &[21, 17], 21), ("8.7", &[25, 17], 17), ("6.5", &[21, 11, 8], 11), ("6.5", &[21, 8], 8)];
    for (v, versions, want) in cases {
        let list: Vec<JdkInfo> = versions.iter().map(|x| info(*x, &format!("{x}.0.1"))).collect();
        assert_eq!(pick_for(&gv(v), &list).map(|j| j.version), Some(want), "{v}");
    }
    assert_eq!(pick_for(&gv("7.3.3"), &[info(17, "17.0.7"), info(17, "17.0.15"), info(17, "17.0.14")]).map(|j| j.full), Some("17.0.15".into()));
}

#[cfg(target_os = "macos")]
#[test]
fn choose_discovered_and_bundles() {
    let c = choose(&with(vec![wrapper("7.3.3"), jdk_of(21), jdk_of(17), jdk_of(11)]), None, &[]);
    let p = c.pick.unwrap();
    assert_eq!((p.home.as_str(), p.version, p.label.as_str(), p.source), (format!("{HOME}/.jdks/corretto-17.0.15").as_str(), Some(17), "corretto-17.0.15", JdkSource::Discovered));
    assert_eq!(c.min_jdk, 11);
    let zulu = "/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home";
    let files = with(vec![wrapper("7.5"), jdk(&format!("{HOME}/.gradle/jdks/adoptium-17/jdk-17.0.9+9"), "17.0.9"), jdk(zulu, "18.0.2")]);
    let p = choose(&files, None, &[("JAVA_HOME", zulu)]).pick.unwrap();
    assert_eq!((p.version, p.label.as_str()), (Some(18), "zulu-18"));
}

#[test]
fn explicit_sources_win() {
    let files = with(vec![wrapper("7.3.3"), jdk_of(21), jdk_of(17), jdk("/opt/jdk11", "11.0.2"), vec![(format!("{ROOT}/gradle.properties"), "org.gradle.java.home=/opt/jdk11\n".into())]]);
    let p = choose(&files, Some(&format!("{HOME}/.jdks/corretto-17.0.15")), &[]).pick.unwrap();
    assert_eq!((p.home.as_str(), p.source), ("/opt/jdk11", JdkSource::GradleProperties));
    let files = with(vec![wrapper("7.3.3"), jdk_of(21), jdk_of(17), jdk_of(11)]);
    let p = choose(&files, Some(&format!("{HOME}/.jdks/jdk-11")), &[]).pick.unwrap();
    assert_eq!((p.home, p.source, p.version), (format!("{HOME}/.jdks/jdk-11"), JdkSource::Setting, Some(11)));
}

#[cfg(target_os = "macos")]
#[test]
fn idea_gradle_jvm() {
    let base = with(vec![wrapper("8.7"), jdk_of(21), jdk_of(17)]);
    let idea = |v: &str| vec![(format!("{ROOT}/.idea/gradle.xml"), format!("<GradleProjectSettings>\n<option name=\"gradleJvm\" value=\"{v}\" />"))];
    let pick = |files: Vec<(String, String)>| {
        let p = choose(&files, None, &[]).pick.unwrap();
        (p.version, p.source)
    };
    assert_eq!(pick(with(vec![base.clone(), idea("corretto-17")])), (Some(17), JdkSource::Idea));
    assert_eq!(pick(with(vec![base.clone(), idea(&format!("{HOME}/.jdks/corretto-17.0.15"))])), (Some(17), JdkSource::Idea));
    let local = with(vec![base.clone(), idea("#GRADLE_LOCAL_JAVA_HOME"), vec![(format!("{ROOT}/.gradle/config.properties"), format!("java.home={HOME}/.jdks/corretto-17.0.15"))]]);
    assert_eq!(pick(local), (Some(17), JdkSource::Idea));
    assert_eq!(pick(with(vec![base.clone(), idea("#JAVA_INTERNAL")])), (Some(21), JdkSource::Discovered));
    assert_eq!(pick(with(vec![wrapper("7.3.3"), jdk_of(21), jdk_of(17), idea("jbr-21")])), (Some(17), JdkSource::Discovered));
}

#[cfg(target_os = "macos")]
#[test]
fn incompatible_and_no_decision() {
    let c = choose(&with(vec![wrapper("7.3.3"), jdk_of(25), jdk_of(21)]), None, &[]);
    assert_eq!(c.pick, None);
    assert_eq!(c.issue, Some(EnvIssue::with("jdk-incompatible", "Gradle 7.3.3 runs on JDK ≤ 17; found 21 (jbr), 25 (eclipse_adoptium-25-amd64)")));
    let c = choose(&jdk_of(17), None, &[]);
    assert_eq!((c.gradle, c.pick, c.min_jdk), (None, None, 17));
    assert_eq!(choose(&wrapper("7.3.3"), None, &[]).issue, None);
}

#[test]
fn discovery_cached_until_cleared() {
    static ID: AtomicU64 = AtomicU64::new(1000);
    let mut deps = fake(&with(vec![wrapper("7.3.3"), jdk_of(17)]));
    deps.key = Some(ID.fetch_add(1, Ordering::SeqCst));
    let o = opts(None, &[]);
    assert_eq!(choose_jdk(ROOT, &o, &deps).pick.and_then(|p| p.version), Some(17));
    for (f, t) in jdk(&format!("{HOME}/.jdks/corretto-17.0.99"), "17.0.99") {
        deps.add(&f, &t);
    }
    assert_eq!(choose_jdk(ROOT, &o, &deps).pick.map(|p| p.label), Some("corretto-17.0.15".into()));
    clear_jdk_cache();
    assert_eq!(choose_jdk(ROOT, &o, &deps).pick.map(|p| p.label), Some("corretto-17.0.99".into()));
}

#[test]
fn windows_program_files_vendors() {
    let files = [
        ("C:\\p\\gradle\\wrapper\\gradle-wrapper.properties", "distributionUrl=https\\://services.gradle.org/distributions/gradle-7.5-bin.zip"),
        ("C:\\Program Files\\Android\\Android Studio\\jbr\\bin\\java.exe", ""),
        ("C:\\Program Files\\Android\\Android Studio\\jbr\\release", "JAVA_VERSION=\"21.0.10\""),
        ("C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.9.9-hotspot\\bin\\java.exe", ""),
        ("C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.9.9-hotspot\\release", "JAVA_VERSION=\"17.0.9\""),
    ];
    let deps = FakeDeps::new(&files).listing('\\');
    let o = JdkPickOpts { setting: None, env: [("ProgramFiles".to_string(), "C:\\Program Files".to_string())].into_iter().collect(), win: true, home: "C:\\Users\\me".into() };
    let p = choose_jdk("C:\\p", &o, &deps).pick.unwrap();
    assert_eq!((p.home.as_str(), p.version), ("C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.9.9-hotspot", Some(17)));
}

#[cfg(target_os = "macos")]
#[test]
fn android_env_uses_pick() {
    let base = with(vec![vec![("/opt/sdk/platform-tools/adb".into(), String::new()), (format!("{ROOT}/gradlew"), String::new())], wrapper("7.3.3")]);
    let o = AndroidEnvOpts { env: [("ANDROID_HOME".to_string(), "/opt/sdk".to_string())].into_iter().collect(), home: HOME.into(), ..Default::default() };
    let deps = fake(&with(vec![base.clone(), jdk_of(21), jdk_of(11)])).java(&format!("{HOME}/.jdks/jdk-11/bin/java"), "11.0.21");
    let e = android_env_with(ROOT, &o, &deps);
    assert_eq!((e.jdk, e.jdk_version, e.issues.len(), e.jdk_note.as_deref()), (Some(format!("{HOME}/.jdks/jdk-11")), Some(11), 0, Some("JDK 11 (jdk-11) for Gradle 7.3.3")));
    let deps = fake(&with(vec![base, jdk_of(21)])).java(&format!("{JBR}/bin/java"), "21.0.10");
    let e = android_env_with(ROOT, &o, &deps);
    assert_eq!((e.jdk.as_deref(), e.jdk_version, e.jdk_note.as_deref()), (Some(JBR), Some(21), Some("JDK 21 (jbr) for Gradle 7.3.3")));
    assert_eq!(e.issues, vec![EnvIssue::with("jdk-incompatible", "Gradle 7.3.3 runs on JDK ≤ 17; found 21 (jbr)")]);
}
