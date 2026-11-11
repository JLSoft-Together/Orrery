use super::*;
use crate::run::android::testutil::fixture;

const LAUNCHER_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
  <application>
    <activity android:name=".Settings" android:exported="false" />
    <activity
        android:name=".MainActivity"
        android:exported="true">
      <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
      </intent-filter>
    </activity>
  </application>
</manifest>"#;
const PLAIN_MANIFEST: &str = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application /></manifest>"#;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn j(parts: &[&str]) -> String {
    pathx::join(pathx::host_win(), parts)
}

#[test]
fn settings_parsing() {
    assert_eq!(parse_includes("include ':app', ':core:data'\ninclude ':lib1',\n  ':lib2'\n// include ':ghost'"), s(&[":app", ":core:data", ":lib1", ":lib2"]));
    assert_eq!(parse_includes("include(\n  \":app\",\n  \":feature:login\",\n)\ninclude(\":a\", \":b\")\nincludeBuild(\"build-logic\")"), s(&[":app", ":feature:login", ":a", ":b"]));
    assert_eq!(parse_includes("include 'app'"), s(&[":app"]));
    let dirs = parse_project_dirs("project(':lib').projectDir = new File(settingsDir, 'libs/lib')\nproject(\":x\").projectDir = file(\"modules/x\")");
    assert_eq!(dirs.get(":lib").map(String::as_str), Some("libs/lib"));
    assert_eq!(dirs.get(":x").map(String::as_str), Some("modules/x"));
}

#[test]
fn build_script_parsing() {
    let mut catalogs = Catalogs::new();
    catalogs.insert(
        "libs".into(),
        parse_catalog_plugins("[versions]\nagp = \"8.5.0\"\n[plugins]\nandroid-application = { id = \"com.android.application\", version.ref = \"agp\" }\nandroid_library = \"com.android.library:8.5.0\"\nkotlin-multiplatform = { id = \"org.jetbrains.kotlin.multiplatform\", version = \"2.0.0\" }"),
    );
    assert_eq!(plugin_ids("plugins {\n  id 'com.android.application'\n  id 'org.jetbrains.kotlin.android'\n}", &catalogs), s(&["com.android.application", "org.jetbrains.kotlin.android"]));
    assert_eq!(plugin_ids("apply plugin: 'com.android.library'", &catalogs), s(&["com.android.library"]));
    assert_eq!(plugin_ids("plugins { alias(libs.plugins.android.application) }", &catalogs), s(&["com.android.application"]));
    assert_eq!(plugin_ids("plugins { alias(libs.plugins.android.library) }", &catalogs), s(&["com.android.library"]));
    assert_eq!(plugin_ids("plugins {\n  id(\"com.android.application\") version \"8.5.0\" apply false\n  alias(libs.plugins.android.library) apply false\n}", &catalogs), Vec::<String>::new());
    assert_eq!(plugin_ids("plugins { kotlin(\"multiplatform\") }", &catalogs), s(&["org.jetbrains.kotlin.multiplatform"]));
}

