use super::adb::{fail, Adb, TIMING};
use super::ipc::{denied, invalid, is_id, MODULE};
use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok, ok};
use super::toggles::{is_package, parse_launch, sh_quote};
use crate::run::android::jsre::{g, Js};
use crate::run::parse::{install_failure, InstallFailure};
use crate::{arg, js, Desk, DeskError};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

pub const INSTALL_MS: u64 = 120_000;
pub const PAIR_APP: &str = "dev.ccm.orrery.remote/.MainActivity";
pub const PAIR_QR_MAX: usize = 640;
const PATH_MAX: usize = 4096;

pub fn classify_install(out: &str) -> Option<&'static str> {
    let low = out.to_lowercase();
    if out.lines().any(|l| l.trim() == "Success") {
        return None;
    }
    if low.contains("unauthorized") {
        return Some("unauthorized");
    }
    if low.contains("device offline") || low.contains("no devices/emulators found") || (low.contains("device '") && low.contains("not found")) {
        return Some("offline");
    }
    if out.contains("INSTALL_FAILED_USER_RESTRICTED") {
        return Some("restricted");
    }
    if low.contains("signatures do not match") {
        return Some("signature");
    }
    Some(match install_failure(out) {
        Some(InstallFailure::Signature) => "signature",
        Some(InstallFailure::Storage) => "storage",
        Some(InstallFailure::Abi) => "abi",
        Some(InstallFailure::VersionDowngrade) => "downgrade",
        _ => "other",
    })
}

pub fn clean_apk(v: &Value) -> Option<&str> {
    let p = v.as_str().filter(|s| js::len(s) <= PATH_MAX && !s.contains('\0'))?;
    let path = Path::new(p);
    let is_apk = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("apk"));
    (path.is_absolute() && is_apk && path.is_file()).then_some(p)
}

pub fn clean_qr(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && s.len() <= PAIR_QR_MAX && !s.chars().any(char::is_control))
}

pub fn clean_port(v: &Value) -> Option<u16> {
    v.as_u64().filter(|p| (1..=65535).contains(p)).map(|p| p as u16)
}

pub fn parse_version(out: &str) -> Option<(String, Option<i64>)> {
    static NAME: Js = Js::new(r"versionName=(\S+)", "");
    static CODE: Js = Js::new(r"versionCode=(\d+)", "");
    let name = NAME.exec(out).and_then(|m| g(&m, 1).map(String::from))?;
    let code = CODE.exec(out).and_then(|m| g(&m, 1).and_then(|c| c.parse().ok()));
    Some((name, code))
}

pub fn install(adb: &Adb, id: &str, apk: &str) -> Value {
    let Some(r) = adb.run(&["-s", id, "install", "-r", "-t", apk], INSTALL_MS) else { return err("adb-missing") };
    if r.timed_out {
        return err("timeout");
    }
    if r.code.is_none() {
        return fail("other", &r.out);
    }
    match (r.code, classify_install(&r.out)) {
        (Some(0), None) => ok(),
        (_, Some(code)) => fail(code, &r.out),
        (_, None) => fail("other", &r.out),
    }
}

pub fn open_pair(adb: &Adb, id: &str, qr: &str) -> Value {
    let args: Vec<String> = ["am", "start", "-W", "-n", PAIR_APP, "--es", "pairQr"].iter().map(|s| s.to_string()).chain([sh_quote(qr)]).collect();
    let Some(out) = adb.shell(id, &args, Some(TIMING.connect_ms)) else { return err("adb-missing") };
    if out.timed_out {
        return err("timeout");
    }
    let (success, detail) = parse_launch(&out.out);
    if success {
        return ok();
    }
    err_detail("launch-failed", &detail.unwrap_or_default())
}

pub fn reverse(adb: &Adb, id: &str, port: u16, on: bool) -> Value {
    let spec = format!("tcp:{port}");
    let args: Vec<&str> = if on { vec!["-s", id, "reverse", &spec, &spec] } else { vec!["-s", id, "reverse", "--remove", &spec] };
    let Some(r) = adb.run(&args, TIMING.cmd_ms) else { return err("adb-missing") };
    if r.timed_out {
        return err("timeout");
    }
    if r.code == Some(0) {
        return ok();
    }
    fail("reverse-failed", &r.out)
}

pub fn package_version(adb: &Adb, id: &str, package: &str) -> Value {
    let args: Vec<String> = ["dumpsys", "package", package].iter().map(|s| s.to_string()).collect();
    let Some(r) = adb.shell(id, &args, Some(TIMING.connect_ms)) else { return err("adb-missing") };
    if r.timed_out {
        return err("timeout");
    }
    if r.code != Some(0) {
        return fail("dumpsys-failed", &r.out);
    }
    match parse_version(&r.out) {
        Some((name, code)) => json!({"ok": true, "installed": true, "versionName": name, "versionCode": code}),
        None => json!({"ok": true, "installed": false}),
    }
}

fn guarded(m: &DeviceManager, id: &Value, run: impl FnOnce(&Adb, &str) -> Value) -> Value {
    if !is_id(id) {
        return denied();
    }
    let id = id.as_str().unwrap_or("");
    let ready = m.ready_check(id);
    if is_ok(&ready) {
        run(&m.adb, id)
    } else {
        ready
    }
}

pub fn register(desk: &mut Desk, m: &Arc<DeviceManager>) {
    let mm = m.clone();
    desk.on(MODULE, "ccm:dev-install", move |_, args| {
        Ok::<Value, DeskError>(match clean_apk(arg(args, 1)) {
            Some(apk) => guarded(&mm, arg(args, 0), |adb, id| install(adb, id, apk)),
            None if is_id(arg(args, 0)) => invalid(),
            None => denied(),
        })
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:dev-open-pair", move |_, args| {
        Ok::<Value, DeskError>(match clean_qr(arg(args, 1)) {
            Some(qr) => guarded(&mm, arg(args, 0), |adb, id| open_pair(adb, id, qr)),
            None if is_id(arg(args, 0)) => invalid(),
            None => denied(),
        })
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:dev-reverse", move |_, args| {
        Ok::<Value, DeskError>(match (clean_port(arg(args, 1)), arg(args, 2).as_bool()) {
            (Some(port), Some(on)) => guarded(&mm, arg(args, 0), |adb, id| reverse(adb, id, port, on)),
            _ if is_id(arg(args, 0)) => invalid(),
            _ => denied(),
        })
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:dev-pkg-version", move |_, args| {
        Ok::<Value, DeskError>(match arg(args, 1).as_str().filter(|p| is_package(p)) {
            Some(pkg) => guarded(&mm, arg(args, 0), |adb, id| package_version(adb, id, pkg)),
            None if is_id(arg(args, 0)) => invalid(),
            None => denied(),
        })
    });
}
