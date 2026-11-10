#![cfg(unix)]

use core_desk::layout::daemon::{DaemonCfg, LayoutDaemon};
use core_desk::layout::download::{kit_dir_for, Bundle, BundleCfg, DlError, Fetcher, HttpBody, KitSpec};
use core_desk::layout::ipc::{LazySvc, Svc, SvcCfg};
use core_desk::layout::jdk::Jdk;
use core_desk::layout::proc::Spawn;
use core_desk::run::settings::SettingsStore;
use core_desk::{Ctx, Desk, DeskError, DeskIo};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("core-desk-layout-{name}-{}", core_desk::util::uuid()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::canonicalize(&d).unwrap()
}

fn fake_script() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/test/fixtures/fake-layout-daemon.cjs").to_string_lossy().to_string()
}

fn have_node() -> bool {
    Command::new("node").arg("-v").stdout(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn node_spawn() -> Spawn {
    let script = fake_script();
    Arc::new(move |_java, _args, cwd| {
        use std::os::unix::process::CommandExt;
        Command::new("node").arg(&script).current_dir(cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).process_group(0).spawn()
    })
}

fn cfg(dir: &Path) -> DaemonCfg {
    let mut c = DaemonCfg::new(
        node_spawn(),
        Arc::new(|| Jdk::Ok { home: "/jdk".into(), version: 21, java: "/jdk/bin/java".into() }),
        "/daemon.jar".into(),
        Arc::new(|| "/layoutlib".into()),
        dir.join("cache").to_string_lossy().to_string(),
        false,
    );
    c.cds = false;
    c
}

fn render_params(xml: &str) -> Value {
    json!({"layoutPath": "/p/res/layout/a.xml", "xmlContent": xml, "device": {"width": 100, "height": 200, "density": 160}, "theme": null, "night": false, "locale": null, "scale": 1.0})
}

fn wait(what: &str, f: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < end, "timeout waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn daemon_renders_cancels_and_recovers() {
    if !have_node() {
        return;
    }
    let dir = tmp("daemon");
    let states = Arc::new(Mutex::new(Vec::<String>::new()));
    let stages = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut c = cfg(&dir);
    let s = states.clone();
    c.on_state = Arc::new(move |st, _| s.lock().unwrap().push(st.to_string()));
    let g = stages.clone();
    c.on_stage = Arc::new(move |st| g.lock().unwrap().push(st.to_string()));
    c.idle = Duration::from_millis(1500);
    let d = LayoutDaemon::new(c);
    assert_eq!(d.state(), "stopped");
    let res = d.render(&render_params("<a/>"), "s1").unwrap();
    assert_eq!(res["width"], json!(100));
    assert!(res["png"].as_str().unwrap().ends_with(".png"));
    assert_eq!(d.state(), "ready");
    assert_eq!(d.api_level(), Some(36));
    assert_eq!(*stages.lock().unwrap(), vec!["jvm", "setup"]);

    let slow = {
        let d = d.clone();
        std::thread::spawn(move || d.render(&render_params("SLOW"), "s2"))
    };
    std::thread::sleep(Duration::from_millis(100));
    let fast = d.render(&render_params("<b/>"), "s2");
    assert!(fast.is_ok());
    assert_eq!(slow.join().unwrap().unwrap_err().code, "cancelled");

    let err = d.render(&render_params("ERR"), "s3").unwrap_err();
    assert_eq!((err.code.as_str(), err.detail.as_deref()), ("render-failed", Some("boom")));

    let crash = d.render(&render_params("CRASH"), "s4").unwrap_err();
    assert_eq!(crash.code, "crashed");
    wait("auto re-warm", || d.state() == "ready" && states.lock().unwrap().iter().filter(|s| *s == "ready").count() >= 2);
    assert!(d.render(&render_params("<c/>"), "s4").is_ok());

    let load = d.request("project/load", json!({"x": 1}), Duration::from_secs(5)).unwrap();
    assert_eq!(load["echo"], json!({"x": 1}));
    assert_eq!(d.request("nope", Value::Null, Duration::from_secs(5)).unwrap_err().code, "request-failed");

    wait("idle shutdown", || d.state() == "stopped");
    d.kill_now();
}

#[test]
fn daemon_times_out_and_gives_up_without_jdk() {
    if !have_node() {
        return;
    }
    let dir = tmp("timeout");
    let mut c = cfg(&dir);
    c.render_timeout = Duration::from_millis(400);
    c.max_restarts = 1;
    let d = LayoutDaemon::new(c);
    assert_eq!(d.render(&render_params("HANG"), "s").unwrap_err().code, "timeout");
    d.kill_now();

    let mut c = cfg(&dir);
    c.find_java = Arc::new(|| Jdk::Bad { code: "jdk-old", detail: Some("11".into()) });
    let d = LayoutDaemon::new(c);
    let e = d.ensure(None).unwrap_err();
    assert_eq!((e.code.as_str(), e.detail.as_deref()), ("jdk-old", Some("11")));
    assert_eq!(d.state(), "stopped");
}

#[test]
fn daemon_stop_is_clean() {
    if !have_node() {
        return;
    }
    let dir = tmp("stop");
    let d = LayoutDaemon::new(cfg(&dir));
    d.ensure(None).unwrap();
    assert_eq!(d.state(), "ready");
    d.stop();
    wait("stopped", || d.state() == "stopped");
}

struct Mem(Vec<u8>);

impl Fetcher for Mem {
    fn get(&self, url: &str) -> Result<HttpBody, DlError> {
        if !url.starts_with("https://github.com/") {
            return Err(DlError("network".into()));
        }
        Ok(HttpBody { status: 200, length: Some(self.0.len() as u64), location: None, body: Box::new(std::io::Cursor::new(self.0.clone())) })
    }
}

fn kit_zip() -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let entries: Vec<(&str, Vec<u8>)> = vec![("layout-daemon.jar", b"jar".to_vec()), ("loose-kit/kit.json", b"{}".to_vec())];
    let (mut out, mut cd) = (Vec::new(), Vec::new());
    for (name, data) in &entries {
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        let comp = e.finish().unwrap();
        let offset = out.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&comp);
        cd.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        cd.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
        cd.extend_from_slice(&[0; 12]);
        cd.extend_from_slice(&offset.to_le_bytes());
        cd.extend_from_slice(name.as_bytes());
    }
    let cd_off = out.len() as u32;
    out.extend_from_slice(&cd);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

#[test]
fn bundle_downloads_the_kit_and_checks_the_hash() {
    let dir = tmp("bundle");
    let ll = dir.join("ll");
    std::fs::create_dir_all(&ll).unwrap();
    std::fs::write(ll.join("layoutlib-16.2.3.jar"), "x").unwrap();
    let zip = kit_zip();
    let sha: String = Sha256::digest(&zip).iter().map(|b| format!("{b:02x}")).collect();
    let kit = |sha: &str| KitSpec { version: "9.9.9".into(), size: zip.len() as u64, sha256: sha.to_string(), url: "https://github.com/o/r/releases/download/v9.9.9/layout-daemon-9.9.9.zip".into() };
    let events = Arc::new(Mutex::new(Vec::<Value>::new()));
    let mk = |sha: &str, data: Vec<u8>| {
        let ev = events.clone();
        Bundle::new(BundleCfg {
            base: dir.join("base").to_string_lossy().to_string(),
            platform: "darwin".into(),
            arch: "arm64".into(),
            override_dir: Some(ll.to_string_lossy().to_string()),
            kit: Some(kit(sha)),
            fetcher: Arc::new(Mem(data)),
            on_status: Arc::new(move |s| ev.lock().unwrap().push(s)),
        })
    };
    let bad = mk(&"0".repeat(64), zip.clone());
    assert_eq!(bad.status(), json!({"state": "missing"}));
    assert!(bad.kit_pending());
    assert_eq!(bad.download_bytes(), zip.len() as u64);
    assert_eq!(bad.download(), json!({"state": "error", "code": "daemon-sha256-mismatch"}));
    let good = mk(&sha, zip.clone());
    let status = good.download();
    assert_eq!(status["state"], json!("ready"), "{status}");
    assert!(good.is_ready());
    assert!(Path::new(&kit_dir_for(&dir.join("base").to_string_lossy(), "9.9.9")).join("loose-kit/kit.json").exists());
    assert!(!dir.join("base/daemon").read_dir().unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("partial")));
    let ev = events.lock().unwrap();
    assert!(ev.iter().any(|e| e["state"] == json!("downloading") && e["pct"] == json!(0)));
    assert_eq!(ev.last().unwrap()["state"], json!("ready"));
    drop(ev);
    let st = core_desk::layout::install::install_status(&good);
    assert_eq!((st["available"].clone(), st["installed"].clone(), st["version"].clone(), st["pinnedVersion"].clone(), st["downloadBytes"].clone()), (json!(true), json!(true), json!("9.9.9"), json!("9.9.9"), json!(0)));
    assert!(st["sizeBytes"].as_u64().unwrap() > 0);
    let stopped = std::cell::Cell::new(false);
    let res = core_desk::layout::install::uninstall(&good, || stopped.set(true));
    assert!(stopped.get());
    assert_eq!((res["ok"].clone(), res["status"]["installed"].clone(), res["status"]["version"].clone(), res["status"]["sizeBytes"].clone()), (json!(true), json!(false), Value::Null, json!(0)));
    assert!(!dir.join("base").exists());
    assert_eq!(good.status(), json!({"state": "missing"}));
    assert_eq!(events.lock().unwrap().last().unwrap()["state"], json!("missing"));
    assert_eq!(core_desk::layout::install::uninstall(&good, || {})["ok"], json!(true));
}

