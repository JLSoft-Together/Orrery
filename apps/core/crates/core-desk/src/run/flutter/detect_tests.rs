use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

fn fixture(files: &[(String, String)]) -> String {
    let base = std::fs::canonicalize(std::env::temp_dir()).unwrap().join(format!("ccm-flutter-rs-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&base);
    for (rel, body) in files {
        let file = base.join(rel);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        if !rel.ends_with('/') {
            std::fs::write(&file, body).unwrap();
        }
    }
    std::fs::create_dir_all(&base).unwrap();
    base.to_string_lossy().to_string()
}

fn pubspec(name: &str, extra: &str) -> String {
    format!("name: {name}\ndescription: \"A thing # not a comment\"\n\nenvironment:\n  sdk: ^3.5.0\n\ndependencies:\n  flutter:\n    sdk: flutter\n  http: ^1.2.0\n\ndev_dependencies:\n  flutter_test:\n    sdk: flutter\n{extra}")
}

fn app(dir: &str, name: &str) -> Vec<(String, String)> {
    vec![
        (format!("{dir}pubspec.yaml"), pubspec(name, "")),
        (format!("{dir}.metadata"), "version:\n  revision: abc\n  channel: stable\n\nproject_type: app\n".into()),
        (format!("{dir}lib/main.dart"), "void main() {}".into()),
        (format!("{dir}android/app/build.gradle"), "android {}".into()),
        (format!("{dir}ios/Runner/Info.plist"), String::new()),
        (format!("{dir}pubspec.lock"), "# lock".into()),
        (format!("{dir}.dart_tool/package_config.json"), "{}".into()),
    ]
}

fn with(mut base: Vec<(String, String)>, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    for (k, v) in extra {
        base.retain(|(r, _)| r != k);
        base.push((k.to_string(), v.to_string()));
    }
    base
}

fn without(mut base: Vec<(String, String)>, rel: &str) -> Vec<(String, String)> {
    base.retain(|(r, _)| r != rel);
    base
}

fn names(list: &[Value]) -> Vec<String> {
    list.iter().map(|p| p["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn pubspec_parsing() {
    assert_eq!(parse_pubspec(&pubspec("demo", "")), PubspecInfo { name: "demo".into(), flutter: true, plugin: false });
    assert_eq!(parse_pubspec("name: \"dartish\"\ndependencies:\n  path: any\ndev_dependencies:\n  flutter:\n    sdk: flutter\n"), PubspecInfo { name: "dartish".into(), flutter: false, plugin: false });
    assert!(parse_pubspec("name: p\ndependencies:\n  flutter: {sdk: flutter}\nflutter:\n  plugin:\n    platforms:\n      android:\n        package: x\n").plugin);
    assert!(parse_pubspec("name: p\ndependencies:\n  flutter: { sdk: flutter }\n").flutter);
    let keys = yaml_keys("name: a\ndescription: |\n  dependencies:\n  flutter:\nversion: 1.0.0\n");
    assert_eq!(keys.get("version").map(String::as_str), Some("1.0.0"));
    assert!(!keys.contains_key("dependencies"));
}

#[test]
fn gradle_flavor_blocks() {
    let groovy = "android {\n  flavorDimensions \"env\"\n  productFlavors {\n    // staging { }\n    dev {\n      dimension \"env\"\n      applicationIdSuffix \".dev\"\n      resValue \"string\", \"app_name\", \"{dev}\"\n    }\n    prod { dimension \"env\" }\n  }\n  buildTypes { release { } }\n}";
    assert_eq!(gradle_flavors(groovy), vec!["dev", "prod"]);
    let kts = "android {\n  productFlavors {\n    create(\"free\") {\n      dimension = \"tier\"\n    }\n    register(\"paid\") { dimension = \"tier\" }\n    create(\"bare\")\n  }\n}";
    assert_eq!(gradle_flavors(kts), vec!["free", "paid", "bare"]);
    assert!(gradle_flavors("android { defaultConfig { } }").is_empty());
}

#[test]
fn standard_app_and_fallbacks() {
    let root = fixture(&app("", "demo"));
    let list = detect_flutter(&root);
    assert_eq!(list[0], json!({"kind": "flutter", "dir": root, "name": "demo", "entrypoints": ["lib/main.dart"], "needsPubGet": false, "fvm": null, "type": "app", "platforms": ["android", "ios"], "flavors": []}));
    assert_eq!(crate::js::stringify(&list[0]).find("\"kind\""), Some(1));
    assert_eq!(detect_flutter(&fixture(&without(app("", "demo"), ".metadata")))[0]["type"], "app");
    let pkg = fixture(&[("pubspec.yaml".into(), pubspec("lib_only", "")), ("lib/src/a.dart".into(), String::new())]);
    assert_eq!(detect_flutter(&pkg)[0]["type"], "package");
}

#[test]
fn entrypoints_and_flavors() {
    let root = fixture(&with(app("", "demo"), &[("lib/main_prod.dart", ""), ("lib/main_dev.dart", ""), ("lib/app.dart", "")]));
    assert_eq!(detect_flutter(&root)[0]["entrypoints"], json!(["lib/main.dart", "lib/main_dev.dart", "lib/main_prod.dart"]));
    let groovy = fixture(&with(
        app("", "demo"),
        &[
            ("android/app/build.gradle", "android { productFlavors { dev { } prod { } } }"),
            ("ios/Runner.xcodeproj/xcshareddata/xcschemes/Runner.xcscheme", ""),
            ("ios/Runner.xcodeproj/xcshareddata/xcschemes/dev.xcscheme", ""),
            ("ios/Runner.xcodeproj/xcshareddata/xcschemes/staging.xcscheme", ""),
        ],
    ));
    assert_eq!(detect_flutter(&groovy)[0]["flavors"], json!(["dev", "prod", "staging"]));
    let kts = fixture(&with(without(app("", "demo"), "android/app/build.gradle"), &[("android/app/build.gradle.kts", "android { productFlavors { create(\"qa\") { } } }")]));
    assert_eq!(detect_flutter(&kts)[0]["flavors"], json!(["qa"]));
}

#[test]
fn monorepo_layouts() {
    let root = fixture(&with(app("mobile/", "demo"), &[("package.json", "{}"), ("server/index.ts", "")]));
    let list = detect_flutter(&root);
    assert_eq!(list.iter().map(|p| p["dir"].as_str().unwrap().to_string()).collect::<Vec<_>>(), vec![PathBuf::from(&root).join("mobile").to_string_lossy().to_string()]);
    let mut two = app("apps/admin/", "admin");
    two.extend(app("apps/client/", "client"));
    two.push(("node_modules/x/pubspec.yaml".into(), pubspec("nope", "")));
    let mut got = names(&detect_flutter(&fixture(&two)));
    got.sort();
    assert_eq!(got, vec!["admin", "client"]);
    let skipped = fixture(&with(
        app("", "demo"),
        &[("build/x/pubspec.yaml", &pubspec("b", "")), (".hidden/pubspec.yaml", &pubspec("h", "")), ("a/b/c/pubspec.yaml", &pubspec("deep", "")), ("ios/Pods/pubspec.yaml", &pubspec("pods", ""))],
    ));
    assert_eq!(names(&detect_flutter(&skipped)), vec!["demo"]);
}

#[test]
fn package_plugin_dart() {
    let pkg = fixture(&[("pubspec.yaml".into(), pubspec("pkg", "")), (".metadata".into(), "project_type: package\n".into()), ("lib/pkg.dart".into(), String::new())]);
    let p = &detect_flutter(&pkg)[0];
    assert_eq!((p["type"].as_str(), p["needsPubGet"].as_bool()), (Some("package"), Some(true)));
    let mut plug = vec![
        ("pubspec.yaml".to_string(), pubspec("plug", "flutter:\n  plugin:\n    platforms:\n      android:\n        pluginClass: Plug\n")),
        (".metadata".into(), "project_type: plugin\n".into()),
        ("android/build.gradle".into(), String::new()),
        ("lib/plug.dart".into(), String::new()),
    ];
    plug.extend(app("example/", "plug_example"));
    let root = fixture(&plug);
    let list = detect_flutter(&root);
    assert_eq!(list.iter().map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap())).collect::<Vec<_>>(), vec![("plug", "plugin"), ("plug_example", "app")]);
    assert_eq!(list[0]["example"], json!(PathBuf::from(&root).join("example").to_string_lossy()));
    let mut deep = vec![("packages/plug/pubspec.yaml".to_string(), pubspec("plug", "flutter:\n  plugin:\n    platforms: {}\n"))];
    deep.extend(app("packages/plug/example/", "plug_example"));
    assert_eq!(names(&detect_flutter(&fixture(&deep))), vec!["plug", "plug_example"]);
    let dart = fixture(&[("pubspec.yaml".into(), "name: cli\ndependencies:\n  args: ^2.0.0\n".into()), ("bin/cli.dart".into(), String::new())]);
    let d = &detect_flutter(&dart)[0];
    assert_eq!((d["type"].as_str(), d["platforms"].clone()), (Some("dart"), json!([])));
}

#[test]
fn fvm_pins_and_pub_get() {
    assert_eq!(detect_flutter(&fixture(&with(app("", "demo"), &[(".fvmrc", "{\"flutter\": \"3.24.3\"}")])))[0]["fvm"], "3.24.3");
    assert_eq!(detect_flutter(&fixture(&with(app("", "demo"), &[(".fvm/fvm_config.json", "{\"flutterSdkVersion\": \"3.10.0\"}")])))[0]["fvm"], "3.10.0");
    assert_eq!(detect_flutter(&fixture(&without(app("", "demo"), ".dart_tool/package_config.json")))[0]["needsPubGet"], true);
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    let root = fixture(&app("", "demo"));
    std::fs::File::options().write(true).open(PathBuf::from(&root).join("pubspec.lock")).unwrap().set_modified(old).unwrap();
    assert_eq!(detect_flutter(&root)[0]["needsPubGet"], true);
    let fresh = fixture(&app("", "demo"));
    std::fs::File::options().write(true).open(PathBuf::from(&fresh).join("pubspec.yaml")).unwrap().set_modified(old).unwrap();
    assert_eq!(detect_flutter(&fresh)[0]["needsPubGet"], false);
}

#[test]
fn non_flutter_folders() {
    let ts = fixture(&[("package.json".into(), "{}".into()), ("src/index.ts".into(), String::new()), ("apps/web/src/main.ts".into(), String::new())]);
    assert!(detect_flutter(&ts).is_empty());
    assert!(detect_flutter(&format!("{ts}/missing")).is_empty());
    assert_eq!(metadata_type("project_type: plugin_ffi"), Some("plugin"));
    assert_eq!(metadata_type("x: 1"), None);
    assert_eq!(parse_fvm_config("{\"flutter\":null,\"flutterSdkVersion\":\" 3.1 \"}"), Some("3.1".into()));
    assert_eq!(parse_fvm_config("nope"), None);
}
