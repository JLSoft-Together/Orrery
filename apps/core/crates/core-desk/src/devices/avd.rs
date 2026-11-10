use crate::run::android::jsre::{g, Js};
use crate::run::android::pathx;
use crate::run::types::Env;
use serde_json::{json, Value};

pub fn is_avd_name(v: &str) -> bool {
    static RE: Js = Js::new(r"^[A-Za-z0-9_][A-Za-z0-9._-]{0,99}$", "");
    RE.test(v)
}

pub fn is_emulator_serial(id: &str) -> bool {
    static RE: Js = Js::new(r"^emulator-\d{1,5}$", "");
    RE.test(id)
}

pub fn parse_list_avds(out: &str) -> Vec<String> {
    static NOISE: Js = Js::new(r"^(INFO|WARNING|ERROR|PANIC)\b", "");
    let mut seen: Vec<String> = Vec::new();
    for raw in out.split('\n') {
        let line = raw.trim();
        if is_avd_name(line) && !NOISE.test(line) && !seen.iter().any(|s| s == line) {
            seen.push(line.to_string());
        }
    }
    seen
}

pub type Ini = Vec<(String, String)>;

pub fn ini_get<'a>(ini: &'a Ini, key: &str) -> Option<&'a str> {
    ini.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

pub fn parse_ini(text: &str) -> Ini {
    let mut out: Ini = Vec::new();
    for raw in text.split('\n') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        if eq == 0 {
            continue;
        }
        let key = line[..eq].trim();
        if !key.is_empty() && ini_get(&out, key).is_none() {
            out.push((key.to_string(), line[eq + 1..].trim().to_string()));
        }
    }
    out
}

pub fn parse_emu_avd_name(out: &str) -> Option<String> {
    for raw in out.split('\n') {
        let line = raw.trim();
        if line.is_empty() || line == "OK" {
            continue;
        }
        return is_avd_name(line).then(|| line.to_string());
    }
    None
}

pub struct Homes {
    pub avd: String,
    pub user: String,
}

pub fn avd_home(env: &Env, win: bool, home: &str) -> Homes {
    let val = |k: &str| env.get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let user = val("ANDROID_USER_HOME")
        .or_else(|| val("ANDROID_EMULATOR_HOME"))
        .unwrap_or_else(|| match val("ANDROID_SDK_HOME") {
            Some(s) => pathx::join(win, &[&s, ".android"]),
            None => pathx::join(win, &[home, ".android"]),
        });
    let avd = val("ANDROID_AVD_HOME").unwrap_or_else(|| pathx::join(win, &[&user, "avd"]));
    Homes { avd, user }
}

pub fn avd_dir(name: &str, ini: Option<&Ini>, homes: &Homes, win: bool) -> String {
    if let Some(p) = ini.and_then(|i| ini_get(i, "path")).filter(|p| !p.is_empty() && pathx::is_absolute(p, win)) {
        return p.to_string();
    }
    if let Some(rel) = ini.and_then(|i| ini_get(i, "path.rel")).filter(|p| !p.is_empty()) {
        return pathx::join(win, &[&homes.user, rel]);
    }
    pathx::join(win, &[&homes.avd, &format!("{name}.avd")])
}

fn api_of(v: Option<&str>) -> Option<u32> {
    static RE: Js = Js::new(r"android-(\d{1,3})(?!\d)", "");
    let m = RE.exec(&v?.replace('\\', "/"))?;
    g(&m, 1)?.parse().ok()
}

pub fn avd_info(name: &str, ini: Option<&Ini>, config: Option<&Ini>, serial: Option<&str>) -> Value {
    let empty: Ini = Vec::new();
    let c = config.unwrap_or(&empty);
    let api = api_of(ini.and_then(|i| ini_get(i, "target"))).or_else(|| api_of(ini_get(c, "target"))).or_else(|| api_of(ini_get(c, "image.sysdir.1")));
    let display = ini_get(c, "avd.ini.displayname").map(|d| crate::js::slice(d, 120)).filter(|d| !d.is_empty()).unwrap_or_else(|| name.replace('_', " "));
    let opt = |k: &str| ini_get(c, k).filter(|v| !v.is_empty()).map(String::from);
    json!({"name": name, "displayName": display, "api": api, "abi": opt("abi.type"), "device": opt("hw.device.name"), "tag": opt("tag.id"), "serial": serial})
}

pub fn emulator_args(name: &str, mode: &str) -> Vec<String> {
    let mut a = vec!["-avd".to_string(), name.to_string()];
    match mode {
        "cold" => a.push("-no-snapshot-load".into()),
        "wipe" => a.push("-wipe-data".into()),
        _ => {}
    }
    a
}