struct Host {
    allowed: PathBuf,
    calls: Mutex<Vec<String>>,
}

impl DeskIo for Host {
    fn emit(&self, _event: &str, _payload: Value) {}

    fn host(&self, _req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        self.calls.lock().unwrap().push(method.to_string());
        Ok(match method {
            "preview.explorerRoot" => match params["root"].as_str() {
                Some(p) if Path::new(p).starts_with(&self.allowed) => json!(p),
                _ => Value::Null,
            },
            "preview.sender" => json!(7),
            "layout.urlFor" => json!({"url": format!("http://127.0.0.1:1/f/{}", Path::new(params["file"].as_str().unwrap()).file_name().unwrap().to_string_lossy())}),
            "layout.confirm" => json!(true),
            _ => return Err(DeskError::Host(format!("no host for {method}"))),
        })
    }
}

fn svc(dir: &Path, with_kit: bool) -> Arc<Svc> {
    let jdk = dir.join("jdk");
    std::fs::create_dir_all(jdk.join("bin")).unwrap();
    std::fs::write(jdk.join("bin/java"), "#!/bin/sh\necho 'openjdk version \"21.0.1\"' >&2\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(jdk.join("bin/java"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("run-settings.json"), json!({"jdk": jdk.to_string_lossy()}).to_string()).unwrap();
    let ll = dir.join("ll");
    std::fs::create_dir_all(&ll).unwrap();
    std::fs::write(ll.join("layoutlib-16.2.3.jar"), "x").unwrap();
    let jar = dir.join("daemon.jar");
    if with_kit {
        std::fs::write(&jar, "j").unwrap();
    }
    let kit = dir.join("kit");
    std::fs::create_dir_all(&kit).unwrap();
    std::fs::write(kit.join("a.jar"), "x").unwrap();
    std::fs::write(kit.join("R.jar"), "x").unwrap();
    std::fs::write(kit.join("kit.json"), json!({"classpath": ["a.jar"], "rJar": "R.jar", "packageName": "k.p", "appTheme": "@style/T", "aarRes": []}).to_string()).unwrap();
    Svc::new(SvcCfg {
        loose_kit_dir: Some(kit.to_string_lossy().to_string()),
        init_script: String::new(),
        model_timing: None,
        data_dir: data.to_string_lossy().to_string(),
        jar: jar.to_string_lossy().to_string(),
        override_dir: Some(ll.to_string_lossy().to_string()),
        kit: None,
        log_file: None,
        settings: SettingsStore::load(&data, false),
        emit: Arc::new(|_| {}),
        spawn: Some(node_spawn()),
        fetcher: None,
        platform: "darwin".into(),
        arch: "arm64".into(),
        tune: Some(Box::new(|c| c.cds = false)),
    })
}

fn call(desk: &Desk, host: &Host, channel: &str, args: Value) -> Value {
    desk.dispatch_with(&Ctx::new(host, 1), channel, args).unwrap()
}

#[test]
fn handlers_gate_render_cache_and_report_status() {
    if !have_node() {
        return;
    }
    let dir = tmp("handlers");
    let layout_dir = dir.join("proj/app/src/main/res/layout");
    std::fs::create_dir_all(&layout_dir).unwrap();
    let file = layout_dir.join("a.xml");
    std::fs::write(&file, "<LinearLayout/>").unwrap();
    let drawable_dir = dir.join("proj/app/src/main/res/drawable");
    std::fs::create_dir_all(&drawable_dir).unwrap();
    std::fs::write(drawable_dir.join("d.xml"), "<shape/>").unwrap();
    let mut desk = Desk::new();
    core_desk::layout::ipc_render::register(&mut desk, LazySvc::ready(svc(&dir, true)));
    let host = Host { allowed: dir.join("proj"), calls: Mutex::new(vec![]) };
    let f = file.to_string_lossy().to_string();
    let dev = json!({"width": 360, "height": 640, "density": 160});

    let st = call(&desk, &host, "ccm:layout-status", Value::Null);
    assert_eq!((st["daemon"].clone(), st["jar"].clone(), st["apiLevel"].clone(), st["jdk"]["ok"].clone()), (json!("stopped"), json!(true), json!(36), json!(true)));

    assert_eq!(call(&desk, &host, "ccm:layout-render", json!([{"file": "/etc/passwd", "device": dev}])), json!({"ok": false, "code": "denied"}));
    assert_eq!(call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": {"width": 1}}])), json!({"ok": false, "code": "bad-params"}));
    assert_eq!(call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev, "text": 5}])), json!({"ok": false, "code": "too-large"}));

    let first = call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev, "slot": 1}]));
    assert_eq!(first["ok"], json!(true));
    assert_eq!((first["cached"].clone(), first["width"].clone(), first["frameworkOnly"].clone(), first["builtinLibs"].clone(), first["prebuiltAt"].clone()), (json!(false), json!(360), json!(true), json!(true), Value::Null));
    assert_eq!(first["viewTree"]["class"], json!("FrameLayout"));
    assert!(first["pngUrl"].as_str().unwrap().starts_with("http://127.0.0.1:1/f/") && first["pngUrl"].as_str().unwrap().contains("?r="));
    let second = call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev, "slot": 1}]));
    assert_eq!(second["cached"], json!(true));
    assert_eq!(second["pngUrl"], first["pngUrl"]);
    let other = call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev, "text": "<Other/>"}]));
    assert_eq!(other["cached"], json!(false));
    let err = call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev, "text": "<ERR/>"}]));
    assert_eq!((err["code"].clone(), err["detail"].clone()), (json!("render-failed"), json!("boom")));

    let d = call(&desk, &host, "ccm:drawable-render", json!([{"file": drawable_dir.join("d.xml").to_string_lossy(), "width": 24, "height": 24, "density": 320, "states": ["pressed", "bogus", "pressed"]}]));
    assert_eq!((d["ok"].clone(), d["kind"].clone(), d["width"].clone(), d["swatches"].clone()), (json!(true), json!("drawable"), json!(48), json!([{"state": "default", "color": "#FF00FF00"}])));
    assert_eq!(call(&desk, &host, "ccm:drawable-render", json!([{"file": f, "width": 24, "height": 24, "density": 320}]))["code"], json!("not-drawable"));

    assert_eq!(call(&desk, &host, "ccm:layout-warmup", json!([{"file": f}])), json!({"ok": true}));
    assert_eq!(call(&desk, &host, "ccm:layout-warmup", json!([{"file": "/etc/hosts"}])), json!({"ok": false, "code": "denied"}));
    assert_eq!(call(&desk, &host, "ccm:layout-download", Value::Null)["state"], json!("ready"));
    let inst = call(&desk, &host, "ccm:layout-install-status", Value::Null);
    assert_eq!((inst["available"].clone(), inst["installed"].clone()), (json!(true), json!(true)));
    assert_eq!(call(&desk, &host, "ccm:layout-uninstall", Value::Null)["ok"], json!(true));

    let req = call(&desk, &host, "desk:layout-request", json!(["project/load", {"a": 1}, 5000]));
    assert_eq!(req["value"]["echo"], json!({"a": 1}));
    let agent = call(&desk, &host, "desk:layout-agent-render", json!([f, {"device": dev, "night": false, "slot": "x"}]));
    assert_eq!((agent["ok"].clone(), agent["frameworkOnly"].clone()), (json!(true), json!(true)));
    assert!(agent["png"].as_str().unwrap().ends_with(".png"));
    call(&desk, &host, "desk:layout-kill", Value::Null);
}

