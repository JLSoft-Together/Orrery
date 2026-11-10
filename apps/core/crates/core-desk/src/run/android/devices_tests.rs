use super::*;
use crate::run::android::testutil::{script, tmp_dir, wait_for};
use crate::run::proc::{is_alive, process_env};

fn frame(body: &str) -> String {
    format!("{:04x}{body}", body.len())
}

struct Sdk {
    dir: String,
    log: String,
}

fn sdk(adb_cases: &str, emulator_cases: &str) -> Sdk {
    let dir = tmp_dir("adbdev");
    let log = format!("{dir}/calls");
    script(&format!("{dir}/sdk/platform-tools/adb"), &format!("echo \"adb $*\" >> {log}\nF={dir}\ncase \"$*\" in\n{adb_cases}\n*) exit 0;;\nesac"));
    script(&format!("{dir}/sdk/emulator/emulator"), &format!("echo \"emulator $*\" >> {log}\nF={dir}\ncase \"$*\" in\n{emulator_cases}\n*) exit 0;;\nesac"));
    Sdk { dir, log }
}

impl Sdk {
    fn service(&self, timing: DevicesTiming) -> Arc<dyn DeviceService> {
        let settings = RunSettings { android_sdk: Some(format!("{}/sdk", self.dir)), ..RunSettings::new() };
        create_with(DevicesOpts { settings, env: process_env(), win: false, home: self.dir.clone() }, timing, Arc::new(FsEnvDeps))
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log).unwrap_or_default().lines().map(String::from).collect()
    }
}

#[test]
fn streams_track_devices_frames() {
    let first = frame("emulator-5554\tdevice product:sdk model:Pixel_7 device:emu transport_id:1\n");
    let second = frame("R58M\tunauthorized transport_id:2\n");
    let s = sdk(&format!("'track-devices -l') printf '%s' '{first}'; sleep 0.3; printf '%s' '{second}'; exec sleep 30;;"), "");
    let svc = s.service(DevicesTiming::default());
    let seen = Arc::new(Mutex::new(Vec::<Vec<RunDevice>>::new()));
    let c = seen.clone();
    svc.on_change(Arc::new(move |d| c.lock().unwrap().push(d)));
    assert!(wait_for(3_000, || !seen.lock().unwrap().is_empty()));
    let d0 = seen.lock().unwrap()[0][0].clone();
    assert_eq!((d0.id.as_str(), d0.name.as_str(), d0.emulator, d0.state.as_str()), ("emulator-5554", "Pixel 7", true, "ready"));
    assert_eq!(s.calls()[0], "adb track-devices -l");
    assert!(wait_for(3_000, || seen.lock().unwrap().len() >= 2));
    assert_eq!(seen.lock().unwrap()[1].iter().map(|d| d.state.clone()).collect::<Vec<_>>(), vec!["unauthorized"]);
    assert_eq!(svc.list().len(), 1);
    svc.stop(false);
}

#[test]
fn restarts_tracker_then_polls() {
    let s = sdk("'track-devices -l') exit 1;;\n'devices -l') printf 'List of devices attached\\nR58M  device usb:1 model:SM_G991B\\n';;", "");
    let svc = s.service(DevicesTiming { watch: AdbWatchTiming { poll_ms: 5, ..Default::default() }, ..Default::default() });
    let seen = Arc::new(Mutex::new(Vec::<Vec<RunDevice>>::new()));
    let c = seen.clone();
    svc.on_change(Arc::new(move |d| c.lock().unwrap().push(d)));
    assert!(wait_for(5_000, || !seen.lock().unwrap().is_empty()));
    assert_eq!(s.calls().iter().filter(|k| k.contains("track-devices")).count(), 4);
    let d = seen.lock().unwrap()[0][0].clone();
    assert_eq!((d.id.as_str(), d.name.as_str()), ("R58M", "SM G991B"));
    svc.stop(false);
}

#[test]
fn lists_avds_skipping_log_lines() {
    let s = sdk("", "'-list-avds') printf 'INFO    | Storing crashdata in: /tmp/x\\nPixel_7_API_34\\nTablet\\n';;");
    let svc = s.service(DevicesTiming::default());
    let got = svc.emulators();
    assert_eq!(got, vec![RunEmulator { id: "Pixel_7_API_34".into(), name: "Pixel 7 API 34".into(), source: "adb".into() }, RunEmulator { id: "Tablet".into(), name: "Tablet".into(), source: "adb".into() }]);
    svc.stop(false);
}

#[test]
fn launches_avd_and_waits_for_boot() {
    let cases = "'devices -l') n=$(cat $F/polls 2>/dev/null || echo 0); echo $((n+1)) > $F/polls; if [ $n -lt 2 ]; then printf 'List of devices attached\\n'; else printf 'emulator-5556\\tdevice\\n'; fi;;\n\
'-s emulator-5556 emu avd name') printf 'Pixel_7_API_34\\nOK\\n';;\n\
'-s emulator-5556 shell getprop sys.boot_completed') n=$(cat $F/boots 2>/dev/null || echo 0); echo $((n+1)) > $F/boots; if [ $n -gt 1 ]; then echo 1; else echo; fi;;";
    let s = sdk(cases, "'-avd Pixel_7_API_34') echo $$ > $F/emu.pid; exec sleep 5;;");
    let svc = s.service(DevicesTiming { boot_poll_ms: 5, ..Default::default() });
    assert_eq!(svc.launch_emulator("Pixel_7_API_34"), RunResult::ok());
    assert!(wait_for(3_000, || s.calls().contains(&"emulator -avd Pixel_7_API_34".to_string()) && std::path::Path::new(&format!("{}/emu.pid", s.dir)).exists()));
    std::thread::sleep(Duration::from_millis(50));
    let pid: u32 = std::fs::read_to_string(format!("{}/emu.pid", s.dir)).unwrap().trim().parse().unwrap();
    assert!(is_alive(pid));
    crate::run::proc::kill_pid_only(pid);
    assert!(wait_for(2_000, || !is_alive(pid)));
    svc.stop(false);
}

#[test]
fn emulator_missing_without_tools() {
    let dir = tmp_dir("adbdev");
    let settings = RunSettings { android_sdk: Some(format!("{dir}/none")), ..RunSettings::new() };
    let env: Env = [("PATH".to_string(), String::new())].into_iter().collect();
    let svc = create_with(DevicesOpts { settings, env, win: false, home: dir }, DevicesTiming::default(), Arc::new(FsEnvDeps));
    if svc.emulators().is_empty() {
        assert_eq!(svc.launch_emulator("x"), RunResult::err("emulator-missing"));
    }
    svc.stop(true);
}
