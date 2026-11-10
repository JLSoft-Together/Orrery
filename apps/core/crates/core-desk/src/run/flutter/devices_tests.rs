use super::*;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

static N: AtomicUsize = AtomicUsize::new(0);

#[cfg(unix)]
fn fake_sdk(daemon: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let d = std::env::temp_dir().join(format!("ccm-fld-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap();
    let body = format!(
        "#!/bin/sh\nD='{}'\ncase \"$1\" in\n daemon)\n  echo x >> \"$D/daemons\"\n{daemon}\n ;;\n devices) echo 'Waiting for flutter...'; echo '[{{\"id\":\"macos\",\"name\":\"macOS\",\"targetPlatform\":\"darwin\",\"emulator\":false}}]' ;;\n emulators) echo 'Pixel_7 • Pixel 7 • Google • android' ; exit 0 ;;\nesac\n",
        d.display()
    );
    std::fs::write(d.join("bin/flutter"), body).unwrap();
    std::fs::set_permissions(d.join("bin/flutter"), std::fs::Permissions::from_mode(0o755)).unwrap();
    d
}

fn service(sdk: &std::path::Path) -> Arc<dyn DeviceService> {
    let root = sdk.to_string_lossy().to_string();
    let opts = DevicesOpts { settings: RunSettings::new(), env: crate::run::proc::process_env(), win: false, home: root.clone() };
    create_with(opts, DevicesDeps { sdk_for: Some(Arc::new(move || Some(root.clone()))), first_batch_ms: Some(5_000) })
}

fn wait_for(what: &str, f: impl Fn() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(10), "timeout waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

const DAEMON: &str = r#"  while read line; do
    echo "$line" >> "$D/sent"
    id=$(echo "$line" | sed -n 's/^\[{"id":\([0-9]*\).*/\1/p')
    case "$line" in
      *device.getDevices*)
        printf '[{"id":%s,"result":[]}]\n' "$id"
        printf '%s\n' '[{"event":"device.added","params":{"id":"emulator-5554","name":"sdk gphone","platform":"android-x64","emulator":true}}]'
        printf '%s\n' '[{"event":"device.added","params":{"id":"chrome","name":"Chrome","platform":"web-javascript","emulator":false}}]'
        ( sleep 1; printf '%s\n' '[{"event":"device.removed","params":{"id":"chrome"}}]' ) & ;;
      *emulator.getEmulators*) printf '[{"id":%s,"result":[{"id":"Pixel_7","name":"Pixel 7","category":"mobile","platformType":"android"}]}]\n' "$id" ;;
      *emulator.launch*) printf '[{"id":%s,"result":null}]\n' "$id" ;;
      *daemon.shutdown*) exit 0 ;;
    esac
  done"#;

#[cfg(unix)]
#[test]
fn daemon_add_remove_emulators_launch_shutdown() {
    let sdk = fake_sdk(DAEMON);
    let svc = service(&sdk);
    let changes = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let c = changes.clone();
    svc.on_change(Arc::new(move |l| c.lock().unwrap().push(l.iter().map(|d| d.id.clone()).collect())));
    let list = svc.list();
    assert_eq!(list.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), vec!["emulator-5554", "chrome"]);
    assert!(list[0].emulator && list[0].source == "flutter" && list[0].state == "ready");
    let sent = std::fs::read_to_string(sdk.join("sent")).unwrap();
    assert!(sent.starts_with(r#"[{"id":1,"method":"device.enable""#) && sent.contains(r#"[{"id":2,"method":"device.getDevices""#));
    wait_for("removed", || changes.lock().unwrap().last() == Some(&vec!["emulator-5554".to_string()]));
    assert_eq!(svc.list().iter().map(|d| d.id.clone()).collect::<Vec<_>>(), vec!["emulator-5554"]);
    assert_eq!(svc.emulators(), vec![RunEmulator { id: "Pixel_7".into(), name: "Pixel 7".into(), source: "flutter".into() }]);
    assert_eq!(svc.launch_emulator("Pixel_7"), RunResult::ok());
    assert!(std::fs::read_to_string(sdk.join("sent")).unwrap().contains(r#""method":"emulator.launch","params":{"emulatorId":"Pixel_7"}"#));
    svc.stop(false);
    wait_for("shutdown", || std::fs::read_to_string(sdk.join("sent")).unwrap().contains("daemon.shutdown"));
    assert_eq!(std::fs::read_to_string(sdk.join("daemons")).unwrap().lines().count(), 1);
}

#[cfg(unix)]
#[test]
fn dead_daemon_falls_back_to_devices_machine() {
    let sdk = fake_sdk("  exit 1");
    let svc = service(&sdk);
    let list = svc.list();
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].id.as_str(), list[0].platform.as_str()), ("macos", "darwin"));
    assert_eq!(svc.emulators(), vec![RunEmulator { id: "Pixel_7".into(), name: "Pixel 7".into(), source: "flutter".into() }]);
}

#[cfg(unix)]
#[test]
fn caps_daemon_restarts() {
    let sdk = fake_sdk("  exit 1");
    let svc = service(&sdk);
    for _ in 0..6 {
        svc.list();
    }
    assert_eq!(std::fs::read_to_string(sdk.join("daemons")).unwrap().lines().count(), 4);
}

#[test]
fn parsers() {
    assert_eq!(machine_json_array("banner\n[1,2]\n"), Some(vec![json!(1), json!(2)]));
    assert_eq!(machine_json_array("{}"), None);
    assert_eq!(machine_json_array("[{]"), None);
    assert_eq!(flutter_emulator(&json!({"id": "a"})), Some(RunEmulator { id: "a".into(), name: "a".into(), source: "flutter".into() }));
    assert_eq!(flutter_emulator(&json!({"id": ""})), None);
    let text = "2 available emulators:\n\nId • Name • Manufacturer • Platform\n\napple_ios_simulator • iOS Simulator • Apple • ios\nPixel_7 •  • Google • android\r\n";
    let got = parse_emulators_text(text);
    assert_eq!(got.iter().map(|e| (e.id.as_str(), e.name.as_str())).collect::<Vec<_>>(), vec![("apple_ios_simulator", "iOS Simulator"), ("Pixel_7", "Pixel_7")]);
}