#[test]
fn handlers_report_missing_pieces() {
    if !have_node() {
        return;
    }
    let dir = tmp("missing");
    let mut desk = Desk::new();
    core_desk::layout::ipc_render::register(&mut desk, LazySvc::ready(svc(&dir, false)));
    let layout_dir = dir.join("p/res/layout");
    std::fs::create_dir_all(&layout_dir).unwrap();
    std::fs::write(layout_dir.join("a.xml"), "<A/>").unwrap();
    let host = Host { allowed: dir.join("p"), calls: Mutex::new(vec![]) };
    let f = layout_dir.join("a.xml").to_string_lossy().to_string();
    let dev = json!({"width": 360, "height": 640, "density": 160});
    assert_eq!(call(&desk, &host, "ccm:layout-render", json!([{"file": f, "device": dev}]))["code"], json!("jar-missing"));
    assert_eq!(call(&desk, &host, "ccm:layout-warmup", json!([{"file": f}])), json!({"ok": false, "code": "jar-missing"}));
    assert_eq!(call(&desk, &host, "desk:layout-agent-render", json!([f, {"device": dev, "night": false, "slot": "x"}]))["code"], json!("jar-missing"));
    assert_eq!(call(&desk, &host, "desk:layout-agent-render", json!(["/x/other.xml", {}]))["code"], json!("not-layout"));
}
