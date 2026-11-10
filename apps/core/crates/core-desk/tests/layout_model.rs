#![cfg(unix)]

use core_desk::layout::model::{ModelCfg, ModelDaemon, ModelSvc, Timing};
use core_desk::layout::LayoutError;
use core_desk::run::settings::SettingsStore;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("core-desk-model-{name}-{}", core_desk::util::uuid()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(&d).unwrap()
}

fn write(file: &Path, text: &str) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

fn exec(file: &Path, text: &str) {
    write(file, text);
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[derive(Default)]
struct Fake {
    calls: Mutex<Vec<(String, Value)>>,
    fail: AtomicBool,
}

impl Fake {
    fn named(&self, method: &str) -> Vec<Value> {
        self.calls.lock().unwrap().iter().filter(|(m, _)| m == method).map(|(_, p)| p.clone()).collect()
    }
}

impl ModelDaemon for Fake {
    fn request(&self, method: &str, params: Value, _timeout: Duration) -> Result<Value, LayoutError> {
        self.calls.lock().unwrap().push((method.to_string(), params));
        if self.fail.load(Ordering::SeqCst) {
            return Err(LayoutError::with("request-failed", "boom"));
        }
        Ok(json!({"ok": true, "loadMs": 3, "resCount": 1}))
    }

    fn request_restart(&self) {}
}

struct Rig {
    dir: PathBuf,
    svc: Arc<ModelSvc>,
    daemon: Arc<Fake>,
    events: Arc<Mutex<Vec<Value>>>,
    logs: Arc<Mutex<Vec<String>>>,
}

fn wait(what: &str, f: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(8);
    while !f() {
        assert!(Instant::now() < end, "timeout waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn rig_with(dir: &Path, with_kit: bool, with_sdk: bool, timing: Timing) -> Rig {
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let jdk = dir.join("jdk");
    exec(&jdk.join("bin/java"), "#!/bin/sh\necho 'openjdk version \"21.0.1\"' >&2\n");
    let sdk = dir.join("sdk");
    write(&sdk.join("platform-tools/adb"), "x");
    let mut settings = json!({"jdk": jdk.to_string_lossy()});
    if with_sdk {
        settings["androidSdk"] = json!(sdk.to_string_lossy());
    }
    write(&data.join("run-settings.json"), &settings.to_string());
    let kit = dir.join("kit");
    if with_kit {
        write(&kit.join("a.jar"), "x");
        write(&kit.join("R.jar"), "x");
        write(&kit.join("kit.json"), &json!({"classpath": ["a.jar"], "rJar": "R.jar", "packageName": "k.p", "appTheme": "@style/T", "aarRes": []}).to_string());
    }
    let daemon = Arc::new(Fake::default());
    let events: Arc<Mutex<Vec<Value>>> = Arc::default();
    let logs: Arc<Mutex<Vec<String>>> = Arc::default();
    let (ev, lg) = (events.clone(), logs.clone());
    let svc = ModelSvc::new(ModelCfg {
        models_dir: data.join("models").to_string_lossy().to_string(),
        init_script: dir.join("init.gradle").to_string_lossy().to_string(),
        win: false,
        env: [("PATH".to_string(), std::env::var("PATH").unwrap_or_default())].into_iter().collect(),
        home: dir.to_string_lossy().to_string(),
        settings: SettingsStore::load(&data, false),
        loose_kit_dir: with_kit.then(|| kit.to_string_lossy().to_string()),
        gradle_home: dir.join("gradle-home").to_string_lossy().to_string(),
        aar_cache_dir: data.join("aars").to_string_lossy().to_string(),
        daemon: daemon.clone(),
        emit: Arc::new(move |v| ev.lock().unwrap().push(v)),
        log: Arc::new(move |l, m| lg.lock().unwrap().push(format!("{l}: {m}"))),
        can_run: Arc::new(|| true),
        timing,
    });
    Rig { dir: dir.to_path_buf(), svc, daemon, events, logs }
}

fn timing() -> Timing {
    Timing { resolve_ttl: Duration::ZERO, debounce: Duration::from_millis(60), gen_debounce: Duration::from_millis(100), ..Timing::default() }
}

fn rig(name: &str) -> Rig {
    rig_with(&tmp(name), false, true, timing())
}

struct Proj {
    root: PathBuf,
    app_layout: String,
    lib_layout: String,
}

fn project(dir: &Path) -> Proj {
    let root = dir.join("proj");
    write(&root.join("settings.gradle.kts"), "rootProject.name = \"demo\"\ninclude(\":app\", \":lib\")\n");
    write(&root.join("app/build.gradle.kts"), "plugins { id(\"com.android.application\") }\nandroid { namespace = \"dev.t\"\ndefaultConfig { minSdk = 24\ntargetSdk = 34 } }\ndependencies {\n implementation(project(\":lib\"))\n implementation(\"g:a:1.0\")\n}\n");
    write(&root.join("app/src/main/AndroidManifest.xml"), "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><application android:theme=\"@style/AppTheme\"><activity android:name=\".Main\" android:theme=\"@style/MainTheme\"/></application></manifest>");
    write(&root.join("app/src/main/res/layout/main.xml"), "<LinearLayout/>");
    write(&root.join("lib/build.gradle.kts"), "plugins { id(\"com.android.library\") }\n");
    write(&root.join("lib/src/main/res/layout/lib.xml"), "<LinearLayout/>");
    write(&root.join("gradlew"), "#!/bin/sh\nexit 0\n");
    exec(&root.join("gradlew"), "#!/bin/sh\nexit 0\n");
    Proj { app_layout: root.join("app/src/main/res/layout/main.xml").to_string_lossy().to_string(), lib_layout: root.join("lib/src/main/res/layout/lib.xml").to_string_lossy().to_string(), root }
}

fn gradle_cache(dir: &Path) {
    let home = dir.join("gradle-home/caches/modules-2/files-2.1/g/a/1.0/abc");
    write(&home.join("a-1.0.jar"), "jar");
    write(&home.join("a-1.0.pom"), "<project><dependencies><dependency><groupId>g</groupId><artifactId>b</artifactId><version>2.0</version></dependency></dependencies></project>");
    write(&dir.join("gradle-home/caches/modules-2/files-2.1/g/b/2.0/def/b-2.0.jar"), "jar");
}

fn model_json(root: &Path, extra_res: bool) -> String {
    let res = root.join("app/src/main/res");
    let classes = root.join("app/build/classes");
    std::fs::create_dir_all(&classes).unwrap();
    write(&root.join("app/build/R.jar"), "r");
    let mut res_dirs = vec![res.to_string_lossy().to_string()];
    if extra_res {
        let more = root.join("lib/src/main/res");
        res_dirs.push(more.to_string_lossy().to_string());
    }
    json!({
        "module": ":app", "variant": "debug", "namespace": "dev.t", "applicationId": "dev.t.app",
        "resDirs": res_dirs, "classpath": [root.join("app/build/classes").to_string_lossy()], "rJar": root.join("app/build/R.jar").to_string_lossy(),
        "aarRes": [], "activityThemes": {"dev.t.Main": "@style/MainTheme"}, "appTheme": "@style/AppTheme", "projectDeps": [":lib"], "warnings": ["w1"], "minSdk": 24,
    })
    .to_string()
}

fn gradle_script(root: &Path, json_text: &str) {
    let script = format!("#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in -Pccm.out=*) OUT=\"${{a#-Pccm.out=}}\";; esac; done\necho \"$@\" > \"{}/args.txt\"\ncat > \"$OUT\" <<'EOF'\n{json_text}\nEOF\n", root.to_string_lossy());
    exec(&root.join("gradlew"), &script);
}

fn state_of(r: &Rig, file: &str) -> Value {
    r.svc.info(file)["state"].clone()
}

fn model_hash(root: &Path, module: &str) -> String {
    let key = format!("{}\u{0}{module}", root.to_string_lossy());
    Sha1::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..16].to_string()
}

#[test]
fn no_project_means_framework_then_kit_once() {
    let dir = tmp("kit");
    let plain = dir.join("plain/res/layout/a.xml");
    write(&plain, "<A/>");
    let file = plain.to_string_lossy().to_string();
    let bare = rig_with(&dir, false, true, timing());
    assert_eq!(bare.svc.prepare(&file), json!({"key": "framework", "frameworkOnly": true, "builtinLibs": false, "sourceOnly": false}));
    assert!(bare.daemon.calls.lock().unwrap().is_empty());
    assert_eq!(bare.svc.info(&file), json!({"target": null, "reason": "no-gradle", "state": {"state": "none"}, "hasModel": false, "themes": []}));

    let r = rig_with(&tmp("kit2"), true, true, timing());
    let f2 = r.dir.join("plain/res/layout/a.xml");
    write(&f2, "<A/>");
    let file = f2.to_string_lossy().to_string();
    let first = r.svc.prepare(&file);
    assert_eq!((first["frameworkOnly"].clone(), first["builtinLibs"].clone(), first["sourceOnly"].clone()), (json!(true), json!(true), json!(false)));
    assert!(first["key"].as_str().unwrap().starts_with("loose-kit#"));
    assert_eq!(r.svc.prepare(&file)["key"], first["key"]);
    let loads = r.daemon.named("project/load");
    assert_eq!(loads.len(), 1);
    assert_eq!((loads[0]["projectId"].clone(), loads[0]["packageName"].clone(), loads[0]["classpath"].as_array().unwrap().len()), (json!("loose-kit"), json!("k.p"), 1));
    let kinds: Vec<Value> = r.events.lock().unwrap().iter().filter(|e| e["type"] == "loading").map(|e| json!([e["kind"], e["active"]])).collect();
    assert_eq!(kinds, vec![json!(["kit", true]), json!(["kit", false])]);
}

#[test]
fn kit_failure_sticks_until_the_daemon_restarts() {
    let r = rig_with(&tmp("kitfail"), true, true, timing());
    let f = r.dir.join("plain/res/layout/a.xml");
    write(&f, "<A/>");
    let file = f.to_string_lossy().to_string();
    r.daemon.fail.store(true, Ordering::SeqCst);
    assert_eq!(r.svc.prepare(&file)["key"], json!("framework"));
    assert_eq!(r.svc.prepare(&file)["key"], json!("framework"));
    assert_eq!(r.daemon.named("project/load").len(), 1);
    assert!(r.logs.lock().unwrap().iter().any(|l| l.starts_with("error: project load: request-failed: boom")));
    r.daemon.fail.store(false, Ordering::SeqCst);
    r.svc.on_daemon_state("starting");
    assert!(r.svc.prepare(&file)["key"].as_str().unwrap().starts_with("loose-kit#"));
}

#[test]
fn source_model_reads_res_and_cached_libraries_then_prebuilt_classes() {
    let r = rig_with(&tmp("source"), true, true, timing());
    let p = project(&r.dir);
    gradle_cache(&r.dir);
    let first = r.svc.prepare(&p.app_layout);
    assert!(first["key"].as_str().unwrap().starts_with("source:"), "{first}");
    assert_eq!((first["frameworkOnly"].clone(), first["sourceOnly"].clone(), first["builtinLibs"].clone()), (json!(false), json!(true), json!(true)));
    let load = &r.daemon.named("project/load")[0];
    let res: Vec<String> = load["resDirs"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(res, vec![p.root.join("lib/src/main/res").to_string_lossy().to_string(), p.root.join("app/src/main/res").to_string_lossy().to_string()]);
    let cp: Vec<&str> = load["classpath"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(cp.iter().any(|c| c.ends_with("a-1.0.jar")) && cp.iter().any(|c| c.ends_with("b-2.0.jar")) && cp.iter().any(|c| c.ends_with("a.jar")), "{cp:?}");
    assert_eq!((load["namespace"].clone(), load["minSdk"].clone(), load["targetSdk"].clone(), load["appTheme"].clone(), load["activityThemes"].clone()), (json!("dev.t"), json!(24), json!(34), json!("@style/AppTheme"), json!({"dev.t.Main": "@style/MainTheme"})));
    assert!(r.logs.lock().unwrap().iter().any(|l| l.contains("project/load source :app (res only)")));
    assert_eq!(r.svc.info(&p.app_layout)["themes"], json!(["@style/AppTheme", "@style/MainTheme"]));

    write(&p.root.join("app/build/intermediates/compile_and_runtime_not_namespaced_r_class_jar/debug/R.jar"), "r");
    write(&p.root.join("app/build/tmp/kotlin-classes/debug/X.class"), "c");
    let second = r.svc.prepare(&p.app_layout);
    assert_eq!((second["sourceOnly"].clone(), second["builtinLibs"].clone()), (json!(false), json!(true)));
    assert!(second["prebuiltAt"].as_f64().unwrap() > 0.0);
    let load = &r.daemon.named("project/load")[1];
    assert!(load["rJar"].as_str().unwrap().ends_with("debug/R.jar"));
    assert!(load["classpath"].as_array().unwrap()[0].as_str().unwrap().ends_with("kotlin-classes/debug"));
    assert_eq!(r.svc.info(&p.app_layout)["themes"], json!(["@style/AppTheme", "@style/MainTheme"]));
}

#[test]
fn source_model_without_res_dirs_falls_back_to_the_kit() {
    let r = rig_with(&tmp("nores"), true, true, timing());
    let root = r.dir.join("p");
    write(&root.join("settings.gradle.kts"), "include(\":app\")\n");
    write(&root.join("app/build.gradle.kts"), "plugins { id(\"com.android.application\") }\n");
    write(&root.join("app/src/main/AndroidManifest.xml"), "<manifest/>");
    let loose = root.join("loose/x.xml");
    write(&loose, "<LinearLayout/>");
    assert!(r.svc.prepare(&loose.to_string_lossy())["key"].as_str().unwrap().starts_with("loose-kit#"));
}

#[test]
fn refresh_runs_the_same_gradle_command_and_caches_the_model_on_disk() {
    let r = rig_with(&tmp("refresh"), false, true, timing());
    let p = project(&r.dir);
    let text = model_json(&p.root, false);
    gradle_script(&p.root, &text);
    assert_eq!(state_of(&r, &p.app_layout), json!({"state": "none"}));
    let info = r.svc.refresh_file(&p.app_layout);
    assert_eq!(info["target"], json!({"root": p.root.to_string_lossy(), "module": ":app", "name": "demo"}));
    assert_eq!((info["state"]["state"].clone(), info["state"]["resCount"].clone(), info["state"]["classpathCount"].clone(), info["state"]["warnings"].clone()), (json!("ready"), json!(1), json!(1), json!(1)));
    assert_eq!((info["hasModel"].clone(), info["codeChanged"].clone(), info["themes"].clone()), (json!(true), json!(false), json!(["@style/AppTheme", "@style/MainTheme"])));
    let args = std::fs::read_to_string(p.root.join("args.txt")).unwrap();
    let out = r.dir.join(format!("data/models/{}.next.json", model_hash(&p.root, ":app")));
    assert_eq!(args.trim(), format!("-q --init-script {} :app:ccmProjectModel -Pccm.out={} -Pccm.variant=debug -Pccm.compile=true", r.dir.join("init.gradle").to_string_lossy(), out.to_string_lossy()));
    let cache = r.dir.join(format!("data/models/{}.json", model_hash(&p.root, ":app")));
    assert!(cache.exists() && !out.exists());
    assert_eq!(std::fs::read_to_string(&cache).unwrap().trim(), text);
    let states: Vec<Value> = r.events.lock().unwrap().iter().filter(|e| e["type"] == "model").map(|e| e["state"]["state"].clone()).collect();
    assert_eq!(states, vec![json!("running"), json!("ready")]);
    assert!(r.events.lock().unwrap().iter().any(|e| e["type"] == "project-changed" && e["module"] == ":app"));
    assert!(r.logs.lock().unwrap().iter().any(|l| l.contains("project model: gradlew -q --init-script")));

    let prepared = r.svc.prepare(&p.app_layout);
    assert_eq!((prepared["frameworkOnly"].clone(), prepared["builtinLibs"].clone(), prepared["sourceOnly"].clone()), (json!(false), json!(false), json!(false)));
    let load = &r.daemon.named("project/load")[0];
    assert_eq!((load["projectId"].clone(), load["classesChanged"].clone(), load["packageName"].clone(), load["namespace"].clone()), (json!(model_hash(&p.root, ":app")), json!(true), json!("dev.t.app"), json!("dev.t")));

    r.svc.refresh_file(&p.app_layout);
    assert_eq!(r.daemon.named("project/changed"), vec![json!({"kind": "res", "files": []}), json!({"kind": "classes", "files": []})]);
    gradle_script(&p.root, &model_json(&p.root, true));
    r.svc.refresh_file(&p.app_layout);
    let loads = r.daemon.named("project/load");
    assert_eq!((loads.len(), loads[1]["classesChanged"].clone(), loads[1]["resDirs"].as_array().unwrap().len()), (2, json!(false), 2));
}

#[test]
fn a_model_from_a_previous_session_is_read_without_running_gradle() {
    let r = rig_with(&tmp("session"), false, true, timing());
    let p = project(&r.dir);
    let cache = r.dir.join(format!("data/models/{}.json", model_hash(&p.root, ":app")));
    write(&cache, &model_json(&p.root, false));
    let info = r.svc.info(&p.app_layout);
    assert_eq!((info["state"]["state"].clone(), info["hasModel"].clone()), (json!("ready"), json!(true)));
    assert!(info["state"]["at"].as_f64().unwrap() > 1.0e12);
    assert!(!p.root.join("args.txt").exists());
    assert!(r.svc.prepare(&p.app_layout)["key"].as_str().unwrap().contains("#"));
    assert_eq!(r.daemon.named("project/load")[0]["projectId"], json!(model_hash(&p.root, ":app")));
}

#[test]
fn refresh_failures_are_reported_with_codes() {
    let r = rig_with(&tmp("fail"), false, true, Timing { refresh: Duration::from_millis(500), compile: Duration::from_millis(500), ..timing() });
    let p = project(&r.dir);
    let code = |r: &Rig| {
        r.svc.refresh_file(&p.app_layout);
        state_of(r, &p.app_layout)
    };
    exec(&p.root.join("gradlew"), "#!/bin/sh\necho oops-line\nexit 1\n");
    let s = code(&r);
    assert_eq!((s["code"].clone(), s["detail"].as_str().unwrap().contains("oops-line")), (json!("gradle-failed"), true));
    exec(&p.root.join("gradlew"), "#!/bin/sh\nexit 0\n");
    assert_eq!(code(&r)["code"], json!("no-output"));
    gradle_script(&p.root, "{\"namespace\": 5}");
    let s = code(&r);
    assert_eq!((s["code"].clone(), s["detail"].clone()), (json!("bad-model"), json!("project model: namespace missing")));
    exec(&p.root.join("gradlew"), "#!/bin/sh\nsleep 5\n");
    let started = Instant::now();
    assert_eq!(code(&r)["code"], json!("timeout"));
    assert!(started.elapsed() < Duration::from_secs(4));
    std::fs::remove_file(p.root.join("gradlew")).unwrap();
    assert_eq!(code(&r)["code"], json!("gradlew-missing"));
    let no_sdk = rig_with(&tmp("nosdk"), false, false, timing());
    let q = project(&no_sdk.dir);
    no_sdk.svc.refresh_file(&q.app_layout);
    assert_eq!(state_of(&no_sdk, &q.app_layout)["code"], json!("sdk-missing"));
}

#[test]
fn library_layouts_borrow_an_app_and_the_choice_is_remembered() {
    let r = rig("library");
    let p = project(&r.dir);
    let info = r.svc.info(&p.lib_layout);
    assert_eq!(info["target"]["module"], json!(":app"));
    assert_eq!(info["library"], json!({"module": ":lib", "candidates": [":app"], "linked": true}));
    assert_eq!(r.svc.info(&p.app_layout).get("library"), None);
    write(&p.root.join("other/build.gradle.kts"), "plugins { id(\"com.android.application\") }\ndependencies { implementation(projects.lib) }\n");
    write(&p.root.join("settings.gradle.kts"), "include(\":app\", \":lib\", \":other\")\n");
    let info = r.svc.info(&p.lib_layout);
    assert_eq!(info["library"]["candidates"], json!([":app", ":other"]));
    let picked = r.svc.pick_app(&p.lib_layout, ":other");
    assert_eq!(picked["target"]["module"], json!(":other"));
    assert_eq!(r.svc.pick_app(&p.lib_layout, ":nope")["target"]["module"], json!(":other"));
    let prefs = std::fs::read_to_string(r.dir.join("data/models/preferred-apps.json")).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&prefs).unwrap(), json!({p.root.to_string_lossy(): ":other"}));
    let again = rig_with(&r.dir, false, true, timing());
    assert_eq!(again.svc.info(&p.lib_layout)["target"]["module"], json!(":other"));
}

#[test]
fn daemon_restart_drops_the_project_and_reloads_it_when_ready() {
    let r = rig("restart");
    let p = project(&r.dir);
    gradle_script(&p.root, &model_json(&p.root, false));
    r.svc.refresh_file(&p.app_layout);
    r.svc.prepare(&p.app_layout);
    assert_eq!(r.daemon.named("project/load").len(), 1);
    r.svc.on_daemon_state("starting");
    r.svc.on_daemon_state("ready");
    wait("reload", || r.daemon.named("project/load").len() == 2);
    assert!(r.svc.prepare(&p.app_layout)["key"].as_str().is_some());
    assert_eq!(r.daemon.named("project/load").len(), 2);
}

#[test]
fn load_failure_marks_the_model_broken() {
    let r = rig("broken");
    let p = project(&r.dir);
    gradle_script(&p.root, &model_json(&p.root, false));
    r.svc.refresh_file(&p.app_layout);
    r.daemon.fail.store(true, Ordering::SeqCst);
    assert_eq!(r.svc.prepare(&p.app_layout)["key"], json!("framework"));
    let info = r.svc.info(&p.app_layout);
    assert_eq!((info["hasModel"].clone(), info["state"]["code"].clone()), (json!(false), json!("load-failed")));
}

#[test]
fn res_and_code_changes_in_a_loaded_project_reach_the_daemon_and_the_page() {
    let r = rig("watch");
    let p = project(&r.dir);
    gradle_script(&p.root, &model_json(&p.root, false));
    r.svc.refresh_file(&p.app_layout);
    r.svc.prepare(&p.app_layout);
    std::thread::sleep(Duration::from_millis(300));
    write(&p.root.join("app/src/main/res/values/strings.xml"), "<resources/>");
    wait("project/changed", || !r.daemon.named("project/changed").is_empty());
    let changed = r.daemon.named("project/changed");
    assert_eq!(changed[0]["kind"], json!("res"));
    assert!(changed[0]["files"].as_array().unwrap().iter().any(|f| f.as_str().unwrap().ends_with("strings.xml")));
    wait("project-changed event", || r.events.lock().unwrap().iter().filter(|e| e["type"] == "project-changed").count() >= 2);
    assert_eq!(r.svc.info(&p.app_layout)["codeChanged"], json!(false));
    write(&p.root.join("app/src/Thing.kt"), "class Thing");
    wait("code-changed", || r.events.lock().unwrap().iter().any(|e| e["type"] == "code-changed"));
    assert_eq!(r.svc.info(&p.app_layout)["codeChanged"], json!(true));
}

#[test]
fn auto_compile_starts_once_per_root_and_after_build_prefers_the_app() {
    let r = rig("auto");
    let p = project(&r.dir);
    gradle_script(&p.root, &model_json(&p.root, false));
    assert!(r.svc.auto_compile(&p.app_layout));
    assert!(!r.svc.auto_compile(&p.app_layout));
    wait("auto compile model", || state_of(&r, &p.app_layout)["state"] == "ready");
    assert!(std::fs::read_to_string(p.root.join("args.txt")).unwrap().contains("-Pccm.compile=true"));
    assert!(!r.svc.auto_compile(&p.app_layout));
    r.svc.after_build(&p.root.to_string_lossy(), ":app", "release");
    wait("after build", || std::fs::read_to_string(p.root.join("args.txt")).unwrap().contains("-Pccm.variant=release"));
    wait("prefs", || r.dir.join("data/models/preferred-apps.json").exists());
}