#[test]
fn launcher_detection() {
    assert!(has_launcher(LAUNCHER_MANIFEST));
    assert!(!has_launcher(PLAIN_MANIFEST));
    assert!(has_launcher(r#"<application><activity-alias android:name=".A"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LEANBACK_LAUNCHER"/></intent-filter></activity-alias></application>"#));
    assert!(!has_launcher(r#"<application><!-- <activity android:name=".A"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity> --></application>"#));
    assert!(!has_launcher(r#"<activity android:name=".A"><intent-filter><action android:name="android.intent.action.VIEW"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity>"#));
}

#[test]
fn catalog_libraries() {
    let c = parse_catalog_libraries("[versions]\nsdp = \"1.1.0\"\nrich = { strictly = \"2.0\" }\n[libraries]\nsdp-android = { module = \"com.intuit.sdp:sdp-android\", version.ref = \"sdp\" }\nshort = \"a.b:c:1.2\"\nbom = { group = \"g\", name = \"n\" }\n[bundles]\nui = [\"sdp-android\", \"short\"]");
    assert_eq!(c.libraries.get("sdp.android"), Some(&CatalogLibrary { group: "com.intuit.sdp".into(), name: "sdp-android".into(), version: Some("1.1.0".into()) }));
    assert_eq!(c.libraries.get("short").and_then(|l| l.version.clone()), Some("1.2".into()));
    assert_eq!(c.libraries.get("bom").map(|l| l.version.clone()), Some(None));
    assert_eq!(c.bundles.get("ui"), Some(&s(&["sdp.android", "short"])));
}

fn first(root: &str) -> Value {
    detect_android(root).into_iter().next().expect("project")
}

#[test]
fn native_groovy_app() {
    let root = fixture("android", &[("settings.gradle", "rootProject.name = 'Hello'\ninclude ':app'"), ("build.gradle", "plugins { id 'com.android.application' version '8.5.0' apply false }"), ("app/build.gradle", "plugins { id 'com.android.application' }"), ("app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST)]);
    let p = first(&root);
    let keys: Vec<&String> = p.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["kind", "dir", "name", "framework", "status", "modules"]);
    assert_eq!((p["dir"].as_str(), p["name"].as_str(), p["framework"].as_str(), p["status"].as_str()), (Some(root.as_str()), Some("Hello"), Some("native"), Some("runnable")));
    assert_eq!(p["modules"], json!([{"path": ":app", "dir": j(&[&root, "app"]), "type": "application", "manifest": j(&[&root, "app", "src", "main", "AndroidManifest.xml"]), "launcher": true, "guessed": false}]));
}

#[test]
fn kts_catalog_multi_module_and_kmp() {
    let root = fixture("android", &[("settings.gradle.kts", "include(\":app\")"), ("gradle/libs.versions.toml", "[plugins]\nandroid-application = { id = \"com.android.application\", version = \"8.5.0\" }"), ("app/build.gradle.kts", "plugins {\n    alias(libs.plugins.android.application)\n}"), ("app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST)]);
    let p = first(&root);
    assert_eq!(p["name"].as_str().unwrap(), pathx::basename(&root, pathx::host_win()));
    assert_eq!(p["modules"][0]["type"], "application");

    let root = fixture("android", &[("settings.gradle", "include ':app', ':core:ui', ':lib'\nproject(':lib').projectDir = file('libs/mylib')"), ("app/build.gradle", "apply plugin: 'com.android.application'"), ("app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST), ("core/ui/build.gradle", "plugins { id 'com.android.library' }"), ("core/ui/src/main/AndroidManifest.xml", PLAIN_MANIFEST), ("libs/mylib/build.gradle", "plugins { id 'java-library' }")]);
    let p = first(&root);
    assert_eq!(p["status"], "runnable");
    let mods: Vec<(String, String, String)> = p["modules"].as_array().unwrap().iter().map(|m| (m["path"].as_str().unwrap().into(), m["type"].as_str().unwrap().into(), m["dir"].as_str().unwrap().into())).collect();
    assert_eq!(mods, vec![(":app".into(), "application".into(), j(&[&root, "app"])), (":core:ui".into(), "library".into(), j(&[&root, "core", "ui"])), (":lib".into(), "unknown".into(), j(&[&root, "libs", "mylib"]))]);

    let root = fixture("android", &[("settings.gradle.kts", "rootProject.name = \"Kmp\"\ninclude(\":composeApp\")"), ("composeApp/build.gradle.kts", "plugins {\n  kotlin(\"multiplatform\")\n  id(\"com.android.application\")\n}"), ("composeApp/src/androidMain/AndroidManifest.xml", LAUNCHER_MANIFEST)]);
    let p = first(&root);
    assert_eq!((p["name"].as_str(), p["framework"].as_str(), p["status"].as_str()), (Some("Kmp"), Some("kmp"), Some("runnable")));
    assert_eq!(p["modules"][0]["manifest"].as_str().unwrap(), j(&[&root, "composeApp", "src", "androidMain", "AndroidManifest.xml"]));
}

#[test]
fn frameworks() {
    let rn = fixture("android", &[("package.json", r#"{"name":"rn","dependencies":{"react":"18","react-native":"0.76.0"}}"#), ("android/settings.gradle", "rootProject.name = 'RnApp'\ninclude ':app'"), ("android/app/build.gradle", "apply plugin: \"com.android.application\""), ("android/app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST)]);
    let res = detect_android(&rn);
    assert_eq!(res.len(), 1);
    assert_eq!((res[0]["dir"].as_str().unwrap(), res[0]["name"].as_str(), res[0]["framework"].as_str()), (j(&[&rn, "android"]).as_str(), Some("RnApp"), Some("react-native")));

    let cap = fixture("android", &[("capacitor.config.ts", "export default {}"), ("android/settings.gradle", "include ':app'"), ("android/app/build.gradle", "apply plugin: 'com.android.application'")]);
    let p = first(&cap);
    assert_eq!((p["framework"].as_str().unwrap(), p["name"].as_str().unwrap()), ("capacitor", pathx::basename(&cap, pathx::host_win()).as_str()));

    let fl = fixture("android", &[("pubspec.yaml", "name: demo\ndependencies:\n  flutter:\n    sdk: flutter\n"), ("android/settings.gradle.kts", "include(\":app\")"), ("android/app/build.gradle.kts", "plugins { id(\"com.android.application\") }"), ("android/app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST)]);
    assert!(detect_android(&fl).is_empty());
    assert_eq!(android_framework(&j(&[&fl, "android"]), false), "flutter");
}

#[test]
fn statuses_and_non_android() {
    let lib = fixture("android", &[("settings.gradle.kts", "include(\":lib\")"), ("lib/build.gradle.kts", "plugins { id(\"com.android.library\") }"), ("lib/src/main/AndroidManifest.xml", PLAIN_MANIFEST)]);
    assert_eq!(first(&lib)["status"], "no-app");
    let java = fixture("android", &[("settings.gradle.kts", "include(\":core\")"), ("build.gradle.kts", "plugins { kotlin(\"jvm\") version \"2.0.0\" }"), ("core/build.gradle.kts", "plugins { id(\"java-library\") }")]);
    assert!(detect_android(&java).is_empty());
    let ts = fixture("android", &[("package.json", "{\"name\":\"x\"}"), ("src/index.ts", "export {}"), ("tsconfig.json", "{}")]);
    assert!(detect_android(&ts).is_empty());
    let files = [("settings.gradle.kts", "pluginManagement { includeBuild(\"build-logic\") }\ninclude(\":app\")"), ("build-logic/settings.gradle.kts", "rootProject.name = \"build-logic\"\ninclude(\":convention\")"), ("build-logic/convention/build.gradle.kts", "plugins { `kotlin-dsl` }"), ("app/build.gradle.kts", "plugins { id(\"nowinandroid.android.application\") }")];
    let mut guessed = files.to_vec();
    guessed.push(("app/src/main/AndroidManifest.xml", LAUNCHER_MANIFEST));
    let res = detect_android(&fixture("android", &guessed));
    assert_eq!(res.len(), 1);
    assert_eq!(res[0]["status"], "runnable");
    assert_eq!((res[0]["modules"][0]["type"].as_str(), res[0]["modules"][0]["launcher"].as_bool(), res[0]["modules"][0]["guessed"].as_bool()), (Some("unknown"), Some(true), Some(true)));
    let mut unknown = files.to_vec();
    unknown.push(("app/src/main/AndroidManifest.xml", PLAIN_MANIFEST));
    let u = first(&fixture("android", &unknown));
    assert_eq!(u["status"], "unknown");
    assert_eq!((u["modules"][0]["launcher"].as_bool(), u["modules"][0]["guessed"].as_bool()), (Some(false), Some(false)));
}

#[test]
fn scans_two_levels() {
    let mut files: Vec<(String, String)> = Vec::new();
    for p in ["apps/native", "apps/rn/android", "node_modules/pkg", "build/out", ".hidden", "a/b/c"] {
        files.push((format!("{p}/settings.gradle"), "include ':app'".into()));
        files.push((format!("{p}/app/build.gradle"), "apply plugin: 'com.android.application'".into()));
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let root = fixture("android", &refs);
    let dirs: Vec<String> = detect_android(&root).iter().map(|p| p["dir"].as_str().unwrap()[root.len() + 1..].replace('\\', "/")).collect();
    assert_eq!(dirs, s(&["apps/native", "apps/rn/android"]));
}
